// FortiGate (FortiOS 7.6) command lists.
//
// Every command below was checked against Fortinet's FortiOS 7.6
// documentation — nothing was added from memory:
//  • CLI troubleshooting cheat sheet (7.6.0)
//    docs.fortinet.com/document/fortigate/7.6.0/cli-troubleshooting-cheat-sheet/420966
//  • Administration guide 7.6.6: "Debugging the packet flow" (54688),
//    "IPsec related diagnose commands" (44240), "Performing a sniffer trace
//    or packet capture" (680228), "Routing concepts" (139692),
//    "Using a session table" (562859)
//  • CLI reference 7.6.6: diagnose vpn ike (309172113), diagnose sys
//    (235530229), diagnose ip router bgp (869760727) / ospf (929006518)
// Left out on purpose: commands that reset, format or reboot the device
// (factoryreset, formatlogdisk, execute reboot…) and ones whose 7.6 syntax
// the docs don't spell out.
import type { VendorLists } from './types'

export const fortigate: VendorLists = {
  label: 'FortiGate · FortiOS 7.6',
  stopDebug: ['diagnose debug disable', 'diagnose debug reset'],
  lists: [
    {
      id: 'system',
      name: 'System',
      icon: 'system',
      color: '#5B4FC4',
      summary: 'Status, resources, hardware, FortiGuard and HA',
      sections: [
        {
          title: 'Status',
          items: [
            {
              kind: 'command',
              cmd: 'get system status',
              desc: 'Version, build, serial number, operation mode',
            },
            {
              kind: 'command',
              cmd: 'get system performance status',
              desc: 'CPU and memory utilization, network usage, sessions',
            },
            { kind: 'command', cmd: 'execute time', desc: 'Current system time' },
            {
              kind: 'command',
              cmd: 'diagnose sys vd list',
              desc: 'Virtual domains and their statistics',
            },
            {
              kind: 'command',
              cmd: 'diagnose sys cmdb info',
              desc: 'Latest configuration change made by a daemon',
            },
          ],
        },
        {
          title: 'Resources',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose sys top',
              desc: 'Top processes (Shift+C sort by CPU, Shift+M by memory, q to quit)',
            },
            {
              kind: 'command',
              cmd: 'diagnose sys mpstat <seconds>',
              desc: 'CPU usage per core every n seconds',
            },
            {
              kind: 'command',
              cmd: 'diagnose hardware sysinfo memory',
              desc: 'System memory information',
            },
            {
              kind: 'command',
              cmd: 'diagnose hardware sysinfo cpu',
              desc: 'CPU information',
            },
            {
              kind: 'command',
              cmd: 'diagnose sys process pidof <daemon>',
              desc: 'PID of a running daemon (names from diagnose sys top)',
            },
            {
              kind: 'command',
              cmd: 'diagnose debug crashlog read',
              desc: 'System and application crashes',
            },
            {
              kind: 'command',
              cmd: 'diagnose debug config-error-log read',
              desc: 'Errors in the configuration file',
            },
          ],
        },
        {
          title: 'Hardware',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose hardware deviceinfo disk',
              desc: 'Disk information',
            },
            { kind: 'command', cmd: 'diagnose sys flash list', desc: 'Flash partitions' },
            {
              kind: 'command',
              cmd: 'diagnose hardware sysinfo interrupts',
              desc: 'Hardware interrupt statistics',
            },
          ],
        },
        {
          title: 'FortiGuard & logging',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose autoupdate status',
              desc: 'FortiGuard update and license status',
            },
            {
              kind: 'command',
              cmd: 'diagnose autoupdate versions',
              desc: 'Versions of engines and signature databases',
            },
            {
              kind: 'command',
              cmd: 'diagnose debug rating',
              desc: 'Web filter rating servers',
            },
            {
              kind: 'command',
              cmd: 'diagnose webfilter fortiguard statistics',
              desc: 'Rating cache and daemon statistics',
            },
            {
              kind: 'command',
              cmd: 'execute log fortianalyzer test-connectivity',
              desc: 'Test the connection to FortiAnalyzer',
            },
          ],
        },
        {
          title: 'High availability',
          items: [
            { kind: 'command', cmd: 'get system ha status', desc: 'HA status and members' },
            {
              kind: 'command',
              cmd: 'diagnose sys ha checksum cluster',
              desc: 'Configuration checksums of every cluster member',
            },
            { kind: 'command', cmd: 'diagnose sys ha history read', desc: 'HA event history' },
          ],
        },
      ],
    },
    {
      id: 'interfaces',
      name: 'Interfaces',
      icon: 'interfaces',
      color: '#2F7A8C',
      summary: 'Interfaces, ARP, sessions and reachability',
      sections: [
        {
          title: 'Interfaces',
          items: [
            {
              kind: 'command',
              cmd: 'get system interface physical',
              desc: 'Physical interfaces: status, speed, IP',
            },
            {
              kind: 'command',
              cmd: 'get system interface',
              desc: 'Summary of every interface and its IP settings',
            },
            { kind: 'command', cmd: 'diagnose ip address list', desc: 'IP addresses in use' },
            {
              kind: 'command',
              cmd: 'get hardware nic <interface>',
              desc: 'Detailed NIC counters and errors',
            },
            {
              kind: 'command',
              cmd: 'get system interface transceiver',
              desc: 'Connected transceivers',
            },
            {
              kind: 'command',
              cmd: 'diagnose netlink brctl list',
              desc: 'Software switches on the FortiGate',
            },
          ],
        },
        {
          title: 'ARP',
          items: [
            { kind: 'command', cmd: 'get system arp', desc: 'ARP table' },
            { kind: 'command', cmd: 'diagnose ip arp list', desc: 'ARP entries in detail' },
          ],
        },
        {
          title: 'Sessions',
          items: [
            { kind: 'command', cmd: 'diagnose sys session stat', desc: 'Session table statistics' },
            {
              kind: 'command',
              cmd: 'diagnose sys session filter dst <ip>',
              desc: 'Filter the session table by destination IP',
            },
            {
              kind: 'command',
              cmd: 'diagnose sys session filter',
              desc: 'Show the session filters that are set',
            },
            {
              kind: 'command',
              cmd: 'diagnose sys session list',
              desc: 'Sessions matching the filter',
            },
            {
              kind: 'command',
              cmd: 'diagnose firewall iprope list',
              desc: 'Internal firewall policy table',
            },
          ],
        },
        {
          title: 'Reachability',
          items: [
            { kind: 'command', cmd: 'execute ping <ip>', desc: 'Ping an address' },
            { kind: 'command', cmd: 'execute traceroute <ip>', desc: 'Traceroute to an address' },
          ],
        },
      ],
    },
    {
      id: 'routing',
      name: 'Routing',
      icon: 'routing',
      color: '#B26A1E',
      summary: 'Routing table, kernel routes, BFD and SD-WAN',
      sections: [
        {
          title: 'Routing table',
          items: [
            { kind: 'command', cmd: 'get router info routing-table all', desc: 'Routing table' },
            {
              kind: 'command',
              cmd: 'get router info routing-table database',
              desc: 'Routing database, including inactive routes',
            },
            {
              kind: 'command',
              cmd: 'get router info routing-table details <ip>',
              desc: 'Route lookup for a destination: next hops and interface',
            },
            {
              kind: 'command',
              cmd: 'get router info kernel',
              desc: 'Kernel routing table',
            },
            {
              kind: 'command',
              cmd: 'get router info protocols',
              desc: 'Routing protocols running',
            },
            {
              kind: 'command',
              cmd: 'get router info6 routing-table database',
              desc: 'IPv6 routing database',
            },
          ],
        },
        {
          title: 'BFD',
          items: [{ kind: 'command', cmd: 'get router info bfd neighbor', desc: 'BFD neighbors' }],
        },
        {
          title: 'SD-WAN',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose sys sdwan health-check status',
              desc: 'Health check statistics per member',
            },
            { kind: 'command', cmd: 'diagnose sys sdwan member', desc: 'SD-WAN members' },
            {
              kind: 'command',
              cmd: 'diagnose sys sdwan service4',
              desc: 'SD-WAN rules in the control plane',
            },
            {
              kind: 'command',
              cmd: 'diagnose firewall proute list',
              desc: 'SD-WAN rules and policy routes in the data plane',
            },
            {
              kind: 'command',
              cmd: 'diagnose sys link-monitor status',
              desc: 'Link monitor statistics',
            },
          ],
        },
      ],
    },
    {
      id: 'bgp',
      name: 'BGP',
      icon: 'bgp',
      color: '#3E6FC2',
      summary: 'Peers, received and advertised routes',
      sections: [
        {
          title: 'Peers',
          items: [
            { kind: 'command', cmd: 'get router info bgp summary', desc: 'BGP summary (IPv4)' },
            { kind: 'command', cmd: 'get router info6 bgp summary', desc: 'BGP summary (IPv6)' },
            {
              kind: 'command',
              cmd: 'get router info bgp neighbors',
              desc: 'Every BGP peer in detail',
            },
          ],
        },
        {
          title: 'Routes of a peer',
          items: [
            {
              kind: 'command',
              cmd: 'get router info bgp neighbors <peer-ip> received-routes',
              desc: 'Routes received from the peer',
            },
            {
              kind: 'command',
              cmd: 'get router info bgp neighbors <peer-ip> advertised-routes',
              desc: 'Routes advertised to the peer',
            },
            {
              kind: 'command',
              cmd: 'get router info bgp neighbors <peer-ip> routes',
              desc: 'Routes learned from the peer',
            },
            {
              kind: 'command',
              cmd: 'get router info bgp network <prefix>',
              desc: 'BGP entry for a prefix and the peers it is advertised to',
            },
          ],
        },
        {
          title: 'Actions',
          items: [
            {
              kind: 'command',
              cmd: 'execute router clear bgp ip <peer-ip> soft',
              desc: 'Soft reset of one peer (routes re-sent both ways)',
              action: true,
            },
          ],
        },
      ],
    },
    {
      id: 'ospf',
      name: 'OSPF',
      icon: 'ospf',
      color: '#6B53B8',
      summary: 'Status, neighbors, interfaces and LSDB',
      sections: [
        {
          title: 'OSPF',
          items: [
            { kind: 'command', cmd: 'get router info ospf status', desc: 'OSPF process status' },
            { kind: 'command', cmd: 'get router info ospf neighbor', desc: 'OSPF neighbors' },
            {
              kind: 'command',
              cmd: 'get router info ospf neighbor all',
              desc: 'OSPF neighbor information in detail',
            },
            {
              kind: 'command',
              cmd: 'get router info ospf interface',
              desc: 'Interfaces running OSPF',
            },
            {
              kind: 'command',
              cmd: 'get router info ospf database brief',
              desc: 'OSPF database in brief',
            },
            {
              kind: 'command',
              cmd: 'get router info6 ospf neighbor',
              desc: 'OSPFv3 neighbors (IPv6)',
            },
          ],
        },
      ],
    },
    {
      id: 'vpn',
      name: 'VPN IPsec',
      icon: 'vpn',
      color: '#1F7A55',
      summary: 'Phase 1, phase 2, IKE counters and filters',
      sections: [
        {
          title: 'Tunnels',
          items: [
            {
              kind: 'command',
              cmd: 'get vpn ipsec tunnel summary',
              desc: 'Summary of every IPsec tunnel',
            },
            {
              kind: 'command',
              cmd: 'get vpn ipsec tunnel details',
              desc: 'Detailed information of every tunnel',
            },
            {
              kind: 'command',
              cmd: 'diagnose vpn ike gateway list',
              desc: 'Phase 1 (IKE gateways)',
            },
            {
              kind: 'command',
              cmd: 'diagnose vpn ike gateway list name <phase1-name>',
              desc: 'Phase 1 of one tunnel',
            },
            { kind: 'command', cmd: 'diagnose vpn tunnel list', desc: 'Phase 2 SAs and counters' },
          ],
        },
        {
          title: 'IKE',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose vpn ipsec status',
              desc: 'Encryption and decryption counters',
            },
            { kind: 'command', cmd: 'diagnose vpn ike status', desc: 'IKE status' },
            { kind: 'command', cmd: 'diagnose vpn ike counts', desc: 'IKE counts' },
            { kind: 'command', cmd: 'diagnose vpn ike errors', desc: 'IKE errors' },
            { kind: 'command', cmd: 'diagnose vpn ike stats', desc: 'IKE statistics' },
            { kind: 'command', cmd: 'diagnose vpn ike routes', desc: 'Routes added by IKE' },
            {
              kind: 'command',
              cmd: 'diagnose vpn ike log filter list',
              desc: 'IKE debug filter currently set',
            },
            {
              kind: 'command',
              cmd: 'diagnose vpn ike log filter clear',
              desc: 'Clear the IKE debug filter',
            },
          ],
        },
        {
          title: 'Actions',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose vpn tunnel flush <phase2-name>',
              desc: 'Flush the phase 2 SAs of a tunnel (it renegotiates)',
              action: true,
            },
            {
              kind: 'command',
              cmd: 'diagnose vpn ike gateway clear name <phase1-name>',
              desc: 'Clear a phase 1 (the tunnel goes down and renegotiates)',
              action: true,
            },
          ],
        },
      ],
    },
    {
      id: 'debug',
      name: 'Debug',
      icon: 'debug',
      color: '#B23B3B',
      summary: 'Traces and real-time debugs — they load the CPU, stop them when done',
      debug: true,
      sections: [
        {
          title: 'Recipes',
          items: [
            {
              kind: 'recipe',
              name: 'Debug flow of an IP',
              desc: 'Why traffic to or from an address is accepted, routed or dropped',
              fields: [
                { key: 'ip', label: 'IP address', placeholder: '10.0.0.25' },
                { key: 'count', label: 'Packets to trace', default: '100' },
              ],
              lines: [
                'diagnose debug reset',
                'diagnose debug flow filter clear',
                'diagnose debug flow filter addr {{ip}}',
                'diagnose debug flow show function-name enable',
                'diagnose debug flow show iprope enable',
                'diagnose debug console timestamp enable',
                'diagnose debug enable',
                'diagnose debug flow trace start {{count}}',
              ],
            },
            {
              kind: 'recipe',
              name: 'Packet sniffer',
              desc: 'Capture packets on an interface with a filter (Ctrl+C to stop)',
              fields: [
                { key: 'iface', label: 'Interface', default: 'any' },
                { key: 'filter', label: 'Filter', placeholder: 'host 10.0.0.25' },
                { key: 'verbose', label: 'Verbose (1–6)', default: '4' },
                { key: 'count', label: 'Packets', default: '100' },
              ],
              lines: ["diagnose sniffer packet {{iface}} '{{filter}}' {{verbose}} {{count}} l"],
            },
            {
              kind: 'recipe',
              name: 'IKE debug of a peer',
              desc: 'Phase 1 and phase 2 negotiation with one remote gateway',
              fields: [{ key: 'peer', label: 'Remote gateway IP', placeholder: '203.0.113.2' }],
              lines: [
                'diagnose debug reset',
                'diagnose vpn ike log filter clear',
                'diagnose vpn ike log filter rem-addr4 {{peer}}',
                'diagnose debug console timestamp enable',
                'diagnose debug application ike -1',
                'diagnose debug enable',
              ],
            },
            {
              kind: 'recipe',
              name: 'BGP debug',
              desc: 'Real-time BGP debugging',
              fields: [],
              lines: [
                'diagnose ip router bgp all enable',
                'diagnose ip router bgp level info',
                'diagnose debug enable',
              ],
            },
            {
              kind: 'recipe',
              name: 'OSPF debug',
              desc: 'Real-time OSPF debugging',
              fields: [],
              lines: [
                'diagnose ip router ospf all enable',
                'diagnose ip router ospf level info',
                'diagnose debug enable',
              ],
            },
          ],
        },
        {
          title: 'Commands',
          items: [
            {
              kind: 'command',
              cmd: 'diagnose debug flow trace stop',
              desc: 'Stop a running flow trace',
            },
            {
              kind: 'command',
              cmd: 'diagnose debug flow filter clear',
              desc: 'Clear the IPv4 debug flow filters',
            },
          ],
        },
      ],
    },
  ],
}
