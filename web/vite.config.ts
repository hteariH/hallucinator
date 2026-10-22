import { defineConfig } from 'vite';

// Cross-origin isolation enables SharedArrayBuffer, i.e. multi-threaded wasm.
// "credentialless" still lets us fetch the model from Hugging Face.
const isolation = {
  'Cross-Origin-Opener-Policy': 'same-origin',
  'Cross-Origin-Embedder-Policy': 'credentialless',
};

export default defineConfig({
  clearScreen: false,
  server: { port: 5180, strictPort: true, headers: isolation, fs: { allow: ['..'] } },
  preview: { port: 5181, strictPort: true, headers: isolation },
  build: { target: 'es2022' },
  optimizeDeps: { exclude: ['@wllama/wllama'] },
  test: { environment: 'node' },
});
