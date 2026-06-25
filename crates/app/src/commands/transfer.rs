//! Native export/import of the ZAPX environment: sessions, folders,
//! broadcast groups, snippets and highlight rules, as one versioned JSON
//! file (`zapx-sessions.json`).
//!
//! Credentials are NEVER exported — passwords live in the OS keyring and
//! `credential_id` is stripped. On import, password-auth sessions simply
//! prompt on first connect (the existing `needsPassword` flow). Login
//! scripts ARE exported verbatim; their `send` steps may contain secrets
//! the user typed, which the export UI warns about.
//!
//! Import is idempotent: items that already exist (same identity — see the
//! `find_*` helpers) are skipped and reported in the summary, so importing
//! the same file twice adds nothing. All ids are remapped: folders first
//! (parents before children), then sessions, then a second pass resolves
//! `via_session_id` (ProxyJump) and group memberships against the new ids.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use core_persistence::{
    BroadcastGroup, Database, Folder, HighlightRule, SavedForward, SavedSession, Snippet,
};

use crate::error::AppError;
use crate::state::AppState;

const EXPORT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFile {
    /// Schema version — bump on breaking changes; import rejects newer files.
    pub zapx_export: u32,
    pub app_version: String,
    pub exported_at: String,
    #[serde(default)]
    pub folders: Vec<Folder>,
    #[serde(default)]
    pub sessions: Vec<SavedSession>,
    #[serde(default)]
    pub broadcast_groups: Vec<BroadcastGroup>,
    #[serde(default)]
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub highlight_rules: Vec<HighlightRule>,
    /// Port-forwards keyed by file-local session id (remapped on import).
    #[serde(default)]
    pub session_forwards: Vec<SessionForwards>,
}

/// The saved forwards of one session, referenced by its file-local id.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionForwards {
    pub session_id: i64,
    pub forwards: Vec<SavedForward>,
}

