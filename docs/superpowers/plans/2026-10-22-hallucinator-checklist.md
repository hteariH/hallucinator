# Hallucinator — owner's manual check list

Branch `feat/hallucinator`. Run `npm run fetch-llama` once if `./llama` is missing, then `npx tauri dev`
(or `npx tauri build` for the installer). Already downloaded models and settings stay where they were:
the app identifier did not change.

## Rename

- [ ] Title bar reads **Hallucinator**.
- [ ] `npx tauri build` produces `Hallucinator_0.1.0_x64-setup.exe`; the Start menu entry is "Hallucinator".
- [ ] First-run screen (only if no model is downloaded) says "Hallucinator doesn't do math…".

## Confidence and mode tag

- [ ] Under the thoughts: green `Confidence: 100%`, always, even while thinking or after an error.
- [ ] Plain: no mode tag. Other personalities: `Mode: <name>` next to it.

## Personalities (Settings → Personality)

For each preset, type `17×23` and press `=`:
- [ ] **Plain** answers like v0.1.
- [ ] **Overconfident** acts like it's obvious; the big result is the number after its `=`.
- [ ] **Anxious** frets; the big result is the number after its `=`.
- [ ] **Gaslighting** claims you typed something else; the big result is the number after its `=`.
- [ ] Close and reopen the app: the chosen personality is still selected.
- [ ] If a character often fails to end with `= number` (the result shows a wrong number from the
      middle of the sentence or `Error`), note the model and the expression; the prompts are in
      `src-tauri/src/ask.rs` and can be tuned.

## Share

- [ ] `Share` is greyed out before the first answer, while thinking, after editing the expression,
      after `C`, and after an Esc-interrupted answer.
- [ ] After an answer, click `Share`: it reads `Copied!` for a moment.
- [ ] Paste into Paint: a 1080×1080 dark card with the expression, the big result, the thoughts,
      `Confidence: 100%` (+ mode tag when not Plain) and `Hallucinator` bottom-right.
- [ ] Paste into Telegram (or Discord/Reddit): the picture arrives as an image.
- [ ] A long answer (try Temperature 2.0) wraps to at most 4 lines ending in `…`; a very long
      result shrinks to fit.

## Recording mode

- [ ] F9 in the calculator: the window becomes frameless 540×960, gear and model name disappear,
      large display on top, keys below.
- [ ] Drag the window by its top edge.
- [ ] Type an expression, `=`, watch the thoughts stream in big type — this is the shot for videos.
- [ ] F9 again: the frame comes back and the window returns to its previous size.
- [ ] Settings → Video → `Recording mode (F9)` enters recording mode and goes back to the calculator.
- [ ] Close the app in recording mode (Alt+F4) and reopen: it starts in the normal window.
