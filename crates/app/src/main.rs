// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux/WebKitGTK: on many NVIDIA setups (and generally under Wayland) the
    // DMABUF renderer drives WebKitWebProcess into a busy repaint loop — it
    // burns a CPU core even when idle. Disabling it fixes that, BUT the variable
    // has to be in the environment *before* WebKit initializes; setting it from
    // inside the process (after the web/GPU process is already spawned) is too
    // late and has no effect — which is why launching with
    // `WEBKIT_DISABLE_DMABUF_RENDERER=1 zapx` works but an in-process set_var
    // didn't. So re-exec ourselves once with the variable set, so the real run
    // starts with it already present. The guard makes this a no-op on the
    // re-exec'd process and whenever the user picked their own value.
    //
    // IMPORTANT: only auto-disable on x86_64. On aarch64 (ARM) WebKitGTK,
    // forcing the DMABUF renderer off pushes rendering onto a fallback path
    // that on several ARM GPUs/VMs paints garbage and swallows keyboard input
    // (reported as "the terminal goes crazy and you can't type"). The high-CPU
    // problem this works around is an x86/NVIDIA/Wayland issue, so ARM keeps
    // WebKit's default. An ARM user who still wants it off can export
    // WEBKIT_DISABLE_DMABUF_RENDERER=1 themselves before launch.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        use std::os::unix::process::CommandExt;
        if let Ok(exe) = std::env::current_exe() {
            // exec() replaces this process image and only returns on failure.
            let err = std::process::Command::new(exe)
                .args(std::env::args_os().skip(1))
                .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
                .exec();
            // Re-exec failed (rare): set it in-process as a best-effort fallback
            // and carry on rather than abort startup.
            eprintln!(
                "zapx: could not re-exec to set WEBKIT_DISABLE_DMABUF_RENDERER ({err}); continuing"
            );
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    app::run();
}
