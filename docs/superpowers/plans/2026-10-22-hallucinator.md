# Hallucinator Launch Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the app to Hallucinator and add personality presets, a fixed "Confidence: 100%" line, a Share PNG card copied to the clipboard, and a 9:16 recording mode.

**Architecture:** Personality lives in Rust settings and picks the system prompt in `ask::request_body`. The frontend gets small pure modules (`personality.ts`, `share/cardLayout.ts`, `recording.ts`) that are unit-tested under vitest's node environment; the canvas drawing, clipboard and window calls are thin adapters checked manually.

**Tech Stack:** Tauri 2, Rust (serde, tokio), vanilla TypeScript + Vite, vitest, `tauri-plugin-clipboard-manager`.

**Spec:** `docs/superpowers/specs/2026-10-22-hallucinator-design.md`

## Global Constraints

- The app never computes math; the result is whatever `extractNumber` finds in the model's answer.
- UI copy in English. Public name `Hallucinator`, tagline `The calculator that hallucinates.`
- `identifier` stays `dev.aberk.llmcalc`; crate/npm/repo names stay `llm-calc`.
- `Confidence: 100%` is a constant, never computed.
- Recording mode is never persisted.
- Nothing is pushed or published.
- Tests: `npm test` and `cargo test --manifest-path src-tauri/Cargo.toml` must pass after every task; `npm run build` (tsc) must pass.

---

### Task 1: Rename to Hallucinator

**Files:**
- Modify: `src-tauri/tauri.conf.json` (`productName`, window `title`)
- Modify: `index.html` (`<title>`)
- Modify: `src-tauri/src/main.rs` (`expect` message)
- Modify: `src/views/firstRun.ts` (intro paragraph)

- [ ] **Step 1:** `productName` → `"Hallucinator"`, window `"title"` → `"Hallucinator"`; `<title>Hallucinator</title>`; `.expect("error while building Hallucinator")`.
- [ ] **Step 2:** First-run paragraph:
  `Hallucinator doesn't do math. A language model on your computer does, and it's often wrong — confidently. Pick a brain to download. You only need to do this once.`
- [ ] **Step 3:** `npm run build` and `cargo test` pass. Commit `chore: rename the app to Hallucinator`.

(README is rewritten in Task 7 once all features exist.)

### Task 2: Personality in Rust

**Files:**
- Modify: `src-tauri/src/settings.rs` — `Personality` enum, `Settings.personality`, lenient deserialize
- Modify: `src-tauri/src/ask.rs` — `system_prompt`, `request_body(expression, sampling, personality)`, `stream(..., personality, ...)`
- Modify: `src-tauri/src/commands.rs` — `Snapshot.personality`, `save_personality`, `ask` passes it
- Modify: `src-tauri/src/main.rs` — register `commands::save_personality`

**Interfaces — Produces:**
- `settings::Personality` serialized as `"plain" | "overconfident" | "anxious" | "gaslighting"`.
- Tauri command `save_personality { personality } -> Personality`; `get_snapshot` returns `personality`.

- [ ] **Step 1: Failing tests** in `settings.rs`:

```rust
#[test]
fn personality_roundtrips_and_defaults_to_plain() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("settings.json");
    let s = Settings { personality: Personality::Gaslighting, ..Settings::default() };
    save(&p, &s).unwrap();
    assert_eq!(load(&p).personality, Personality::Gaslighting);
    assert_eq!(Settings::default().personality, Personality::Plain);
}

#[test]
fn v01_file_without_personality_loads_plain() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("settings.json");
    std::fs::write(&p, r#"{"activeModel":"gemma-3-1b","sampling":{"topK":5}}"#).unwrap();
    let s = load(&p);
    assert_eq!(s.personality, Personality::Plain);
    assert_eq!(s.active_model.as_deref(), Some("gemma-3-1b"));
}

#[test]
fn unknown_personality_loads_plain_and_keeps_other_fields() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("settings.json");
    std::fs::write(&p, r#"{"activeModel":"gemma-3-1b","personality":"sarcastic"}"#).unwrap();
    let s = load(&p);
    assert_eq!(s.personality, Personality::Plain);
    assert_eq!(s.active_model.as_deref(), Some("gemma-3-1b"));
}

#[test]
fn personality_uses_camel_case_names() {
    assert_eq!(serde_json::to_string(&Personality::Overconfident).unwrap(), r#""overconfident""#);
}
```

