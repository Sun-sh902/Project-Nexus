//! Native-owned confirmation (Security §4). The OS dialog is created by Rust from the immutable
//! plan, so a compromised renderer cannot approve, alter or replay a plan: it can only ask Rust to
//! show the dialog. Text comes from the bundled locale catalogs, like the native menu.

use crate::actions::PlanView;
use crate::app::Confirmer;
use std::collections::BTreeMap;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

fn catalog(locale: &str) -> BTreeMap<String, String> {
    let source = if locale == "zh-CN" {
        include_str!("../../src/locales/zh-CN/native.json")
    } else {
        include_str!("../../src/locales/en-US/native.json")
    };
    serde_json::from_str(source).expect("bundled native catalog must be valid")
}

/// Builds the dialog body from plan facts only. Values are already redacted display strings.
pub fn dialog_text(plan: &PlanView, locale: &str) -> (String, String, String, String) {
    let m = catalog(locale);
    let t = |k: &str| m.get(k).cloned().unwrap_or_else(|| k.to_string());
    let param = |k: &str| {
        plan.params
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    };
    let agent = plan
        .agent_id
        .map(|a| crate::adapters::adapter(a).name().to_string())
        .unwrap_or_default();
    let mut lines = vec![format!("{} — {agent}", t(&format!("action.{}", plan.kind)))];
    if let Some(cfg) = &plan.config {
        lines.push(format!("{}: {}", t("label.file"), cfg.location));
        for change in cfg.changes.iter().take(4) {
            lines.push(format!(
                "  {}: {} → {}",
                change.key,
                change.before.clone().unwrap_or_else(|| "∅".into()),
                change.after.clone().unwrap_or_else(|| "∅".into())
            ));
        }
    }
    if let Some(p) = &plan.process {
        lines.push(format!(
            "{}: {} {}",
            t("label.command"),
            p.program,
            p.args.join(" ")
        ));
        if let Some(v) = &p.version {
            lines.push(format!("{}: {v}", t("label.version")));
        }
        if p.runs_package_scripts {
            lines.push(t("note.packageScripts"));
        }
    }
    if let Some(v) = param("model") {
        lines.push(format!("{}: {v}", t("label.model")));
    }
    lines.push(format!(
        "{}: {}",
        t("label.backup"),
        t(&format!("backup.{}", plan.backup))
    ));
    lines.push(format!(
        "{}: {}",
        t("label.rollback"),
        t(&format!("rollback.{}", plan.rollback))
    ));
    lines.push(format!(
        "{}: {}",
        t("label.privileges"),
        t(&format!("privileges.{}", plan.privileges))
    ));
    for warning in &plan.warnings {
        lines.push(format!("• {}", t(warning)));
    }
    (
        t("title"),
        lines.join("\n"),
        t("button.confirm"),
        t("button.cancel"),
    )
}

pub struct NativeConfirmer {
    pub handle: tauri::AppHandle,
}

impl Confirmer for NativeConfirmer {
    fn confirm(&self, plan: &PlanView, locale: &str) -> bool {
        let (title, body, ok, cancel) = dialog_text(plan, locale);
        self.handle
            .dialog()
            .message(body)
            .title(title)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(ok, cancel))
            .blocking_show()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_catalogs_have_identical_keys() {
        let en = super::catalog("en-US");
        let zh = super::catalog("zh-CN");
        assert_eq!(en.keys().collect::<Vec<_>>(), zh.keys().collect::<Vec<_>>());
    }
}
