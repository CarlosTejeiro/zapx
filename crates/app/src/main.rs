// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux/WebKitGTK launch-environment fixups.
    //
    // Two Wayland problems make the terminal unusable on some Linux setups
    // (both reproduced on Fedora + KDE Plasma + Wayland). Both must be fixed in
    // the environment *before* WebKit/GTK initializes — setting them in-process
    // is too late — so we re-exec ourselves once with the vars set. A dedicated
    // ZAPX_REEXEC sentinel on the child prevents an infinite re-exec loop
    // (we can't guard on the vars themselves because the session pre-sets some
    // of them, e.g. GTK_IM_MODULE=ibus, so their presence isn't a reliable
    // guard — we must override).
    //
    // 1. GDK backend. WebKitGTK's *native-Wayland* input handling duplicates
    //    keystrokes in the terminal ("crazy"/ghost characters, one input per
    //    keypress). This is independent of the NVIDIA driver — it reproduces
    //    both with the proprietary driver installed and without it — and it is
    //    NOT the IME issue below: it persists even with the IME modules emptied.
    //    Running under XWayland (GDK_BACKEND=x11) fixes it completely. XWayland
    //    is present in every Wayland session, so forcing x11 there is a safe
    //    default. We only touch this on a Wayland session (on native X11 it's
    //    already x11 and there's no bug), we respect a user-set GDK_BACKEND, and
    //    a user who wants native Wayland opts out with ZAPX_ENABLE_WAYLAND=1.
    //
    // 2. IME. WebKitGTK also mishandles the IBus/Fcitx input-method system on
    //    Wayland and garbles typing. Launching with GTK_IM_MODULE="" and
    //    QT_IM_MODULE="" (empty, i.e. direct keyboard input, no IME) fixes that
    //    part. Forced off by default; a user who needs composed/CJK/emoji input
    //    opts out with ZAPX_ENABLE_IME=1.
    //
    // We deliberately do NOT force WEBKIT_DISABLE_DMABUF_RENDERER. Disabling the
    // DMABUF renderer was a workaround for a busy-repaint/high-CPU issue on the
    // NVIDIA *proprietary* driver, but forcing it off broke rendering on other
    // setups (ARM, and x86_64 hybrid laptops on Intel/nouveau). It's opt-in: a
    // user hitting the high-CPU issue exports WEBKIT_DISABLE_DMABUF_RENDERER=1
    // themselves (it's inherited into the re-exec'd child).
    //
    // Escape hatch: ZAPX_NO_LAUNCH_ENV=1 skips ALL of the above (no re-exec, no
    // env overrides), so a user can isolate whether one of these fixups is
    // causing a problem on their machine.
    #[cfg(target_os = "linux")]
    if std::env::var_os("ZAPX_REEXEC").is_none() && std::env::var_os("ZAPX_NO_LAUNCH_ENV").is_none()
    {
        // Collect the (name, value) env changes we need for this launch.
        let mut planned: Vec<(&str, &str)> = Vec::new();

        // GDK backend: force XWayland on a Wayland session unless the user set
        // GDK_BACKEND themselves or opted back into native Wayland.
        let on_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|v| v.eq_ignore_ascii_case("wayland"))
                .unwrap_or(false);
        if on_wayland
            && std::env::var_os("GDK_BACKEND").is_none()
            && std::env::var_os("ZAPX_ENABLE_WAYLAND").is_none()
        {
            planned.push(("GDK_BACKEND", "x11"));
        }

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
