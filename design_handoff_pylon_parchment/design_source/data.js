// Shared sample data — router/switch CLI output, sessions, snippets.
// This is realistic-looking output for FortiOS, Cisco IOS, and Linux,
// formatted with ANSI-style tokens that the Terminal component renders.

// Each line: { t: 'text type', s: 'string' }
// Types: cmd, prompt, ok, warn, err, dim, head, key, val, comment, blank, link, h1, h2, num

window.PYLON = window.PYLON || {};

window.PYLON.sessions = [
  {
    id: "favorites",
    label: "★ Favorites",
    pinned: true,
    items: [
      { id: "fmg-sophia", name: "10.5.201.175", tag: "fmg", host: "10.5.201.175", proto: "ssh", user: "admin", color: "#22d3ee", status: "active" },
      { id: "fwf40f", name: "10.5.192.12", tag: "FWF40F", host: "10.5.192.12", proto: "ssh", user: "admin", color: "#f472b6", status: "idle" },
      { id: "core01", name: "core-sw-01", tag: "cisco", host: "10.5.0.1", proto: "ssh", user: "netops", color: "#a78bfa", status: "idle" },
    ],
  },
  {
    id: "lab-local",
    label: "LAB-LOCAL",
    items: [
      { id: "fmg-local", name: "192.168.1.100", tag: "FMG", host: "192.168.1.100", proto: "ssh", user: "admin", color: "#f59e0b" },
      { id: "fgt-local", name: "192.168.1.101", tag: "FGT", host: "192.168.1.101", proto: "ssh", user: "admin", color: "#ef4444" },
      { id: "proxmox", name: "Proxmox", tag: "pve", host: "192.168.1.50", proto: "ssh", user: "root", color: "#10b981" },
      { id: "ubuntu-kobe", name: "Ubuntu", tag: "kobe", host: "192.168.1.42", proto: "ssh", user: "ubuntu", color: "#f97316" },
    ],
  },
  {
    id: "lab-sophia",
    label: "LAB-SOPHIA",
    items: [
      { id: "fwf40f-s", name: "10.5.192.12", tag: "FWF40F", host: "10.5.192.12", proto: "ssh", user: "admin", color: "#f472b6" },
      { id: "fmg-sophia2", name: "10.5.201.175", tag: "fmg", host: "10.5.201.175", proto: "ssh", user: "admin", color: "#22d3ee", status: "active" },
      { id: "fgt-59-7", name: "10.5.59.7", tag: "admin", host: "10.5.59.7", proto: "ssh", user: "admin", color: "#84cc16" },
      { id: "fgt-59-97", name: "10.5.59.97", tag: "admin", host: "10.5.59.97", proto: "ssh", user: "admin", color: "#84cc16" },
    ],
  },
  {
    id: "datacenter",
    label: "DC-MADRID",
    items: [
      { id: "core-01", name: "core-sw-01", tag: "cisco", host: "10.5.0.1", proto: "ssh", user: "netops", color: "#a78bfa" },
      { id: "core-02", name: "core-sw-02", tag: "cisco", host: "10.5.0.2", proto: "ssh", user: "netops", color: "#a78bfa" },
      { id: "edge-01", name: "edge-rtr-01", tag: "cisco", host: "10.5.10.1", proto: "ssh", user: "netops", color: "#a78bfa" },
      { id: "fw-01", name: "fw-perimeter", tag: "FGT100F", host: "10.5.20.1", proto: "ssh", user: "admin", color: "#ef4444" },
    ],
  },
];

