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
