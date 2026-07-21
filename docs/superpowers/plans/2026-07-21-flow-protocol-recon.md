# Flow 協定偵察（M0）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付一套 Logitech Flow 封包側錄與分析工具、一支讀裝置秘密的 CLI、以及協定規格骨架，作為把 Flow porting 到 Linux 的偵察基礎（本里程碑不寫任何網路實作）。

**Architecture:** 兩塊互相獨立的產出。(1) `crypto-id` Rust CLI —— 重用既有 `hidpp` crate 讀出共享滑鼠的 `crypto_identifier`（HID++ `0x0021`），供關聯側錄流量。(2) `tools/flow-recon/` Python 偵察工具 —— `flowrecon.py` 純函式庫（載入 pcap、依 Flow 埠分類、TCP 重組、熵分析、握手型別辨識），`analyze.py` CLI 產出報告；全部用 scapy 合成的 pcap 做 TDD，不需真實側錄即可測。搭配跨平台側錄腳本與 `docs/flow-protocol.md` 活文件。

**Tech Stack:** Rust（`hidpp` crate、hidapi）、Python 3（scapy、pytest）、bash/tcpdump（macOS）、Wireshark/pktmon（Windows）。

## Global Constraints

- Rust ≥ 1.85（edition 2024）；Node ≥ 20；前端套件管理器為 **npm**。
- app 內**不新增網路／加密相依**；`crypto-id` CLI 只重用既有 `hidpp` crate，不改動 `hidpp` 現有 API。
- Python 偵察工具是**開發工具**，相依只寫在 `tools/flow-recon/requirements.txt`，不進 app build。
- 範圍限 **LAN 探索**（UDP 59867 / TCP 59866）；雲端路徑（TCP 443 / UDP 59868）僅在分類器保留桶位、不深入。剪貼簿分析**只做文字**。
- Git commit 訊息一律**英文**。
- Linux-first：`crypto-id` 需在 Linux 實機才能真正讀到裝置；macOS 上須能**編譯通過**（device 存取為 best-effort）。

---

## File Structure

- `src-tauri/src/bin/crypto-id.rs` — 獨立 CLI，dump 每個 Logitech 裝置的 `crypto_identifier`。Cargo 自動辨識為 binary `crypto-id`。
- `tools/flow-recon/requirements.txt` — Python 相依。
- `tools/flow-recon/conftest.py` — pytest 設定（把工具目錄加入 `sys.path`）＋合成 pcap fixture。
- `tools/flow-recon/flowrecon.py` — 純分析函式庫（可測）。
- `tools/flow-recon/analyze.py` — CLI，讀 pcap 產出偵察報告。
- `tools/flow-recon/test_flowrecon.py` — pytest 測試。
- `tools/flow-recon/capture-macos.sh` — macOS tcpdump 側錄腳本。
- `tools/flow-recon/capture-windows.md` — Windows 側錄步驟（Wireshark / pktmon）。
- `tools/flow-recon/scenarios.md` — 情境 S1–S6 側錄腳本。
- `docs/flow-protocol.md` — 協定規格活文件骨架。

---

### Task 1: `crypto-id` CLI（Rust）

**Files:**
- Create: `src-tauri/src/bin/crypto-id.rs`

**Interfaces:**
- Consumes（既有，不修改）：`hidpp::device::find_logitech_devices() -> Result<Vec<LogitechDeviceInfo>, HidppError>`；`hidpp::device::open_device(&LogitechDeviceInfo) -> Result<FeatureAccess<HidApiTransport>, HidppError>`；`FeatureAccess::get_crypto_identifier(&self) -> Result<[u8;32], HidppError>`；`LogitechDeviceInfo { path, product_id, product_name, device_index }`。
- Produces：可執行檔 `crypto-id`；純函式 `format_identifier_hex(&[u8]) -> String`。

- [ ] **Step 1: 寫失敗測試（先只放函式與測試，main 之後補）**

建立 `src-tauri/src/bin/crypto-id.rs`：

