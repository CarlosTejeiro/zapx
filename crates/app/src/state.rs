/// Shared application state injected into Tauri commands via [`tauri::State`].
///
/// Fields are added per-block as core crates are wired up.
#[derive(Default)]
pub struct AppState {}
