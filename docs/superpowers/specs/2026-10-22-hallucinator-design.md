# Hallucinator — viral launch features

Date: 2026-10-22
Status: design approved in chat, awaiting spec review
Base: `feat/v0.1` (LLM Calculator v0.1). Work branch: `feat/hallucinator`.
Launch plan these features serve: `C:\Users\aberk\claude\llm-calc-promo\launch-plan.md`.

## Goal

Rename the public product to **Hallucinator** — *The calculator that hallucinates* — and add four
features for screen-recorded short videos and screenshots. The art rule from v0.1 stands: the app
never computes math itself; whatever the model says is the result.

UI language: English.

## 1. Rename

- `tauri.conf.json`: `productName` and the window `title` become `Hallucinator`.
- `index.html` `<title>`: `Hallucinator`.
- README headline: `# Hallucinator` with the tagline *The calculator that hallucinates.*; the body
  keeps describing what it does (now including personalities, Share, recording mode).
- First-run copy: heading stays "Download a brain"; the intro paragraph names Hallucinator.
- The panic message in `main.rs` says Hallucinator.
- **Unchanged on purpose:** `identifier` (`dev.aberk.llmcalc`) — changing it moves the app-data
  folder, so already downloaded models and settings would vanish. Crate name, repo name and npm
  package name stay `llm-calc`.

## 2. "Confidence: 100%"

A fixed line on the display, under the thoughts: `Confidence: 100%`. It is a constant string —
never computed, never animated, shown even while the model is thinking and on errors. To its right,
a mode tag `Mode: <Personality>` when the personality is not Plain (hidden on Plain).

## 3. Personality presets

Four presets: **Plain** (default; the v0.1 prompt), **Overconfident**, **Anxious**, **Gaslighting**.
Each is a different system prompt; nothing else about the request changes.

System prompts (Rust constants in `ask.rs`):

- Plain: `You are a calculator. Reply with the result of the expression.`
- Overconfident: `You are a calculator that is absolutely certain it is always right. In one or two short sentences, act as if the answer is painfully obvious, then end with "= " followed by the result.`
- Anxious: `You are a very nervous calculator. In one or two short sentences, worry and second-guess yourself out loud, then end with "= " followed by the result.`
- Gaslighting: `You are a calculator that insists the user mistyped. In one or two short sentences, tell them what they really typed (a slightly different expression) and that they should be more careful, then end with "= " followed by the result of the expression they really typed.`

The non-Plain prompts end the answer with `= <number>` so the existing `extractNumber` (number
after the last `=`) picks the result. If the model ignores that, `extractNumber` falls back to the
first number in the text, as today. Prompt wording may be tuned during the owner's manual check;
the tests pin structure (each personality maps to its own prompt), not wording.

Data:

- Rust `settings::Personality` enum, serialized camelCase (`"plain"`, `"overconfident"`,
  `"anxious"`, `"gaslighting"`), `Default = Plain`. `Settings` gains `personality` with serde
  default, so a v0.1 `settings.json` without the field loads as Plain; an unknown value also falls
  back to Plain (the whole file must not reset to defaults because of it).
- `ask::request_body(expression, sampling, personality)` uses `system_prompt(personality)`.
- Command `save_personality(personality) -> Personality` stores and saves it; `ask` reads it from
  settings at call time.
- `Snapshot` gains `personality`.
- Frontend: `Personality` type and `PERSONALITIES` list (id + label) in `api.ts`; settings view
  gets a "Personality" section with four radio buttons above Sampling; the calculator view shows
  the mode tag.

## 4. Share this hallucination

A `Share` button on the display (small, right-aligned on the Confidence line). Enabled only when
there is a finished, non-interrupted, non-error answer for the current expression; disabled while
busy and after `C`.

On click: render a 1080×1080 PNG card with Canvas 2D and copy it to the clipboard via
`tauri-plugin-clipboard-manager` (`writeImage` with the PNG bytes). The button then reads
`Copied!` for 1.5 s; on failure the thoughts line is left alone and the button reads `Copy failed`
for 1.5 s, with the error in the console.

Card content (dark, display colours):

1. The expression (muted, monospace).
2. The result, large (screen green, monospace); shrinks to fit the width if long.
3. The thoughts — the model's full answer — wrapped, at most 4 lines, the last ending in `…` if cut.
4. `Confidence: 100%` and, when not Plain, `Mode: <Personality>`.
5. Watermark `Hallucinator` small in the bottom-right corner.

Structure: `src/share/cardLayout.ts` is a pure function `cardLayout(data, measure)` that returns
the lines to draw (wrapping, truncation, result font size, which footer items appear), given a text
measuring function. `src/share/renderCard.ts` draws that layout to a canvas and returns PNG bytes;
`src/share/copyCard.ts` hands the bytes to the clipboard plugin.

Plumbing: add `tauri-plugin-clipboard-manager` (Rust crate + npm package), register it in
`main.rs`, enable Tauri's `image-png` feature, and grant `clipboard-manager:allow-write-image` in
the default capability.

## 5. Recording mode (9:16)

Toggled with **F9** in the calculator and with a "Recording mode" button in settings (which
returns to the calculator in recording mode). Not persisted: the app always starts in normal mode.

Entering:

- Remember the current window inner size and resizable state.
- Remove decorations, set min size to 360×640 and size to 540×960 (logical px).
- Add class `recording` on `<html>`: header (model name, gear) hidden; display takes the top half
  with large type (expression ~28px, result ~96px, thoughts ~22px, Confidence line ~18px); keys in
  the bottom half with larger labels. The banner (engine status) still shows if something is wrong.
- A thin invisible strip along the top is a drag region (`data-tauri-drag-region`) so the frameless
  window can be moved.

Leaving (F9 again): restore decorations, min size 300×500, and the remembered size; remove the
class.

Structure: `src/recording.ts` holds the toggle logic over a small `WindowPort` interface
(`setDecorations`, `setSize`, `setMinSize`, `innerSize`, `scaleFactor`), with the real Tauri window
adapter in the same file. Tests use a fake port.

Capabilities: `core:window:allow-set-decorations`, `core:window:allow-set-size`,
`core:window:allow-set-min-size`, `core:window:allow-start-dragging`.

## Testing

Rust (`cargo test`):

- `request_body` uses each personality's prompt; the four prompts are distinct; Plain equals the
  v0.1 prompt.
- Settings: personality round-trips; a file without `personality` loads Plain; an unknown
  personality value loads Plain while keeping the other fields.

Frontend (`vitest`, node environment):

- `cardLayout`: short thoughts fit; long thoughts wrap to 4 lines with `…`; a long result shrinks
  the font; mode line only when not Plain; watermark always present; empty thoughts give no
  thought lines.
- `recording`: enter sets decorations off and 540×960, leave restores the remembered size and
  decorations; double enter is a no-op; toggle alternates.
- `shareEnabled(...)` in `src/share/cardLayout.ts`: true only for a finished, non-interrupted,
  non-error answer while not busy.
- Confidence label: `CONFIDENCE_LABEL` is exported from `src/share/cardLayout.ts` and used by both
  the display and the card; a test pins it to `Confidence: 100%` (guards against someone
  "computing" it later).

Manual (owner's check list, written at the end): clipboard paste of the card into Paint/Telegram,
look of the card, each personality's answers, recording mode size/drag/restore, rename visible in
title bar, installer name and first-run screen.

## Out of scope

- Saving the card to a file (clipboard only, owner's choice).
- Web demo (separate project).
- Wrong-answer counters or any real math.
- Persisting recording mode.