```rust
//! Standalone CLI that dumps the HID++ CryptoIdentifier (feature 0x0021) of
//! every Logitech HID++ device on the system. Used during Flow protocol
//! reconnaissance to correlate captured traffic with the shared device secret.
//! Linux-first (needs hidraw access); compiles on macOS but device access
//! there is best-effort.

/// Format a raw identifier as lowercase hex, grouped in 4-byte words for
/// readability, e.g. `a1b2c3d4 000f`.
fn format_identifier_hex(bytes: &[u8]) -> String {
    bytes
        .chunks(4)
        .map(|word| word.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_grouped_lowercase_hex() {
        let bytes = [0xa1u8, 0xb2, 0xc3, 0xd4, 0x00, 0x0f];
        assert_eq!(format_identifier_hex(&bytes), "a1b2c3d4 000f");
    }

    #[test]
    fn empty_input_is_empty_string() {
        assert_eq!(format_identifier_hex(&[]), "");
    }
}
```

- [ ] **Step 2: 執行測試確認失敗**

Run（於 `src-tauri/`）：`cargo test --bin crypto-id`
Expected: 編譯成功、兩個測試 PASS，但 `cargo build --bin crypto-id` 之後執行會 panic（`main` 是 `unimplemented!()`）。此步驟目的是先確立測試綠燈；main 於下一步實作。

（說明：此處純函式已可通過測試；失敗點在 `main` 尚未實作，於 Step 3 補完並以「build + run 不再 panic」驗證。）

- [ ] **Step 3: 實作 `main`**

把 `fn main() { unimplemented!() }` 替換為：

```rust
fn main() {
    let devices = match hidpp::device::find_logitech_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to enumerate Logitech devices: {e}");
            std::process::exit(1);
        }
    };

    if devices.is_empty() {
        eprintln!("No Logitech HID++ devices found.");
        std::process::exit(2);
    }

    for info in &devices {
        print!(
            "{} (index {}, pid 0x{:04x}) @ {} -> ",
            info.product_name, info.device_index, info.product_id, info.path
        );
        match hidpp::device::open_device(info) {
            Ok(access) => match access.get_crypto_identifier() {
                Ok(id) => println!("{}", format_identifier_hex(&id)),
                Err(e) => println!("no crypto identifier ({e})"),
            },
            Err(e) => println!("open failed ({e})"),
        }
    }
}
```

- [ ] **Step 4: 驗證編譯、測試、可執行**

