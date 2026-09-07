<p align="center">
  <img src="public/nexq-icon.png" alt="NexQ" width="120">
</p>
<p align="center">
  <strong>AI Meeting Assistant & Real-Time Interview Copilot</strong>
</p>

<p align="center">

[![Release](https://img.shields.io/github/v/release/hanhan761/forbb?style=flat-square&color=blue)](https://github.com/hanhan761/forbb/releases)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/hanhan761/forbb/release.yml?style=flat-square&label=build)](https://github.com/hanhan761/forbb/actions/workflows/release.yml)
[![Downloads](https://img.shields.io/github/downloads/hanhan761/forbb/total?style=flat-square&color=orange)](https://github.com/hanhan761/forbb/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?style=flat-square&logo=windows)](https://github.com/hanhan761/forbb/releases)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=flat-square&logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=white)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square)](CONTRIBUTING.md)

</p>

<p align="center">
  <img src="website/public/screenshots/live-meeting-demo.gif" alt="NexQ in action — live meeting with transcript, translation, and AI assist" width="700">
</p>
<p align="center"><em>NexQ overlay during a live interview — real-time transcription and AI suggestions</em></p>

<p align="center">
  <img src="docs/assets/readme-feature-overview.png" alt="NexQ feature overview — system audio, live transcription, translation, and local knowledge base" width="900">
</p>
<p align="center"><em>System-audio loopback, live transcription, bilingual translation, and local knowledge retrieval in one assistive overlay</em></p>

### Why NexQ?

🔒 **Local-first** — personal files, indexes, sessions, and recordings stay local; cloud services are opt-in

🆓 **Free & Open Source** — no subscriptions, no limits, ever

⚡ **10 STT + 9 LLM providers** — from local Whisper, Ollama, and Codex to cloud Deepgram & OpenAI

## Features

- **Dual-party transcription** — captures mic ("You") and system audio ("Them") simultaneously
- **Real-time AI copilot** — get streaming, speakable answers and follow-up suggestions from local Codex or other LLM providers
- **Local RAG pipeline** — index your own documents (PDF, DOCX, TXT, MD) for context-aware AI responses
- **Obsidian Vault import** — bring a local Markdown Vault into the knowledge base without an Obsidian plugin
- **Interview Query Router** — automatically choose direct Codex, local files, or web search and show source/confidence metadata
- **Interview preparation + review** — check devices/services before starting, save bilingual transcripts, and write a dated review back to Obsidian
- **Mock Interview** — text-only practice with follow-up questions and coaching across technical accuracy, English, structure, confidence, and profile consistency
- **Gemini Context Cache** — upload documents to Gemini once, skip local embedding entirely for ~3-5s faster queries
- **10 STT providers** — Web Speech API, Deepgram, Groq, Whisper, ONNX Runtime, and more
- **Always-on-top overlay** — compact, transparent floating window visible only to you
- **Bookmarks & action items** — pin key moments and auto-extract tasks
- **Speaker labeling** — identify and name each speaker in the transcript
- **Multi-language translation** — real-time translation via 5 providers (100+ languages)
- **Audio recording & playback** — record meetings as WAV, replay with synced transcript
- **Meeting scenarios** — pre-configured templates for interviews, lectures, and team meetings

## Installation

### Windows users: install the pure API version

The pure API version is the prebuilt **Remote-only** installer. It does not require
Node.js, Rust, the Tauri CLI, or local AI model downloads.

**Current direct download:** [NexQ_2.20.10_x64-setup.exe](https://github.com/hanhan761/forbb/releases/download/remote-v2.20.10/NexQ_2.20.10_x64-setup.exe)

1. Open the [Releases](https://github.com/hanhan761/forbb/releases) page.
2. Open the latest release whose name starts with **NexQ Remote-only**.
3. Download the `x64-setup.exe` asset and run it. The installer uses the current-user
   install mode and does not require administrator privileges.
4. In NexQ Settings, configure the cloud STT, translation, and LLM API keys.

This version keeps audio capture, meeting history, recording, and the UI on the
device. Speech recognition, translation, and LLM requests use the configured cloud
APIs. It does not include local Whisper, Ollama embeddings, or other local AI
engines; document search uses local SQLite keyword search.

> End users should download the installer above. Do not run `npm install` or
> `npx tauri build` unless you are building the application from source.

After installation, continue with the [Getting Started Guide](docs/user-guide/getting-started.md)
or browse the [full user guides](docs/user-guide/).

For live interviews, follow the meeting platform, school, and local consent rules for recording and AI assistance.

### Developer build: Remote-only from source

NexQ also provides a smaller remote-only desktop build for machines that should not
download or run local AI models. It keeps microphone/system-audio capture, the local
SQLite meeting history, recording, and the UI on the device, while sending speech
recognition, translation, and LLM requests to the configured cloud APIs. Document RAG
uses local SQLite keyword search; Ollama embeddings are not included or started.

This path compiles the Tauri/Rust application and is intended for maintainers and
developers, not end users.

Build it from source with:

```bash
npm ci
npm run build:remote
```

The `build:remote` command disables Cargo's default `local-ai` feature. Do not replace
it with `npx tauri build`, or omit `--no-default-features` from the remote build, because
that will compile the slower all-in-one local AI version. The remote-only build requires
API keys for the selected STT, translation, and LLM providers; those keys are stored
through NexQ's credential manager and are not hard-coded into the app.

Remote-only installers are published by the
[remote release workflow](.github/workflows/release-remote.yml) for tags matching
`remote-v*`. To publish one, push a tag such as `remote-v2.20.10`, or run the
workflow manually from GitHub Actions and provide the tag as an input. A release
must be published before end users can download the installer.

## Gemini Context Cache

For users running NexQ on a laptop without a dedicated GPU, local embedding can add 2–5 seconds of latency per AI query. The **Gemini Context Cache** feature eliminates this entirely.

Instead of embedding documents locally via Ollama on every query, NexQ uploads your context documents to Gemini's servers once per meeting session. Gemini pre-processes and stores the KV state. Every subsequent query skips local embedding completely — only the live transcript and your question are sent fresh.

**Setup:**
1. Load your context documents (PDF, DOCX, TXT) in the Context panel
2. Go to **Settings → Context Strategy**
3. Select **Gemini Context Cache**
4. Choose your model and TTL, then click **Create Cache from Context Docs**

**Requirements:** Google Gemini API key, documents loaded in context.

**Speed comparison (CPU-only laptop):**

| Mode | Per-query overhead | Notes |
|------|-------------------|-------|
| Local RAG (`all-minilm`) | ~1–2s | Fastest local option |
| Local RAG (`nomic-embed-text`) | ~3–5s | Default model |
| **Gemini Context Cache** | **~0s** | No local embedding at all |

Cache expires after your chosen TTL (30 min – 24 hours). Delete it early from the same settings panel.

## Why NexQ vs. Others?

| | NexQ | Otter.ai | Granola | Krisp |
|---|:---:|:---:|:---:|:---:|
| **Price** | **Free** | $8+/mo | $18/mo | $16/mo |
| **100% Local** | Yes | No | Partial | Partial |
| **Open Source** | Yes | No | No | No |
| **No Bot Joins** | Yes | No | Yes | Yes |
| **STT Providers** | **10** | 1 | 1 | 1 |
| **LLM Providers** | **8** | 1 | 1 | 1 |
| **Local LLM** | Yes | No | No | No |
| **RAG / Doc Context** | Yes | No | No | No |

## Screenshots

| Live Interview | Lecture Mode | Past Meeting Review |
|:---:|:---:|:---:|
| ![Interview](website/public/screenshots/Interview.png) | ![Lecture](website/public/screenshots/Lecture.png) | ![Past Meeting](website/public/screenshots/Past-meeting.png) |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri 2 (Rust + WebView2) |
| Frontend | React 18, TypeScript 5.5, Vite 6 |
| State | Zustand 4.5 |
| Styling | Tailwind CSS 3.4, shadcn/ui |
| Audio | cpal, WASAPI (Windows loopback) |
| STT | whisper-rs, ONNX Runtime, Deepgram, Groq, Web Speech API |
| LLM | Local Codex app-server, OpenAI, Anthropic, Groq, Ollama, LM Studio, Gemini |
| Database | SQLite (rusqlite) |

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) (`npm install -g @tauri-apps/cli`)

### Setup

```bash
# Clone the repository
git clone https://github.com/hanhan761/forbb.git
cd forbb

# Install frontend dependencies
npm install

# Run in development mode (launches Rust backend + React frontend)
npx tauri dev

# Build production installer
npx tauri build
```

### Other Commands

```bash
npm run dev       # Vite dev server only (port 5173)
npm run build     # TypeScript check + Vite production build
npm run build:remote # Remote-only Tauri installer without local AI features
```

## Windows SmartScreen

When you first run NexQ, Windows SmartScreen may display a warning. This is normal for open-source applications that are not code-signed. To proceed:

1. Click **"More info"**
2. Click **"Run anyway"**

Code signing certificates are expensive and not feasible for most open-source projects. The application is safe to run — you can verify by building from source.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT License](LICENSE) — free forever.

## Acknowledgments

- [Tauri](https://tauri.app/) — desktop application framework
- [React](https://react.dev/) — user interface library
- [whisper-rs](https://github.com/tazz4843/whisper-rs) — Rust bindings for OpenAI Whisper
- [Deepgram](https://deepgram.com/) — speech-to-text API
- [shadcn/ui](https://ui.shadcn.com/) — UI component library
