# Hallucinator web demo — design

Date: 2026-10-22
Status: approved in chat, awaiting spec review

## Idea

A browser version of Hallucinator (the desktop LLM calculator) so people can try the gag in
one click from a Reddit/HN post. Same look and behaviour as the desktop app; every answer
comes from a tiny LLM running fully client-side via `wllama` (llama.cpp in WebAssembly). The
app never does real math — whatever the model says is the result. The wide launch (Show HN,
r/ProgrammerHumor, TikTok) waits for this demo.

## Decisions

| Topic | Decision |
|---|---|
| Model | Gemma 3 270M instruct, `ggml-org/gemma-3-270m-it-GGUF`, file `gemma-3-270m-it-Q8_0.gguf` (291,545,600 bytes ≈ 278 MiB), not gated. Same family as the desktop app. |
| Runtime | `@wllama/wllama` 3.8.x from npm; its `.wasm` is bundled into our build by Vite. |
| Model hosting | Downloaded straight from Hugging Face; cached by wllama's cache manager. We host only the static app. |
| Repo layout | `web/` folder inside `llm-calc`, its own `package.json`/Vite config; imports the pure modules `../../src/calc/{keypad,extractNumber}.ts` directly (single source of truth, shared tests). Branch `feat/web-demo`. |
| Hosting | Cloudflare Pages (static `web/dist`), `_headers` enabling cross-origin isolation for multi-threaded wasm. Deploy only after the owner's yes. |
| Download UX | Starts immediately on page load; the calculator is usable for typing while it downloads. |
| Extras in v1 | Temperature slider (0–2), footer with links, fixed "Confidence: 100%" label. |

Out (YAGNI): model choice, top-k/top-p, share card, personalities, recording mode, analytics,
PWA/offline install, i18n. UI language: English.

## User-facing behaviour

Identical to the desktop calculator view (`src/views/calculator.ts`) unless stated:

- Display: expression input (editable), big result (number extracted from the model's answer
  by `extractNumber`; `Error` if none), small scrollable thoughts line streaming the raw answer.
- Under the result, always: `Confidence: 100%` (static text, never computed).
- Keypad layout, keyboard mapping, chaining from the model's number, `Enter`/`=` asks,
  `Esc` cancels a running answer (thoughts get ` (interrupted)`, result unchanged) or clears
  when idle — all as on desktop, reusing `press`/`applyAnswer`.
- Header: left `Gemma 3 270M · in your browser`, right ⚙.
- Loading: while the model is not ready, the banner shows
  `Downloading a brain… 42% (117 / 278 MB)`, then `Waking up the brain…`; on a cached
  reload it goes straight to `Waking up the brain…`. Pressing `=` before ready does nothing
  (same as desktop when the engine is not ready). Typing works throughout.
- Errors (banner + Retry, which re-runs loading):
  - no WebAssembly → `Your browser can't run the brain (no WebAssembly).` (no Retry);
  - download/network failure → `Couldn't download the brain: <reason>`;
  - load failure (e.g. out of memory) → `The brain failed to start: <reason>`;
  - a failed answer → result `Error`, thoughts show the error text (as on desktop).
- Settings (⚙ opens a small panel over the keypad; ✕ closes):
  - Temperature slider 0.0–2.0, step 0.1, default 0.7; persisted in `localStorage`
    (try/catch; falls back to default).
  - `Forget the brain (278 MB)` — clears wllama's cache; the next page load downloads again.
- Footer (small, muted): `Runs entirely in your browser · Desktop version · GitHub`. Link
  targets come from one config object (`web/src/config.ts`); until the repo is public they
  point to `#` and are hidden when empty.
- Layout: the desktop window size (≈360×560) centred on wide screens; full-width on phones
  (16px gutter, no horizontal scroll).

## Architecture