and in `ask.rs` (replace `body_has_prompt_sampling_and_stream`):

```rust
#[test]
fn body_has_prompt_sampling_and_stream() {
    let b = request_body("2+2", &Sampling { temperature: 1.5, top_k: 7, top_p: 0.5 }, Personality::Anxious);
    assert_eq!(b["messages"][0], json!({"role": "system", "content": system_prompt(Personality::Anxious)}));
    // ...rest unchanged
}

#[test]
fn plain_prompt_is_the_v01_prompt() {
    assert_eq!(system_prompt(Personality::Plain), "You are a calculator. Reply with the result of the expression.");
}

#[test]
fn every_personality_has_its_own_prompt() {
    let all = [Personality::Plain, Personality::Overconfident, Personality::Anxious, Personality::Gaslighting];
    let prompts: std::collections::HashSet<_> = all.iter().map(|p| system_prompt(*p)).collect();
    assert_eq!(prompts.len(), all.len());
    for p in &all[1..] {
        assert!(system_prompt(*p).contains("end with \"= \""), "{p:?} must ask for a trailing = result");
    }
}
```

- [ ] **Step 2:** Run `cargo test` → compile failure (no `Personality`).
- [ ] **Step 3: Implement.** `settings.rs`:

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Personality {
    #[default]
    Plain,
    Overconfident,
    Anxious,
    Gaslighting,
}

// An unknown value (a newer or hand-edited file) must not reset every other setting.
fn lenient_personality<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Personality, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}
```
and `Settings` gains `#[serde(deserialize_with = "lenient_personality")] pub personality: Personality,`.

`ask.rs`: prompts per spec §3 as constants, `pub fn system_prompt(p: Personality) -> &'static str`, `request_body` and `stream` take `personality: Personality` (stream: after `sampling`). Existing stream tests pass `Personality::Plain`.

`commands.rs`: `Snapshot { ..., personality: Personality }` filled from settings;

```rust
#[tauri::command]
pub fn save_personality(state: State<AppState>, personality: Personality) -> Personality {
    state.settings.lock().unwrap().personality = personality;
    state.save_settings();
    personality
}
```
`ask` reads `(sampling, personality)` under one lock and passes both.

- [ ] **Step 4:** `cargo test` passes. Commit `feat: personality presets pick the system prompt`.

### Task 3: Personality list and card layout (pure TS)

**Files:**
- Create: `src/personality.ts`, `src/personality.test.ts`
- Create: `src/share/cardLayout.ts`, `src/share/cardLayout.test.ts`
- Modify: `src/api.ts` — re-export `Personality`, `Snapshot.personality`, `api.savePersonality`

**Interfaces — Produces:**
- `personality.ts`: `type Personality = 'plain' | 'overconfident' | 'anxious' | 'gaslighting'`, `PERSONALITIES: { id: Personality; label: string }[]`, `modeLabel(p: Personality): string | null` (`null` for plain, else `Mode: <Label>`).
- `cardLayout.ts`: `CONFIDENCE_LABEL`, `WATERMARK = 'Hallucinator'`, `CARD_SIZE = 1080`, `CARD_PAD = 80`, `MAX_THOUGHT_LINES = 4`, `THOUGHT_PX = 40`, `interface ShareData { expr; result; thoughts; personality }`, `type FontKind = 'mono' | 'sans'`, `type Measure = (text: string, font: FontKind, px: number) => number`, `interface CardLayout { expr: string; exprPx: number; result: string; resultPx: number; thoughts: string[]; footer: string[]; watermark: string }`, `cardLayout(d: ShareData, measure: Measure): CardLayout`, `shareEnabled(busy: boolean, data: ShareData | null): boolean`.

- [ ] **Step 1: Failing tests.** `personality.test.ts`:

```ts
import { describe, test, expect } from 'vitest';
import { PERSONALITIES, modeLabel } from './personality';

describe('personalities', () => {
  test('plain first, then the three characters', () => {
    expect(PERSONALITIES.map((p) => p.id)).toEqual(['plain', 'overconfident', 'anxious', 'gaslighting']);
  });
  test('mode label is hidden on plain only', () => {
    expect(modeLabel('plain')).toBeNull();
    expect(modeLabel('gaslighting')).toBe('Mode: Gaslighting');
  });
});
```

