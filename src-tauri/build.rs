fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_metadata",
            "apply_app_preferences",
            "nexus_overview",
            "nexus_agent_history",
            "nexus_models",
            "nexus_create_provider",
            "nexus_set_provider_secret",
            "nexus_delete_provider",
            "nexus_create_model",
            "nexus_delete_model",
            "nexus_save_profile",
            "nexus_set_assignment",
            "nexus_prepare_action",
            "nexus_confirm_plan",
            "nexus_discard_plan",
            "nexus_launch_agent",
        ]),
    ))
    .expect("Tauri build configuration must be valid");
}