```
web/
  index.html
  package.json            (@wllama/wllama, vite, typescript, vitest)
  vite.config.ts          (dev/preview servers send COOP/COEP like production)
  public/_headers         (Cloudflare Pages headers)
  src/
    config.ts             model repo/file/size, links, prompt, max tokens
    engine.ts             the only module that knows wllama
    format.ts             pure: progress text, temperature parse/clamp
    format.test.ts
    calculator.ts         view: DOM, keys, settings panel; uses engine + ../../src/calc
    main.ts               boot: WebAssembly check, mount view, start engine.load()
    style.css             desktop style.css + header/footer/panel/responsive additions
  README.md               dev, build, hosting plan
```

### engine.ts

```ts
type EngineState =
  | { state: 'downloading'; loaded: number; total: number }
  | { state: 'starting' }
  | { state: 'ready' }
  | { state: 'failed'; reason: string; stage: 'download' | 'start' };

interface Engine {
  load(): Promise<void>;                       // idempotent while loading; re-runnable after failure
  onState(cb: (s: EngineState) => void): void;
  ask(expr: string, temperature: number, onToken: (t: string) => void, signal: AbortSignal)
    : Promise<{ text: string; interrupted: boolean }>;
  forget(): Promise<void>;                     // clears the model cache
}
```

- `load()` → `new Wllama({ default: wasmUrl })` (wasm URL via Vite `?url` import),
  `loadModelFromHF({ repo, file }, { progressCallback, n_ctx: 512 })`. Progress events map to
  `downloading`; after the download completes (or when the file is cached) state is
  `starting`; resolve → `ready`. Download vs start failure decided by whether the last progress
  reached `total`.
- `ask()` → `createChatCompletion({ messages: [system, user], stream: true, onData,
  abortSignal, max_tokens: 64, temperature, top_k: 40, top_p: 0.95 })`.
  System prompt is the desktop one: `You are a calculator. Reply with the result of the
  expression.`; user message is the expression verbatim. Tokens from
  `chunk.choices[0].delta.content` go to `onToken` and accumulate. An abort resolves with
  `interrupted: true` and the text so far (never throws for an abort).
- One request at a time; the view already blocks input while busy.
- WebGPU: wllama enables it automatically when available; we leave its defaults.

### Data flow

`main.ts` mounts the view with the engine, then calls `engine.load()`. The view subscribes to
`onState` for the banner. `=` → `engine.ask(calc.expr, temperature, …)` → on completion
`applyAnswer(calc, text)`; on interrupt thoughts only. Shared modules are unchanged.

### Hosting

- `web/public/_headers`:
  ```
  /*
    Cross-Origin-Opener-Policy: same-origin
    Cross-Origin-Embedder-Policy: credentialless
  ```
  `credentialless` (not `require-corp`) keeps the cross-origin model download from Hugging Face
  working while enabling `SharedArrayBuffer` → multi-threaded wasm. Browsers without
  `credentialless` (older Safari) fall back to wllama's single-thread build automatically.
- Cloudflare Pages project `hallucinator`, build output `web/dist`. Deploy by
  `npx wrangler pages deploy web/dist --project-name hallucinator` or a Git-connected project
  (root `web`, build `npm ci && npm run build`). Neither happens without the owner's yes.
- Pages per-file limit is 25 MiB; the wasm is well under it, the model is not hosted by us.

## Testing

- Vitest: `format.ts` (progress text rounding/MB, unknown total; temperature parse/clamp).
  Existing `src/calc/*` tests keep covering keypad/extraction.
- `tsc` + `vite build` succeed.
- Manual-by-agent in the built-in browser against `vite preview` (same headers):
  first load downloads with progress; `crossOriginIsolated === true` and multi-thread in use;
  `2+2`, `17×23` produce a result and streamed thoughts; chaining `+1` uses the model's number;
  `Esc` mid-answer interrupts; temperature persists across reload; reload loads from cache
  (no download); "Forget the brain" then reload re-downloads; phone-width layout has no
  horizontal scroll.

## Risks

- 278 MB on mobile data — accepted (owner's choice); the size is shown in the banner.
- Tiny model may produce no number → `Error`; that is part of the joke.
- Gemma terms of use: we don't redistribute weights (HF serves them); the header names the model (Gemma 3 270M).