#[derive(Debug, Serialize)]
pub struct ExportSummary {
    pub path: String,
    pub sessions: usize,
    pub folders: usize,
    pub groups: usize,
    pub snippets: usize,
    pub rules: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct ImportSummary {
    pub sessions_added: usize,
    pub sessions_skipped: usize,
    pub folders_added: usize,
    pub groups_added: usize,
    pub snippets_added: usize,
    pub rules_added: usize,
    pub warnings: Vec<String>,
}

// ── export ──────────────────────────────────────────────────────────────────

/// Snapshot the whole environment. Pure DB → struct, no I/O.
pub fn build_export(db: &Database, app_version: &str) -> Result<ExportFile, AppError> {
    let internal = |e: core_persistence::Error| AppError::Internal(e.to_string());
    let sessions = db
        .list_sessions()
        .map_err(internal)?
        .into_iter()
        .map(|mut s| {
            // Local-only fields: keyring reference and usage timestamp.
            s.credential_id = None;
            s.last_used_at = None;
            s
        })
        .collect();
    // Forwards per session — only sessions that actually have any.
    let session_forwards = {
        let sessions: &Vec<SavedSession> = &sessions;
        let mut out = Vec::new();
        for s in sessions {
            let fwds = db.list_session_forwards(s.id).map_err(internal)?;
            if !fwds.is_empty() {
                out.push(SessionForwards {
                    session_id: s.id,
                    forwards: fwds,
                });
            }
        }
        out
    };
    Ok(ExportFile {
        zapx_export: EXPORT_VERSION,
        app_version: app_version.to_owned(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        folders: db.list_folders().map_err(internal)?,
        sessions,
        broadcast_groups: db.list_broadcast_groups().map_err(internal)?,
        snippets: db.list_snippets().map_err(internal)?,
        highlight_rules: db.list_highlight_rules().map_err(internal)?,
        session_forwards,
    })
}

// ── import ──────────────────────────────────────────────────────────────────

/// Apply an export file onto `db`, skipping items that already exist.
pub fn apply_import(db: &Database, file: &ExportFile) -> Result<ImportSummary, AppError> {
    if file.zapx_export > EXPORT_VERSION {
        return Err(AppError::Internal(format!(
            "el fichero es de una versión de ZAPX más nueva (export v{}, soportado v{EXPORT_VERSION})",
            file.zapx_export
        )));
    }
    let internal = |e: core_persistence::Error| AppError::Internal(e.to_string());
    let mut summary = ImportSummary::default();

    // 1. Folders — parents before children so parent_id can be remapped.
    //    Identity: (name, remapped parent). Matching folders are reused.
    let existing_folders = db.list_folders().map_err(internal)?;
    let mut folder_map: HashMap<i64, i64> = HashMap::new();
    let mut pending: Vec<&Folder> = file.folders.iter().collect();
    while !pending.is_empty() {
        let before = pending.len();
        pending.retain(|f| {
            let parent_new = match f.parent_id {
                None => None,
                // Parent not imported yet — keep for the next round.
                Some(p) => match folder_map.get(&p) {
                    Some(&new) => Some(new),
                    None => return true,
                },
            };
            let found = existing_folders
                .iter()
                .chain(std::iter::empty())
                .find(|e| e.name == f.name && e.parent_id == parent_new)
                .map(|e| e.id);
            let new_id = match found {
                Some(id) => id,
                None => match db.create_folder(&f.name, parent_new) {
                    Ok(id) => {
                        summary.folders_added += 1;
                        id
                    }
                    Err(e) => {
                        summary.warnings.push(format!("carpeta «{}»: {e}", f.name));
                        return false;
                    }
                },
            };
            folder_map.insert(f.id, new_id);
            false
        });
        if pending.len() == before {
            // Orphans (parent missing from the file) land at root.
            for f in pending.drain(..) {
                summary.warnings.push(format!(
                    "carpeta «{}»: padre no incluido en el export — movida a la raíz",
                    f.name
                ));
                if let Ok(id) = db.create_folder(&f.name, None) {
                    summary.folders_added += 1;
                    folder_map.insert(f.id, id);
                }
            }
        }
    }

    // 2. Sessions — identity: (name, protocol, host, port). First pass
    //    creates rows with via_session_id = NULL; pass 3 resolves bastions.
    let existing_sessions = db.list_sessions().map_err(internal)?;
    let identity = |s: &SavedSession| (s.name.clone(), s.protocol.clone(), s.host.clone(), s.port);
    let mut session_map: HashMap<i64, i64> = HashMap::new();
    // Old ids of sessions we actually created (not skipped) — only these get
    // their forwards applied, so re-import doesn't clobber existing sessions.
    let mut added_olds: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut needs_via: Vec<(i64, i64)> = Vec::new(); // (new_id, old_via_id)
    for s in &file.sessions {
        if let Some(existing) = existing_sessions
            .iter()
            .find(|e| identity(e) == identity(s))
        {
            session_map.insert(s.id, existing.id);
            summary.sessions_skipped += 1;
            continue;
        }
        let folder_new = s.folder_id.and_then(|f| folder_map.get(&f).copied());
        let new_id = db
            .create_session_full(
                folder_new,
                &s.name,
                &s.protocol,
                s.host.as_deref(),
                s.port,
                s.username.as_deref(),
                None, // credential_id never travels
                &s.options_json,
                s.auth_method.as_deref(),
                None, // via resolved in pass 3
            )
            .map_err(internal)?;
        if let Some(script) = &s.login_script_json {
            db.set_login_script(new_id, Some(script))
                .map_err(internal)?;
        }
        if let Some(via) = s.via_session_id {
            needs_via.push((new_id, via));
        }
        // Heads-up for key-auth sessions whose key file may not exist here.
        if s.auth_method.as_deref() == Some("key") {
            if let Ok(opts) = serde_json::from_str::<serde_json::Value>(&s.options_json) {
                if let Some(path) = opts.get("key_path").and_then(|v| v.as_str()) {
                    if !std::path::Path::new(path).exists() {
                        summary.warnings.push(format!(
                            "sesión «{}»: la clave {path} no existe en esta máquina",
                            s.name
                        ));
                    }
                }
            }
        }
        session_map.insert(s.id, new_id);
        added_olds.insert(s.id);
        summary.sessions_added += 1;
    }

    // 2b. Saved port-forwards — applied only to sessions we just created.
    for sf in &file.session_forwards {
        if !added_olds.contains(&sf.session_id) {
            continue;
        }
        if let Some(&new_id) = session_map.get(&sf.session_id) {
            db.set_session_forwards(new_id, &sf.forwards)
                .map_err(internal)?;
        }
    }

    // 3. Second pass: ProxyJump bastions, now that every id is known.
    for (new_id, old_via) in needs_via {
        match session_map.get(&old_via) {
            Some(&via_new) => db
                .set_session_via(new_id, Some(via_new))
                .map_err(internal)?,
            None => summary.warnings.push(format!(
                "sesión id {new_id}: bastión (jump host) no incluido en el export — queda sin via"
            )),
        }
    }

    // 4. Broadcast groups — identity: name.
    let existing_groups = db.list_broadcast_groups().map_err(internal)?;
    for g in &file.broadcast_groups {
        if existing_groups.iter().any(|e| e.name == g.name) {
            continue;
        }
        let members: Vec<i64> = g
            .session_ids
            .iter()
            .filter_map(|old| session_map.get(old).copied())
            .collect();
        if members.len() < g.session_ids.len() {
            summary.warnings.push(format!(
                "grupo «{}»: {} miembro(s) no resueltos, omitidos",
                g.name,
                g.session_ids.len() - members.len()
            ));
        }
        let gid = db.create_broadcast_group(&g.name).map_err(internal)?;
        db.set_broadcast_group_members(gid, &members)
            .map_err(internal)?;
        summary.groups_added += 1;
    }

    // 5. Snippets — identity: (name, platform).
    let existing_snippets = db.list_snippets().map_err(internal)?;
    for sn in &file.snippets {
        if existing_snippets
            .iter()
            .any(|e| e.name == sn.name && e.platform == sn.platform)
        {
            continue;
        }
        db.create_snippet(
            &sn.name,
            &sn.content,
            sn.platform.as_deref(),
            sn.color.as_deref(),
        )
        .map_err(internal)?;
        summary.snippets_added += 1;
    }

    // 6. Highlight rules — identity: (pattern, is_regex).
    let existing_rules = db.list_highlight_rules().map_err(internal)?;
    for r in &file.highlight_rules {
        if existing_rules
            .iter()
            .any(|e| e.pattern == r.pattern && e.is_regex == r.is_regex)
        {
            continue;
        }
        db.create_highlight_rule(
            &r.name,
            &r.pattern,
            r.is_regex,
            r.fg_color.as_deref(),
            r.bg_color.as_deref(),
            r.bold,
            r.underline,
        )
        .map_err(internal)?;
        summary.rules_added += 1;
    }

    Ok(summary)
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn export_sessions(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<ExportSummary, AppError> {
    let version = app.package_info().version.to_string();
    let file = build_export(&state.db, &version)?;
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::write(&path, json).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(ExportSummary {
        path,
        sessions: file.sessions.len(),
        folders: file.folders.len(),
        groups: file.broadcast_groups.len(),
        snippets: file.snippets.len(),
        rules: file.highlight_rules.len(),
    })
}

#[tauri::command]
pub async fn import_sessions(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportSummary, AppError> {
    let raw = std::fs::read_to_string(&path).map_err(|e| AppError::Internal(e.to_string()))?;
    let file: ExportFile = serde_json::from_str(&raw)
        .map_err(|e| AppError::Internal(format!("no es un export válido de ZAPX: {e}")))?;
    let summary = apply_import(&state.db, &file)?;
    if summary.rules_added > 0 {
        super::highlight::rebuild_highlighter(&state);
    }
    Ok(summary)
}

/// Import SSH sessions from an OpenSSH client config (`~/.ssh/config` by
/// default). The parser feeds the same `apply_import` pipeline as the
/// native format, so duplicates/remapping/summary behave identically.
#[tauri::command]
pub async fn import_ssh_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<ImportSummary, AppError> {
    use tauri::Manager;
    let home = app
        .path()
        .home_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let cfg = match path {
        Some(p) => std::path::PathBuf::from(p),
        None => home.join(".ssh").join("config"),
    };
    let parsed = crate::importers::ssh_config::parse(&cfg, &home)
        .map_err(|e| AppError::Internal(format!("{}: {e}", cfg.display())))?;
    finish_import(&state, parsed.file, parsed.warnings)
}

/// Apply a parsed third-party envelope and prepend the parser's warnings to
/// the pipeline summary (parser warnings first — they explain what the
/// source dropped before import even ran).
fn finish_import(
    state: &AppState,
    file: ExportFile,
    parser_warnings: Vec<String>,
) -> Result<ImportSummary, AppError> {
    let mut summary = apply_import(&state.db, &file)?;
    if summary.rules_added > 0 {
        super::highlight::rebuild_highlighter(state);
    }
    let mut warnings = parser_warnings;
    warnings.append(&mut summary.warnings);
    summary.warnings = warnings;
    Ok(summary)
}

/// Import PuTTY saved sessions. With a `path` (a `.reg` export) the file is
/// parsed everywhere; without one, on Windows it reads HKCU directly, and on
/// other platforms it errors asking for a `.reg` export.
#[tauri::command]
pub async fn import_putty(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<ImportSummary, AppError> {
    let parsed = match path {
        Some(p) => {
            let raw = std::fs::read_to_string(&p).map_err(|e| AppError::Internal(e.to_string()))?;
            crate::importers::putty::from_sessions(&crate::importers::putty::parse_reg(&raw))
        }
        None => {
            let sessions = crate::importers::putty::read_registry();
            if sessions.is_empty() && !cfg!(target_os = "windows") {
                return Err(AppError::Internal(
                    "en macOS/Linux exporta primero las sesiones de PuTTY a un .reg y elígelo"
                        .into(),
                ));
            }
            crate::importers::putty::from_sessions(&sessions)
        }
    };
    finish_import(&state, parsed.file, parsed.warnings)
}

/// Import a MobaXterm `MobaXterm.ini` (or `.mxtsessions` export).
#[tauri::command]
pub async fn import_mobaxterm(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportSummary, AppError> {
    let raw = std::fs::read_to_string(&path).map_err(|e| AppError::Internal(e.to_string()))?;
    let parsed = crate::importers::mobaxterm::parse(&raw);
    finish_import(&state, parsed.file, parsed.warnings)
}

/// Import a SecureCRT `Sessions` directory (the folder of per-session .ini
/// files under its `Config/`).
#[tauri::command]
pub async fn import_securecrt(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportSummary, AppError> {
    let parsed = crate::importers::securecrt::parse_dir(std::path::Path::new(&path))
        .map_err(|e| AppError::Internal(format!("{path}: {e}")))?;
    finish_import(&state, parsed.file, parsed.warnings)
}

/// Write arbitrary UTF-8 text to `path`. Used by "save terminal buffer to
/// file" (the frontend dumps the xterm scrollback and picks the path via the
/// save dialog). Kept generic and tiny; no env data is touched.
#[tauri::command]
pub async fn save_text_file(path: String, content: String) -> Result<(), AppError> {
    std::fs::write(&path, content).map_err(|e| AppError::Internal(format!("{path}: {e}")))
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(tag: &str) -> (Database, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "zapx-transfer-test-{tag}-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        (Database::open(&path).expect("open temp db"), path)
    }

    /// Full round-trip: nested folders, jump host, login script, group,
    /// snippet and rule survive export → import with remapped ids; a second
    /// import of the same file is a no-op.
    #[test]
    fn roundtrip_and_idempotence() {
        let (src, src_path) = temp_db("src");
        let parent = src.create_folder("DC", None).unwrap();
        let child = src.create_folder("Spines", Some(parent)).unwrap();
        let bastion = src
            .create_session_full(
                None,
                "jump",
                "ssh",
                Some("j.example"),
                Some(22),
                Some("ops"),
                None,
                "{}",
                Some("agent"),
                None,
            )
            .unwrap();
        let spine = src
            .create_session_full(
                Some(child),
                "spine-01",
                "ssh",
                Some("10.0.0.1"),
                Some(22),
                Some("admin"),
                None,
                "{}",
                Some("password"),
                Some(bastion),
            )
            .unwrap();
        src.set_login_script(spine, Some(r#"[{"expect":"login:","send":"admin"}]"#))
            .unwrap();
        src.set_session_forwards(
            spine,
            &[core_persistence::SavedForward {
                kind: "local".into(),
                bind_addr: "127.0.0.1".into(),
                bind_port: 8080,
                target_host: Some("10.0.0.9".into()),
                target_port: Some(80),
            }],
        )
        .unwrap();
        let gid = src.create_broadcast_group("grid").unwrap();
        src.set_broadcast_group_members(gid, &[bastion, spine])
            .unwrap();
        src.create_snippet("brief", "show ip int brief", Some("cisco_ios"), None)
            .unwrap();
        src.create_highlight_rule("err", "ERROR", false, Some("#f00"), None, true, false)
            .unwrap();

        let export = build_export(&src, "test").unwrap();
        assert!(export.sessions.iter().all(|s| s.credential_id.is_none()));

        let (dst, dst_path) = temp_db("dst");
        let s1 = apply_import(&dst, &export).unwrap();
        assert_eq!(s1.sessions_added, 2);
        assert_eq!(s1.folders_added, 2);
        assert_eq!(s1.groups_added, 1);
        assert_eq!(s1.snippets_added, 1);
        assert_eq!(s1.rules_added, 1);
        assert!(s1.warnings.is_empty(), "warnings: {:?}", s1.warnings);

        // Remapping: spine lives in DC/Spines and jumps through the new bastion id.
        let sessions = dst.list_sessions().unwrap();
        let new_spine = sessions.iter().find(|s| s.name == "spine-01").unwrap();
        let new_bastion = sessions.iter().find(|s| s.name == "jump").unwrap();
        assert_eq!(new_spine.via_session_id, Some(new_bastion.id));
        assert!(new_spine.login_script_json.is_some());
        // Forwards travel with their session, remapped to the new id.
        let fwds = dst.list_session_forwards(new_spine.id).unwrap();
        assert_eq!(fwds.len(), 1);
        assert_eq!(fwds[0].bind_port, 8080);
        assert_eq!(fwds[0].target_host.as_deref(), Some("10.0.0.9"));
        let folders = dst.list_folders().unwrap();
        let new_child = folders.iter().find(|f| f.name == "Spines").unwrap();
        let new_parent = folders.iter().find(|f| f.name == "DC").unwrap();
        assert_eq!(new_child.parent_id, Some(new_parent.id));
        assert_eq!(new_spine.folder_id, Some(new_child.id));
        let groups = dst.list_broadcast_groups().unwrap();
        assert_eq!(groups[0].session_ids.len(), 2);
        assert!(groups[0].session_ids.contains(&new_spine.id));

        // Idempotence: importing again adds nothing.
        let s2 = apply_import(&dst, &export).unwrap();
        assert_eq!(s2.sessions_added, 0);
        assert_eq!(s2.sessions_skipped, 2);
        assert_eq!(s2.folders_added, 0);
        assert_eq!(s2.groups_added, 0);
        assert_eq!(s2.snippets_added, 0);
        assert_eq!(s2.rules_added, 0);

        let _ = std::fs::remove_file(src_path);
        let _ = std::fs::remove_file(dst_path);
    }

    /// Files from a newer schema are rejected with a clear error.
    #[test]
    fn rejects_newer_version() {
        let (db, path) = temp_db("ver");
        let file = ExportFile {
            zapx_export: EXPORT_VERSION + 1,
            app_version: "9.9.9".into(),
            exported_at: String::new(),
            folders: vec![],
            sessions: vec![],
            broadcast_groups: vec![],
            snippets: vec![],
            highlight_rules: vec![],
            session_forwards: vec![],
        };
        assert!(apply_import(&db, &file).is_err());
        let _ = std::fs::remove_file(path);
    }
}
