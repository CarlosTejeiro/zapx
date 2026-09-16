//! Sync folder — phase 2 of the encrypted backup.
//!
//! The user points ZAPX at a folder that some other program keeps in sync
//! (Dropbox, Drive, iCloud, OneDrive, Syncthing, a NAS share…). ZAPX keeps
//! one sealed bundle there, `zapx-sync.zapxb`, plus a tiny plaintext sidecar
//! `zapx-sync.json` saying which device wrote it and when. Nothing about the
//! provider is assumed: to us it's just a directory.
//!
//! Model — deliberately simple and additive:
//! - **Push**: seal the local environment and overwrite the bundle. Done
//!   automatically (in the background, every few minutes) whenever the local
//!   content changed since the last push/pull, unless the remote has changed
//!   in the meantime.
//! - **Pull**: open the remote bundle and merge it in (same rules as a manual
//!   restore: existing sessions are kept, vault entries matched by name,
//!   settings replaced, host keys added). Never automatic — it replaces
//!   settings and needs a restart to show, so the UI asks first.
//! - **Conflict**: the remote bundle is newer than what we last pulled *and*
//!   the local content changed. Nothing is written; the UI offers "merge the
//!   remote first, then publish mine" or "overwrite the remote with mine".
//!
//! Change detection uses a *canonical fingerprint* of the content (ids,
//! ordering and timestamps stripped, identities instead of ids), so two
//! devices holding the same sessions agree even though their row ids differ
//! and the bundles stop ping-ponging once both sides converge.
//!
//! Known limitation, by design of the merge: renames and deletions don't
//! propagate — the other device's copy comes back on the next pull.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};

use super::backup::{self, BackupCtx, Bundle, KeyringStore, RestoreSummary};
use crate::error::AppError;
use crate::state::AppState;

// Settings keys. All under `backup.` so they never travel in a bundle.
pub const SYNC_DIR_KEY: &str = "backup.sync.dir";
pub const SYNC_LAST_PUSH_KEY: &str = "backup.sync.last_push";
pub const SYNC_LAST_PULL_KEY: &str = "backup.sync.last_pull";
pub const SYNC_FINGERPRINT_KEY: &str = "backup.sync.fingerprint";
/// Encrypted-DB fallback for the passphrase (hex of `encrypt_with_seed`),
/// consulted when the keyring won't hand it back (unsigned/portable builds).
pub const SYNC_PASSPHRASE_KEY: &str = "backup.sync.passphrase";
/// Keyring entry holding the sync passphrase.
pub const SYNC_KEYRING_KEY: &str = "backup:sync";

pub const SYNC_BUNDLE_FILE: &str = "zapx-sync.zapxb";
pub const SYNC_META_FILE: &str = "zapx-sync.json";
/// Tauri event carrying a [`SyncStatus`] whenever the background check finds
/// something worth telling the user (remote changes, a conflict, an error).
pub const SYNC_EVENT: &str = "backup-sync";

const SYNC_FIRST_DELAY: Duration = Duration::from_secs(15);
const SYNC_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// One sync operation at a time — the background thread and the UI commands
/// share the folder and the settings rows.
static SYNC_LOCK: Mutex<()> = Mutex::new(());

// ── wire types ──────────────────────────────────────────────────────────────

/// The plaintext sidecar next to the bundle. Cheap "did anything change?"
/// check that needs neither the passphrase nor an Argon2 run. Only ever
/// treated as a hint: the bundle's own (authenticated) header is what a pull
/// records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncMeta {
    pub zapx_sync: u32,
    pub device_id: String,
    pub device_name: String,
    /// RFC 3339, equals the bundle's `created_at`.
    pub written_at: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    /// No folder configured.
    Disabled,
    /// Folder set but the passphrase is missing on this device.
    NoPassphrase,
    InSync,
    /// Local content differs from what was last pushed/pulled; a push is due.
    LocalChanges,
    /// The folder holds a bundle newer than what we last pulled.
    RemoteChanges,
    /// Both of the above.
    Conflict,
    /// The folder is unreadable, the passphrase is wrong, etc. See `detail`.
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub state: SyncState,
    pub dir: Option<String>,
    pub has_passphrase: bool,
    pub last_push: Option<String>,
    pub last_pull: Option<String>,
    /// Sidecar of the bundle currently in the folder, if any.
    pub remote: Option<SyncMeta>,
    pub detail: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Serialize)]
