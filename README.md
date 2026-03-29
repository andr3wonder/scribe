# Scribe

Personal meeting transcription and intelligence tool. Records meetings, transcribes locally, generates summaries, and lets you chat with your meeting history.

Built on [Meetily](https://github.com/Zackriya-Solutions/meetily) (open-source meeting assistant).

## What it does

- **Record meetings** — captures mic + system audio (Zoom, Teams, Meet, etc.)
- **Live transcription** — Whisper/Parakeet, GPU-accelerated, runs locally
- **AI summaries** — auto-generated when recording stops (Qwen 3.5 4B on-device or Claude CLI)
- **Chat Q&A** — ask questions about any meeting, grounded in the transcript
- **Global search** — search and ask across all your meetings at once
- **Meeting detection** — system notification when audio apps start, one-click to record
- **Web search** — auto-searches DuckDuckGo when questions need external info
- **Edit via chat** — fix names, correct transcript errors, update summaries by asking
- **Import** — Granola meetings, Teams transcripts, or paste any text
- **Runs in menu bar** — stays alive in background, window hides on close

## LLM providers

| Provider | Setup | Best for |
|----------|-------|----------|
| **Claude CLI** | `claude` installed, no API key | Best quality, uses your existing Claude auth |
| **Qwen 3.5 4B** | Downloads ~2.8GB model | Fully offline, chain-of-thought reasoning |
| **Gemma 3 4B** | Downloads ~2.4GB model | Fast offline fallback |
| **Ollama** | Run `ollama serve` | Any Ollama model |
| **Claude/OpenAI/Groq API** | API key in settings | Cloud providers |

## Install

```bash
# Build from source
cd frontend
pnpm install
pnpm run tauri:build:metal  # macOS with Metal GPU

# App bundle at: target/release/bundle/macos/meetily.app
# Copy to Applications and rename
cp -R target/release/bundle/macos/meetily.app /Applications/Scribe.app
```

## Import existing meetings

```bash
# From Granola (reads local cache)
python3 scripts/import_granola.py

# From Teams (downloaded .txt transcripts)
python3 scripts/import_teams.py ~/Downloads/call-transcript--*.txt

# Or use the Import Transcript button in the sidebar to paste any text
```

## Architecture

- **Desktop**: Tauri (Rust backend + Next.js frontend)
- **Transcription**: Whisper via whisper-rs (Metal GPU) or Parakeet (ONNX)
- **LLM**: llama-helper sidecar (GGUF models) or Claude CLI subprocess
- **Database**: SQLite (via SQLx)
- **Audio**: cpal + CoreAudio, dual mic/system capture with VAD

## What's added over upstream Meetily

- Per-meeting chat Q&A
- Global chat across all meetings
- Mid-meeting live chat during recording
- Auto-summary on recording stop
- Claude CLI as LLM provider
- Web search integration (DuckDuckGo)
- Transcript/summary editing via chat
- Meeting auto-detection (system notifications)
- Granola and Teams import scripts
- Import Transcript UI
- Qwen 3.5 4B as default on-device model
- Stripped all branding/analytics/telemetry
- Menu bar persistence (hide on close)
