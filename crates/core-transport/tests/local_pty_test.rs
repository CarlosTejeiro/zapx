use std::io::{Read, Write};
use std::time::Duration;

use core_transport::LocalPty;

/// Spawn a local PTY, write a command, and verify the output contains expected text.
///
/// This test is intentionally simple: it validates the happy path of spawn + read + write.
/// It is marked `ignore` on CI because it requires a real PTY allocation; run locally with
/// `cargo test -p core-transport -- --ignored`.
#[test]
#[ignore = "requires PTY allocation (run locally, not in CI containers)"]
fn local_pty_spawn_and_echo() {
    let pty = LocalPty::spawn(80, 24).expect("spawn failed");

    let mut reader = pty.take_reader().expect("take_reader failed");
    let writer = pty.writer();

    // Give the shell a moment to print its prompt.
    std::thread::sleep(Duration::from_millis(200));

    // Drain any prompt bytes already in the buffer.
    let mut buf = vec![0u8; 4096];
    let _ = reader.read(&mut buf);

    // Send a command and a newline.
    {
        let mut w = writer.lock().unwrap();
        w.write_all(b"echo zapx_marker\n").expect("write failed");
        w.flush().expect("flush failed");
    }

    // Read output for up to 2 seconds.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut output = String::new();
    while std::time::Instant::now() < deadline {
        let n = reader.read(&mut buf).unwrap_or(0);
        if n > 0 {
            output.push_str(&String::from_utf8_lossy(&buf[..n]));
            if output.contains("zapx_marker") {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(
        output.contains("zapx_marker"),
        "did not find 'zapx_marker' in PTY output.\nGot: {:?}",
        output
    );
}