`cardLayout.test.ts` with a fixed-width fake measure (every char = `px * 0.5`):

```ts
import { describe, test, expect } from 'vitest';
import { cardLayout, shareEnabled, CONFIDENCE_LABEL, MAX_THOUGHT_LINES, ShareData, Measure } from './cardLayout';

const measure: Measure = (text, _font, px) => Array.from(text).length * px * 0.5;
const data = (over: Partial<ShareData> = {}): ShareData =>
  ({ expr: '17×23', result: '401', thoughts: 'The answer is clearly 401.', personality: 'plain', ...over });

describe('cardLayout', () => {
  test('confidence is the fixed 100%', () => {
    expect(CONFIDENCE_LABEL).toBe('Confidence: 100%');
  });
  test('short answer: one thought line, confidence only, watermark', () => {
    const l = cardLayout(data(), measure);
    expect(l.thoughts).toEqual(['The answer is clearly 401.']);
    expect(l.footer).toEqual([CONFIDENCE_LABEL]);
    expect(l.watermark).toBe('Hallucinator');
  });
  test('mode appears in the footer when not plain', () => {
    expect(cardLayout(data({ personality: 'anxious' }), measure).footer).toEqual([CONFIDENCE_LABEL, 'Mode: Anxious']);
  });
  test('long thoughts wrap to at most 4 lines ending in an ellipsis, each fitting the width', () => {
    const l = cardLayout(data({ thoughts: 'word '.repeat(200) }), measure);
    expect(l.thoughts).toHaveLength(MAX_THOUGHT_LINES);
    expect(l.thoughts.at(-1)!.endsWith('…')).toBe(true);
    for (const line of l.thoughts) expect(measure(line, 'sans', 40)).toBeLessThanOrEqual(920);
  });
  test('thoughts that fit are not ellipsized; newlines collapse', () => {
    const l = cardLayout(data({ thoughts: 'one\n\ntwo' }), measure);
    expect(l.thoughts).toEqual(['one two']);
  });
  test('an unbreakable word is split to fit', () => {
    const l = cardLayout(data({ thoughts: 'x'.repeat(100) }), measure);
    expect(l.thoughts.join('')).toBe('x'.repeat(100));
    for (const line of l.thoughts) expect(measure(line, 'sans', 40)).toBeLessThanOrEqual(920);
  });
  test('empty thoughts give no lines', () => {
    expect(cardLayout(data({ thoughts: '  ' }), measure).thoughts).toEqual([]);
  });
  test('a long result shrinks to fit; a short one stays big', () => {
    expect(cardLayout(data(), measure).resultPx).toBe(220);
    const long = cardLayout(data({ result: '1234567890123456789' }), measure);
    expect(long.resultPx).toBeLessThan(220);
    expect(measure(long.result, 'mono', long.resultPx)).toBeLessThanOrEqual(920);
  });
});

describe('shareEnabled', () => {
  test('only with an answer and not busy', () => {
    expect(shareEnabled(false, data())).toBe(true);
    expect(shareEnabled(true, data())).toBe(false);
    expect(shareEnabled(false, null)).toBe(false);
  });
});
```

- [ ] **Step 2:** `npm test` → fails (modules missing).
- [ ] **Step 3: Implement** `personality.ts` and `cardLayout.ts`: `fit(text, font, maxPx, minPx)` shrinks by 10 px until it fits; `wrap` collapses whitespace, greedy word wrap at width `CARD_SIZE - 2*CARD_PAD`, hard-splits overlong words by character; if more than 4 lines, keep 4 and trim the 4th until `line + '…'` fits. Expression fits from 56 px down to 24 px, result from 220 px down to 60 px.
- [ ] **Step 4:** `npm test` passes; `npm run build` passes. Commit `feat: personality list and share card layout`.

### Task 4: Personality UI, Confidence line and mode tag

**Files:**
- Modify: `src/views/settings.ts` — "Personality" section above Sampling
- Modify: `src/views/calculator.ts` — `.meta` line: confidence, mode tag
- Modify: `src/style.css`

