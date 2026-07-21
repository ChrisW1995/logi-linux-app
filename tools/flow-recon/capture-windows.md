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
