# Vispa Flow — agent.md (v1)

## What This App Is

Windows desktop voice dictation app. Hold a hotkey, speak, release — clean text appears
in whatever app is active. Free, local-first, no cloud required by default.

This is a real desktop product, not a script. Build it like one.

---

## Architecture Overview

```
┌─────────────────────────────────────────┐
│  Tauri Shell (Rust)                     │
│  • Global hotkey listener               │
│  • System tray + menu                   │
│  • Text injection into active window    │
│  • IPC bridge to React UI               │
│  • Tauri updater                        │
│  • MSI packaging                        │
├─────────────────────────────────────────┤
│  React Frontend                         │
│  • Settings window (shown on demand)    │
│  • Tray popup (status + quick settings) │
├─────────────────────────────────────────┤
│  Python AI Service (sidecar process)    │
│  • Mic capture (sounddevice)            │
│  • Silero VAD (speech detection)        │
│  • faster-whisper (local STT)           │
│  • Filler word removal                  │
│  • Cloud STT fallback (Groq / OpenAI)   │
│  • Communicates with Rust via stdin/    │
│    stdout JSON-RPC or local TCP socket  │
└─────────────────────────────────────────┘
```

Rust owns the OS layer. Python owns AI. React owns UI. They talk via IPC.

---

## Tech Stack (v1 — Locked)

| Layer | Choice |
|---|---|
| Desktop shell | Tauri v2 |
| Rust (Tauri backend) | stable toolchain |
| Frontend | React 18 + TypeScript |
| Frontend styling | Tailwind CSS |
| Python version | 3.11 |
| STT | faster-whisper |
| Default model | auto-selected by RAM (see below) |
| VAD | Silero VAD (via silero-vad Python package) |
| Mic capture | sounddevice |
| Cloud STT (opt-in) | Groq API or OpenAI API (openai SDK, same interface) |
| Database / config | SQLite via rusqlite (Rust side) |
| Packaging | Tauri MSI builder (built-in) |
| Auto-update | Tauri updater (built-in) |

Do not introduce any library outside this list without explicit approval.

---

## Project File Structure

```
vispa-flow/
├── src-tauri/                  # Rust / Tauri backend
│   ├── src/
│   │   ├── main.rs             # Tauri entry point
│   │   ├── hotkey.rs           # Global hotkey registration
│   │   ├── injector.rs         # Text injection into active window
│   │   ├── tray.rs             # System tray icon + menu
│   │   ├── db.rs               # SQLite config read/write
│   │   └── ipc.rs              # Commands exposed to frontend + sidecar bridge
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                        # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── pages/
│   │   └── Settings.tsx        # All settings on one page
│   └── components/
│       └── TrayPopup.tsx       # Small status popup from tray
│
├── python/                     # Python AI sidecar
│   ├── main.py                 # Entry point — starts socket server, listens for commands
│   ├── transcriber.py          # faster-whisper wrapper (lazy load/unload)
│   ├── recorder.py             # sounddevice mic capture
│   ├── vad.py                  # Silero VAD wrapper
│   ├── cleaner.py              # Filler word removal
│   ├── cloud.py                # Optional Groq / OpenAI STT
│   └── ram_guard.py            # Auto model tier selection
│
├── assets/
│   └── icon.png                # 32x32 app icon
│
├── tests/
│   ├── test_transcriber.py
│   ├── test_vad.py
│   └── test_cleaner.py
│
├── requirements.txt            # Python deps
├── package.json                # React deps
└── agent.md                    # This file
```

---

## Rust Side — Behavior Contracts

### `hotkey.rs`
- Register a global hotkey (default: `Ctrl+Alt+Space`) using Tauri's global shortcut plugin.
- On press: send `{"cmd": "start_recording"}` to Python sidecar.
- On release: send `{"cmd": "stop_recording"}` to Python sidecar.
- Hotkey combo is read from SQLite at startup. Re-register on settings save.

