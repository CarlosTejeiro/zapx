//! MobaXterm `.ini` / `.mxtsessions` → native [`ExportFile`].
//!
//! Bookmarks live in `[Bookmarks]` (root) and `[Bookmarks_N]` sections. Each
//! section has a `SubRep` (folder path, `\`-separated, empty at root) and
//! `ImgNum` (icon, ignored); every other `key=value` is a session, the key
//! being the display name.
//!
//! The value encodes `#<icon>#<typeId>%<f1>%<f2>%…`. For SSH (typeId 0 /
//! icon 109) the field layout is well documented: f1=host, f2=port,
//! f3=username — that's the high-confidence path. Telnet (icon 98) and
//! serial (icon 131) are best-effort and warned; RDP/VNC/other are skipped
//! with a warning (ZAPX has no such protocols).
//!
//! See https://gist.github.com/Ruzgfpegk/ab597838e4abbe8de30d7224afd062ea

use std::collections::HashMap;

use core_persistence::Folder;

use crate::importers::common::{self, Auth, Parsed};

pub fn parse(content: &str) -> Parsed {
    let mut warnings = Vec::new();

    // Section → (SubRep folder path, [(name, value)]).
    let mut section_subrep = String::new();
    let mut in_bookmarks = false;
    // Folder path string → local folder id; sessions reference these.
    let mut folder_ids: HashMap<String, i64> = HashMap::new();
    let mut folders: Vec<Folder> = Vec::new();
    let mut sessions = Vec::new();
    let mut next_folder_id: i64 = 0;
    let mut next_session_id: i64 = 0;

    // Resolve a `\`-separated SubRep path to a leaf folder id, creating each
    // component (and its parents) once. Empty path → root (None).
    let mut ensure_folder =
        |path: &str, folders: &mut Vec<Folder>, folder_ids: &mut HashMap<String, i64>| -> Option<i64> {
            let path = path.trim().trim_matches('\\');
            if path.is_empty() {
                return None;
            }
            let mut parent: Option<i64> = None;
            let mut acc = String::new();
            for part in path.split('\\').filter(|p| !p.is_empty()) {
                if !acc.is_empty() {
                    acc.push('\\');
                }
                acc.push_str(part);
                if let Some(&id) = folder_ids.get(&acc) {
                    parent = Some(id);
                    continue;
                }
                next_folder_id += 1;
                let id = next_folder_id;
                folders.push(common::folder(id, parent, part, (folders.len()) as i32));
                folder_ids.insert(acc.clone(), id);
                parent = Some(id);
            }
            parent
        };

    for raw in content.lines() {
        let line = raw.trim_end_matches(['\r', '\n']).trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_bookmarks = name == "Bookmarks" || name.starts_with("Bookmarks_");
            section_subrep.clear();
            continue;
        }
        if !in_bookmarks {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key.eq_ignore_ascii_case("SubRep") {
            section_subrep = value.trim().to_owned();
            continue;
        }
        if key.eq_ignore_ascii_case("ImgNum") {
            continue;
        }
        // A session line. Decode its protocol + fields.
        let folder_id = ensure_folder(&section_subrep, &mut folders, &mut folder_ids);
        if let Some(row) = decode_session(key, value, folder_id, &mut next_session_id, &mut warnings) {
            sessions.push(row); // None = skipped, already warned
        }
    }

    Parsed { file: common::envelope(folders, sessions), warnings }
}

