//! Shared primitives: UTC instants, opaque identifiers, stable error codes and redaction.
//! Nothing here touches the filesystem or processes.

use serde::Serialize;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// UTC instant in whole seconds. Health evaluation takes this explicitly so tests control the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(pub i64);

impl Instant {
    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        Instant(secs as i64)
    }
    pub fn plus_secs(self, secs: i64) -> Self {
        Instant(self.0 + secs)
    }
    /// RFC 3339 / ISO 8601 UTC, e.g. `2026-10-27T02:30:00Z`.
    pub fn to_rfc3339(self) -> String {
        let days = self.0.div_euclid(86_400);
        let rem = self.0.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            (rem % 3600) / 60,
            rem % 60
        )
    }
    #[cfg(test)]
    pub fn parse_rfc3339(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != 20
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
            || b[19] != b'Z'
        {
            return None;
        }
        let n = |s: &str| s.parse::<i64>().ok();
        let (y, m, d) = (n(&text[0..4])?, n(&text[5..7])?, n(&text[8..10])?);
        let (hh, mm, ss) = (n(&text[11..13])?, n(&text[14..16])?, n(&text[17..19])?);
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
            return None;
        }
        Some(Instant(
            days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60 + ss,
        ))
    }
}

impl Serialize for Instant {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_rfc3339())
    }
}

// Howard Hinnant's civil calendar algorithms (public domain).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}
#[cfg(test)]
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 128-bit random identifier, hex encoded, with a short domain prefix (e.g. `plan_…`).
pub fn new_id(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("operating system random source unavailable");
    format!("{prefix}_{}", hex(&bytes))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex(&Sha256::digest(bytes))
}

/// Stable machine-readable error. `code` is a translation key in the frontend (`errors:<code>`);
/// `params` hold only safe, already-redacted values.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusError {
    pub code: &'static str,
    pub params: Vec<(String, String)>,
    pub retryable: bool,
}

impl NexusError {
    pub fn new(code: &'static str) -> Self {
        NexusError {
            code,
            params: Vec::new(),
            retryable: false,
        }
    }
    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }
    pub fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.push((key.to_string(), redact(&value.into())));
        self
    }
}

impl fmt::Display for NexusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)
    }
}
impl std::error::Error for NexusError {}

pub type NexusResult<T> = Result<T, NexusError>;

/// Display-safe identifier (config key): control characters removed, length bounded, never
/// masked, because the exact name is needed to address the entry later.
pub fn clean_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control())
        .take(128)
        .collect()
}

/// Removes terminal control sequences and masks token-shaped substrings. Applied to every
/// untrusted string (process output, config-derived labels) before IPC or persistence.
pub fn redact(input: &str) -> String {
    let mut clean = String::with_capacity(input.len().min(4096));
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip CSI / OSC sequences entirely.
            if chars.peek() == Some(&'[') || chars.peek() == Some(&']') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() || n == '\u{7}' {
                        break;
                    }
                }
            }
            continue;
        }
        if c.is_control() && c != '\n' && c != '\t' {
            continue;
        }
        clean.push(c);
        if clean.len() >= 4096 {
            break;
        }
    }
    clean
        .split_inclusive(|c: char| {
            c.is_whitespace() || c == '"' || c == '\'' || c == '=' || c == ','
        })
        .map(|part| {
            let (word, sep) = split_trailing_sep(part);
            if looks_like_secret(word) {
                format!("[redacted]{sep}")
            } else {
                part.to_string()
            }
        })
        .collect()
}

fn split_trailing_sep(part: &str) -> (&str, &str) {
    match part.char_indices().last() {
        Some((i, c)) if c.is_whitespace() || c == '"' || c == '\'' || c == '=' || c == ',' => {
            (&part[..i], &part[i..])
        }
        _ => (part, ""),
    }
}

fn looks_like_secret(word: &str) -> bool {
    const PREFIXES: [&str; 9] = [
        "sk-",
        "sk_",
        "ghp_",
        "gho_",
        "github_pat_",
        "xox",
        "AIza",
        "ya29.",
        "Bearer",
    ];
    if PREFIXES.iter().any(|p| word.starts_with(p)) && word.len() >= 12 {
        return true;
    }
    // Long high-entropy tokens without path separators.
    word.len() >= 32
        && !word.contains('/')
        && !word.contains('\\')
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && word.chars().any(|c| c.is_ascii_digit())
        && word.chars().any(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_round_trips() {
        for text in [
            "1970-01-01T00:00:00Z",
            "2026-10-27T02:30:59Z",
            "2000-02-29T23:59:59Z",
        ] {
            assert_eq!(Instant::parse_rfc3339(text).unwrap().to_rfc3339(), text);
        }
        assert!(Instant::parse_rfc3339("2026-13-01T00:00:00Z").is_none());
        assert!(Instant::parse_rfc3339("not a date").is_none());
    }

    #[test]
    fn ids_are_unique_and_prefixed() {
        let a = new_id("plan");
        assert!(a.starts_with("plan_") && a.len() == 37);
        assert_ne!(a, new_id("plan"));
    }

    #[test]
    fn redaction_masks_tokens_and_control_sequences() {
        let canary = "sk-ant-api03-CANARYCANARYCANARY1234567890";
        let out = redact(&format!("\u{1b}[31merror\u{1b}[0m key={canary} done\u{7}"));
        assert!(!out.contains("CANARY"), "{out}");
        assert!(!out.contains('\u{1b}'));
        assert!(out.contains("error") && out.contains("[redacted]"));
        let path = "/Users/me/.npm-global/lib/node_modules/@google/gemini-cli/package.json";
        assert_eq!(redact(path), path);
    }

    #[test]
    fn error_params_are_redacted() {
        let e = NexusError::new("config.parse_failed")
            .with("detail", "token ghp_abcdefghijklmnopqrstuv1234");
        assert!(!e.params[0].1.contains("ghp_abc"));
    }
}
