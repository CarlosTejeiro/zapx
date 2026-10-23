# Automating network devices with ZAPX

A step-by-step guide for people who have never used ZAPX. By the end you will
have saved a device, run a ready-made Python script against it (first as a dry
run, then for real) and explored a device from a notebook.

| Part | What you'll do | Time |
|---|---|---|
| [1. Getting started](1-getting-started.md) | Install, save your first device, store its password in the vault, log in automatically. | 10 min |
| [2. Python scripts](2-scripts.md) | What a script is, run `hello.py`, then `sase_reset.py` step by step — dry run first. | 15 min |
| [3. Notebooks](3-notebooks.md) | Connect VS Code or JupyterLab to ZAPX and explore a device cell by cell. | 15 min |

## The idea in one picture

```text
 your script / notebook                        ZAPX                       devices
 ┌──────────────────────┐   "run get system  ┌──────────────────┐  types  ┌──────────┐
 │ import zapx          │ ─── status on ───▶ │ a real terminal  │ ──────▶ │ FortiGate│
 │ out = fw.run(...)    │     fw-branch-1"   │ tab you can see  │         │  switch  │
 │ if "down" in out: …  │ ◀── the output ─── │                  │ ◀────── │  router  │
 └──────────────────────┘                    └──────────────────┘         └──────────┘
```

- **The script decides, ZAPX does.** Your Python code never opens a connection
  itself: ZAPX types every command in a normal tab, so you watch it happen and
  can take over at any moment.
- **Connections are ZAPX's job.** Jump hosts, passwords from the vault, login
  banners and host keys are handled exactly as when you open the session by
  hand.
- **Passwords never reach your code.** A script can ask ZAPX to type a vault
  password into a session, but it can't read it.

## Before you start

- **ZAPX 0.26.0 or newer** (notebooks need 0.26.0; scripts 0.25.0).
- **Python 3.9 or newer** on the same computer, for parts 2 and 3:
  - Linux: usually already there — check with `python3 --version`.
  - Windows: install it from [python.org](https://www.python.org/downloads/)
    and tick **"Add python.exe to PATH"** in the installer.
  - macOS: install it from [python.org](https://www.python.org/downloads/).
- A device to try it on. The screenshots use two FortiGates called
  `fw-branch-1` and `fw-branch-2` in a folder called `SASE`; any device you can
  reach works for parts 1 and 3.

## Reference

- [Python scripting reference](../scripting.md) — every `zapx` call, how
  `run()` detects the end of a command, notebook details, security.
- [User guide](../README.md) — every ZAPX feature (sessions, vault, macros,
  SFTP, splits, …).