fn decode_session(
    name: &str,
    value: &str,
    folder_id: Option<i64>,
    next_id: &mut i64,
    warnings: &mut Vec<String>,
) -> Option<core_persistence::SavedSession> {
    // value = #<icon>#<fields...>
    let body = value.trim().strip_prefix('#')?;
    let (icon, rest) = body.split_once('#')?;
    let fields: Vec<&str> = rest.split('%').collect();
    let type_id = fields.first().copied().unwrap_or("");
    let icon: u32 = icon.parse().unwrap_or(0);

    // Protocol: prefer the documented type id, fall back to the icon.
    let protocol = if type_id == "0" || icon == 109 {
        "ssh"
    } else if icon == 98 {
        "telnet"
    } else if icon == 131 {
        "serial"
    } else {
        warnings.push(format!(
            "«{name}»: tipo no soportado (icon {icon}, type {type_id}) — omitida"
        ));
        return None;
    };

    let field = |i: usize| fields.get(i).map(|s| s.trim()).filter(|s| !s.is_empty());

    if protocol == "serial" {
        // Best-effort: field 1 tends to be the COM/tty line, field 2 the speed.
        let device = field(1).unwrap_or("").to_owned();
        if device.is_empty() {
            warnings.push(format!("«{name}»: serial sin línea — omitida"));
            return None;
        }
        warnings.push(format!("«{name}»: serial importada en modo best-effort, revisa device/baudios"));
        *next_id += 1;
        let mut row = common::session(*next_id, folder_id, name, "serial", None, None, None, Auth::Password, *next_id - 1);
        let baud: u32 = field(2).and_then(|s| s.parse().ok()).unwrap_or(9600);
        row.options_json = format!(
            r#"{{"device":{},"baud_rate":{baud}}}"#,
            serde_json::to_string(&device).unwrap_or_else(|_| "\"\"".into())
        );
        return Some(row);
    }

    // SSH / Telnet: f1=host, f2=port, f3=username.
    let host = field(1)?.to_owned();
    let port = field(2)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or_else(|| common::default_port(protocol));
    let username = field(3).map(|s| s.to_owned());
    let auth = if protocol == "ssh" { Auth::Agent } else { Auth::Password };
    *next_id += 1;
    Some(common::session(
        *next_id,
        folder_id,
        name,
        protocol,
        Some(host),
        Some(port),
        username,
        auth,
        *next_id - 1,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
[Bookmarks]
SubRep=
ImgNum=42
jump=#109#0%bastion.example%22%ops%%-1%-1%%%22%%0%0%0%%%-1%0%0%0

[Bookmarks_1]
SubRep=Datacenter\\Spines
ImgNum=41
spine-01=#109#0%10.0.0.1%2222%admin%%-1%-1
old-sw=#98#1%10.0.0.9%23%

[Bookmarks_2]
SubRep=Misc
ImgNum=41
desktop=#91#4%10.0.0.50%3389%user
";

    #[test]
    fn parses_bookmarks_with_folders() {
        let parsed = parse(SAMPLE);
        let s = &parsed.file.sessions;
        // jump, spine-01, old-sw  (desktop = RDP, skipped)
        assert_eq!(s.len(), 3);

        let jump = s.iter().find(|x| x.name == "jump").unwrap();
        assert_eq!(jump.protocol, "ssh");
        assert_eq!(jump.host.as_deref(), Some("bastion.example"));
        assert_eq!(jump.port, Some(22));
        assert_eq!(jump.username.as_deref(), Some("ops"));
        assert_eq!(jump.folder_id, None, "root section");

        let spine = s.iter().find(|x| x.name == "spine-01").unwrap();
        assert_eq!(spine.port, Some(2222));
        // Nested folder Datacenter\Spines → spine sits in the leaf "Spines".
        let folders = &parsed.file.folders;
        let spines = folders.iter().find(|f| f.name == "Spines").unwrap();
        let dc = folders.iter().find(|f| f.name == "Datacenter").unwrap();
        assert_eq!(spines.parent_id, Some(dc.id));
        assert_eq!(spine.folder_id, Some(spines.id));

        let sw = s.iter().find(|x| x.name == "old-sw").unwrap();
        assert_eq!(sw.protocol, "telnet");
        assert_eq!(sw.port, Some(23));

        // RDP desktop was skipped with a warning.
        assert!(parsed.warnings.iter().any(|w| w.contains("desktop")));
    }
}