pub struct SyncPullResult {
    pub restore: RestoreSummary,
    pub status: SyncStatus,
}

// ── settings helpers ────────────────────────────────────────────────────────

fn internal(e: impl std::fmt::Display) -> AppError {
    AppError::Internal(e.to_string())
}

fn setting(ctx: &BackupCtx<'_>, key: &str) -> Option<String> {
    ctx.db
        .get_setting(key)
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())
}

fn sync_dir(ctx: &BackupCtx<'_>) -> Option<PathBuf> {
    setting(ctx, SYNC_DIR_KEY).map(PathBuf::from)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Keyring first (best effort), encrypted-DB fallback always — the sync
/// passphrase must survive a keyring that denies reads, or the background
/// push would silently stop working on unsigned builds.
fn store_passphrase(ctx: &BackupCtx<'_>, passphrase: &str) -> Result<(), AppError> {
    if let Err(e) = ctx.secrets.put(SYNC_KEYRING_KEY, passphrase) {
        tracing::warn!("sync passphrase not stored in keyring: {e}");
    }
    let blob = core_vault::encrypt_with_seed(ctx.vault_seed, passphrase).map_err(internal)?;
    ctx.db
        .set_setting(SYNC_PASSPHRASE_KEY, &hex_encode(&blob))
        .map_err(internal)
}

fn load_passphrase(ctx: &BackupCtx<'_>) -> Option<String> {
    ctx.secrets.get(SYNC_KEYRING_KEY).or_else(|| {
        let hex = setting(ctx, SYNC_PASSPHRASE_KEY)?;
        let blob = hex_decode(&hex)?;
        core_vault::decrypt_with_seed(ctx.vault_seed, &blob).ok()
    })
}

fn clear_passphrase(ctx: &BackupCtx<'_>) {
    ctx.secrets.delete(SYNC_KEYRING_KEY);
    ctx.db.delete_setting(SYNC_PASSPHRASE_KEY).ok();
}

// ── canonical fingerprint ───────────────────────────────────────────────────

/// Hash of the bundle's *content*, independent of row ids, ordering,
/// timestamps and the writing device. Two devices holding the same
/// environment produce the same fingerprint even though every id differs.
pub(crate) fn fingerprint(bundle: &Bundle) -> String {
    use serde_json::{json, Value};
    let env = &bundle.environment;

    // Folder id → "Parent/Child" path.
    let folder_path = |id: i64| -> String {
        let mut parts = Vec::new();
        let mut cur = Some(id);
        let mut guard = 0;
        while let Some(fid) = cur {
            guard += 1;
            if guard > 64 {
                break;
            }
            match env.folders.iter().find(|f| f.id == fid) {
                Some(f) => {
                    parts.push(f.name.clone());
                    cur = f.parent_id;
                }
                None => break,
            }
        }
        parts.reverse();
        parts.join("/")
    };
    let session_identity = |id: i64| -> Option<String> {
        env.sessions.iter().find(|s| s.id == id).map(|s| {
            format!(
                "{}|{}|{}|{}",
                s.name,
                s.protocol,
                s.host.as_deref().unwrap_or(""),
                s.port.map(|p| p.to_string()).unwrap_or_default()
            )
        })
    };

    let mut folders: Vec<String> = env.folders.iter().map(|f| folder_path(f.id)).collect();
    folders.sort();

    let mut sessions: Vec<Value> = env
        .sessions
        .iter()
        .map(|s| {
            json!({
                "id": session_identity(s.id),
                "username": s.username,
                "auth_method": s.auth_method,
                "options_json": s.options_json,
                "login_script_json": s.login_script_json,
                "folder": s.folder_id.map(folder_path),
                "via": s.via_session_id.and_then(session_identity),
            })
        })
        .collect();
    sessions.sort_by_key(|v| v["id"].to_string());

    let mut forwards: Vec<Value> = env
        .session_forwards
        .iter()
        .filter_map(|sf| {
            let id = session_identity(sf.session_id)?;
            let mut fw: Vec<String> = sf
                .forwards
                .iter()
                .map(|f| {
                    format!(
                        "{}|{}|{}|{}|{}",
                        f.kind,
                        f.bind_addr,
                        f.bind_port,
                        f.target_host.as_deref().unwrap_or(""),
                        f.target_port.map(|p| p.to_string()).unwrap_or_default()
                    )
                })
                .collect();
            fw.sort();
            Some(json!({ "session": id, "forwards": fw }))
        })
        .collect();
    forwards.sort_by_key(|v| v["session"].to_string());

    let mut groups: Vec<Value> = env
        .broadcast_groups
        .iter()
        .map(|g| {
            let mut members: Vec<String> = g
                .session_ids
                .iter()
                .filter_map(|id| session_identity(*id))
                .collect();
            members.sort();
            json!({ "name": g.name, "members": members })
        })
        .collect();
    groups.sort_by_key(|v| v["name"].to_string());

    let mut snippets: Vec<Value> = env
        .snippets
        .iter()
        .map(|s| {
            json!({
                "name": s.name, "platform": s.platform, "content": s.content,
                "color": s.color, "steps_json": s.steps_json, "folder": s.folder,
            })
        })
        .collect();
    snippets.sort_by_key(|v| format!("{}|{}", v["name"], v["platform"]));

    let mut rules: Vec<Value> = env
        .highlight_rules
        .iter()
        .map(|r| {
            json!({
                "name": r.name, "pattern": r.pattern, "is_regex": r.is_regex,
                "fg": r.fg_color, "bg": r.bg_color, "bold": r.bold,
                "underline": r.underline, "enabled": r.enabled,
            })
        })
        .collect();
    rules.sort_by_key(|v| format!("{}|{}", v["pattern"], v["is_regex"]));

    let settings: std::collections::BTreeMap<&String, &String> = bundle.settings.iter().collect();

    let mut vault: Vec<Value> = bundle
        .vault_entries
        .iter()
        .map(|v| json!({ "name": v.name, "username": v.username, "secret": v.secret }))
        .collect();
    vault.sort_by_key(|v| v["name"].to_string());

    let mut creds: Vec<Value> = bundle
        .session_credentials
        .iter()
        .filter_map(|c| {
            let id = session_identity(c.session_id)?;
            Some(json!({
                "session": id, "vault_name": c.vault_name,
                "kind": c.kind, "secret": c.secret,
            }))
        })
        .collect();
    creds.sort_by_key(|v| v["session"].to_string());

    let mut known_hosts: Vec<&str> = bundle
        .known_hosts
        .as_deref()
        .map(|t| {
            t.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        })
        .unwrap_or_default();
    known_hosts.sort_unstable();
    known_hosts.dedup();

    let canonical = json!({
        "folders": folders, "sessions": sessions, "forwards": forwards,
        "groups": groups, "snippets": snippets, "rules": rules,
        "settings": settings, "vault": vault, "credentials": creds,
        "known_hosts": known_hosts,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    hex_encode(&Sha256::digest(&bytes))
}

// ── engine ──────────────────────────────────────────────────────────────────

fn read_meta(dir: &Path) -> Result<Option<SyncMeta>, String> {
    let path = dir.join(SYNC_META_FILE);
    if !dir.is_dir() {
        return Err(format!("sync folder not found: {}", dir.display()));
    }
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<SyncMeta>(&raw)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn write_meta(dir: &Path, meta: &SyncMeta) -> Result<(), AppError> {
    let path = dir.join(SYNC_META_FILE);
    let tmp = dir.join(format!("{SYNC_META_FILE}.tmp"));
    let json = serde_json::to_string_pretty(meta).map_err(internal)?;
    std::fs::write(&tmp, json).map_err(|e| internal(format!("{}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        internal(format!("{}: {e}", path.display()))
    })
}

fn parse_ts(s: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(s).ok()
}

/// `a` strictly after `b`; unparsable timestamps fall back to string order.
fn newer(a: &str, b: &str) -> bool {
    match (parse_ts(a), parse_ts(b)) {
        (Some(x), Some(y)) => x > y,
        _ => a > b,
    }
}

fn base_status(ctx: &BackupCtx<'_>, state: SyncState, detail: Option<String>) -> SyncStatus {
    SyncStatus {
        state,
        dir: setting(ctx, SYNC_DIR_KEY),
        has_passphrase: load_passphrase(ctx).is_some(),
        last_push: setting(ctx, SYNC_LAST_PUSH_KEY),
        last_pull: setting(ctx, SYNC_LAST_PULL_KEY),
        remote: None,
        detail,
        checked_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Where things stand, without touching the folder's bundle.
pub(crate) fn status(ctx: &BackupCtx<'_>, app_version: &str) -> SyncStatus {
    let Some(dir) = sync_dir(ctx) else {
        return base_status(ctx, SyncState::Disabled, None);
    };
    if load_passphrase(ctx).is_none() {
        return base_status(ctx, SyncState::NoPassphrase, None);
    }
    let remote = match read_meta(&dir) {
        Ok(m) => m,
        Err(e) => return base_status(ctx, SyncState::Error, Some(e)),
    };
    let mut warnings = Vec::new();
    let local_fp = match backup::build_bundle(ctx, app_version, &mut warnings) {
        Ok(b) => fingerprint(&b),
        Err(e) => return base_status(ctx, SyncState::Error, Some(e.to_string())),
    };
    // No fingerprint yet = this device never synced with this folder. If the
    // folder already holds a bundle, the first move is to pull it (the pull
    // then pushes back whatever this device adds); otherwise, to publish.
    let local_changed = match setting(ctx, SYNC_FINGERPRINT_KEY) {
        None => remote.is_none(),
        Some(fp) => fp != local_fp,
    };
    let last_push = setting(ctx, SYNC_LAST_PUSH_KEY);
    let last_pull = setting(ctx, SYNC_LAST_PULL_KEY);
    let remote_newer = remote.as_ref().is_some_and(|m| {
        // Our own last push is not "remote news"; anything else newer than
        // the last pull is.
        last_push.as_deref() != Some(m.written_at.as_str())
            && last_pull
                .as_deref()
                .is_none_or(|lp| newer(&m.written_at, lp))
    });
    let state = match (local_changed, remote_newer) {
        (true, true) => SyncState::Conflict,
        (false, true) => SyncState::RemoteChanges,
        (true, false) => SyncState::LocalChanges,
        (false, false) => SyncState::InSync,
    };
    let mut st = base_status(ctx, state, None);
    st.remote = remote;
    st
}

/// Seal the local environment into the folder, overwriting what's there.
pub(crate) fn push(ctx: &BackupCtx<'_>, app_version: &str) -> Result<SyncStatus, AppError> {
    let dir = sync_dir(ctx).ok_or_else(|| internal("sync folder not configured"))?;
    let passphrase = load_passphrase(ctx).ok_or_else(|| internal("sync passphrase not set"))?;
    std::fs::create_dir_all(&dir).map_err(|e| internal(format!("{}: {e}", dir.display())))?;
    let mut warnings = Vec::new();
    let bundle = backup::build_bundle(ctx, app_version, &mut warnings)?;
    for w in &warnings {
        tracing::warn!("sync push: {w}");
    }
    backup::write_bundle(&dir.join(SYNC_BUNDLE_FILE), &passphrase, &bundle)?;
    write_meta(
        &dir,
        &SyncMeta {
            zapx_sync: 1,
            device_id: bundle.device.id.clone(),
            device_name: bundle.device.name.clone(),
            written_at: bundle.created_at.clone(),
            app_version: app_version.to_owned(),
        },
    )?;
    ctx.db
        .set_setting(SYNC_LAST_PUSH_KEY, &bundle.created_at)
        .map_err(internal)?;
    ctx.db
        .set_setting(SYNC_FINGERPRINT_KEY, &fingerprint(&bundle))
        .map_err(internal)?;
    tracing::info!(dir = %dir.display(), "sync bundle pushed");
    Ok(status(ctx, app_version))
}

/// Merge the folder's bundle into this device. Afterwards, if the merged
/// content still differs from the remote (this device had extra items),
/// the status reads `LocalChanges` and the caller pushes.
pub(crate) fn pull(
    ctx: &BackupCtx<'_>,
    app_version: &str,
) -> Result<(RestoreSummary, SyncStatus), AppError> {
    let dir = sync_dir(ctx).ok_or_else(|| internal("sync folder not configured"))?;
    let passphrase = load_passphrase(ctx).ok_or_else(|| internal("sync passphrase not set"))?;
    let remote = backup::read_bundle(&dir.join(SYNC_BUNDLE_FILE), &passphrase)?;
    let summary = backup::apply_bundle(ctx, &remote)?;
    ctx.db
        .set_setting(SYNC_LAST_PULL_KEY, &remote.created_at)
        .map_err(internal)?;
    // Converged? Then record it so we don't push an identical bundle back.
    let mut warnings = Vec::new();
    let local = backup::build_bundle(ctx, app_version, &mut warnings)?;
    if fingerprint(&local) == fingerprint(&remote) {
        ctx.db
            .set_setting(SYNC_FINGERPRINT_KEY, &fingerprint(&local))
            .map_err(internal)?;
        ctx.db
            .set_setting(SYNC_LAST_PUSH_KEY, &remote.created_at)
            .map_err(internal)?;
    }
    tracing::info!(dir = %dir.display(), "sync bundle pulled");
    Ok((summary, status(ctx, app_version)))
}

/// The unattended step: push when only the local side changed; otherwise
/// just report. Pulls are always a user decision.
pub(crate) fn auto(ctx: &BackupCtx<'_>, app_version: &str) -> Result<SyncStatus, AppError> {
    let st = status(ctx, app_version);
    match st.state {
        SyncState::LocalChanges => push(ctx, app_version),
        _ => Ok(st),
    }
}

/// Point sync at `dir` (created if missing), optionally (re)setting the
/// passphrase, and forget the previous folder's bookkeeping so the next
/// status is computed against the new one.
pub(crate) fn configure(
    ctx: &BackupCtx<'_>,
    dir: &Path,
    passphrase: Option<&str>,
    app_version: &str,
) -> Result<SyncStatus, AppError> {
    std::fs::create_dir_all(dir).map_err(|e| internal(format!("{}: {e}", dir.display())))?;
    if let Some(pw) = passphrase {
        backup::validate_passphrase(pw)?;
        store_passphrase(ctx, pw)?;
    }
    let dir_str = dir.to_string_lossy().into_owned();
    let changed_dir = setting(ctx, SYNC_DIR_KEY).as_deref() != Some(dir_str.as_str());
    ctx.db
        .set_setting(SYNC_DIR_KEY, &dir_str)
        .map_err(internal)?;
    if changed_dir {
        for k in [SYNC_LAST_PUSH_KEY, SYNC_LAST_PULL_KEY, SYNC_FINGERPRINT_KEY] {
            ctx.db.delete_setting(k).ok();
        }
    }
    Ok(status(ctx, app_version))
}

pub(crate) fn disable(ctx: &BackupCtx<'_>) {
    for k in [
        SYNC_DIR_KEY,
        SYNC_LAST_PUSH_KEY,
        SYNC_LAST_PULL_KEY,
        SYNC_FINGERPRINT_KEY,
    ] {
        ctx.db.delete_setting(k).ok();
    }
    clear_passphrase(ctx);
}

// ── background checker ──────────────────────────────────────────────────────

/// Periodic check on a plain thread (the work is blocking SQLite + Argon2).
/// Pushes silently when only the local side changed; emits [`SYNC_EVENT`]
/// when the status changes to something the user should see.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("zapx-sync".into())
        .spawn(move || {
            std::thread::sleep(SYNC_FIRST_DELAY);
            let mut last_signature: Option<String> = None;
            loop {
                run_once(&app, &mut last_signature);
                std::thread::sleep(SYNC_INTERVAL);
            }
        })
        .map(|_| ())
        .unwrap_or_else(|e| tracing::warn!("sync thread not started: {e}"));
}

fn run_once(app: &AppHandle, last_signature: &mut Option<String>) {
    let state = app.state::<AppState>();
    let version = app.package_info().version.to_string();
    let store = KeyringStore {
        portable: state.data_dir.is_portable(),
    };
    let ctx = BackupCtx::from_state(&state, &store);
    let st = {
        let _g = SYNC_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        if sync_dir(&ctx).is_none() {
            return;
        }
        match auto(&ctx, &version) {
            Ok(st) => st,
            Err(e) => base_status(&ctx, SyncState::Error, Some(e.to_string())),
        }
    };
    let signature = format!(
        "{:?}|{}|{}",
        st.state,
        st.remote
            .as_ref()
            .map(|m| m.written_at.as_str())
            .unwrap_or(""),
        st.detail.as_deref().unwrap_or("")
    );
    if last_signature.as_deref() == Some(signature.as_str()) {
        return;
    }
    *last_signature = Some(signature);
    if matches!(
        st.state,
        SyncState::RemoteChanges | SyncState::Conflict | SyncState::Error
    ) {
        let _ = app.emit(SYNC_EVENT, &st);
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

fn with_ctx<T>(
    app: &AppHandle,
    state: &AppState,
    f: impl FnOnce(&BackupCtx<'_>, &str) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let version = app.package_info().version.to_string();
    let store = KeyringStore {
        portable: state.data_dir.is_portable(),
    };
    let ctx = BackupCtx::from_state(state, &store);
    let _g = SYNC_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    f(&ctx, &version)
}

#[tauri::command]
pub async fn backup_sync_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, AppError> {
    with_ctx(&app, &state, |ctx, v| Ok(status(ctx, v)))
}

#[tauri::command]
pub async fn backup_sync_configure(
    app: AppHandle,
    state: State<'_, AppState>,
    dir: String,
    passphrase: Option<String>,
) -> Result<SyncStatus, AppError> {
    with_ctx(&app, &state, |ctx, v| {
        configure(ctx, Path::new(&dir), passphrase.as_deref(), v)
    })
}

#[tauri::command]
pub async fn backup_sync_disable(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, AppError> {
    with_ctx(&app, &state, |ctx, v| {
        disable(ctx);
        Ok(status(ctx, v))
    })
}

/// "Sync now": push if only the local side changed; otherwise report so the
/// UI can offer a pull or conflict resolution.
#[tauri::command]
pub async fn backup_sync_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, AppError> {
    with_ctx(&app, &state, auto)
}

/// Force-publish the local environment, overwriting the remote bundle
/// (conflict resolution: "keep mine").
#[tauri::command]
pub async fn backup_sync_push(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, AppError> {
    with_ctx(&app, &state, push)
}

/// Merge the remote bundle in, then publish the merged result if it differs
/// (conflict resolution: "merge remote first").
#[tauri::command]
pub async fn backup_sync_pull(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncPullResult, AppError> {
    let result = with_ctx(&app, &state, |ctx, v| {
        let (restore, st) = pull(ctx, v)?;
        let status = if st.state == SyncState::LocalChanges {
            push(ctx, v)?
        } else {
            st
        };
        Ok(SyncPullResult { restore, status })
    })?;
    if result.restore.environment.rules_added > 0 {
        super::highlight::rebuild_highlighter(&state);
    }
    super::settings::apply_persisted_keepalive(&state.db);
    Ok(result)
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::backup::SecretStore;
    use crate::state::PasswordCache;
    use core_persistence::Database;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MemStore(Mutex<HashMap<String, String>>);
    impl SecretStore for MemStore {
        fn get(&self, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }
        fn put(&self, key: &str, secret: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().insert(key.into(), secret.into());
            Ok(())
        }
        fn delete(&self, key: &str) {
            self.0.lock().unwrap().remove(key);
        }
    }

    struct Device {
        dir: PathBuf,
        db: Database,
        store: MemStore,
        cache: PasswordCache,
    }

    impl Device {
        fn new(root: &Path, name: &str) -> Self {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let db = Database::open(&dir.join("zapx.db")).unwrap();
            db.set_setting(backup::DEVICE_ID_KEY, &format!("dev-{name}"))
                .unwrap();
            Self {
                dir,
                db,
                store: MemStore::default(),
                cache: Arc::new(Mutex::new(HashMap::new())),
            }
        }
        fn ctx(&self) -> BackupCtx<'_> {
            BackupCtx {
                db: &self.db,
                secrets: &self.store,
                vault_seed: "seed",
                password_cache: Some(&self.cache),
                portable: false,
                known_hosts_path: Some(self.dir.join("known_hosts")),
            }
        }
        fn add_ssh(&self, name: &str) -> i64 {
            self.db
                .create_session_full(
                    None,
                    name,
                    "ssh",
                    Some(&format!("{name}.example")),
                    Some(22),
                    Some("admin"),
                    None,
                    "{}",
                    Some("agent"),
                    None,
                )
                .unwrap()
        }
    }

    fn root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "zapx-sync-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Two devices, one shared folder: A pushes, B sees remote changes, pulls,
    /// converges without pushing back; B adds a session, pushes; A sees
    /// remote changes and, after pulling, is in sync. Fingerprints agree
    /// across devices despite different row ids.
    #[test]
    fn two_devices_converge_through_the_folder() {
        let root = root("converge");
        let shared = root.join("Dropbox").join("zapx");
        let a = Device::new(&root, "a");
        let b = Device::new(&root, "b");
        a.add_ssh("core");
        a.db.set_setting("ui.theme", "fjord").unwrap();

        // Unconfigured → disabled; configured without passphrase → waiting.
        assert_eq!(status(&a.ctx(), "1").state, SyncState::Disabled);
        let st = configure(&a.ctx(), &shared, None, "1").unwrap();
        assert_eq!(st.state, SyncState::NoPassphrase);
        assert!(configure(&a.ctx(), &shared, Some("short"), "1").is_err());
        let st = configure(&a.ctx(), &shared, Some("shared passphrase"), "1").unwrap();
        assert_eq!(st.state, SyncState::LocalChanges, "{st:?}");
        assert!(st.has_passphrase);

        // A publishes.
        let st = auto(&a.ctx(), "1").unwrap();
        assert_eq!(st.state, SyncState::InSync);
        assert!(shared.join(SYNC_BUNDLE_FILE).is_file());
        let meta = read_meta(&shared).unwrap().unwrap();
        assert_eq!(meta.device_id, "dev-a");
        assert_eq!(st.remote.as_ref().unwrap().written_at, meta.written_at);
        // The passphrase survives a keyring that forgets it (DB fallback).
        a.store.delete(SYNC_KEYRING_KEY);
        assert_eq!(status(&a.ctx(), "1").state, SyncState::InSync);

        // B joins: the folder already holds A's bundle → remote changes.
        let st = configure(&b.ctx(), &shared, Some("shared passphrase"), "1").unwrap();
        assert_eq!(st.state, SyncState::RemoteChanges, "{st:?}");
        // Unattended run must NOT pull on its own.
        assert_eq!(auto(&b.ctx(), "1").unwrap().state, SyncState::RemoteChanges);
        let (restore, st) = pull(&b.ctx(), "1").unwrap();
        assert_eq!(restore.environment.sessions_added, 1);
        assert_eq!(
            b.db.get_setting("ui.theme").unwrap().as_deref(),
            Some("fjord")
        );
        // B now holds exactly A's content → converged, nothing to push back.
        assert_eq!(st.state, SyncState::InSync, "{st:?}");
        assert_eq!(auto(&a.ctx(), "1").unwrap().state, SyncState::InSync);

        // B adds a session and publishes; A sees it as remote news.
        b.add_ssh("edge");
        let st = auto(&b.ctx(), "1").unwrap();
        assert_eq!(st.state, SyncState::InSync);
        assert_eq!(read_meta(&shared).unwrap().unwrap().device_id, "dev-b");
        let st = status(&a.ctx(), "1");
        assert_eq!(st.state, SyncState::RemoteChanges, "{st:?}");
        let (restore, st) = pull(&a.ctx(), "1").unwrap();
        assert_eq!(restore.environment.sessions_added, 1);
        assert_eq!(st.state, SyncState::InSync, "{st:?}");
        assert_eq!(a.db.list_sessions().unwrap().len(), 2);

        // A local edit on A while B also published → conflict; "merge remote
        // first" resolves it and republishes.
        a.db.set_setting("ui.theme", "oxide").unwrap();
        b.add_ssh("lab");
        assert_eq!(auto(&b.ctx(), "1").unwrap().state, SyncState::InSync);
        let st = status(&a.ctx(), "1");
        assert_eq!(st.state, SyncState::Conflict, "{st:?}");
        assert_eq!(auto(&a.ctx(), "1").unwrap().state, SyncState::Conflict);
        let (_, st) = pull(&a.ctx(), "1").unwrap();
        // Remote settings won (fjord), and A's merged set has nothing extra.
        assert_eq!(
            a.db.get_setting("ui.theme").unwrap().as_deref(),
            Some("fjord")
        );
        assert_eq!(st.state, SyncState::InSync, "{st:?}");

        // Disable wipes the bookkeeping and the passphrase.
        disable(&a.ctx());
        let st = status(&a.ctx(), "1");
        assert_eq!(st.state, SyncState::Disabled);
        assert!(!st.has_passphrase);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The fingerprint ignores ids and ordering but sees every content change.
    #[test]
    fn fingerprint_is_canonical() {
        let root = root("fp");
        let a = Device::new(&root, "a");
        let b = Device::new(&root, "b");
        // Same content, inserted in different order → different ids.
        a.add_ssh("one");
        a.add_ssh("two");
        b.add_ssh("two");
        b.add_ssh("one");
        for d in [&a, &b] {
            d.db.set_setting("ui.theme", "fjord").unwrap();
        }
        let mut w = Vec::new();
        let fa = fingerprint(&backup::build_bundle(&a.ctx(), "1", &mut w).unwrap());
        let fb = fingerprint(&backup::build_bundle(&b.ctx(), "1", &mut w).unwrap());
        assert_eq!(fa, fb);
        // Device identity and timestamps don't count…
        let fa2 = fingerprint(&backup::build_bundle(&a.ctx(), "2", &mut w).unwrap());
        assert_eq!(fa, fa2);
        // …but a setting, a session or a host key does.
        a.db.set_setting("ui.theme", "oxide").unwrap();
        assert_ne!(
            fa,
            fingerprint(&backup::build_bundle(&a.ctx(), "1", &mut w).unwrap())
        );
        b.add_ssh("three");
        assert_ne!(
            fb,
            fingerprint(&backup::build_bundle(&b.ctx(), "1", &mut w).unwrap())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wrong_passphrase_and_missing_folder_are_errors() {
        let root = root("err");
        let shared = root.join("shared");
        let a = Device::new(&root, "a");
        let b = Device::new(&root, "b");
        a.add_ssh("x");
        configure(&a.ctx(), &shared, Some("passphrase-a"), "1").unwrap();
        auto(&a.ctx(), "1").unwrap();
        configure(&b.ctx(), &shared, Some("passphrase-b"), "1").unwrap();
        let err = pull(&b.ctx(), "1").unwrap_err().to_string();
        assert!(err.contains("wrong passphrase"), "{err}");
        std::fs::remove_dir_all(&shared).unwrap();
        let st = status(&a.ctx(), "1");
        assert_eq!(st.state, SyncState::Error);
        assert!(st.detail.unwrap().contains("not found"));
        assert_eq!(
            hex_decode(&hex_encode(b"\x00\xff\x10")).unwrap(),
            b"\x00\xff\x10"
        );
        assert!(hex_decode("abc").is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
