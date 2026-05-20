#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;

use crate::error::Error;
use crate::SessionCmd;

// ---------------------------------------------------------------------------
// Telnet option bytes (RFC 854 / RFC 1091 / RFC 1073)
// ---------------------------------------------------------------------------

const IAC: u8 = 255; // Interpret As Command
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250; // Subnegotiation begin
const SE: u8 = 240; // Subnegotiation end

const OPT_ECHO: u8 = 1;
const OPT_SGA: u8 = 3; // Suppress Go Ahead
const OPT_TTYPE: u8 = 24; // Terminal Type
const OPT_NAWS: u8 = 31; // Negotiate About Window Size

const TTYPE_SEND: u8 = 1; // subneg subcommand: server asking us to send

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

fn naws_subneg(cols: u16, rows: u16) -> Vec<u8> {
    vec![
        IAC,
        SB,
        OPT_NAWS,
        (cols >> 8) as u8,
        (cols & 0xff) as u8,
        (rows >> 8) as u8,
        (rows & 0xff) as u8,
        IAC,
        SE,
    ]
}

fn ttype_is_response() -> Vec<u8> {
    let ttype = b"xterm-256color";
    let mut v = vec![IAC, SB, OPT_TTYPE, 0u8]; // 0 = IS
    v.extend_from_slice(ttype);
    v.extend_from_slice(&[IAC, SE]);
    v
}

// ---------------------------------------------------------------------------
// IAC state machine
// ---------------------------------------------------------------------------

enum IacState {
    Data,
    Iac,
    IacVerb(u8), // saw IAC + WILL/WONT/DO/DONT, waiting for option byte
    IacSb,       // saw IAC SB, waiting for option byte
    IacSbData(u8, Vec<u8>),
    IacSbIac(u8, Vec<u8>), // saw IAC inside SB data (expect SE)
}

fn process_bytes(
    input: &[u8],
    state: &mut IacState,
    clean: &mut Vec<u8>,
    responses: &mut Vec<Vec<u8>>,
    cols: u16,
    rows: u16,
) {
    for &b in input {
        let cur = std::mem::replace(state, IacState::Data);
        *state = match cur {
            IacState::Data => {
                if b == IAC {
                    IacState::Iac
                } else {
                    clean.push(b);
                    IacState::Data
                }
            }
            IacState::Iac => match b {
                IAC => {
                    clean.push(IAC); // escaped IAC in data stream
                    IacState::Data
                }
                WILL | WONT | DO | DONT => IacState::IacVerb(b),
                SB => IacState::IacSb,
                _ => IacState::Data, // NOP and other single-byte commands
            },
            IacState::IacVerb(verb) => {
                handle_verb(verb, b, responses, cols, rows);
                IacState::Data
            }
            IacState::IacSb => IacState::IacSbData(b, vec![]),
            IacState::IacSbData(opt, mut data) => {
                if b == IAC {
                    IacState::IacSbIac(opt, data)
                } else {
                    data.push(b);
                    IacState::IacSbData(opt, data)
                }
            }
            IacState::IacSbIac(opt, data) => {
                if b == SE {
                    handle_subneg(opt, &data, responses);
                    IacState::Data
                } else if b == IAC {
                    IacState::IacSbIac(opt, data) // absorb stray IAC
                } else {
                    let mut d = data;
                    d.push(IAC);
                    d.push(b);
                    IacState::IacSbData(opt, d)
                }
            }
        };
    }
}

fn handle_verb(verb: u8, opt: u8, responses: &mut Vec<Vec<u8>>, cols: u16, rows: u16) {
    match verb {
        WILL => match opt {
            OPT_ECHO | OPT_SGA => responses.push(vec![IAC, DO, opt]),
            _ => responses.push(vec![IAC, DONT, opt]),
        },
        DO => match opt {
            OPT_NAWS => {
                responses.push(vec![IAC, WILL, OPT_NAWS]);
                responses.push(naws_subneg(cols, rows));
            }
            OPT_TTYPE => {
                responses.push(vec![IAC, WILL, OPT_TTYPE]);
            }
            _ => responses.push(vec![IAC, WONT, opt]),
        },
        WONT => responses.push(vec![IAC, DONT, opt]),
        DONT => responses.push(vec![IAC, WONT, opt]),
        _ => {}
    }
}

fn handle_subneg(opt: u8, data: &[u8], responses: &mut Vec<Vec<u8>>) {
    if opt == OPT_TTYPE && data.first() == Some(&TTYPE_SEND) {
        responses.push(ttype_is_response());
    }
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

pub struct TelnetTransport {
    stream: TcpStream,
}

impl TelnetTransport {
    pub async fn connect(host: String, port: u16) -> Result<Self, Error> {
        let stream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(|_| Error::ConnectionFailed)?;
        Ok(Self { stream })
    }

    pub fn start_io_loop<F>(self, cols: u16, rows: u16, on_data: F) -> UnboundedSender<SessionCmd>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<SessionCmd>();
        let (iac_tx, mut iac_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let (mut read_half, mut write_half) = self.stream.into_split();
        let size = Arc::new(Mutex::new((cols, rows)));
        let size_r = Arc::clone(&size);

        // Writer task: merges application data + IAC responses from reader
        tokio::spawn(async move {
            // Send initial option proposals so network gear gets terminal info early.
            let init: &[&[u8]] = &[
                &[IAC, WILL, OPT_TTYPE],
                &[IAC, WILL, OPT_NAWS],
                &[IAC, DO, OPT_SGA],
                &[IAC, DO, OPT_ECHO],
            ];
            for pkt in init {
                if write_half.write_all(pkt).await.is_err() {
                    return;
                }
            }
            // Send initial window size proactively (some equipment skips the negotiation).
            let (c, r) = *size.lock().unwrap();
            if write_half.write_all(&naws_subneg(c, r)).await.is_err() {
                return;
            }

            loop {
                tokio::select! {
                    cmd = cmd_rx.recv() => match cmd {
                        None => break,
                        Some(SessionCmd::Data(data)) => {
                            if write_half.write_all(&data).await.is_err() { break; }
                        }
                        Some(SessionCmd::Resize { cols, rows }) => {
                            *size.lock().unwrap() = (cols, rows);
                            if write_half.write_all(&naws_subneg(cols, rows)).await.is_err() { break; }
                        }
                    },
                    resp = iac_rx.recv() => match resp {
                        None => break,
                        Some(bytes) => {
                            if write_half.write_all(&bytes).await.is_err() { break; }
                        }
                    },
                }
            }
        });

        // Reader task: strips IAC sequences, feeds clean data to on_data callback
        tokio::spawn(async move {
            let mut iac_state = IacState::Data;
            let mut buf = vec![0u8; 4096];
            loop {
                match read_half.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut clean = Vec::new();
                        let mut responses = Vec::new();
                        let (c, r) = *size_r.lock().unwrap();
                        process_bytes(&buf[..n], &mut iac_state, &mut clean, &mut responses, c, r);
                        for resp in responses {
                            iac_tx.send(resp).ok();
                        }
                        if !clean.is_empty() {
                            on_data(clean);
                        }
                    }
                }
            }
        });

        cmd_tx
    }
}
