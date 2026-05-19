# zapx

> Modern terminal for network engineers.

**Status: pre-alpha — no usable code yet.**

zapx is a multi-protocol terminal client (SSH · Telnet · Serial) built for network engineers who
need a lightweight, fast, and modern replacement for SecureCRT, MobaXterm, or PuTTY. It is free
and open source under Apache 2.0.

---

## What it is

- A desktop terminal application for Windows (macOS and Linux planned).
- SSH2, Telnet, and Serial (COM/TTY) support in one app.
- Session manager with folder tree and drag-and-drop.
- Keyword highlighting for network device output (Cisco, Juniper, Arista, and others).
- Full session logging with rotation and search.
- Customisable themes and fonts.

## What it is not

- A general-purpose terminal emulator (Kitty, Alacritty, WezTerm).
- A cloud-managed session tool.
- Production-ready software yet.

## Building from source

Prerequisites: Rust stable (≥ 1.80), Node.js LTS, pnpm.

```sh
cargo xtask dev
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development setup.

## License

Apache 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
