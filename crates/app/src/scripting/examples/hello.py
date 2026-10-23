"""Smallest ZAPX script: look at the focused tab and run one command.

Run it from Tools → Run Python script…, with Dry run ticked first: nothing is
sent, the panel lists what would be. Untick it to run for real.
"""

import zapx

s = zapx.session()  # the tab that was focused when the script started
zapx.log(f"Tab: {s.name} — platform: {s.platform or 'unknown'}")

# run() types the command + Enter, waits for the prompt and returns the output.
out = s.run("get system status" if s.platform == "fortigate" else "uname -a")

first_line = out.splitlines()[0] if out else "(dry run: no output)"
zapx.report(s.name, "OK", first_line)
