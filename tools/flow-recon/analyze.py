#!/usr/bin/env python3
"""Analyze Logitech Flow packet captures and emit a reconnaissance report.

Usage:
    python3 analyze.py capture1.pcap [capture2.pcap ...]

For each pcap it prints packet counts per Flow role, and for each TCP control
direction the reassembled length, entropy, handshake classification and a hex
preview of the first bytes. Feed the findings into docs/flow-protocol.md.
"""
import sys

from scapy.all import IP, TCP  # type: ignore

import flowrecon


def _tcp_directions(control_pkts):
    dirs = set()
    for pkt in control_pkts:
        if IP in pkt and TCP in pkt:
            dirs.add((pkt[IP].src, pkt[IP].dst, pkt[TCP].sport, pkt[TCP].dport))
    return sorted(dirs)


def summarize(pcap_path: str) -> str:
    packets = flowrecon.load_packets(pcap_path)
    buckets = flowrecon.classify(packets)
    lines = [f"# {pcap_path}", "", f"total packets: {len(packets)}"]
    for name, pkts in buckets.items():
        lines.append(f"  {name}: {len(pkts)}")
    lines.append("")
    lines.append("## UDP discovery signatures")
    for length, head_hex in flowrecon.udp_payload_signatures(buckets["discovery_udp"]):
        lines.append(f"  len={length} head={head_hex}")
    lines.append("")
    lines.append("## TCP control streams")
    for src, dst, sport, dport in _tcp_directions(buckets["control_tcp"]):
        stream = flowrecon.reassemble_tcp_stream(buckets["control_tcp"], src, dst, sport, dport)
        lines.append(
            f"  {src}:{sport} -> {dst}:{dport}  "
            f"{len(stream)} bytes  "
            f"entropy={flowrecon.shannon_entropy(stream):.2f}  "
            f"kind={flowrecon.identify_handshake(stream)}"
        )
        lines.append(f"    head: {stream[:32].hex(' ')}")
    return "\n".join(lines)


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 1
    for path in argv[1:]:
        print(summarize(path))
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
