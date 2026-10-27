//! The only process-spawning primitive in Nexus. It takes an absolute, pre-validated executable
//! and a typed argv; there is no shell, no string command, and no inherited environment.
//! Callers are the reviewed version probes and the compiled lifecycle recipes.

use crate::core::{redact, NexusError, NexusResult};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant as StdInstant};

#[derive(Clone, Debug)]
pub struct ProcessSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Complete environment for the child. Nothing is inherited.
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub max_output: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitKind {
    Exited(i32),
    Signalled,
    TimedOut,
}

#[derive(Clone, Debug)]
pub struct ProcessOutcome {
    pub exit: ExitKind,
    /// Redacted and bounded. Never persisted or logged verbatim.
    pub stdout: String,
    pub stderr: String,
    /// Output exceeded the bound and was cut to its tail.
    #[allow(dead_code)]
    pub truncated: bool,
}

impl ProcessOutcome {
    pub fn success(&self) -> bool {
        self.exit == ExitKind::Exited(0)
    }
}

pub fn validate_spec(spec: &ProcessSpec) -> NexusResult<()> {
    if !spec.program.is_absolute() || !super::fs::is_executable_file(&spec.program) {
        return Err(NexusError::new("process.program_invalid"));
    }
    if !spec.cwd.is_absolute() || !spec.cwd.is_dir() {
        return Err(NexusError::new("process.cwd_invalid"));
    }
    for arg in &spec.args {
        if arg.contains('\0') || arg.contains('\n') || arg.contains('\r') {
            return Err(NexusError::new("process.argument_invalid"));
        }
    }
    for (k, v) in &spec.env {
        if k.is_empty() || k.contains('=') || k.contains('\0') || v.contains('\0') {
            return Err(NexusError::new("process.argument_invalid"));
        }
    }
    Ok(())
}

pub fn run(spec: &ProcessSpec) -> NexusResult<ProcessOutcome> {
    validate_spec(spec)?;
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .env_clear()
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    // Own process group so a timeout can terminate package-manager grandchildren too.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|_| NexusError::new("process.spawn_failed"))?;
    let cap = spec.max_output;
    let buffers: [Arc<Mutex<(Vec<u8>, bool)>>; 2] = [Arc::default(), Arc::default()];
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let streams: [Option<Box<dyn Read + Send>>; 2] = [child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>)];
    let mut readers = 0;
    for (stream, buffer) in streams.into_iter().zip(buffers.iter()) {
        if let Some(mut stream) = stream {
            readers += 1;
            let buffer = buffer.clone();
            let done = done_tx.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let mut guard = buffer.lock().unwrap_or_else(|p| p.into_inner());
                            // Keep the tail: errors are usually at the end of installer output.
                            guard.0.extend_from_slice(&buf[..n]);
                            if guard.0.len() > cap {
                                let excess = guard.0.len() - cap;
                                guard.0.drain(..excess);
                                guard.1 = true;
                            }
                        }
                    }
                }
                let _ = done.send(());
            });
        }
    }
    drop(done_tx);
    let started = StdInstant::now();
    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().map(ExitKind::Exited).unwrap_or(ExitKind::Signalled),
            Ok(None) if started.elapsed() >= spec.timeout => {
                kill_tree(&mut child);
                let _ = child.wait();
                break ExitKind::TimedOut;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => {
                kill_tree(&mut child);
                return Err(NexusError::new("process.wait_failed"));
            }
        }
    };
    // Readers end when the pipes close; bound the wait in case a detached grandchild holds them.
    let deadline = StdInstant::now() + Duration::from_secs(2);
    for _ in 0..readers {
        if done_rx.recv_timeout(deadline.saturating_duration_since(StdInstant::now())).is_err() {
            break;
        }
    }
    let take = |b: &Arc<Mutex<(Vec<u8>, bool)>>| {
        let g = b.lock().unwrap_or_else(|p| p.into_inner());
        (g.0.clone(), g.1)
    };
    let ((stdout, t1), (stderr, t2)) = (take(&buffers[0]), take(&buffers[1]));
    let truncated = t1 || t2;
    Ok(ProcessOutcome {
        exit,
        stdout: redact(&String::from_utf8_lossy(&stdout)),
        stderr: redact(&String::from_utf8_lossy(&stderr)),
        truncated,
    })
}

fn kill_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
}

/// Minimal child environment: explicit PATH, HOME and a neutral locale. Agent-specific
/// variables are never forwarded implicitly.
pub fn base_env(home: &std::path::Path, path_dirs: &[PathBuf]) -> Vec<(String, String)> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let path = path_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(sep);
    #[allow(unused_mut)]
    let mut env = vec![
        ("PATH".to_string(), path),
        (if cfg!(windows) { "USERPROFILE" } else { "HOME" }.to_string(), home.display().to_string()),
        ("NO_COLOR".to_string(), "1".to_string()),
        ("TERM".to_string(), "dumb".to_string()),
        ("LANG".to_string(), "en_US.UTF-8".to_string()),
    ];
    #[cfg(windows)]
    for name in ["SystemRoot", "TEMP", "TMP", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(name) {
            env.push((name.to_string(), v));
        }
    }
    env
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    fn spec(program: PathBuf, args: &[&str], dir: &std::path::Path) -> ProcessSpec {
        ProcessSpec {
            program,
            args: args.iter().map(|s| s.to_string()).collect(),
            env: base_env(dir, &[PathBuf::from("/usr/bin"), PathBuf::from("/bin")]),
            cwd: dir.to_path_buf(),
            timeout: Duration::from_secs(5),
            max_output: 64,
        }
    }

    #[test]
    fn arguments_are_not_shell_interpreted() {
        let dir = tempfile::tempdir().unwrap();
        let echo = script(dir.path(), "echoargs", "for a in \"$@\"; do printf '[%s]' \"$a\"; done");
        let marker = dir.path().join("pwned");
        let hostile = format!("x; touch {}", marker.display());
        let out = run(&spec(echo, &[&hostile, "$(id)", "--flag"], dir.path())).unwrap();
        assert!(out.success());
        assert!(!marker.exists());
        assert!(out.stdout.contains("[$(id)]"));
    }

    #[test]
    fn environment_is_not_inherited() {
        std::env::set_var("NEXUS_TEST_SECRET_CANARY", "leak");
        let dir = tempfile::tempdir().unwrap();
        let p = script(dir.path(), "env", "env");
        let out = run(&spec(p, &[], dir.path())).unwrap();
        assert!(!out.stdout.contains("NEXUS_TEST_SECRET_CANARY"));
    }

    #[test]
    fn timeouts_kill_and_output_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let p = script(dir.path(), "slow", "i=0; while [ $i -lt 200 ]; do echo line$i; i=$((i+1)); done; sleep 30");
        let mut s = spec(p, &[], dir.path());
        s.timeout = Duration::from_millis(400);
        let out = run(&s).unwrap();
        assert_eq!(out.exit, ExitKind::TimedOut);
        assert!(out.stdout.len() <= 64 && out.truncated);
    }

    #[test]
    fn invalid_programs_and_arguments_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(run(&spec(PathBuf::from("sh"), &[], dir.path())).unwrap_err().code, "process.program_invalid");
        let p = script(dir.path(), "ok", "true");
        assert_eq!(run(&spec(p, &["a\nb"], dir.path())).unwrap_err().code, "process.argument_invalid");
        let non_exec = dir.path().join("plain");
        std::fs::write(&non_exec, "x").unwrap();
        assert_eq!(run(&spec(non_exec, &[], dir.path())).unwrap_err().code, "process.program_invalid");
    }
}