### `injector.rs`
- Receive transcribed text string from Python via IPC.
- Write text to clipboard, then simulate `Ctrl+V` into the currently focused window.
- On Windows: use `windows` crate for clipboard and `SendInput` for keypress simulation.
- Wait 50ms before simulating paste to allow focus to settle.
- Save clipboard content before injection, restore it after.

### `tray.rs`
- System tray icon with three visual states:
  - **Idle** — grey icon
  - **Recording** — red icon
  - **Transcribing** — amber icon
- Right-click menu: `Settings` | `Status: <state>` (non-clickable) | `Quit`
- Left-click: show small TrayPopup React window.

### `db.rs`
- Single SQLite file at `%APPDATA%\VispaFlow\config.db`.
- Create on first run if missing.
- One table: `settings (key TEXT PRIMARY KEY, value TEXT)`.
- Default rows inserted on first run (see Config Defaults below).
- Expose Tauri commands: `get_setting(key)`, `set_setting(key, value)`.

### `ipc.rs`
- Spawn Python sidecar (`python/main.py` compiled to sidecar exe via PyInstaller).
- Communicate via local TCP socket on `127.0.0.1:51337`.
- Message format: newline-delimited JSON.
  - Rust → Python: `{"cmd": "start_recording"}` / `{"cmd": "stop_recording"}`
  - Python → Rust: `{"event": "transcribed", "text": "..."}` / `{"event": "state", "value": "recording|transcribing|idle"}`
- On receiving `transcribed` event: call `injector.rs` with the text.
- On receiving `state` event: update tray icon state.

---

## Python Side — Behavior Contracts

### `ram_guard.py`
Auto-select Whisper model based on available system RAM:

| Total RAM | Model |
|---|---|
| < 4 GB | `tiny.en` |
| 4–6 GB | `tiny` |
| 6–10 GB | `base` |
| > 10 GB | `small` |

- If user has set `model_override` in config, use that instead.
- Expose: `get_model() -> str`

### `vad.py`
- Load Silero VAD model on startup (it is small — ~1MB, always resident).
- Expose: `is_speech(audio_chunk: np.ndarray, sample_rate: int) -> bool`
- Used by `recorder.py` to gate transcription — only send audio to Whisper if VAD confirms speech was detected.
- This prevents wasted transcriptions on silence or background noise.

