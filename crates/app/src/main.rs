// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux/WebKitGTK launch-time environment fixups.
    //
    // Some WebKitGTK problems can only be fixed by having a variable present in
    // the environment *before* WebKit initializes; setting it from inside the
    // process (after the web/GPU process is already spawned) is too late and has
    // no effect. So we re-exec ourselves once with the needed variables set, so
    // the real run starts with them already present. Two distinct fixes are
    // batched into this single re-exec:
    //
    // 1. IME (all Linux arches). On Wayland (reproduced on Fedora + KDE Plasma),
    //    WebKitGTK mishandles the IBus/Fcitx input-method system and garbles
    //    terminal typing — duplicated/ghost characters, "crazy" input per
    //    keystroke. Launching with GTK_IM_MODULE="" and QT_IM_MODULE="" (empty,
    //    i.e. direct keyboard input with no IME) fixes it. We force this off by
    //    default because it's the primary fix. NOTE: this forces direct keyboard
    //    input — no composed/CJK/emoji input. A user who needs IME opts out by
    //    setting ZAPX_ENABLE_IME=1, which keeps the session's IME modules.
    //
    // 2. DMABUF (x86_64 only). On many NVIDIA setups (and generally under
    //    Wayland) the DMABUF renderer drives WebKitWebProcess into a busy
    //    repaint loop — it burns a CPU core even when idle. Disabling it fixes
    //    that. We only auto-disable on x86_64: on aarch64 (ARM) WebKitGTK,
    //    forcing the DMABUF renderer off pushes rendering onto a fallback path
    //    that on several ARM GPUs/VMs paints garbage and swallows keyboard input
    //    (reported as "the terminal goes crazy and you can't type"). The
    //    high-CPU problem this works around is an x86/NVIDIA/Wayland issue, so
    //    ARM keeps WebKit's default. An ARM user who still wants it off can
    //    export WEBKIT_DISABLE_DMABUF_RENDERER=1 themselves before launch. We
    //    keep the existing guard: only plan this when the var is currently unset.
    //
    // Loop prevention: unlike DMABUF, we can't guard the IME change on
    // "GTK_IM_MODULE unset?" because Fedora/KDE *sets* GTK_IM_MODULE=ibus in the
    // session, so we must OVERRIDE it — its presence can't be the guard. Instead
    // we use a dedicated ZAPX_REEXEC sentinel: the re-exec'd child carries
    // ZAPX_REEXEC=1, and if that's already set we skip straight to app::run(),
    // which makes an infinite re-exec loop impossible.
    #[cfg(target_os = "linux")]
    if std::env::var_os("ZAPX_REEXEC").is_none() {
        // Collect the (name, value) env changes we need for this launch.
        let mut planned: Vec<(&str, &str)> = Vec::new();

        // IME (all Linux arches): force direct keyboard input unless the user
        // opted back into their session IME via ZAPX_ENABLE_IME.
        if std::env::var_os("ZAPX_ENABLE_IME").is_none() {
            planned.push(("GTK_IM_MODULE", ""));
            planned.push(("QT_IM_MODULE", ""));
        }

        // DMABUF (x86_64 only): keep existing behaviour — only when unset.
        #[cfg(target_arch = "x86_64")]
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            planned.push(("WEBKIT_DISABLE_DMABUF_RENDERER", "1"));
        }

        if !planned.is_empty() {
            use std::os::unix::process::CommandExt;
            if let Ok(exe) = std::env::current_exe() {
                let mut cmd = std::process::Command::new(exe);
                cmd.args(std::env::args_os().skip(1));
                for (k, v) in &planned {
                    cmd.env(k, v);
                }
                // Sentinel so the re-exec'd child short-circuits above and
                // doesn't re-exec again (prevents an infinite loop).
                cmd.env("ZAPX_REEXEC", "1");
                // exec() replaces this process image and only returns on failure.
                let err = cmd.exec();
                // Re-exec failed (rare): set the vars in-process as a best-effort
                // fallback and carry on rather than abort startup. In-process may
                // be too late for WebKit/GTK (same caveat as always), but it's
                // better than nothing.
                eprintln!("zapx: could not re-exec to set launch env ({err}); continuing");
                for (k, v) in &planned {
                    std::env::set_var(k, v);
                }
            }
        }
    }

    app::run();
}
