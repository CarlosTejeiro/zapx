# Python scripting

Macros cover fixed expect / send sequences. When a task needs logic — read a
command's output, decide, loop over devices, wait and re-check — write it as a
Python script and let ZAPX run it.

ZAPX does the work on real terminal tabs: it types each command, waits for the
prompt and hands the output back to the script. You watch every command in the
tab as if you had typed it. The script only holds the logic; connections, jump
hosts, vault passwords and login scripts are handled by ZAPX exactly as for any
saved session.

## Requirements

Python **3.9 or newer** installed on the machine running ZAPX (the bundled
examples use `list[str]` annotations). No packages are needed: ZAPX puts its
`zapx` module on the script's path.

ZAPX looks for, in order: the interpreter set in the run dialog (saved as
setting `scripts.python`), then `py -3` (Windows), `python3`, `python`. The
first one that reports "Python 3" is used; the dialog shows which one.

On Windows, install Python from python.org (tick "Add python.exe to PATH") or
point the dialog at `python.exe`.

## Running a script

**Tools → Run Python script…** picks a `.py` file and opens the run dialog:

- **Dry run** (ticked by default): nothing is sent or opened. `run()` returns
  an empty string, `sleep()` doesn't wait, `confirm()` answers yes, and every
  action is listed in the panel as `[dry-run] would …`. A syntax error or a
  typo in a name shows up straight away, before anything touches a device.
- **Python**: leave empty to auto-detect, or give a path / command.

The script panel (bottom right) shows the status, the summary table the script
fills with `zapx.report()`, and the live log (`print`, errors, `[dry-run]`
lines). **Stop** kills the script; after a dry run, **Run for real** runs it
again with sending enabled. The panel doesn't block the window, so you can
watch and use the tabs while a script runs.

One script runs at a time.

## Examples

**Tools → Script examples…** opens `<data dir>/scripting/examples/`, which ZAPX
fills on first use:

- `hello.py` — the focused tab: runs a command and reports it.
- `sase_reset.py` — resets the SASE IPsec tunnels found down on a folder of
  FortiGates (see below).
- `explore.ipynb` — a notebook to explore a FortiGate cell by cell (see
  [Notebooks](#notebooks-vs-code-jupyterlab)).

Edit them in place or copy them: ZAPX only writes an example that is missing,
so your changes are kept. A copy of `zapx.py` sits next to them, refreshed on
each start, so an editor can autocomplete the API.

### `sase_reset.py`

Edit the settings at the top (`FOLDER`, or `DEVICES`, `TUNNELS`, waits). For
each device it opens the saved session, runs `get system status` and
`get vpn ipsec tunnel summary`, and leaves the device untouched if VDOMs are
enabled or a tunnel in `TUNNELS` doesn't exist (`edit` of a missing name would
create it). Only the tunnels found down are reset: `set status down` on their
interfaces, `diagnose vpn tunnel flush`, `diagnose vpn ike gateway clear name`,
a wait, and `set status up` — always, even if a step before failed. It then
re-checks the summary until every selector is up or `VERIFY_FOR` seconds pass,
and reports OK / NOT UP / FAILED per device. One device failing doesn't stop
the others.

Run it as a dry run first: it lists every command it would send, per device.

## Notebooks (VS Code, JupyterLab)

To work out a script step by step — run a command, look at the real output,
write the next bit against it — connect a notebook to ZAPX instead of running a
file:

1. In ZAPX: **Tools → Allow notebook connection…**. The script panel shows
   "Waiting for a notebook…" and the lines to paste.
2. In the notebook's Python, install `ipykernel` once (VS Code offers to when
   you pick an interpreter without it).
3. First cell:

   ```python
   import zapx
   zapx.connect()
   zapx.tabs()          # what is open in ZAPX
   ```

   Opened from the examples folder (`explore.ipynb`), `import zapx` finds the
   copy there. Anywhere else, paste the `sys.path.insert(…)` line the panel
   shows first.
4. Then, cell by cell: `fw = zapx.tab("fw-branch-1")` (a tab already open) or
   `zapx.open("saved session")`, `out = fw.run("get vpn ipsec tunnel summary")`,
   and work on `out` in the next cell.