### `recorder.py`
- Capture mic audio at 16kHz mono float32 (Whisper's required format).
- On `start_recording` command: begin buffering audio chunks.
- On `stop_recording` command: run VAD on the buffer. If speech detected, return audio array. If no speech, return `None` (skip transcription silently).
- Max recording: 120 seconds. Auto-stop if exceeded.
- Expose: `start()`, `stop() -> np.ndarray | None`

### `transcriber.py`
- **Lazy load:** Do not load Whisper at startup. Load on first transcription call.
- **Warm window:** After transcription, keep model in RAM for 60 seconds. Unload if idle longer.
- Unload: set model to `None`, call `gc.collect()`.
- Expose: `transcribe(audio: np.ndarray) -> str`
- All exceptions caught, logged, re-raised as `TranscriptionError`.
- Log load/unload at INFO level.

### `cleaner.py`
- Post-process transcribed text to remove filler words.
- Only runs if `remove_fillers = true` in config.
- Regex-based for v1. Remove: `um`, `uh`, `hmm`, `you know`, `basically`, `literally`, and leading/trailing `so`, `right`.
- Expose: `clean(text: str) -> str`

### `cloud.py`
- Only used when `use_cloud = true` in config.
- Use `openai` SDK for both Groq and OpenAI (Groq is OpenAI-API-compatible).
  - Groq base URL: `https://api.groq.com/openai/v1`
  - Model: `whisper-large-v3-turbo`
- Save audio to temp `.wav`, send to API, delete temp file.
- If cloud call fails: log warning, fall back to local transcription automatically.
- Expose: `transcribe_cloud(audio: np.ndarray) -> str`

### `main.py`
- Start TCP socket server on `127.0.0.1:51337`.
- Listen for JSON commands from Rust.
- On `start_recording`: call `recorder.start()`.
- On `stop_recording`: call `recorder.stop()` → if audio returned → run VAD → transcribe (local or cloud) → clean → send `transcribed` event back to Rust.
- Send `state` events to Rust at each stage transition.

---

## React Frontend — Behavior Contracts

### `Settings.tsx`
Single-page settings window. No tabs. Fields:

1. **Hotkey** — text display + "Change" button
2. **Model** — dropdown: Auto / tiny.en / tiny / base / small
3. **Language** — dropdown: Auto + common languages
4. **Remove filler words** — toggle (default: on)
5. **--- Cloud API ---** section
6. **Enable cloud STT** — toggle (default: off)
7. **Provider** — dropdown: Groq / OpenAI (shown only when cloud enabled)
8. **API Key** — password input (shown only when cloud enabled)
9. **Save** button

On save: call `set_setting` Tauri command for each changed field.

### `TrayPopup.tsx`
Small floating window triggered by left-click on tray icon.
Shows: current status, hotkey reminder, quick link to Settings.
Closes on click-outside or Escape.

---

## Config Defaults (SQLite rows on first run)

```
hotkey          = ctrl+alt+space
model_override  = (empty — means auto)
use_cloud       = false
cloud_provider  = groq
cloud_api_key   = (empty)
language        = auto
remove_fillers  = true
warm_seconds    = 60
```

---

## Threading / Async Rules

- Rust: use Tauri's async command system. No blocking on main thread.
- Python: use `threading` for socket server + recorder. Keep it simple — no asyncio for v1.
- React: all Tauri IPC calls are async/await. Always handle errors with try/catch.
- Never call UI updates from Python — only Rust talks to the frontend.

---

## Error Handling

- **User-visible:** Windows toast notification for: mic not found, model download failed, hotkey conflict.
- **Silent:** Injection failure, VAD edge cases — log only.
- **Fatal crash:** Write to `%APPDATA%\VispaFlow\crash.log` before exit.
- Log file: `%APPDATA%\VispaFlow\vispa.log`. Rotate at 5MB, keep 2 backups.
- Python logging: use stdlib `logging`. Rust logging: use `log` + `tauri-plugin-log`.

---

## First Run Flow

1. App launches → Python sidecar starts → socket connection established.
2. Check if `config.db` exists. If not: create it, insert defaults.
3. Auto-detect RAM → select model tier → store as active model.
4. Whisper model is NOT downloaded yet — deferred to first hotkey use.
5. Show tray icon in idle state.
6. Show one balloon: `"Vispa Flow ready. Hold Ctrl+Alt+Space to dictate."`

---

## Out of Scope for v1

- Custom vocabulary / personal dictionary
- LLM post-processing (Gemini, GPT, Claude, Llama) — v2
- NVIDIA Parakeet — v2
- whisper.cpp — v2
- Voice commands / command mode
- Mac / Linux
- Any analytics or telemetry — never

---

## Definition of Done (v1)

- [ ] Hold hotkey → mic records
- [ ] Release → VAD confirms speech → transcribes locally
- [ ] Clean text injected into active window via paste
- [ ] Filler words removed
- [ ] Tray icon reflects idle / recording / transcribing
- [ ] Settings window opens, saves, applies without restart
- [ ] Cloud STT path works with valid Groq key
- [ ] Idle RAM under 50MB
- [ ] Peak RAM under 600MB on 8GB machine
- [ ] App starts in under 3 seconds
- [ ] Cold transcription (model load) under 8 seconds
- [ ] Warm transcription under 3 seconds
- [ ] MSI installer produced by `tauri build`
- [ ] Runs on Windows 10 and Windows 11 without any manual dependency install
