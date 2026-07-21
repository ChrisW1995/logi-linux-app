from conftest import udp, tcp
import flowrecon


def test_classify_buckets_by_flow_ports(write_pcap):
    packets = [
        udp(40000, flowrecon.DISCOVERY_UDP_PORT),
        udp(flowrecon.CLOUD_PING_UDP_PORT, 40001),
        tcp("10.0.0.1", "10.0.0.2", 50000, flowrecon.CONTROL_TCP_PORT, payload=b"x"),
        tcp("10.0.0.1", "10.0.0.2", 50000, 443, payload=b"y"),
    ]
    path = write_pcap(packets)
    loaded = flowrecon.load_packets(path)
    buckets = flowrecon.classify(loaded)
    assert len(buckets["discovery_udp"]) == 1
    assert len(buckets["cloud_ping_udp"]) == 1
    assert len(buckets["control_tcp"]) == 1
    assert len(buckets["other"]) == 1


def test_reassemble_orders_by_seq_and_dedups():
    packets = [
        tcp("10.0.0.1", "10.0.0.2", 50000, 59866, seq=10, payload=b"BB"),
        tcp("10.0.0.1", "10.0.0.2", 50000, 59866, seq=1, payload=b"AA"),
        tcp("10.0.0.1", "10.0.0.2", 50000, 59866, seq=1, payload=b"ZZ"),  # retransmit
        tcp("10.0.0.2", "10.0.0.1", 59866, 50000, seq=1, payload=b"XX"),  # other direction
    ]
    stream = flowrecon.reassemble_tcp_stream(packets, "10.0.0.1", "10.0.0.2", 50000, 59866)
    assert stream == b"AABB"


def test_shannon_entropy_bounds():
    assert flowrecon.shannon_entropy(b"") == 0.0
    assert flowrecon.shannon_entropy(b"\x00" * 100) == 0.0
    assert abs(flowrecon.shannon_entropy(bytes(range(256))) - 8.0) < 1e-9


def test_identify_handshake():
    assert flowrecon.identify_handshake(b"") == "empty"
    assert flowrecon.identify_handshake(b"\x16\x03\x01\x00\x50") == "tls"
    assert flowrecon.identify_handshake(b"GET / HELLO PEER name=abc " * 4) == "plaintext"
    assert flowrecon.identify_handshake(bytes(range(256))) == "encrypted"


def test_udp_payload_signatures():
    from conftest import udp
    packets = [
        udp(40000, 59867, payload=b"\xaa\xbb\xcc\xddEXTRA"),
        udp(40000, 59867, payload=b"\xaa\xbb\xcc\xddEXTRA"),  # duplicate shape
        udp(40000, 59867, payload=b"\x01\x02"),
    ]
    sigs = flowrecon.udp_payload_signatures(packets)
    assert sigs == [(2, "0102"), (9, "aabbccdd")]


def test_summarize_reports_control_stream(write_pcap):
    import analyze
    from conftest import udp, tcp
    packets = [
        udp(40000, 59867, payload=b"\xaa\xbb\xcc\xdd"),
        tcp("10.0.0.1", "10.0.0.2", 50000, 59866, seq=1, payload=b"\x16\x03\x01\x00\x10"),
    ]
    path = write_pcap(packets)
    report = analyze.summarize(path)
    assert "TCP control streams" in report
    assert "kind=tls" in report
    assert "discovery_udp: 1" in report