How it behaves:

- Same API as scripts. `zapx.session()` is the tab focused *now*.
- **Not a dry run**: every `run()` / `send()` is typed on the device.
- One notebook at a time; a second `connect()` is refused while one is
  connected.
- Stopping a cell (■) makes ZAPX abandon what that call was waiting on; the
  next cell starts clean.
- **Turn off** in the panel ends it. It also turns itself off after 60 minutes
  without notebook calls (setting `notebook.idle_minutes`, `0` = never) and
  when ZAPX closes. After it is turned on again, the next call fails once with
  "ZAPX closed the connection"; run the cell again and it reconnects.
- The connection is local only (`127.0.0.1`), with a new random token each time
  it is turned on, kept in `<data dir>/scripting/connection.json` (readable by
  your user only).

The example `explore.ipynb` (Tools → Script examples…) walks through it on a
FortiGate: list tabs, `get system status`, tunnel summary, parse the selectors.

## API

```python
import zapx

s = zapx.session()                    # the tab focused when the script started
out = s.run("get system status")      # send + Enter, wait for the prompt, return the output
if "HA mode: standalone" in out:
    zapx.report(s.name, "OK", "standalone")
```

| Call | What it does |
|---|---|
| `zapx.dry_run` | `True` when started as a dry run. |
| `zapx.session()` | The focused tab (in a script, call it first to get the one focused when it started). |
| `zapx.open(name, timeout=60)` | Opens the saved session `name` in a new tab, waits until it is connected and its output settles, returns it. |
| `zapx.sessions(folder=None)` | Saved session names, or those in `folder` and its sub-folders. |
| `zapx.tabs()` | The tabs open in ZAPX now. |
| `zapx.tab(name)` | An open tab by its name (as shown on the tab). |
| `zapx.connect(path=None)` | Notebook only: connect to the running ZAPX (see above). |
| `s.run(command, timeout=30)` | Types `command` + Enter, returns what the device printed after the echo, up to the prompt. |
| `s.send(text, enter=True)` | Types `text` without waiting. |
| `s.expect(pattern, timeout=30, regex=False)` | Waits for `pattern` in new output, returns the text up to and including it. |
| `s.send_secret("Name.Password")` | Types a vault field (`Password` or `Username`) into the tab. The value never reaches the script. |
| `s.poll(command, until, every=5, max=60)` | Runs `command` every `every` s until `until(output)` is true or `max` s pass. Returns `(ok, last_output)`. |
| `s.close()` | Closes the tab. |
| `zapx.report(target, status, detail="")` | Adds a row to the summary table. A status starting with `OK` shows green; `FAIL…`, `NOT …`, `ERROR…` red. |
| `zapx.confirm(message)` | Asks yes / no in ZAPX. Always yes in a dry run. |
| `zapx.sleep(seconds)` | Waits (only logged in a dry run). |
| `zapx.log(*args)` | Writes a line to the panel, in order with the actions around it. |

Errors raise `zapx.Error`; `run()`, `expect()` and `open()` raise
`zapx.Timeout` (a subclass) when time runs out, with what was printed so far in
`.output`.

### How `run()` knows a command finished

After sending, ZAPX waits for the command's echo, then for the prompt: the
detected platform's prompt (FortiGate, Cisco…), or — when no platform was
detected, e.g. on Telnet — a last line that looks like a prompt (`host #`,
`host>`, `user@host:~$`) after 400 ms of quiet, or otherwise 2 s of quiet.
A FortiOS / Cisco `--More--` pager is answered with a space and stripped from
the output. For a command that prints slowly with pauses, or ends with an
unusual prompt, use `send()` + `expect()` instead.

## Security

- Scripts are ordinary Python running with your user's rights. Only run
  scripts you wrote or read.
- Never put passwords in a script: store them in the vault and use
  `send_secret()`, or let the saved session log in.
- ZAPX talks to the script over a local socket (`127.0.0.1`) with a random
  per-run token; other programs can't drive your tabs through it.