Run（於 `src-tauri/`）：
- `cargo test --bin crypto-id` → Expected: PASS（2 tests）
- `cargo build --bin crypto-id` → Expected: 編譯成功
- `cargo run --bin crypto-id` → Expected（macOS 開發機）：印出找到的裝置或 `No Logitech HID++ devices found.`，**不 panic**。（實際 `crypto_identifier` 值需 Linux 實機 + 共享滑鼠。）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/bin/crypto-id.rs
git commit -m "Add crypto-id CLI to dump HID++ CryptoIdentifier for Flow recon"
```

---

### Task 2: 偵察函式庫 —— 載入與分類（Python）

**Files:**
- Create: `tools/flow-recon/requirements.txt`
- Create: `tools/flow-recon/conftest.py`
- Create: `tools/flow-recon/flowrecon.py`
- Create: `tools/flow-recon/test_flowrecon.py`

**Interfaces:**
- Produces：`load_packets(pcap_path: str) -> list`；`classify(packets: list) -> dict`（keys：`discovery_udp`、`control_tcp`、`cloud_ping_udp`、`other`）；常數 `DISCOVERY_UDP_PORT=59867`、`CONTROL_TCP_PORT=59866`、`CLOUD_PING_UDP_PORT=59868`。

- [ ] **Step 1: 建立相依與 pytest 設定**

`tools/flow-recon/requirements.txt`：

```
scapy>=2.5
pytest>=8.0
```

`tools/flow-recon/conftest.py`：

```python
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
```

安裝：於 `tools/flow-recon/` 執行 `python3 -m pip install -r requirements.txt`。

- [ ] **Step 2: 寫失敗測試**

`tools/flow-recon/test_flowrecon.py`：

```python
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
```

- [ ] **Step 3: 執行測試確認失敗**

Run（於 `tools/flow-recon/`）：`python3 -m pytest test_flowrecon.py -q`
Expected: FAIL（`ModuleNotFoundError: No module named 'flowrecon'`）。

- [ ] **Step 4: 實作 `flowrecon.py`（載入 + 分類）**

`tools/flow-recon/flowrecon.py`：

```python
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
```

- [ ] **Step 5: 執行測試確認通過**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: PASS（1 test）。

- [ ] **Step 6: Commit**

```bash
git add tools/flow-recon/requirements.txt tools/flow-recon/conftest.py tools/flow-recon/flowrecon.py tools/flow-recon/test_flowrecon.py
git commit -m "Add flow-recon pcap load/classify library with tests"
```

---

### Task 3: TCP 重組與熵分析

**Files:**
- Modify: `tools/flow-recon/flowrecon.py`
- Modify: `tools/flow-recon/test_flowrecon.py`

**Interfaces:**
- Produces：`reassemble_tcp_stream(packets, src, dst, sport, dport) -> bytes`（依 TCP seq 排序、重傳去重）；`shannon_entropy(data: bytes) -> float`（bits/byte）。

- [ ] **Step 1: 寫失敗測試**

在 `test_flowrecon.py` 追加：

```python
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
```

- [ ] **Step 2: 執行確認失敗**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: FAIL（`AttributeError: module 'flowrecon' has no attribute 'reassemble_tcp_stream'`）。

- [ ] **Step 3: 實作**

在 `flowrecon.py` 末端追加：

```python
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
```

- [ ] **Step 4: 執行確認通過**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: PASS（3 tests）。

- [ ] **Step 5: Commit**

```bash
git add tools/flow-recon/flowrecon.py tools/flow-recon/test_flowrecon.py
git commit -m "Add TCP stream reassembly and Shannon entropy to flow-recon"
```

---

### Task 4: 握手型別辨識與 UDP 訊息簽章

**Files:**
- Modify: `tools/flow-recon/flowrecon.py`
- Modify: `tools/flow-recon/test_flowrecon.py`

**Interfaces:**
- Produces：`identify_handshake(stream: bytes) -> str`（回傳 `'tls'`/`'plaintext'`/`'encrypted'`/`'empty'`）；`udp_payload_signatures(packets: list) -> list`（`(length, first4bytes_hex)` 的排序清單）。

- [ ] **Step 1: 寫失敗測試**

在 `test_flowrecon.py` 追加：

```python
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
```

- [ ] **Step 2: 執行確認失敗**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: FAIL（`AttributeError: ... 'identify_handshake'`）。

- [ ] **Step 3: 實作**

在 `flowrecon.py` 末端追加：

```python
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
```

- [ ] **Step 4: 執行確認通過**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: PASS（5 tests）。

- [ ] **Step 5: Commit**

```bash
git add tools/flow-recon/flowrecon.py tools/flow-recon/test_flowrecon.py
git commit -m "Add handshake classification and UDP payload signatures to flow-recon"
```

---

### Task 5: `analyze.py` CLI 與偵察報告

**Files:**
- Create: `tools/flow-recon/analyze.py`
- Modify: `tools/flow-recon/test_flowrecon.py`

**Interfaces:**
- Consumes：`flowrecon.*`（Task 2–4 的所有函式）。
- Produces：CLI `analyze.py <pcap...>`；純函式 `summarize(pcap_path: str) -> str`。

- [ ] **Step 1: 寫失敗測試**

在 `test_flowrecon.py` 追加：

```python
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
```

- [ ] **Step 2: 執行確認失敗**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: FAIL（`ModuleNotFoundError: No module named 'analyze'`）。

- [ ] **Step 3: 實作 `analyze.py`**

`tools/flow-recon/analyze.py`：

```python
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
```

- [ ] **Step 4: 執行確認通過**

Run：`python3 -m pytest test_flowrecon.py -q`
Expected: PASS（6 tests）。

- [ ] **Step 5: 冒煙測試 CLI**

Run：`python3 analyze.py` （無參數）
Expected: 印出 docstring 用法、回傳碼 1。

- [ ] **Step 6: Commit**

```bash
git add tools/flow-recon/analyze.py tools/flow-recon/test_flowrecon.py
git commit -m "Add analyze.py flow-recon report CLI"
```

---

### Task 6: 側錄腳本、情境腳本與協定規格骨架

**Files:**
- Create: `tools/flow-recon/capture-macos.sh`
- Create: `tools/flow-recon/capture-windows.md`
- Create: `tools/flow-recon/scenarios.md`
- Create: `docs/flow-protocol.md`

**Interfaces:**
- Produces：使用者可實際執行的側錄流程與規格骨架（無程式介面）。

- [ ] **Step 1: 建立 macOS 側錄腳本**

`tools/flow-recon/capture-macos.sh`：

```bash
#!/usr/bin/env bash
# Capture Logitech Flow traffic on macOS during one scenario.
# Usage: sudo ./capture-macos.sh <scenario-label> [interface]
# Produces <scenario-label>-macos-<epoch>.pcap in the current directory.
set -euo pipefail