// Realistic terminal transcripts
window.PYLON.transcripts = {
  fortimanager: [
    { t: "banner", s: "PYLON SSH · session established · admin@10.5.201.175:22" },
    { t: "ok",     s: "✓ key exchange: curve25519-sha256" },
    { t: "ok",     s: "✓ host fingerprint matched (SHA256:k8/3Hq…)" },
    { t: "ok",     s: "✓ compression: zlib@openssh.com" },
    { t: "blank",  s: "" },
    { t: "warn",   s: "Warning: Image is Not Certified" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "FMG-Sophia-LAB # " , cmd: "get system status" },
    { t: "kv", k: "Platform Type",            v: "FMG-VM64-KVM" },
    { t: "kv", k: "Platform Full Name",       v: "FortiManager-VM64-KVM" },
    { t: "kv", k: "Version",                  v: "v7.6.7-build3713 260410 (Interim)" },
    { t: "kv", k: "Serial Number",            v: "FMGVMSTM25001181" },
    { t: "kv", k: "BIOS version",             v: "04000002" },
    { t: "kv", k: "Hostname",                 v: "FMG-Sophia-LAB" },
    { t: "kv", k: "Max Admin Domains",        v: "505" },
    { t: "kv", k: "Max Device Groups",        v: "10000" },
    { t: "kv", k: "Admin Domain Config",      v: "Enabled",     vt: "ok" },
    { t: "kv", k: "FIPS Mode",                v: "Disabled",    vt: "dim" },
    { t: "kv", k: "HA Mode",                  v: "Stand Alone" },
    { t: "kv", k: "Branch Point",             v: "3713" },
    { t: "kv", k: "Current Time",             v: "Thu May 21 14:16:32 CEST 2026" },
    { t: "kv", k: "Time Zone",                v: "(GMT+1:00) Madrid, Paris" },
    { t: "kv", k: "x86-64 Applications",      v: "Yes",         vt: "ok" },
    { t: "kv", k: "Disk Usage",               v: "Free 109.85GB, Total 117.55GB" },
    { t: "kv", k: "File System",              v: "Ext4" },
    { t: "kv", k: "License Status",           v: "Valid",       vt: "ok" },
    { t: "kv", k: "Image Signature",          v: "Warning: Image is Not Certified", vt: "warn" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "FMG-Sophia-LAB # ", cmd: "diagnose system print uptime" },
    { t: "out",    s: "  14:16:35 up 8 days, 22:14, 1 user, load average: 0.21, 0.18, 0.13" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "FMG-Sophia-LAB # ", cur: true },
  ],

  ciscoCore: [
    { t: "banner", s: "PYLON SSH · session established · netops@core-sw-01:22" },
    { t: "ok",     s: "✓ key exchange: curve25519-sha256" },
    { t: "ok",     s: "✓ algo: chacha20-poly1305@openssh" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "core-sw-01# ", cmd: "show ip interface brief" },
    { t: "table",
      head: ["Interface","IP-Address","OK?","Method","Status","Protocol"],
      rows: [
        ["Vlan1",         "10.5.0.1",     "YES","NVRAM","up","up"],
        ["Vlan10",        "10.5.10.254",  "YES","NVRAM","up","up"],
        ["Vlan20",        "10.5.20.254",  "YES","NVRAM","up","up"],
        ["Vlan30",        "10.5.30.254",  "YES","NVRAM","up","up"],
        ["Te1/0/1",       "unassigned",   "YES","unset","up","up"],
        ["Te1/0/2",       "unassigned",   "YES","unset","up","up"],
        ["Te1/0/24",      "unassigned",   "YES","unset","down","down"],
        ["Po1",           "unassigned",   "YES","unset","up","up"],
        ["Lo0",           "10.255.255.1", "YES","NVRAM","up","up"],
      ]
    },
    { t: "blank",  s: "" },
    { t: "prompt", s: "core-sw-01# ", cmd: "show version | inc Cisco" },
    { t: "out",    s: "Cisco IOS XE Software, Version 17.09.04a" },
    { t: "out",    s: "Cisco Catalyst 9300-48P Switch (rev D0) with 8388608K bytes of memory." },
    { t: "blank",  s: "" },
    { t: "prompt", s: "core-sw-01# ", cur: true },
  ],

  ciscoEdge: [
    { t: "banner", s: "PYLON SSH · session established · netops@edge-rtr-01:22" },
    { t: "ok",     s: "✓ MFA accepted (TOTP)" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "edge-rtr-01# ", cmd: "show ip bgp summary" },
    { t: "out",    s: "BGP router identifier 10.255.255.10, local AS number 65001" },
    { t: "out",    s: "BGP table version is 184729, main routing table version 184729" },
    { t: "out",    s: "892341 network entries using 213 MB of memory" },
    { t: "blank",  s: "" },
    { t: "table",
      head: ["Neighbor","V","AS","MsgRcvd","MsgSent","Up/Down","State/PfxRcd"],
      rows: [
        ["10.0.0.1",   "4","65000","82931", "12042", "5w2d", "489102"],
        ["10.0.0.5",   "4","65002","91823", "13049", "3w1d", "382910"],
        ["10.0.0.9",   "4","65003","48291", "9023",  "1d4h", "20281"],
        ["10.0.0.13",  "4","65004","12",    "98",    "00:02:18", "Idle"],
      ]
    },
    { t: "blank",  s: "" },
    { t: "prompt", s: "edge-rtr-01# ", cmd: "ping 8.8.8.8 repeat 5" },
    { t: "out",    s: "Type escape sequence to abort." },
    { t: "out",    s: "Sending 5, 100-byte ICMP Echos to 8.8.8.8, timeout is 2 seconds:" },
    { t: "out",    s: "!!!!!" },
    { t: "ok",     s: "Success rate is 100 percent (5/5), round-trip min/avg/max = 4/6/12 ms" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "edge-rtr-01# ", cur: true },
  ],

  linux: [
    { t: "banner", s: "PYLON SSH · session established · root@proxmox-01:22" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "root@pve:~# ", cmd: "uptime && free -h" },
    { t: "out",    s: " 14:16:48 up 47 days,  3:21,  2 users,  load average: 0.42, 0.51, 0.48" },
    { t: "blank",  s: "" },
    { t: "table",
      head: ["",       "total","used","free","shared","buff/cache","available"],
      rows: [
        ["Mem:",  "62Gi","41Gi","2.1Gi","892Mi","19Gi","20Gi"],
        ["Swap:", "8.0Gi","312Mi","7.7Gi","","",""],
      ]
    },
    { t: "blank",  s: "" },
    { t: "prompt", s: "root@pve:~# ", cmd: "qm list | head" },
    { t: "table",
      head: ["VMID","NAME","STATUS","MEM","BOOTDISK","PID"],
      rows: [
        ["100","fmg-sophia","running","8192","60.00","48291"],
        ["101","fgt-edge",  "running","4096","32.00","48512"],
        ["102","ubuntu-dev","running","16384","120.00","48733"],
        ["103","win11-test","stopped","8192","80.00","-"],
      ]
    },
    { t: "blank",  s: "" },
    { t: "prompt", s: "root@pve:~# ", cur: true },
  ],

  multiexec: [
    { t: "banner", s: "PYLON MultiExec · broadcasting to 4 sessions" },
    { t: "dim",    s: "→ fmg-sophia, fwf40f, core-sw-01, edge-rtr-01" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "[all] # ", cmd: "execute ping 8.8.8.8" },
    { t: "h2",     s: "── fmg-sophia ─────────" },
    { t: "ok",     s: "PING 8.8.8.8: 5/5 received, avg 8.2ms" },
    { t: "h2",     s: "── fwf40f ─────────────" },
    { t: "ok",     s: "PING 8.8.8.8: 5/5 received, avg 12.1ms" },
    { t: "h2",     s: "── core-sw-01 ─────────" },
    { t: "ok",     s: "Success rate is 100 percent (5/5), avg 6ms" },
    { t: "h2",     s: "── edge-rtr-01 ────────" },
    { t: "warn",   s: "Success rate is 80 percent (4/5), avg 14ms" },
    { t: "blank",  s: "" },
    { t: "prompt", s: "[all] # ", cur: true },
  ],
};

