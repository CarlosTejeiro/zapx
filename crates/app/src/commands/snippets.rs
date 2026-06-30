//! Snippet CRUD commands. Snippets are short named text blocks the frontend
//! can dispatch to the focused session via the existing `send_input` command.
//!
//! A snippet may be **global** (visible in every session) or scoped to a
//! single **platform** key (matching `core_hints::Platform::as_str`).
//! The bottom button bar filters by the focused session's platform; the
//! manage dialog lists everything.

use serde::{Deserialize, Serialize};
use tauri::State;

use core_persistence::{Database, Snippet};

use crate::error::AppError;
use crate::state::AppState;

/// All snippets — used by the manage dialog so the user can edit / delete
/// regardless of which session is focused.
#[tauri::command]
pub async fn list_snippets(state: State<'_, AppState>) -> Result<Vec<Snippet>, AppError> {
    state
        .db
        .list_snippets()
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Subset visible to a session of `platform` — globals (NULL) plus matches.
/// On first call for a never-seen platform we seed a curated default set so
/// the bar isn't empty out of the box.
#[tauri::command]
pub async fn list_snippets_for_platform(
    state: State<'_, AppState>,
    platform: String,
) -> Result<Vec<Snippet>, AppError> {
    // Skip seeding when the session has no recognised platform (Generic).
    if !platform.is_empty() && platform != "generic" {
        let already = state
            .db
            .snippets_exist_for_platform(&platform)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if !already {
            for (name, content) in default_snippets_for(&platform) {
                let _ = state
                    .db
                    .create_snippet(name, content, Some(&platform), None);
            }
        }
    }
    state
        .db
        .list_snippets_for_platform(&platform)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn create_snippet(
    state: State<'_, AppState>,
    name: String,
    content: String,
    platform: Option<String>,
    color: Option<String>,
) -> Result<i64, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("snippet name is required".into()));
    }
    state
        .db
        .create_snippet(name.trim(), &content, platform.as_deref(), color.as_deref())
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn set_snippets_order(
    state: State<'_, AppState>,
    ordered_ids: Vec<i64>,
) -> Result<(), AppError> {
    state
        .db
        .set_snippets_order(&ordered_ids)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn update_snippet(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    content: String,
    platform: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("snippet name is required".into()));
    }
    state
        .db
        .update_snippet(
            id,
            name.trim(),
            &content,
            platform.as_deref(),
            color.as_deref(),
        )
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn delete_snippet(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    state
        .db
        .delete_snippet(id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Set (`Some`) or clear (`None`) a snippet's macro steps (expect/send/wait
/// JSON). When set, the snippet runs as a macro on the focused session instead
/// of sending its plain `content`.
#[tauri::command]
pub async fn set_snippet_steps(
    state: State<'_, AppState>,
    id: i64,
    steps_json: Option<String>,
) -> Result<(), AppError> {
    state
        .db
        .set_snippet_steps(id, steps_json.as_deref())
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Set (`Some`) or clear (`None`) a snippet's folder (the sidebar Macros
/// library groups by this). Separate from `update_snippet`, mirroring
/// [`set_snippet_steps`].
#[tauri::command]
pub async fn set_snippet_folder(
    state: State<'_, AppState>,
    id: i64,
    folder: Option<String>,
) -> Result<(), AppError> {
    state
        .db
        .set_snippet_folder(id, folder.as_deref())
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Top recently-typed commands for the focused saved session. Drives the
/// "Recents" zone of the bottom button bar. Trailing newline is appended so
/// dispatching the snippet executes the command exactly like the original
/// keystroke did.
#[tauri::command]
pub async fn list_recent_command_snippets(
    state: State<'_, AppState>,
    saved_session_id: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<RecentCommand>, AppError> {
    let entries = state
        .db
        .recent_commands(saved_session_id, limit.unwrap_or(5))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(entries
        .into_iter()
        .map(|e| RecentCommand {
            text: e.command,
            freq: e.freq,
        })
        .collect())
}

/// Frontend payload for [`list_recent_command_snippets`]. Smaller than
/// `Snippet` because a recent doesn't have an id, sort order, etc.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecentCommand {
    pub text: String,
    pub freq: i64,
}

// ── Macro export / import (versioned JSON) ───────────────────────────────────

const MACRO_EXPORT_VERSION: u32 = 1;

/// A single macro in the export envelope. Only the macro-relevant fields are
/// carried; ids, ordering and timestamps are local and re-derived on import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedMacro {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub folder: Option<String>,
    pub steps_json: String,
}

/// The versioned macro export envelope written to / read from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroExportFile {
    /// Schema version — bump on breaking changes; import rejects newer files.
    pub zapx_macro_export: u32,
    pub macros: Vec<ExportedMacro>,
}

/// Summary returned to the frontend after importing a macro file.
#[derive(Debug, Default, Serialize)]
pub struct MacroImportSummary {
    pub added: usize,
    pub skipped: usize,
}

/// Snapshot the requested macros (snippets with a non-null `steps_json`) into a
/// versioned envelope. `ids = None` exports every macro. Pure DB → struct.
fn build_macro_export(db: &Database, ids: Option<&[i64]>) -> Result<MacroExportFile, AppError> {
    let all = db
        .list_snippets()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let macros = all
        .into_iter()
        .filter(|s| s.steps_json.is_some())
        .filter(|s| ids.map(|ids| ids.contains(&s.id)).unwrap_or(true))
        .map(|s| ExportedMacro {
            name: s.name,
            color: s.color,
            folder: s.folder,
            // Filtered above to Some, but stay defensive.
            steps_json: s.steps_json.unwrap_or_else(|| "[]".to_string()),
        })
        .collect();
    Ok(MacroExportFile {
        zapx_macro_export: MACRO_EXPORT_VERSION,
        macros,
    })
}

/// Recreate exported macros, skipping any whose name already exists (idempotent,
/// like session import). Returns added/skipped counts.
fn apply_macro_import(
    db: &Database,
    file: &MacroExportFile,
) -> Result<MacroImportSummary, AppError> {
    if file.zapx_macro_export > MACRO_EXPORT_VERSION {
        return Err(AppError::Internal(format!(
            "file is from a newer macro export version (v{}, supported v{MACRO_EXPORT_VERSION})",
            file.zapx_macro_export
        )));
    }
    let internal = |e: core_persistence::Error| AppError::Internal(e.to_string());
    let existing = db.list_snippets().map_err(internal)?;
    let mut summary = MacroImportSummary::default();
    for m in &file.macros {
        if existing.iter().any(|e| e.name == m.name) {
            summary.skipped += 1;
            continue;
        }
        let id = db
            .create_snippet(&m.name, "", None, m.color.as_deref())
            .map_err(internal)?;
        db.set_snippet_steps(id, Some(&m.steps_json))
            .map_err(internal)?;
        if let Some(folder) = &m.folder {
            db.set_snippet_folder(id, Some(folder)).map_err(internal)?;
        }
        summary.added += 1;
    }
    Ok(summary)
}

/// Export macros to a JSON file. `ids = None` exports all macros.
#[tauri::command]
pub async fn export_macros(
    state: State<'_, AppState>,
    path: String,
    ids: Option<Vec<i64>>,
) -> Result<(), AppError> {
    let file = build_macro_export(&state.db, ids.as_deref())?;
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::write(&path, json).map_err(|e| AppError::Internal(format!("{path}: {e}")))
}

/// Import macros from a JSON file (skip-by-name idempotent).
#[tauri::command]
pub async fn import_macros(
    state: State<'_, AppState>,
    path: String,
) -> Result<MacroImportSummary, AppError> {
    let raw = std::fs::read_to_string(&path).map_err(|e| AppError::Internal(e.to_string()))?;
    let file: MacroExportFile = serde_json::from_str(&raw)
        .map_err(|e| AppError::Internal(format!("not a valid macro export: {e}")))?;
    apply_macro_import(&state.db, &file)
}

/// Curated default snippet set per platform — what most users would benefit
/// from on day one. Seeded the first time the platform is observed.
///
/// Returns `(name, content_with_trailing_newline)` pairs. Empty `Vec` for
/// platforms we don't have curated defaults for.
fn default_snippets_for(platform: &str) -> Vec<(&'static str, &'static str)> {
    fn nl(commands: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        // The newline is appended by callers via `&'static str` interpolation;
        // we can't do that at runtime for &'static, so the curated tables
        // below carry the `\n` already.
        commands.to_vec()
    }
    match platform {
        "linux" => nl(&[
            ("ls -la", "ls -la\n"),
            ("df -h", "df -h\n"),
            ("free -h", "free -h\n"),
            ("ps aux | grep ", "ps aux | grep "),
            ("systemctl status ", "systemctl status "),
            ("journalctl -xe", "journalctl -xe\n"),
        ]),
        "cisco_ios" => nl(&[
            ("show ip int brief", "show ip int brief\n"),
            ("show ip route", "show ip route\n"),
            ("show interfaces status", "show interfaces status\n"),
            ("show running-config", "show running-config\n"),
            ("clear counters", "clear counters\n"),
            ("wr mem", "wr mem\n"),
        ]),
        "juniper" => nl(&[
            ("show interfaces terse", "show interfaces terse\n"),
            ("show route", "show route\n"),
            ("show bgp summary", "show bgp summary\n"),
            ("show chassis hardware", "show chassis hardware\n"),
            ("commit confirmed", "commit confirmed\n"),
        ]),
        "mikrotik" => nl(&[
            ("/interface print", "/interface print\n"),
            ("/ip address print", "/ip address print\n"),
            ("/ip route print", "/ip route print\n"),
            ("/log print", "/log print\n"),
        ]),
        "aruba" => nl(&[
            ("show vlan", "show vlan\n"),
            ("show ip route", "show ip route\n"),
            ("show interfaces brief", "show interfaces brief\n"),
        ]),
        "fortigate" => nl(&[
            ("get system status", "get system status\n"),
            (
                "get router info routing-table all",
                "get router info routing-table all\n",
            ),
            (
                "get vpn ipsec tunnel summary",
                "get vpn ipsec tunnel summary\n",
            ),
            (
                "diagnose sniffer packet any 'icmp' 4",
                "diagnose sniffer packet any 'icmp' 4\n",
            ),
            (
                "execute backup config flash",
                "execute backup config flash\n",
            ),
        ]),
        "fortimanager" => nl(&[
            ("get system status", "get system status\n"),
            ("diagnose dvm device list", "diagnose dvm device list\n"),
            (
                "execute backup all-settings ftp",
                "execute backup all-settings ftp ",
            ),
        ]),
        "paloalto" => nl(&[
            ("show system info", "show system info\n"),
            ("show session info", "show session info\n"),
            (
                "show running resource-monitor minute",
                "show running resource-monitor minute\n",
            ),
            ("show routing route", "show routing route\n"),
            ("commit", "commit\n"),
        ]),
        "f5_bigip" => nl(&[
            ("tmsh show sys version", "tmsh show sys version\n"),
            ("tmsh show ltm virtual", "tmsh show ltm virtual\n"),
            ("tmsh show net interface", "tmsh show net interface\n"),
            ("tmsh save sys config", "tmsh save sys config\n"),
        ]),
        "checkpoint_gaia" => nl(&[
            ("show configuration", "show configuration\n"),
            ("cphaprob state", "cphaprob state\n"),
            ("fw stat", "fw stat\n"),
            ("save config", "save config\n"),
        ]),
        "hp_comware" => nl(&[
            (
                "display current-configuration",
                "display current-configuration\n",
            ),
            ("display interface brief", "display interface brief\n"),
            ("display ip routing-table", "display ip routing-table\n"),
            ("save force", "save force\n"),
        ]),
        "brocade_fos" => nl(&[
            ("switchshow", "switchshow\n"),
            ("cfgshow", "cfgshow\n"),
            ("nsshow", "nsshow\n"),
            ("portshow", "portshow\n"),
        ]),
        _ => Vec::new(),
    }
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(tag: &str) -> (Database, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "zapx-macro-export-test-{tag}-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        (Database::open(&path).expect("open temp db"), path)
    }

    /// Macro export envelope round-trips: a macro survives export → import into a
    /// fresh DB (name, color, folder, steps), and a second import is a no-op.
    #[test]
    fn macro_export_roundtrip() {
        let (src, src_path) = temp_db("src");
        let id = src.create_snippet("login", "", None, Some("#0af")).unwrap();
        src.set_snippet_steps(id, Some(r#"[{"kind":"send","send":"a\r"}]"#))
            .unwrap();
        src.set_snippet_folder(id, Some("Net")).unwrap();
        // A plain snippet (no steps) must NOT be exported as a macro.
        src.create_snippet("plain", "ls\n", None, None).unwrap();

        let file = build_macro_export(&src, None).unwrap();
        assert_eq!(file.zapx_macro_export, MACRO_EXPORT_VERSION);
        assert_eq!(file.macros.len(), 1);
        assert_eq!(file.macros[0].name, "login");
        assert_eq!(file.macros[0].color.as_deref(), Some("#0af"));
        assert_eq!(file.macros[0].folder.as_deref(), Some("Net"));

        // Serialize + parse back, exercising the on-disk shape.
        let json = serde_json::to_string(&file).unwrap();
        let parsed: MacroExportFile = serde_json::from_str(&json).unwrap();

        let (dst, dst_path) = temp_db("dst");
        let s1 = apply_macro_import(&dst, &parsed).unwrap();
        assert_eq!(s1.added, 1);
        assert_eq!(s1.skipped, 0);
        let imported = dst.list_snippets().unwrap();
        let m = imported.iter().find(|s| s.name == "login").unwrap();
        assert_eq!(m.color.as_deref(), Some("#0af"));
        assert_eq!(m.folder.as_deref(), Some("Net"));
        assert_eq!(
            m.steps_json.as_deref(),
            Some(r#"[{"kind":"send","send":"a\r"}]"#)
        );

        // Idempotence: re-import skips by name.
        let s2 = apply_macro_import(&dst, &parsed).unwrap();
        assert_eq!(s2.added, 0);
        assert_eq!(s2.skipped, 1);

        let _ = std::fs::remove_file(src_path);
        let _ = std::fs::remove_file(dst_path);
    }

    /// A newer-schema file is rejected.
    #[test]
    fn macro_import_rejects_newer_version() {
        let (db, path) = temp_db("ver");
        let file = MacroExportFile {
            zapx_macro_export: MACRO_EXPORT_VERSION + 1,
            macros: vec![],
        };
        assert!(apply_macro_import(&db, &file).is_err());
        let _ = std::fs::remove_file(path);
    }
}
