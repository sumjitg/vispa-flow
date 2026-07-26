# VoiceType — Windows Voice Dictation

Hold a hotkey, speak, release — clean text appears in whatever app is active. Free, local-first, no cloud required by default.

## Architecture

- **Tauri** (Rust) — OS shell: global hotkeys, tray icon, text injection, Python bridge
- **React** (TypeScript) — Settings window + tray popup
- **Python** (sidecar) — AI engine: mic capture, VAD, faster-whisper STT, filler word cleaning

## Tech Stack

| Layer | Choice |
|---|---|
| Desktop shell | Tauri v2 |
| Frontend | React 18 + TypeScript + Tailwind CSS |
| STT | faster-whisper (local), Groq/OpenAI (optional cloud) |
| VAD | Silero VAD |
| Database | SQLite (Rust, rusqlite) |
| Packaging | MSI (Tauri bundler) |

## Development

```bash
# Install Python deps
pip install -r requirements.txt

# Run (Tauri dev)
npm run tauri dev
```

## License

GPLv3 — see [LICENSE](LICENSE). Copyright (c) 2026 Soumyajit Ghosh.
