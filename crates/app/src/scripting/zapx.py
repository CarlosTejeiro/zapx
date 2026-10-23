"""ZAPX scripting API.

ZAPX runs your script with Python 3 and puts this module on its path. Every
call below is carried out by ZAPX on a real terminal tab — you see each
command and its output there — while your script only decides what to do.

    import zapx

    s = zapx.session()                       # the tab focused when the script started
    out = s.run("get system status")         # send, wait for the prompt, return the output
    if "HA mode: standalone" in out:
        zapx.report(s.name, "OK", "standalone")

Passwords never reach the script: ``send_secret("Name.Password")`` asks ZAPX to
type a credential from its vault straight into the session.

With **Dry run** ticked in ZAPX, ``zapx.dry_run`` is True: nothing is sent or
opened, ``run()``/``expect()`` return an empty string and every action is
logged as "would …" instead.
"""

from __future__ import annotations

import json
import os
import socket
import threading
import time
from typing import Callable, List, Optional, Tuple

__all__ = [
    "Error",
    "Timeout",
    "Session",
    "dry_run",
    "session",
    "open",
    "sessions",
    "report",
    "confirm",
    "sleep",
    "log",
]

#: True when ZAPX started the script in dry-run mode.
dry_run: bool = os.environ.get("ZAPX_DRY_RUN") == "1"


class Error(Exception):
    """A request ZAPX could not carry out (no session, unknown name, …)."""


class Timeout(Error):
    """``run()``/``expect()``/``open()`` did not finish in time.

    ``output`` holds whatever the session printed before the deadline.
    """

    def __init__(self, message: str, output: str = "") -> None:
        super().__init__(message)
        self.output = output


# ── transport ────────────────────────────────────────────────────────────────

_lock = threading.Lock()
_sock: Optional[socket.socket] = None
_reader = None
_next_id = 0


def _connect() -> None:
    global _sock, _reader
    port = os.environ.get("ZAPX_PORT")
    token = os.environ.get("ZAPX_TOKEN")
    if not port or not token:
        raise Error("not started by ZAPX (run it from Tools → Run script…)")
    _sock = socket.create_connection(("127.0.0.1", int(port)))
    _reader = _sock.makefile("r", encoding="utf-8", newline="\n")
    _sock.sendall((json.dumps({"token": token}) + "\n").encode("utf-8"))


def _call(method: str, **params):
    """Send one request and wait for its reply (calls are sequential)."""
    global _next_id
    with _lock:
        if _sock is None:
            _connect()
        _next_id += 1
        req = {"id": _next_id, "method": method, "params": params}
        _sock.sendall((json.dumps(req) + "\n").encode("utf-8"))
        line = _reader.readline()
    if not line:
        raise Error("ZAPX closed the connection (script stopped?)")
    reply = json.loads(line)
    if reply.get("ok"):
        return reply.get("result")
    err = reply.get("error") or {}
    message = err.get("message", "unknown error")
    if err.get("kind") == "timeout":
        raise Timeout(message, err.get("output", ""))
    raise Error(message)


# ── sessions ─────────────────────────────────────────────────────────────────


class Session:
    """A live terminal tab driven by the script."""

    def __init__(self, info: dict) -> None:
        self.id: str = info["id"]
        self.name: str = info.get("name", "")
        #: Detected platform key (``"fortigate"``, ``"cisco_ios"``…) or None.
        self.platform: Optional[str] = info.get("platform")

    def __repr__(self) -> str:
        return f"<zapx.Session {self.name!r}>"

    def run(self, command: str, timeout: float = 30) -> str:
        """Type ``command`` + Enter and return its output.

        The output is what the device printed after echoing the command, up
        to (not including) the prompt that ends it. Raises :class:`Timeout`
        when the prompt does not come back within ``timeout`` seconds.
        """
        return _call("run", session=self.id, command=command, timeout=timeout)

    def send(self, text: str, enter: bool = True) -> None:
        """Type ``text`` (and Enter, unless ``enter=False``) without waiting."""
        _call("send", session=self.id, text=text, enter=enter)

    def expect(self, pattern: str, timeout: float = 30, regex: bool = False) -> str:
        """Wait until ``pattern`` appears in the output and return the text up
        to and including it. Output already consumed by an earlier
        ``run()``/``expect()`` is not searched again."""
        return _call(
            "expect", session=self.id, pattern=pattern, timeout=timeout, regex=regex
        )

    def send_secret(self, ref: str, enter: bool = True) -> None:
        """Type a vault credential: ``"Name.Password"`` or ``"Name.Username"``.
        The value goes from ZAPX's vault to the session; the script never
        sees it."""
        _call("send_secret", session=self.id, ref=ref, enter=enter)

    def poll(
        self,
        command: str,
        until: Callable[[str], bool],
        every: float = 5,
        max: float = 60,
        timeout: float = 30,
    ) -> Tuple[bool, str]:
        """Run ``command`` every ``every`` seconds until ``until(output)`` is
        true or ``max`` seconds have passed. Returns ``(ok, last_output)``."""
        deadline = time.monotonic() + max
        while True:
            out = self.run(command, timeout=timeout)
            if until(out):
                return True, out
            if dry_run or time.monotonic() + every > deadline:
                return False, out
            sleep(every)

    def close(self) -> None:
        """Close this tab."""
        _call("close", session=self.id)


def session() -> Session:
    """The tab that was focused when the script started."""
    return Session(_call("session"))


def open(name: str, timeout: float = 60) -> Session:  # noqa: A001 - mirrors "open a session"
    """Open the saved session called ``name`` in a new tab and wait until it
    is connected and its output settles (jump host, vault password and login
    script are handled by ZAPX as for any saved session)."""
    return Session(_call("open", name=name, timeout=timeout))


def sessions(folder: Optional[str] = None) -> List[str]:
    """Names of the saved sessions, or only those in ``folder`` (and its
    sub-folders) when given."""
    return list(_call("sessions", folder=folder))


# ── reporting & interaction ──────────────────────────────────────────────────


def report(target: str, status: str, detail: str = "") -> None:
    """Add a row to the run's summary table, e.g.
    ``report("fw-branch-1", "OK", "SASE_INET 1/1")``."""
    _call("report", target=target, status=status, detail=detail)


def confirm(message: str) -> bool:
    """Ask the user a yes/no question in ZAPX. In dry-run mode the answer is
    always True (nothing will be sent anyway)."""
    return bool(_call("confirm", message=message))


def sleep(seconds: float) -> None:
    """Pause the script. Skipped (only logged) in dry-run mode."""
    if dry_run:
        _call("note", text=f"[dry-run] would wait {seconds:g} s")
        return
    time.sleep(seconds)


def log(*args, sep: str = " ") -> None:
    """Write a line to ZAPX's script panel, in order with the actions around it
    (plain ``print`` also shows up, but may land a little out of order)."""
    _call("note", text=sep.join(str(a) for a in args))
