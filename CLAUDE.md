# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Logi Linux App** — a Tauri v2 desktop app that replaces Logitech Options+ on Linux. A Rust/Tauri backend talks to Logitech devices over the **HID++ protocol** via hidraw; the frontend is React 19 + TypeScript + Vite. It is **Linux-first** (real device I/O only works on Linux), but the whole codebase compiles and unit-tests on macOS/dev machines because hardware access is `#[cfg(target_os = "linux")]`-gated and the protocol layer is tested against a mock transport.

## Commands

**Frontend** (repo root; package manager is **npm** — only `package-lock.json` exists):
- `npm install` — install JS deps
- `npm run dev` — Vite dev server on `http://localhost:1420` (strict port)
- `npm run build` — `tsc && vite build`; this is also the **only typecheck path** (there is no standalone lint/typecheck script). For a typecheck-only pass: `npx tsc --noEmit`.
- `npm run test` — Vitest (jsdom), single run. `npm run test:watch` for watch mode.
- Run one test file: `npm run test -- src/components/devices/__tests__/device-card.test.tsx`
- Run tests by name: `npm run test -- -t "renders battery"`

**Full app** (Tauri backend + frontend):
- `cargo tauri dev` (needs `cargo install tauri-cli`) **or** `npm run tauri dev` — both trigger `beforeDevCommand: npm run dev`.
- `cargo tauri build` — release bundle (.deb/.rpm/AppImage on Linux; `bundle.targets: "all"`).

**Rust backend** — run from `src-tauri/` (**there is no workspace-root `Cargo.toml`**; the Rust project root is `src-tauri/`):
- `cargo check` / `cargo build`
- `cargo test` — runs the `hidpp` protocol unit tests (via `MockTransport`, no hardware) + config roundtrip tests
- Run one Rust test: `cargo test <name>` (e.g. `cargo test dpi_range`)
- `cargo clippy` / `cargo fmt` — toolchain defaults; there is no project config for either.

There is **no** ESLint/Prettier/Biome/`rustfmt.toml`/`clippy.toml` and **no CI**. TypeScript strictness in `tsconfig.json` (`strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`) is the only static check.

## Linux setup & hardware access

`scripts/setup-linux.sh` is the real setup source of truth (the root `README.md` is the stock Tauri template). It installs distro deps (webkit2gtk, libudev, libhidapi, pkg-config, …), requires **Rust ≥ 1.85** and **Node ≥ 20**, installs `cargo-tauri`, and — critically — writes a udev rule at `/etc/udev/rules.d/99-logitech-hidpp.rules`:

```
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="046d", MODE="0666"
```

so Logitech devices (VID `046d`) are reachable without root. Replug/reboot after installing the rule.

**Runtime gotcha:** if **Solaar** is running it competes for the same hidraw devices and corrupts responses. The app `pgrep`s for it on startup and prints a warning; stop it with `killall solaar` before use.

## Architecture

Two layers bridged by Tauri IPC.

### Rust backend — `src-tauri/`
- `src/lib.rs` `run()` — entry point: inits `tracing` (default filter `hidpp=debug,logi_linux_app=debug`, override via `RUST_LOG`), warns if Solaar is running, registers **16 commands**.
- `src/commands/devices.rs` — `list_devices`, `get_device_battery`. `get_device_battery` returns a DTO with `Option` battery + `Option<String>` error (never fails) so one unreachable device doesn't break a batched UI refresh.
- `src/commands/settings.rs` — settings + persistence commands: `get/set_dpi`, `get/set_smart_shift`, `get/set_hires_wheel*`, `get/set_reprog_control`, `get_device_capabilities`, `get_device_firmware`, `save/load/apply_device_settings`. The `with_device(path, index, closure)` helper opens a device and maps `HidppError` → `String`.
- `src/config.rs` — persists settings to `~/.config/logi-linux-app/devices.toml`, **keyed by device product_name** (e.g. `"MX Master 4"`), not by path/index. `apply_saved_settings` is best-effort per field (a failing field only `warn!`s).

**Command conventions:** every hardware command is `async` and wraps its blocking hidapi body in `tokio::task::spawn_blocking` (tokio only has the `rt` feature; Tauri drives async). Commands take `path: String` + `device_index: u8`, open a **fresh** device, do the work, and drop it. **There is no persistent device handle or pool** — short-lived, serialized opens are the deliberate strategy to avoid hidraw response cross-talk.

