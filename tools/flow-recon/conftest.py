import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

import pytest
from scapy.all import Ether, IP, TCP, UDP, Raw, wrpcap


@pytest.fixture
def write_pcap(tmp_path):
    """Write scapy packets to a temp pcap and return its path."""
    def _write(packets, name="cap.pcap"):
        path = tmp_path / name
        wrpcap(str(path), packets)
        return str(path)
    return _write


def udp(sport, dport, payload=b""):
    return Ether() / IP(src="10.0.0.1", dst="10.0.0.2") / UDP(sport=sport, dport=dport) / Raw(payload)


def tcp(src, dst, sport, dport, seq=1, payload=b""):
    return Ether() / IP(src=src, dst=dst) / TCP(sport=sport, dport=dport, seq=seq) / Raw(payload)
