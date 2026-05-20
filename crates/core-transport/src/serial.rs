#![forbid(unsafe_code)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::UnboundedSender;
use tokio_serial::{DataBits, FlowControl, Parity, SerialPortBuilderExt, StopBits};

use crate::error::Error;
use crate::SessionCmd;

// ---------------------------------------------------------------------------
// Port enumeration (synchronous — called from blocking context or at startup)
// ---------------------------------------------------------------------------

pub fn list_ports() -> Vec<String> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.port_name)
        .collect()
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

pub struct SerialTransport {
    port: tokio_serial::SerialStream,
}

impl SerialTransport {
    /// Open a serial port with common 8N1 settings.
    pub fn open(device: &str, baud_rate: u32) -> Result<Self, Error> {
        let port = tokio_serial::new(device, baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .open_native_async()?;
        Ok(Self { port })
    }

    pub fn start_io_loop<F>(self, on_data: F) -> UnboundedSender<SessionCmd>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<SessionCmd>();
        let (mut read_half, mut write_half) = tokio::io::split(self.port);

        // Writer task
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    SessionCmd::Data(data) => {
                        if write_half.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    // Serial ports have no PTY resize concept — silently ignore.
                    SessionCmd::Resize { .. } => {}
                }
            }
        });

        // Reader task
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match read_half.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => on_data(buf[..n].to_vec()),
                }
            }
        });

        cmd_tx
    }
}