window.PYLON.snippets = [
  { id: "s1", trig: "fgt-status",   desc: "FortiGate health snapshot", body: "get system status\nget system performance status\ndiagnose sys top 5" },
  { id: "s2", trig: "bgp-check",    desc: "BGP neighbor summary",      body: "show ip bgp summary\nshow ip bgp neighbors | inc state" },
  { id: "s3", trig: "iface-up",     desc: "Show all up interfaces",    body: "show ip interface brief | exclude down" },
  { id: "s4", trig: "save-config",  desc: "Save running to startup",   body: "write memory" },
  { id: "s5", trig: "fmg-uptime",   desc: "FortiManager uptime",       body: "diagnose system print uptime" },
];

window.PYLON.palette = [
  { id: "p1", icon: "⚡", label: "Quick connect…",       sub: "host:port", kbd: "Ctrl K" },
  { id: "p2", icon: "⎘",  label: "Split vertical",        sub: "Layout",   kbd: "Ctrl \\" },
  { id: "p3", icon: "⊞",  label: "Grid 2x2",              sub: "Layout",   kbd: "Ctrl Shift G" },
  { id: "p4", icon: "↹",  label: "Toggle MultiExec",      sub: "Broadcast input to all panes", kbd: "Ctrl Shift M" },
  { id: "p5", icon: "✦",  label: "Run snippet…",          sub: "Snippets", kbd: "Ctrl ;" },
  { id: "p6", icon: "◔",  label: "Start session log",     sub: "Logging",  kbd: "Ctrl L" },
  { id: "p7", icon: "⌕",  label: "Find in terminal…",     sub: "Search",   kbd: "Ctrl F" },
  { id: "p8", icon: "✶",  label: "Theme: Neon Noir",      sub: "Themes",   kbd: "" },
];
