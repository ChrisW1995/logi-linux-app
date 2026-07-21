"""Core analysis helpers for Logitech Flow packet-capture reconnaissance.

Pure functions over scapy packet lists so they can be unit-tested with
synthetic pcaps (no real capture needed).
"""
from __future__ import annotations

import math
from collections import defaultdict

from scapy.all import TCP, UDP, rdpcap  # type: ignore

# Logitech Flow well-known ports (LAN path).
DISCOVERY_UDP_PORT = 59867
CONTROL_TCP_PORT = 59866
CLOUD_PING_UDP_PORT = 59868


def load_packets(pcap_path: str) -> list:
    """Read a pcap/pcapng file into a list of scapy packets."""
    return list(rdpcap(pcap_path))


def _ports(pkt):
    if UDP in pkt:
        return pkt[UDP].sport, pkt[UDP].dport
    if TCP in pkt:
        return pkt[TCP].sport, pkt[TCP].dport
    return None, None


def classify(packets: list) -> dict:
    """Bucket packets by Flow role using their transport ports."""
    buckets = {
        "discovery_udp": [],
        "control_tcp": [],
        "cloud_ping_udp": [],
        "other": [],
    }
    for pkt in packets:
        sport, dport = _ports(pkt)
        ports = {sport, dport}
        if UDP in pkt and DISCOVERY_UDP_PORT in ports:
            buckets["discovery_udp"].append(pkt)
        elif UDP in pkt and CLOUD_PING_UDP_PORT in ports:
            buckets["cloud_ping_udp"].append(pkt)
        elif TCP in pkt and CONTROL_TCP_PORT in ports:
            buckets["control_tcp"].append(pkt)
        else:
            buckets["other"].append(pkt)
    return buckets


def reassemble_tcp_stream(packets: list, src: str, dst: str, sport: int, dport: int) -> bytes:
    """Concatenate one TCP direction's payload, ordered by sequence number.

    Only packets matching the given 4-tuple (IP src/dst, TCP sport/dport) are
    included. Retransmissions (duplicate seq) keep the first occurrence.
    """
    from scapy.all import IP  # local import keeps module load fast

    seen: dict[int, bytes] = {}
    for pkt in packets:
        if TCP not in pkt or IP not in pkt:
            continue
        if pkt[IP].src != src or pkt[IP].dst != dst:
            continue
        if pkt[TCP].sport != sport or pkt[TCP].dport != dport:
            continue
        payload = bytes(pkt[TCP].payload)
        if not payload:
            continue
        seq = int(pkt[TCP].seq)
        if seq not in seen:
            seen[seq] = payload
    return b"".join(seen[s] for s in sorted(seen))


def shannon_entropy(data: bytes) -> float:
    """Shannon entropy in bits/byte (0.0 = uniform, ~8.0 = random)."""
    if not data:
        return 0.0
    counts: dict[int, int] = defaultdict(int)
    for byte in data:
        counts[byte] += 1
    total = len(data)
    return -sum((c / total) * math.log2(c / total) for c in counts.values())


def identify_handshake(stream: bytes) -> str:
    """Best-effort classification of a TCP control-channel opening.

    - 'tls'       : TLS record (handshake byte 0x16, version 0x03 0x00..0x04)
    - 'plaintext' : low entropy / mostly printable -> framing likely in clear
    - 'encrypted' : high entropy -> opaque ciphertext from the first byte
    - 'empty'     : no data
    """
    if not stream:
        return "empty"
    if len(stream) >= 3 and stream[0] == 0x16 and stream[1] == 0x03 and stream[2] <= 0x04:
        return "tls"
    head = stream[:256]
    entropy = shannon_entropy(head)
    printable = sum(1 for b in head if 0x20 <= b < 0x7F) / len(head)
    if entropy < 6.0 or printable > 0.6:
        return "plaintext"
    return "encrypted"


def udp_payload_signatures(packets: list) -> list:
    """Distinct (length, first-4-bytes-hex) signatures of UDP payloads.

    Diffing these across scenario captures isolates which discovery/broadcast
    message shapes are specific to an event (e.g. an edge crossing).
    """
    from scapy.all import UDP  # local import

    sigs = set()
    for pkt in packets:
        if UDP not in pkt:
            continue
        payload = bytes(pkt[UDP].payload)
        sigs.add((len(payload), payload[:4].hex()))
    return sorted(sigs)
