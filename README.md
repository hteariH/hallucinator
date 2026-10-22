# Hallucinator

*The calculator that hallucinates.*

A calculator that doesn't do math. Every expression you enter is answered by a language model
running on your own computer, and whatever it says is the result. Sometimes `2 + 2 = 5`.
It is always `Confidence: 100%`.

- Works offline after a one-time model download (Gemma 3 1B, 0.8 GB, or Gemma 3 4B, 2.5 GB).
- Uses your GPU via Vulkan when it can, otherwise the CPU.
- The small line under the result shows the model's full answer as it types it.
- Personalities: **Plain**, **Overconfident**, **Anxious** and **Gaslighting** (insists you mistyped).
- **Share** copies a picture of the answer to your clipboard, ready to paste anywhere.
- **Recording mode** (F9, or Settings → Video): a frameless 9:16 window with large type for
  vertical screen recordings. Drag it by its top edge; F9 again to go back.
- Settings: model, personality, temperature, top-k, top-p.

## Install

Download the installer from [Releases](../../releases) and run it. Windows 10/11 x64.

## Build from source

Requirements: Node 22+, Rust stable, Windows.

    npm install
    npm run fetch-llama   # downloads the pinned llama.cpp build into ./llama
    npx tauri dev

Tests: `npm test` and `cargo test --manifest-path src-tauri/Cargo.toml`.

## Credits

- [llama.cpp](https://github.com/ggml-org/llama.cpp) (MIT) runs the model.
- Gemma models by Google, under the [Gemma Terms of Use](https://ai.google.dev/gemma/terms).
