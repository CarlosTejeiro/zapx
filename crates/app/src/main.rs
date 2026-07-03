// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux/WebKitGTK IME fixup.
    //
    // On Wayland (reproduced on Fedora 44 + KDE Plasma), WebKitGTK mishandles
    // the IBus/Fcitx input-method system and garbles terminal typing —
    // duplicated/ghost characters, "crazy" input per keystroke. Launching with
    // GTK_IM_MODULE="" and QT_IM_MODULE="" (empty, i.e. direct keyboard input,
    // no IME) fixes it. This must be in the environment *before* WebKit
    // initializes (setting it in-process is too late), so we re-exec ourselves
    // once with those vars set. We force it off by default because it's the
    // primary fix; a user who needs composed/CJK/emoji input opts out with
    // ZAPX_ENABLE_IME=1, which keeps the session's IME modules.
    //
    // We deliberately do NOT force WEBKIT_DISABLE_DMABUF_RENDERER anymore.
    // Disabling the DMABUF renderer was a workaround for a busy-repaint/high-CPU
    // issue on the NVIDIA *proprietary* driver, but forcing it off broke
    // rendering on other setups — aarch64/ARM, and x86_64 hybrid laptops running
    // on Intel/nouveau under Wayland (NVIDIA present but its driver not loaded).
    // It only helped NVIDIA-proprietary and hurt everyone else, and "is NVIDIA
    // present?" is not a reliable guard (a hybrid laptop has an NVIDIA GPU that
    // isn't in use). So it's opt-in: a user hitting the high-CPU issue exports
    // WEBKIT_DISABLE_DMABUF_RENDERER=1 themselves (it's inherited into the
    // re-exec'd child).
    //
    // Loop prevention: we can't guard the IME change on "GTK_IM_MODULE unset?"
    // because Fedora/KDE *sets* GTK_IM_MODULE=ibus in the session, so we must
    // OVERRIDE it — its presence can't be the guard. Instead a dedicated
    // ZAPX_REEXEC sentinel: the re-exec'd child carries ZAPX_REEXEC=1, and if
    // that's already set we skip straight to app::run(), making an infinite
    // re-exec loop impossible.
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