- [ ] **Step 1:** Settings: a `<section class="personality"><h2>Personality</h2></section>` with one `label.choice` + radio per `PERSONALITIES` entry, checked from `snap.personality`; on change `api.savePersonality(id).catch(showError)`.
- [ ] **Step 2:** Calculator display gains
  `<div class="meta"><span class="confidence"></span><span class="mode"></span><button class="share" disabled>Share</button></div>`;
  `.confidence` text is `CONFIDENCE_LABEL`, `.mode` text is `modeLabel(snap.personality)`, hidden when null. (Share wiring in Task 5.)
- [ ] **Step 3:** CSS: `.meta { display:flex; gap:10px; align-items:center; font-size:11px; color:var(--muted) }`, `.confidence { color: var(--screen) }`, `.share { margin-left:auto; background:var(--op); border-radius:6px; padding:2px 8px; font-size:11px }`, `.share:disabled { opacity:.4; cursor:default }`.
- [ ] **Step 4:** `npm test`, `npm run build` pass. Commit `feat: personality settings, Confidence: 100% and mode tag`.

### Task 5: Share this hallucination

**Files:**
- Create: `src/share/renderCard.ts` — draws `cardLayout` on a canvas, returns PNG bytes
- Create: `src/share/copyCard.ts` — `copyCard(data)` → `writeImage(png)`
- Modify: `src/views/calculator.ts` — `shareable: ShareData | null`, button state and click
- Modify: `package.json` (`@tauri-apps/plugin-clipboard-manager`), `src-tauri/Cargo.toml` (`tauri-plugin-clipboard-manager = "2"`, tauri feature `image-png`), `src-tauri/src/main.rs` (`.plugin(tauri_plugin_clipboard_manager::init())`), `src-tauri/capabilities/default.json` (`clipboard-manager:allow-write-image`)

- [ ] **Step 1:** Install: `npm install @tauri-apps/plugin-clipboard-manager@^2`; add the crate and feature; register the plugin; grant the permission. `cargo build` succeeds.
- [ ] **Step 2:** `renderCard(data): Promise<Uint8Array>` — canvas 1080², background `#10140f`; measure via `ctx.measureText` with `mono = Consolas, monospace` / `sans = 'Segoe UI', system-ui, sans-serif`; expression right-aligned at y=200 (muted `#8a8fa8`), result right-aligned at y=200+resultPx (green `#9be7a8`), thoughts left-aligned from y=560 step 56 (`#c9cbd8`, 40 px sans), footer joined with `  ·  ` at y=1000 32 px (first item green, rest muted, drawn left to right), watermark right-aligned at y=1000 28 px muted. `canvas.toBlob(png)` → `Uint8Array`.
- [ ] **Step 3:** Calculator: `shareable` set after a successful, non-interrupted answer whose result is not `Error`, to `{ expr: calc.expr, result: calc.result, thoughts: r.text, personality }`; cleared at the start of `evaluate`, in `pressKey`, and on `input`. `render()` sets `share.disabled = !shareEnabled(busy, shareable)`. Click → `copyCard(shareable)` → button text `Copied!` (or `Copy failed` + `console.error`) for 1500 ms, then `Share`; timer cleared on dispose; refocus `expr`.
- [ ] **Step 4:** `npm test`, `npm run build`, `cargo test` pass. Commit `feat: share a hallucination as a PNG card`.

### Task 6: Recording mode

**Files:**
- Create: `src/recording.ts`, `src/recording.test.ts`
- Modify: `src/views/calculator.ts` (F9, drag strip), `src/views/settings.ts` ("Video" section button), `src/style.css`, `src-tauri/capabilities/default.json`

**Interfaces — Produces:** `interface Size { width; height }`, `interface WindowPort { setDecorations(on): Promise<void>; setSize(s: Size): Promise<void>; setMinSize(s: Size): Promise<void>; innerSize(): Promise<Size> }`, `RECORDING_SIZE = 540×960`, `RECORDING_MIN = 360×640`, `NORMAL_MIN = 300×500`, `createRecording(port, onChange: (on: boolean) => void): { isOn(): boolean; enter(): Promise<void>; leave(): Promise<void>; toggle(): Promise<void> }`, `appRecording()` lazy singleton over the real window that toggles `recording` on `<html>`.

- [ ] **Step 1: Failing test** with a fake port recording calls:

