//! Configuration edit planning. A `ConfigEdit` is a fully computed, immutable proposal: exact
//! before/after bytes bound to the fingerprint that was read. Applying it is the job of
//! `transaction`, which re-verifies the fingerprint, backs up, writes atomically and re-reads.

pub mod jsonc;
pub mod transaction;

use crate::core::{redact, sha256_hex, NexusError, NexusResult};
use crate::discovery::ReadConfig;
use crate::domain::{ConfigFormat, ObsStatus};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLine {
    /// Dotted key path, e.g. `model.name` or `mcp_servers.docs.enabled`.
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: char,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigEdit {
    pub target_id: String,
    pub location: String,
    pub format: ConfigFormat,
    pub creates_file: bool,
    pub before_fingerprint: Option<String>,
    pub after_fingerprint: String,
    pub changes: Vec<ChangeLine>,
    pub diff: Vec<DiffLine>,
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub after: Vec<u8>,
}

impl ConfigEdit {
    pub fn new(read: &ReadConfig, after: String, changes: Vec<ChangeLine>) -> NexusResult<Self> {
        let before_text = read.text.clone();
        if before_text.as_deref() == Some(after.as_str()) {
            return Err(NexusError::new("config.no_change"));
        }
        Ok(ConfigEdit {
            target_id: read.source.id.clone(),
            location: read.source.location.clone(),
            format: read.source.format,
            creates_file: read.source.status == ObsStatus::NotFound,
            before_fingerprint: read.source.fingerprint.clone(),
            after_fingerprint: sha256_hex(after.as_bytes()),
            changes,
            diff: line_diff(before_text.as_deref().unwrap_or(""), &after),
            path: read.source.path.clone(),
            after: after.into_bytes(),
        })
    }
}

/// Changed lines only (common prefix/suffix trimmed), redacted, capped at 40 lines. Edits are
/// localized, so this is the exact region that changes; unrelated content is never shown.
pub fn line_diff(before: &str, after: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..].iter().rev().zip(b[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
    let mut out = Vec::new();
    for line in &a[prefix..a.len() - suffix] {
        out.push(DiffLine { kind: '-', text: redact(line) });
    }
    for line in &b[prefix..b.len() - suffix] {
        out.push(DiffLine { kind: '+', text: redact(line) });
    }
    out.truncate(40);
    out
}

/// Precondition shared by all edits: the source must be readable and parseable. A malformed file
/// is never minimally merged (Architecture §8); only a separately confirmed restore may replace it.
pub fn require_editable(read: &ReadConfig) -> NexusResult<()> {
    match read.source.status {
        ObsStatus::Observed | ObsStatus::NotFound => {}
        ObsStatus::ParseFailed => return Err(NexusError::new("config.malformed_read_only").with("location", &read.source.location)),
        ObsStatus::AccessDenied => return Err(NexusError::new("fs.access_denied").with("location", &read.source.location)),
        _ => return Err(NexusError::new("config.unsafe_path").with("location", &read.source.location)),
    }
    if read.source.via_symlink {
        return Err(NexusError::new("config.unsafe_symlink").with("location", &read.source.location));
    }
    Ok(())
}

fn json_dialect(format: ConfigFormat) -> jsonc::Dialect {
    if format == ConfigFormat::Json { jsonc::Dialect::Json } else { jsonc::Dialect::Jsonc }
}

fn display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Plans a JSON/JSONC value change at `path`. A missing file starts from `{}`.
pub fn plan_json_set(read: &ReadConfig, path: &[&str], value: serde_json::Value) -> NexusResult<ConfigEdit> {
    require_editable(read)?;
    let text = read.text.clone().unwrap_or_else(|| "{}\n".to_string());
    let doc = jsonc::Document::parse(&text, json_dialect(read.source.format))?;
    let before = doc.root.get(path).map(|n| display(&n.to_value()));
    let out = doc.set(path, &value)?;
    // Post-condition on the planned bytes: the result parses and only `path` changed.
    let after_doc = jsonc::Document::parse(&out, json_dialect(read.source.format))?;
    verify_only_changed(&doc.value(), &after_doc.value(), path)?;
    ConfigEdit::new(read, out, vec![ChangeLine { key: path.join("."), before, after: Some(display(&value)) }])
}

pub fn plan_json_remove(read: &ReadConfig, path: &[&str]) -> NexusResult<ConfigEdit> {
    require_editable(read)?;
    let text = read.text.clone().ok_or_else(|| NexusError::new("config.entry_missing"))?;
    let doc = jsonc::Document::parse(&text, json_dialect(read.source.format))?;
    let before = doc.root.get(path).map(|n| display(&n.to_value()));
    let out = doc.remove(path)?;
    let after_doc = jsonc::Document::parse(&out, json_dialect(read.source.format))?;
    verify_only_changed(&doc.value(), &after_doc.value(), path)?;
    ConfigEdit::new(read, out, vec![ChangeLine { key: path.join("."), before, after: None }])
}

/// Every value outside `path` must be identical before and after an edit.
pub fn verify_only_changed(before: &serde_json::Value, after: &serde_json::Value, path: &[&str]) -> NexusResult<()> {
    fn strip(v: &serde_json::Value, path: &[&str]) -> serde_json::Value {
        let mut v = v.clone();
        if let Some((last, parents)) = path.split_last() {
            let mut cur = &mut v;
            for p in parents {
                match cur.get_mut(*p) {
                    Some(next) => cur = next,
                    None => return v,
                }
            }
            if let Some(obj) = cur.as_object_mut() {
                obj.remove(*last);
                if obj.is_empty() && !parents.is_empty() {
                    // An intermediate object created by the edit is part of the change.
                }
            }
        }
        v
    }
    let mut a = strip(before, path);
    let mut b = strip(after, path);
    // Intermediate objects that the edit created (and that are now empty after stripping).
    prune_empty(&mut a, path);
    prune_empty(&mut b, path);
    if a != b {
        return Err(NexusError::new("config.unexpected_side_effect"));
    }
    Ok(())
}

fn prune_empty(v: &mut serde_json::Value, path: &[&str]) {
    for depth in (1..path.len()).rev() {
        let pointer: String = path[..depth - 1].iter().map(|p| format!("/{}", p.replace('~', "~0").replace('/', "~1"))).collect();
        let key = path[depth - 1];
        if let Some(obj) = v.pointer_mut(&pointer).and_then(|p| p.as_object_mut()) {
            if obj.get(key).and_then(|x| x.as_object()).map(|o| o.is_empty()).unwrap_or(false) {
                obj.remove(key);
            }
        }
    }
}

/// TOML edit through `toml_edit`, which preserves comments, ordering and formatting.
pub fn plan_toml_edit(read: &ReadConfig, key: &str, mutate: impl FnOnce(&mut toml_edit::DocumentMut) -> NexusResult<(Option<String>, Option<String>)>) -> NexusResult<ConfigEdit> {
    require_editable(read)?;
    let text = read.text.clone().unwrap_or_default();
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|_| NexusError::new("config.malformed_read_only"))?;
    let (before, after) = mutate(&mut doc)?;
    let out = doc.to_string();
    out.parse::<toml_edit::DocumentMut>().map_err(|_| NexusError::new("config.unexpected_side_effect"))?;
    ConfigEdit::new(read, out, vec![ChangeLine { key: key.to_string(), before, after }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ConfigScope, ConfigSource};

    pub fn fake_read(text: Option<&str>, format: ConfigFormat) -> ReadConfig {
        ReadConfig {
            source: ConfigSource {
                id: "t".into(),
                scope: ConfigScope::User,
                format,
                location: "~/t".into(),
                status: if text.is_some() { ObsStatus::Observed } else { ObsStatus::NotFound },
                fingerprint: text.map(|t| sha256_hex(t.as_bytes())),
                via_symlink: false,
                tolerant: false,
                path: PathBuf::from("/nonexistent/t"),
            },
            text: text.map(String::from),
        }
    }

    #[test]
    fn json_set_reports_change_and_diff() {
        let read = fake_read(Some("{\n  \"theme\": \"dark\",\n  \"model\": \"sonnet\"\n}\n"), ConfigFormat::Json);
        let edit = plan_json_set(&read, &["model"], serde_json::json!("opus")).unwrap();
        assert_eq!(edit.changes[0].before.as_deref(), Some("sonnet"));
        assert_eq!(edit.diff.len(), 2);
        assert!(String::from_utf8(edit.after.clone()).unwrap().contains("\"theme\": \"dark\""));
        assert_eq!(plan_json_set(&read, &["model"], serde_json::json!("sonnet")).unwrap_err().code, "config.no_change");
    }

    #[test]
    fn malformed_and_symlinked_sources_are_read_only() {
        let mut read = fake_read(Some("{ broken"), ConfigFormat::Json);
        read.source.status = ObsStatus::ParseFailed;
        assert_eq!(plan_json_set(&read, &["model"], serde_json::json!("x")).unwrap_err().code, "config.malformed_read_only");
        let mut read = fake_read(Some("{}"), ConfigFormat::Json);
        read.source.via_symlink = true;
        assert_eq!(plan_json_set(&read, &["model"], serde_json::json!("x")).unwrap_err().code, "config.unsafe_symlink");
    }

    #[test]
    fn missing_file_is_created_from_empty_object() {
        let read = fake_read(None, ConfigFormat::Json);
        let edit = plan_json_set(&read, &["model", "name"], serde_json::json!("g")).unwrap();
        assert!(edit.creates_file && edit.before_fingerprint.is_none());
    }

    #[test]
    fn toml_edit_preserves_comments() {
        let read = fake_read(Some("# mine\nmodel = \"a\" # trailing\n\n[tui]\nx = 1\n"), ConfigFormat::Toml);
        let edit = plan_toml_edit(&read, "model", |doc| {
            let before = doc.get("model").and_then(|v| v.as_str()).map(String::from);
            doc["model"] = toml_edit::value("b");
            Ok((before, Some("b".into())))
        })
        .unwrap();
        let out = String::from_utf8(edit.after).unwrap();
        assert!(out.contains("# mine") && out.contains("[tui]\nx = 1"));
        assert!(out.contains("model = \"b\""));
    }

    #[test]
    fn side_effect_detection() {
        let a = serde_json::json!({"a": 1, "b": {"c": 2}});
        let b = serde_json::json!({"a": 1, "b": {"c": 3}});
        assert!(verify_only_changed(&a, &b, &["b", "c"]).is_ok());
        assert!(verify_only_changed(&a, &b, &["a"]).is_err());
        let created = serde_json::json!({"a": 1, "model": {"name": "x"}});
        assert!(verify_only_changed(&serde_json::json!({"a": 1}), &created, &["model", "name"]).is_ok());
    }
}
