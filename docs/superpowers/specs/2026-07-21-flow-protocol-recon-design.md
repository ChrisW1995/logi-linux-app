# Flow 協定偵察（M0）設計

> 狀態：已核准，準備進入 writing-plans
> 日期：2026-07-21
> 里程碑：M0（偵察 + 協定規格），是「把 Logitech Flow porting 到 Linux」的第一步

## 背景與最終目標

`logi-linux-app` 是 Linux 版的 Logitech Options+ 替代品，透過 HID++（經 hidraw）與 Logitech 裝置溝通。

**最終目標**：把 Logitech Flow **porting 到 Linux** —— 讓本 app 成為一個**真正的 Flow peer**，使 macOS / Windows 上的官方 Options+ 能將它視為 Flow 對象：游標滑到螢幕邊緣即漫遊到 Linux、剪貼簿跨機同步。

**M0 的定位**：官方 Flow 協定是無公開文件的黑盒，控制通道加密。在寫任何 porting 程式碼前，必須先側錄真實流量、看懂協定，並判斷 porting 是否可行。**M0 不寫任何網路實作**，只交付偵察工具、分析結果與協定規格。

## 現況（既有積木）

Flow 相關程式碼目前只有兩塊 HID++ 積木，位於 `src-tauri/crates/hidpp/src/features.rs`：

- `change_host(host_index)` — HID++ `0x1814`，命令裝置切換到指定 host channel。
- `get_crypto_identifier()` — HID++ `0x0021`，讀出裝置的穩定 32-byte 值；程式碼註解已標明「Flow 用它推導共享 discovery key」。

網路層（UDP/TCP/TLS/探索/加密/輸入注入）**完全不存在**。`src/pages/flow.tsx` 是 "Coming soon" 佔位。

## 心智模型（決定可行性的關鍵）

官方 Flow 白皮書與支援文件揭露的權威事實：

- **探索**：同區網用 UDP `59867` 廣播；跨網段用 Logitech 雲端（TCP `443` + UDP `59868`）。
- **控制通道**：TCP `59866`，全程加密。
- **金鑰**：每個 session **輪替的 ephemeral 金鑰**，在探索／握手過程中建立，不存永久儲存 → 對**被動側錄者**有前向保密，事後無法解密側錄。
- **剪貼簿**：文字／圖片／檔案，切換時送出 "clipboard hints"，在對端建立 "clipboard proxies"。

因此我們**不走「事後解密側錄」**這條死路。我們的立場是：Linux 端握有「兩台共享的那顆滑鼠」的裝置秘密（`crypto_identifier`），目標是**成為一個合法 peer、自己跑握手拿到 session key**，而不是解密別人的流量。

- **側錄** → 學到訊息**格式／框架／握手步驟**（即使 payload 加密，框架仍可觀察）。
- **裝置秘密 + 二進位分析** → 還原握手的密碼學構造（KEX 曲線／KDF／如何用 `crypto_identifier`），讓我們能自行導出 session key。

## 實驗環境

- **機器 A**：macOS，官方 Options+，Flow 已啟用。內建 `tcpdump`。
- **機器 B**：Windows，官方 Options+，Flow 已啟用。用 Wireshark（推薦）或內建 `pktmon` 側錄。
- **機器 C（porting 目標）**：Linux，跑本 app。
- **共享裝置**：一顆同時配對 A 與 B 的 Logitech 滑鼠（Flow 的前提）。其 `crypto_identifier` 是裝置屬性，與連到哪台無關 → 在 Linux 上把滑鼠切過去讀一次即可，值即 A/B 也看到的值。
- 假設 A、B **同一區網**（M0 聚焦 LAN 探索，雲端路徑先略）。

## M0 範圍

### 做什麼
1. 交付可攜式側錄工具，讓使用者在 A、B **同時**側錄 Flow 流量。
2. 交付分析 harness，把 pcap 解析成可讀的協定結構。
3. 交付 `crypto-id` CLI，在 Linux 從共享滑鼠 dump `crypto_identifier` 供關聯比對。
4. 產出協定規格活文件 `docs/flow-protocol.md`。
5. 給出 **go/no-go 判斷**：能否用裝置秘密重現握手、成為 peer。

### 不做什麼（Non-goals）
- app 內不加任何網路／探索／握手／輸入實作（那是 M1–M4）。
- 不做跨網段雲端探索（TCP 443 / UDP 59868）——延後。
- 剪貼簿第一輪只分析文字；圖片／檔案（clipboard proxies）延後。
- 不嘗試對 ephemeral 金鑰的側錄做離線解密（已知不可行）。

## 交付物

| 路徑 | 作用 |
|---|---|
| `tools/flow-recon/capture-macos.sh` | macOS `tcpdump` 包裝，依情境貼標籤，過濾 `port 59866 or 59867 or 59868` |
| `tools/flow-recon/capture-windows.md` | Windows 側錄步驟：Wireshark（capture filter）為主、`pktmon` 為免安裝備援 |
| `tools/flow-recon/scenarios.md` | 側錄情境腳本 S1–S6 與逐步操作 |
| `tools/flow-recon/analyze.py` | pcap 分析 harness（見下） |
| `tools/flow-recon/requirements.txt` | 分析相依（`scapy` 或 `dpkt`） |
| `crypto-id` CLI（小 Rust `bin`；因需真實 hidapi transport，置於 `src-tauri`，例如 `src-tauri/src/bin/crypto-id.rs`，而非硬體無關的 hidpp crate） | 不開 GUI 即可 dump `crypto_identifier` + 配對資訊 |
| `docs/flow-protocol.md` | 逐步長出的協定規格（活文件） |