### HID++ protocol crate — `src-tauri/crates/hidpp/`
Hardware-agnostic protocol layer, pulled in as a path dependency (implicit Cargo member). Decoupled from hidapi via the `HidTransport` trait, so the whole feature layer is **unit-tested with `MockTransport`** (canned byte responses) — the tests double as the spec for exact wire layouts.
- `report.rs` — `HidppReport` models short (report id `0x10`, 7 bytes) and long (`0x11`, 20 bytes) frames. Layout: `[0]=report_id, [1]=device_index, [2]=feature_index, [3]=(function_id<<4)|sw_id, [4..]=params`. `SW_ID = 0x01` is stamped on every request.
- `device.rs` — discovery + probing. `find_logitech_devices()` filters VID `0x046d` **and** usage page `0xFF00`. Receivers (hardcoded `RECEIVER_PIDS`) are probed via `probe_paired_devices()` (loops device indices `1..=6`); a direct-USB device uses index `0xFF`. `read_device_name()` does the HID++ 2.0 handshake for feature `0x0005` — this yields the *paired device's* real name, not the receiver's USB product string.
- `features.rs` — **the heart.** `FeatureAccess` with:
  - `get_feature_index(feature_id)` — IRoot lookup. **Invariant: index 0 == "feature not supported" → `FeatureNotFound`. Feature indices are per-device; never hardcode them.**
  - `request()` — writes a report, reads up to 10 frames matching device/feature/function id (skipping notifications), surfaces error frames as `ProtocolError`.
  - Feature ops: battery, firmware, DPI (`0x2201`), SmartShift (`0x2110`/`0x2111`), hi-res wheel (`0x2121`), reprogrammable controls (`0x1B04`), change-host (`0x1814`).
- `cid_names.rs` — CID → human-readable name table, derived from Solaar's `special_keys.py`. Consult/extend when adding button-remap support.

**Critical protocol invariants:**
- **HID++ 1.0 and 2.0 coexist on the wire.** `0x8F` in byte[2] is a 1.0 error (seen during receiver probing in `device.rs`); `feature_index == 0xFF` is a 2.0 error (in `features.rs`). Both must be handled, in their respective code paths.
- **Battery: call `get_battery()`**, which tries UnifiedBattery (`0x1004` fn 1) then falls back to BatteryStatus (`0x1000` fn 0). Calling the sub-functions directly loses the fallback.
- **SmartShift** tries Enhanced (`0x2111`) first, then standard (`0x2110`); the function ids differ between the two paths and `SmartShiftInfo.is_enhanced` records which was used.
- **Reprog controls (`0x1B04` v4):** a control is remappable iff flags1 **bit 6 (`0x40`, "divertable")** is set.
- **DPI list (`0x2201` fn 1)** uses a range-marker encoding: a value whose top 3 bits are `0b111` is a range (low 13 bits = step, next 2 bytes = range end), otherwise it's a literal DPI; `0x0000` terminates.

### Frontend — `src/`
React 19 + TypeScript + Vite 7. Tailwind **v4** (CSS-first config — no `tailwind.config.js`; tokens live in `src/index.css`), shadcn/ui (new-york) primitives over Radix (unified `radix-ui` package), path alias `@/` → `src/`. Routing via `react-router-dom` v7. No global state store — state is local to hooks and passed by props; the only React Context is the theme provider.
- `src/lib/tauri.ts` — **the single IPC layer.** Every backend call is a typed async wrapper around `invoke<T>("command_name", { camelCaseArgs })`, and all TS interfaces mirroring the Rust structs live here. Components and hooks never call `invoke` directly.
- `src/hooks/use-devices.ts` — device list. Fetches battery **sequentially** per device (avoid hidraw contention), polls every 30s, uses a `refreshingRef` guard against React StrictMode double-mount and overlapping polls.
- `src/hooks/use-device-settings.ts` — the detail-page core. Fetches `DeviceCapabilities`, then **conditionally** loads only the supported features in parallel (each `.catch`ed independently), exposes **optimistic** setters, and **debounce-persists** (500ms) to TOML via `saveDeviceSettings`.
- `src/pages/device-detail.tsx` — reads `:path`/`:deviceIndex` route params (path is URL-encoded), renders `<Tabs>` whose triggers/content are gated on `DeviceCapabilities`. Tab bodies are in `src/components/device-settings/*-tab.tsx`.
- Images: `src/lib/device-image.ts` resolves a device thumbnail by name/id with priority **CDN → local catalog → Lucide icon fallback**. CDN map: `src/data/device-cdn-map.ts`. Local catalog: `src/data/device-catalog.ts` — **auto-generated by `scripts/extract-device-images.py`, do not hand-edit.**

### Adding a new device setting (capability-driven UI)
The entire detail screen — both which data is fetched and which controls render — is gated on backend-reported `DeviceCapabilities`. To add one, touch all layers in order:
1. Add the HID++ feature op in `hidpp::features` (with a `MockTransport` unit test).
2. Add the Tauri command in `src/commands/settings.rs` and register it in the `invoke_handler!` in `src/lib.rs`.
3. Add a typed wrapper + interface in `src/lib/tauri.ts`, and a capability flag on `DeviceCapabilities`.
4. Load it conditionally in `use-device-settings.ts` (state + setter); persist it in `config.rs` if it should survive a restart.
5. Render a section in the relevant `device-settings/*-tab.tsx`, gated on the capability. The `SettingRow` helper in `point-scroll-tab.tsx` is the repeated label+control layout idiom.
