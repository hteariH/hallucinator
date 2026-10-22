# Hallucinator — web demo

The calculator that hallucinates, in your browser. Every answer comes from Gemma 3 270M running
locally via [wllama](https://github.com/ngxson/wllama) (llama.cpp compiled to WebAssembly).
Nothing is computed for real; whatever the model says is the result.

## Develop

```bash
cd web
npm install
npm run dev       # http://localhost:5180
npm test
npm run build     # static site in web/dist
npm run preview   # serve web/dist with production headers on :5181
```

The keypad logic, number extraction and base styles are shared with the desktop app (`../src`).

## Model

`ggml-org/gemma-3-270m-it-GGUF` / `gemma-3-270m-it-Q8_0.gguf` (278 MB), fetched from Hugging Face
on first visit and cached by the browser. Settings → "Forget the brain" clears it.

## Hosting (Cloudflare Pages)

`web/public/_headers` sets `Cross-Origin-Opener-Policy: same-origin` and
`Cross-Origin-Embedder-Policy: credentialless`, which enables multi-threaded wasm while still
allowing the model download from Hugging Face. Browsers without `credentialless` fall back to
single-thread automatically. The largest file we serve is the 8.8 MB wasm (Pages limit: 25 MiB).

One-off deploy from this machine:

```bash
cd web && npm run build
npx wrangler pages deploy dist --project-name hallucinator
```

Or connect the GitHub repo in the Cloudflare dashboard: root directory `web`, build command
`npm ci && npm run build`, output `dist`. The site appears at `https://hallucinator.pages.dev`.
