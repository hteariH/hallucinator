# LLM Calculator — design

Date: 2026-10-22
Status: approved in chat, awaiting spec review

## Idea

A desktop calculator that looks and behaves like an ordinary calculator, but every
expression is evaluated by a local LLM. It is an art piece / joke: whatever the model
answers is the truth. The app never checks or corrects the model with real math.

## Scope

In:
- Windows desktop app (Tauri 2), public release on GitHub with an NSIS installer.
- Fully self-contained: bundles its own LLM runtime (llama.cpp `llama-server`), no Ollama
  or other prerequisites. Model weights are downloaded on first run.
- Two models: Gemma 3 4B (default) and Gemma 3 1B (for weaker PCs).
- Settings: model choice, download/delete model, temperature, top-k, top-p.

Out (YAGNI):
- History tape, memory keys (M+, MR), scientific functions.
- Real-math verification or error counting.
- Linux / macOS builds (possible later; nothing in the design blocks them).
- Custom / user-supplied models.

UI language: English.

## User-facing behaviour

### Window and display

Small resizable window with a classic calculator layout. The display has three lines:

1. **Expression** — a real text input. Type, paste, move the caret.
2. **Result** — large; the number extracted from the model's answer.
3. **Thoughts** — small, scrollable; the model's full raw answer, streamed as it is generated.

### Buttons and keys

Buttons: `0–9 . + − × ÷ ( ) % ± = C ⌫` and a gear icon for settings.
Keyboard: digits and operators type into the expression; `Enter` = `=`;
`Esc` = cancel a running request, otherwise `C`; `Backspace` = `⌫`.

### Pressing `=`

- The expression string is sent to the model as-is. The app does not parse, validate,
  or balance parentheses.
- While generating: a "thinking…" animation, tokens stream into the Thoughts line,
  `=` is disabled, `Esc` aborts.
- On completion: the first number in the answer goes to the Result line. If there is no
  number, Result shows `Error`; Thoughts keep the raw answer.
- Chaining is honest: pressing an operator after a result continues from the model's
  number, even if it is wrong (`2+2` → `5`, then `×3` sends `5×3`).
- Aborted request: Result keeps its previous value, Thoughts show what arrived plus
  "(interrupted)".

### Prompt

Short and neutral, roughly: "You are a calculator. Reply with the result of the
expression." The expression is passed as the user turn through the model's chat
template. No few-shot examples, no chain-of-thought instructions.

### Settings

- Model: Gemma 3 4B / Gemma 3 1B, each with size, status (not downloaded /
  downloading N% / ready) and Download / Delete buttons. Selecting a ready model
  switches the engine to it.
- Temperature: 0–2, default 0.7.
- Top K: 1–100, default 40.
- Top P: 0–1, default 0.95.
- "Reset to defaults" for the three sampling values.
- All settings persist between launches and apply from the next `=`.

### First run

1. No model on disk → "Download a brain" screen.
2. The app detects dedicated VRAM. ≥ 4 GB → 4B preselected, otherwise 1B. The user can
   pick either.
3. Download progress → engine start → calculator.

## Architecture

### Frontend (TypeScript, no framework)

Responsible only for presentation: calculator view, settings view, first-run view.
Talks to the Rust core via Tauri commands and listens to Tauri events.

Pure modules with unit tests:
- `extractNumber(text): string | null` — first number in the text. Supports a leading
  minus, decimals, thousands separators (`1,234.5`), exponent (`1e6`). Returns the
  number as the model wrote it (normalised separators), never re-computed.
- Expression editing helpers (append operator, `±`, `⌫`, chaining from the last result).

### Rust core — three modules

**`engine`** — owns the `llama-server` sidecar process.
- Picks a free localhost port, starts the server with the selected model file, polls
  `/health` until ready.
- Restarts it when the model changes.
- Kills it on app exit (no zombie processes).
- If startup with GPU (Vulkan) fails, retries once with CPU only (`-ngl 0`).
- Emits engine status events: `starting`, `ready`, `failed(reason)`, `restarting`.

**`models`** — the model catalogue and files on disk.
- Static catalogue of two entries: id, display name, Hugging Face URL, size, SHA-256.
  Files: Q4_K_M GGUFs from `ggml-org/gemma-3-4b-it-GGUF` and `ggml-org/gemma-3-1b-it-GGUF`;
  exact URLs and hashes are pinned in the catalogue when it is written.
- Stores files in `%LOCALAPPDATA%\llm-calc\models`.
- Downloads with progress events and resume (HTTP Range) into a `.part` file, verifies
  SHA-256, then renames. Hash mismatch → delete and report.
- Delete model. Deleting the active model stops the engine and returns to first-run if
  no model is left.

**`ask`** — one calculation.
- Builds the request to `/completion` (or `/v1/chat/completions`) with
  `stream: true`, temperature, top_k, top_p, and a modest `n_predict` cap (e.g. 128).
- Forwards tokens to the frontend as events; returns the full text at the end.
- Supports cancellation (Esc) by dropping the HTTP stream.
- One request at a time.

**Settings persistence** — a small JSON file in the app data dir (or the Tauri store
plugin).

**VRAM detection** — DXGI adapter enumeration on Windows (max dedicated video memory
across adapters). Failure → treat as < 4 GB.

### Data flow for `=`

```
UI expression ──invoke("ask", expr, sampling)──▶ ask ──HTTP stream──▶ llama-server
UI thoughts  ◀──event("token")────────────────── ask ◀────tokens─────
UI result    ◀──resolve(full text) → extractNumber
```

## Error handling

| Situation | Behaviour |
|---|---|
| Engine fails to start on GPU | Retry once on CPU; if that fails, error screen with the reason and a Retry button |
| Engine crashes during a request | Result `Error`, Thoughts "brain crashed, restarting…", engine restarted |
| Download network error | Retry button; resumes from the `.part` file |
| Hash mismatch | Delete file, show message, allow re-download |
| Not enough disk space | Show message before/at failure, no partial model left marked ready |
| No number in answer | Result `Error`, Thoughts show raw answer |

## Build and release

- GitHub Actions on `windows-latest`, triggered by a version tag.
- Downloads a pinned `llama.cpp` release (`llama-<build>-bin-win-vulkan-x64.zip`),
  places `llama-server.exe` and its DLLs as a Tauri sidecar / resources.
- Builds the NSIS installer with `tauri build` and attaches it to the GitHub release.
- Same release flow as rinkaStreamTools.

## Testing

- Unit tests (frontend): `extractNumber` cases (integers, negatives, decimals, thousands
  separators, exponent, no number, number buried in prose), expression helpers.
- Rust tests: `models` — hash verification, resume from `.part` against a local test
  HTTP server; `ask` — request body built from settings.
- Manual run check: install, first-run download of the 1B model, `=` on a few
  expressions, Esc cancellation, model switch, settings change applied.

## Open details settled during implementation

- Exact llama.cpp build number and GGUF URLs/hashes are pinned in code when written.
- App name / identifier: working name "LLM Calculator", folder and id `llm-calc`.