```ts
import { describe, test, expect } from 'vitest';
import { createRecording, NORMAL_MIN, RECORDING_MIN, RECORDING_SIZE, Size, WindowPort } from './recording';

function fakePort(start: Size) {
  const calls: string[] = [];
  let size = start;
  let decorations = true;
  const port: WindowPort = {
    setDecorations: async (on) => { decorations = on; calls.push(`decorations ${on}`); },
    setSize: async (s) => { size = s; calls.push(`size ${s.width}x${s.height}`); },
    setMinSize: async (s) => { calls.push(`min ${s.width}x${s.height}`); },
    innerSize: async () => size,
  };
  return { port, calls, state: () => ({ size, decorations }) };
}

describe('recording mode', () => {
  test('enter goes frameless 540x960, leave restores size and frame', async () => {
    const f = fakePort({ width: 340, height: 580 });
    const seen: boolean[] = [];
    const r = createRecording(f.port, (on) => seen.push(on));
    await r.enter();
    expect(f.state()).toEqual({ size: RECORDING_SIZE, decorations: false });
    expect(f.calls).toContain(`min ${RECORDING_MIN.width}x${RECORDING_MIN.height}`);
    expect(r.isOn()).toBe(true);
    await r.leave();
    expect(f.state()).toEqual({ size: { width: 340, height: 580 }, decorations: true });
    expect(f.calls).toContain(`min ${NORMAL_MIN.width}x${NORMAL_MIN.height}`);
    expect(seen).toEqual([true, false]);
  });
  test('enter twice and leave while off do nothing', async () => {
    const f = fakePort({ width: 340, height: 580 });
    const r = createRecording(f.port, () => {});
    await r.leave();
    expect(f.calls).toEqual([]);
    await r.enter();
    const n = f.calls.length;
    await r.enter();
    expect(f.calls.length).toBe(n);
  });
  test('rapid toggles run in order and end where they should', async () => {
    const f = fakePort({ width: 340, height: 580 });
    const r = createRecording(f.port, () => {});
    await Promise.all([r.toggle(), r.toggle(), r.toggle()]);
    expect(r.isOn()).toBe(true);
    expect(f.state().size).toEqual(RECORDING_SIZE);
  });
});
```

- [ ] **Step 2:** `npm test` → fails.
- [ ] **Step 3: Implement** `recording.ts` (operations chained on one promise queue so overlapping toggles serialize); real port via `getCurrentWindow()`, `LogicalSize`, `innerSize().toLogical(scaleFactor())`.
- [ ] **Step 4:** Calculator: `F9` (any modifiers off) → `appRecording().toggle()`; `<div class="drag" data-tauri-drag-region></div>` as the first child of `.calc`. Settings: `<section><h2>Video</h2><button class="record">Recording mode (F9)</button></section>` → `appRecording().enter().then(back)`. Capability permissions `core:window:allow-set-decorations`, `core:window:allow-set-size`, `core:window:allow-set-min-size`, `core:window:allow-start-dragging`.
- [ ] **Step 5:** CSS under `.recording`: header hidden, `.drag` shown (18 px strip, `cursor: grab`), `.calc` padding 24 / gap 16, `.display { flex: 1; justify-content: flex-end; padding: 24px; gap: 12px }`, expr 28 px, result 96 px, thoughts 22 px (max-height 140 px), `.meta` 18 px, `.share` hidden, keys 32 px.
- [ ] **Step 6:** `npm test`, `npm run build`, `cargo test` pass. Commit `feat: 9:16 recording mode`.

### Task 7: README and owner's check list

**Files:**
- Modify: `README.md`
- Create: `docs/superpowers/plans/2026-10-22-hallucinator-checklist.md`

- [ ] **Step 1:** README: `# Hallucinator`, tagline, what it does (personalities, Confidence: 100%, Share, recording mode with F9), install/build/credits as before.
- [ ] **Step 2:** Check list covering: title bar and installer name; first-run copy; each personality answering `17×23` (answer ends with a number that lands in the result); Confidence/mode line; Share → paste into Paint and Telegram, card looks right with long thoughts and long results; Share disabled while thinking / after edits / after Esc-interrupt; F9 in and out, drag, size restore, settings button; settings survive restart with personality kept, recording mode not kept.
- [ ] **Step 3:** All tests and build pass. Commit `docs: README for Hallucinator and manual check list`.
