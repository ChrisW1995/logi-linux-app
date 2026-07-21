# Flow 側錄快速上手（M0）

一頁照著做。目標：側錄兩台官方 Options+（macOS A ↔ Windows B）互相 Flow 的流量，交給 `analyze.py` 產出報告。詳細情境見 [`scenarios.md`](./scenarios.md)、Windows 抓法見 [`capture-windows.md`](./capture-windows.md)。

## 0. 前置（一次性）
- [ ] A（macOS）、B（Windows）都裝官方 Options+、已互相啟用 Flow、**同一區網**。
- [ ] 同一顆 Logitech 滑鼠已同時配對 A、B。
- [ ] 記下兩台的區網 IP（`ipconfig getifaddr en0` / `ipconfig`），方便事後對照封包方向。

## 1. 在 Linux 記下裝置秘密（比對金鑰的關鍵）
把滑鼠切到 Linux host，然後：
```bash
cd /Users/chriswang/Developer/logi-linux-app/src-tauri
cargo run --bin crypto-id
```
- [ ] 把輸出的 `crypto_identifier`（那串 hex）存起來備用。

## 2. 在 Linux 準備分析環境（一次性）
```bash
cd /Users/chriswang/Developer/logi-linux-app/tools/flow-recon
/Users/chriswang/miniconda3/bin/python3 -m pip install -r requirements.txt   # 已裝可略
PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 /Users/chriswang/miniconda3/bin/python3 -m pytest -q   # 應 6 passed
```

## 3. 逐情境側錄（A、B 同時開始，各存一份）
情境：**S1** 重新啟用 Flow／**S2** 閒置 60 秒／**S3** 游標 A→B／**S4** 游標 B→A／**S5** A 複製文字 B 貼上／**S6** B 斷網 10 秒重連。

對每個 `Sn`：
- [ ] **A（macOS）** 先開側錄（會一直錄到你 Ctrl-C）：
  ```bash
  cd /Users/chriswang/Developer/logi-linux-app/tools/flow-recon
  sudo ./capture-macos.sh Sn en0          # en0 換成實際 LAN 介面
  ```
- [ ] **B（Windows）** 同時開 Wireshark，capture filter 填 `port 59866 or port 59867 or port 59868`，開始擷取。
- [ ] 重現該情境動作 → A 按 Ctrl-C 停、B 存成 `Sn-windows.pcapng`。
- [ ] 六個情境跑完，A 端會有 `Sn-macos-<epoch>.pcap`、B 端有 `Sn-windows.pcapng`。

## 4. 匯整到 Linux 分析
把所有 pcap 複製到 `tools/flow-recon/captures/`（自建），然後：
```bash
cd /Users/chriswang/Developer/logi-linux-app/tools/flow-recon
/Users/chriswang/miniconda3/bin/python3 analyze.py captures/S1-* captures/S2-* captures/S3-* captures/S4-* captures/S5-* captures/S6-*
```
- [ ] 報告裡看：`discovery_udp` 封包數、每個 TCP 控制流的 `entropy` 與 `kind=`（tls / plaintext / encrypted）、head hex。

## 5. 交回給我
把 **步驟 1 的 `crypto_identifier`** ＋ **步驟 4 的 analyze.py 報告**（或直接給 pcap）貼回來。我會據此：
- [ ] 填 `docs/flow-protocol.md`（探索欄位、握手型別、訊息型別、金鑰構造）。
- [ ] 給出 **go/no-go**：能否用裝置秘密重現握手成為 peer → 決定是否進 M1（探索層 porting）。

> 卡住排查：A、B 互 ping 不通 → 不同子網或防火牆擋（Flow 會改走雲端 443/59868，M0 先不處理）；`capture-macos.sh` 抓不到封包 → `en0` 換成 `ipconfig getifaddr` 顯示的介面；Wireshark 沒東西 → 確認用的是 **capture** filter 不是 display filter。
