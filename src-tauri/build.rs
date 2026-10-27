fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["app_metadata", "apply_app_preferences"]),
    ))
    .expect("Tauri build configuration must be valid");
}