## 側錄方法

### 情境（每個一個標籤 pcap，A、B 兩端同時抓）
- **S1** 首次啟用 Flow／配對兩台（金鑰交換發生於此，最關鍵）
- **S2** 閒置穩態（心跳／keepalive 節奏）
- **S3** 游標 A→B 過邊緣
- **S4** 游標 B→A 過邊緣
- **S5** A 複製文字、B 貼上
- **S6** 一端離線／重連

每個情境檔名內含：情境代號、端點（A/B）、時間戳（人工）。A、B 需**時間對齊**（開始側錄前簡單對時，方便交叉比對同一封包在兩端的樣貌）。

### 工具
- macOS：`sudo tcpdump -i any -w S3-A.pcap 'port 59866 or port 59867 or port 59868'`
- Windows：Wireshark，capture filter `port 59866 or port 59867 or port 59868`；或 `pktmon start --capture` → `pktmon etl2pcap`。

## 分析方法（analyze.py）

輸入一組 pcap，輸出結構化報告。重點功能：

1. **分流分類**：拆出 UDP `59867` 探索 vs TCP `59866` 控制；重組 TCP stream。
2. **UDP 探索先解**（最可能含明文）：抽出 peer 身分欄位、IP/port、看是否出現 `crypto_identifier` 的雜湊痕跡（拿 CLI 讀到的值做 SHA-256/HMAC 等候選比對）。
3. **TCP 握手型別辨識**：看 stream 開頭位元組判定——TLS record（`16 03 0x`）？Noise？自訂 KEX？據此判斷「持有裝置秘密的 peer 能否參與握手」。
4. **熵圖**：對 payload 做位元組熵分析，分離低熵框架（header/length/type）與高熵密文。
5. **情境 diff**：S3 vs S4（方向反轉應在某欄位可預測地不同）、S1 vs S2（握手 vs 穩態）、有／無剪貼簿（S5 vs S2）→ 標出訊息型別。
6. 輸出餵入 `docs/flow-protocol.md` 的欄位表與訊息型別表。

harness 對固定 pcap 是**確定性解析**，可寫單元測試（用小段擷取的封包當 fixture）。

## 協定規格文件（`docs/flow-protocol.md`）結構

- 埠與傳輸總覽（已知事實）
- UDP 59867 探索封包格式（欄位表，隨分析填入）
- TCP 59866 握手步驟（訊息序列圖）
- 訊息型別表（型別碼 → 語意 → 觀察到的情境）
- 金鑰／認證構造（隨二進位分析填入；含 `crypto_identifier` 的角色）
- 邊緣切換訊息語意（座標／方向 → 對應 `change_host`）
- 剪貼簿訊息（文字）
- 未解區塊與待驗證假設清單

## go/no-go 判準（M0 結束時回答）

**GO**（可 porting）若：握手是可由「持有裝置秘密的一方」重現的方案（例如以 `crypto_identifier` 導出的 PSK 做 TLS-PSK／自訂 DH），且探索封包的身分欄位可由 Linux 端合成。

**NO-GO / 需重新評估**若：握手依賴無法從裝置導出的秘密（綁 Options+ 帳號的伺服器簽章、硬體不可讀的私鑰、遠端 attestation 等），或控制通道用了我們無法在 Linux 重建的專有加密且金鑰不可導出。

## 完整路線圖（M0 之後，各自另開 spec → plan）

| 里程碑 | 內容 |
|---|---|
| **M0（本文件）** | 偵察 + 協定規格 + go/no-go |
| M1 | 探索層 porting：Linux 參與 UDP 59867，出現在 Mac/Win 的 Flow 電腦清單 |
| M2 | 握手 + 金鑰 porting：用共享滑鼠秘密重現握手、建立加密 session、成為認證 peer |
| M3 | 控制通道 porting：解析／發送邊緣切換訊息，接上 `change_host` 讓實體裝置漫遊、游標進入定位 |
| M4 | 剪貼簿同步 porting：先文字，再圖片／檔案 |

## 驗證方式

- **CLI**：在真機讀出共享滑鼠的 `crypto_identifier`，值穩定、可重現。
- **analyze.py**：對側錄 pcap 的解析有單元測試（fixture 為擷取封包）；跑真實 pcap 產出報告。
- **規格**：用情境 diff 交叉驗證每個結論（例如 A→B 與 B→A 的差異欄位可預測）。
- **go/no-go**：以規格中「金鑰／認證構造」章節的證據支撐，而非臆測。

## 風險與未知

- **握手可能不可重現**（見 NO-GO）——這正是 M0 要儘早查明的，避免燒 M1–M4 的工。
- **控制通道全密文**：若框架也加密（連 length/type 都在密文內），情境 diff 的解析力會下降，需更依賴握手參與後的明文觀察（可能得延到 M2 才真正看懂 payload）。
- **二進位分析成本**：還原確切 KDF/曲線可能需反組譯 Options+，工程量不確定；M0 先以「從握手位元組辨識方案家族」為主，反組譯為備援。
- **Options+ 更新**：協定可能隨版本改變；規格需記錄側錄時的 Options+ 版本。
