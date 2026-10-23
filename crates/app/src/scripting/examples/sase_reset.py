"""Reset the SASE IPsec tunnels found down on a group of FortiGates.

Edit the settings below, then run it from Tools → Run Python script… — keep
Dry run ticked the first time: nothing is sent or opened, the panel lists every
command it would type. Untick it to run for real.

The devices are ZAPX saved sessions (all those in FOLDER, or the names in
DEVICES). ZAPX handles the connection like for any saved session: jump host,
password from the vault, login script (e.g. answering a login disclaimer), host
keys. This script only holds the logic.

On each device:
1. `get system status` + `get vpn ipsec tunnel summary`. A device with VDOMs,
   or without every tunnel in TUNNELS, is left untouched (`edit` of a missing
   name would create it).
2. Only for the tunnels found down: `set status down` on their interfaces,
   flush the tunnel and clear its IKE gateway, wait, then `set status up`
   (always, even if a step before failed).
3. Check `get vpn ipsec tunnel summary` every VERIFY_EVERY s, up to VERIFY_FOR s.
A tunnel is down when not every selector is up: `selectors(total,up): 1/0`.
"""

import re

import zapx

# ── settings ────────────────────────────────────────────────────────────────
FOLDER = "SASE"           # saved sessions in this folder (and its sub-folders)…
DEVICES: list[str] = []   # …or list saved-session names here instead
TUNNELS = ("SASE_INET", "SASE_INET2")
AFTER_DOWN = 1            # seconds after `set status down`
AFTER_FLUSH = 30          # seconds after the flush / IKE clear
VERIFY_FOR = 60           # check the tunnels for up to this long…
VERIFY_EVERY = 5          # …every this many seconds
CLOSE_TABS = True         # close each device's tab when done


def selectors(summary: str, tunnel: str):
    """(total, up) for `tunnel` in a tunnel summary, or None if absent."""
    m = re.search(rf"'{re.escape(tunnel)}'.*?selectors\(total,up\):\s*(\d+)/(\d+)", summary)
    return (int(m.group(1)), int(m.group(2))) if m else None


def is_up(summary: str, tunnel: str) -> bool:
    sel = selectors(summary, tunnel)
    return bool(sel) and sel[0] > 0 and sel[0] == sel[1]


def describe(summary: str) -> str:
    return ", ".join(f"{t} {selectors(summary, t) or '?'}" for t in TUNNELS)


def set_status(s: zapx.Session, tunnels, status: str) -> None:
    s.run("config system interface")
    for t in tunnels:
        s.run(f"edit {t}")
        s.run(f"set status {status}")
        s.run("next")
    s.run("end")


def reset_device(name: str) -> None:
    s = zapx.open(name)
    try:
        status = s.run("get system status")
        if not zapx.dry_run and "Virtual domain configuration: disable" not in status:
            zapx.report(name, "LEFT UNTOUCHED", "VDOMs enabled (or unexpected status)")
            return
        summary = s.run("get vpn ipsec tunnel summary")
        missing = [t for t in TUNNELS if selectors(summary, t) is None]
        if missing and not zapx.dry_run:
            zapx.report(name, "LEFT UNTOUCHED", f"no tunnel {', '.join(missing)}")
            return
        down = [t for t in TUNNELS if not is_up(summary, t)]
        if not down:
            zapx.report(name, "OK — already up, nothing sent", describe(summary))
            return

        set_status(s, down, "down")
        try:
            zapx.sleep(AFTER_DOWN)
            for t in down:
                s.run(f"diagnose vpn tunnel flush {t}")
                s.run(f"diagnose vpn ike gateway clear name {t}")
            zapx.sleep(AFTER_FLUSH)
        finally:
            set_status(s, down, "up")  # always bring them back up

        ok, last = s.poll(
            "get vpn ipsec tunnel summary",
            until=lambda out: all(is_up(out, t) for t in TUNNELS),
            every=VERIFY_EVERY,
            max=VERIFY_FOR,
        )
        if zapx.dry_run:
            zapx.report(name, "DRY RUN — nothing sent", f"would reset {', '.join(down)}")
        else:
            zapx.report(name, "OK" if ok else f"NOT UP after {VERIFY_FOR} s", describe(last))
    finally:
        if CLOSE_TABS:
            s.close()


devices = DEVICES or zapx.sessions(folder=FOLDER)
if not devices:
    raise SystemExit(f'No devices: put saved sessions in folder "{FOLDER}" or fill DEVICES.')
zapx.log(f"{len(devices)} device(s): {', '.join(devices)}")
if not zapx.confirm(f"Reset the SASE tunnels found down on {len(devices)} device(s)?"):
    raise SystemExit("Cancelled.")

for name in devices:
    try:
        reset_device(name)
    except zapx.Error as e:  # one device failing doesn't stop the others
        zapx.report(name, "FAILED", str(e))