LABEL="${1:?usage: sudo ./capture-macos.sh <scenario-label> [interface]}"
IFACE="${2:-en0}"
OUT="${LABEL}-macos-$(date +%s).pcap"

echo "Capturing Flow ports on ${IFACE} -> ${OUT}"
echo "Reproduce scenario '${LABEL}' now. Press Ctrl-C to stop."
exec tcpdump -i "${IFACE}" -w "${OUT}" \
  'port 59866 or port 59867 or port 59868'
```

- [ ] **Step 2: 驗證腳本語法**

Run：`bash -n tools/flow-recon/capture-macos.sh`
Expected: 無輸出、回傳碼 0。
接著 `chmod +x tools/flow-recon/capture-macos.sh`。

- [ ] **Step 3: 建立 Windows 側錄說明**

`tools/flow-recon/capture-windows.md`：

````markdown
# Windows 側錄 Logitech Flow

兩種方式，推薦 Wireshark。

## A. Wireshark（推薦）
1. 安裝 Wireshark（含 Npcap）。
2. 選取連到 LAN 的網卡開始擷取。
3. 套用 **capture filter**（不是 display filter）：
   ```
   port 59866 or port 59867 or port 59868
   ```
4. 重現情境（見 `scenarios.md`）。
5. File → Save As，存成 `<情境代號>-windows.pcapng`。

## B. pktmon（免安裝，Windows 10/11 內建）
以系統管理員 PowerShell：
```powershell
pktmon filter remove
pktmon filter add -p 59866
pktmon filter add -p 59867
pktmon filter add -p 59868
pktmon start --capture --pkt-size 0 -f S3-windows.etl
# 重現情境，然後：
pktmon stop
pktmon etl2pcap S3-windows.etl -o S3-windows.pcapng
```
把 `.pcapng` 交給 `analyze.py`。
````

- [ ] **Step 4: 建立情境腳本**

`tools/flow-recon/scenarios.md`：

````markdown
# Flow 側錄情境腳本（M0）

前置：
- 機器 A（macOS）與機器 B（Windows）都裝官方 Options+、已互相啟用 Flow、在**同一區網**。
- 共享滑鼠已同時配對 A、B。
- 側錄前先在 Linux 用 `crypto-id` 記下該滑鼠的 `crypto_identifier`（把滑鼠切到 Linux host 讀一次）：
  ```
  cd src-tauri && cargo run --bin crypto-id
  ```
- A、B **同時**側錄；每個情境各存一對檔案（A 用 `capture-macos.sh`，B 見 `capture-windows.md`）。

| 代號 | 情境 | 操作 |
|---|---|---|
| S1 | 首次啟用 Flow／配對 | 在 Options+ 關閉再重新啟用 Flow，讓兩台重新握手 |
| S2 | 閒置穩態 | 啟用 Flow 後靜置 60 秒，不動滑鼠 |
| S3 | 游標 A→B | 從 A 把游標推過右邊緣進入 B |
| S4 | 游標 B→A | 從 B 把游標推回左邊緣進入 A |
| S5 | 剪貼簿（文字） | 在 A 複製一段文字，游標移到 B 後貼上 |
| S6 | 離線／重連 | 關掉 B 的 Wi-Fi 10 秒再開，觀察重新探索 |

命名建議：`S3-macos-<epoch>.pcap`、`S3-windows.pcapng`。

分析：
```
cd tools/flow-recon
python3 -m pip install -r requirements.txt
python3 analyze.py S1-macos-*.pcap S1-windows.pcapng
```
把每個情境的報告貼進 `docs/flow-protocol.md` 對應章節，並用 S3/S4、S1/S2 的 `udp_payload_signatures` 差異標出訊息型別。
````

- [ ] **Step 5: 建立協定規格骨架**

`docs/flow-protocol.md`：

````markdown
# Logitech Flow 協定規格（逆向中）

> 活文件。隨側錄分析逐步填入。側錄時的 Options+ 版本：`TODO 填版本`。

## 埠與傳輸（已知）
- UDP 59867：LAN peer 探索（廣播）
- TCP 59866：控制通道（加密，每 session ephemeral 金鑰）
- UDP 59868 + TCP 443：雲端輔助探索（跨網段；M0 不深入）

## UDP 59867 探索封包
| 欄位 | offset | 長度 | 語意 | 來源情境 |
|---|---|---|---|---|
| _待填_ | | | | |

`crypto_identifier` 是否（雜湊後）出現於探索封包：_待驗證_。

## TCP 59866 握手
- 開頭位元組型別（analyze.py `identify_handshake`）：_待填_
- 訊息序列：_待填_

## 訊息型別表
| 型別碼/簽章 | 語意 | 觀察情境 |
|---|---|---|
| _待填_ | | |

## 金鑰／認證構造
- 握手方案家族（TLS-PSK？Noise？自訂 DH？）：_待填_
- `crypto_identifier` 的角色：_待填_
- **go/no-go**：能否用裝置秘密重現握手成為 peer → _待判斷_

## 邊緣切換語意（S3/S4）
_待填_

## 剪貼簿（文字，S5）
_待填_

## 未解區塊與待驗證假設
- _待填_
````

- [ ] **Step 6: 驗證檔案齊備**

Run：`ls -1 tools/flow-recon/ docs/flow-protocol.md`
Expected: 列出 `capture-macos.sh`、`capture-windows.md`、`scenarios.md`、（既有的 py 檔）與 `docs/flow-protocol.md`。

- [ ] **Step 7: Commit**

```bash
git add tools/flow-recon/capture-macos.sh tools/flow-recon/capture-windows.md tools/flow-recon/scenarios.md docs/flow-protocol.md
git commit -m "Add Flow capture scripts, scenario runbook, and protocol spec skeleton"
```

---

## Self-Review

**Spec coverage：**
- 側錄工具（macOS/Windows）→ Task 6 ✓
- 情境 S1–S6 → Task 6 `scenarios.md` ✓
- analyze.py（分流/重組/熵/握手辨識/情境 diff）→ Task 2–5 ✓
- `crypto-id` CLI → Task 1 ✓
- `docs/flow-protocol.md` 規格骨架 → Task 6 ✓
- go/no-go 判準 → 寫入 `flow-protocol.md`「金鑰／認證構造」章節（Task 6）✓
- Non-goals（不寫網路碼、LAN-only、文字剪貼）→ Global Constraints ✓

**Placeholder scan：** 規格骨架中的「_待填_／TODO」是**活文件刻意的待補欄位**（M0 產物是骨架，非最終規格），非計畫步驟的佔位；所有 code step 皆含完整程式碼。

**Type consistency：** `flowrecon` 函式名在 Task 2–5 一致（`load_packets`/`classify`/`reassemble_tcp_stream`/`shannon_entropy`/`identify_handshake`/`udp_payload_signatures`/`summarize`）；Rust `format_identifier_hex` 命名一致；埠常數 `DISCOVERY_UDP_PORT`/`CONTROL_TCP_PORT`/`CLOUD_PING_UDP_PORT` 一致。
````
