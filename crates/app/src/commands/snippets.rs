//! Snippet CRUD commands. Snippets are short named text blocks the frontend
//! can dispatch to the focused session via the existing `send_input` command.
//!
//! A snippet may be **global** (visible in every session) or scoped to a
//! single **platform** key (matching `core_hints::Platform::as_str`).
//! The bottom button bar filters by the focused session's platform; the
//! manage dialog lists everything.

use tauri::State;

use core_persistence::Snippet;

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
