# LLM Calculator v0.1 — deferred review findings

Minor findings from per-task and final reviews, left for later (none block v0.1).

- Task 1: minor (deferred): very new majors typescript ^7 / vitest ^5 / vite ^8 — fine while build passes
- Task 2-3: minor (deferred): toggleSign mangles exponent results (1e6 → 1e−6) on ±
- Task 2-3: minor (deferred): extractNumber "2 + 2 =" (nothing after =) falls back to operand 2
- Task 2-3: minor (deferred): hyphen-as-dash taken as minus ("x-5" → -5)
- Task 2-3: minor (deferred): backspace multi-byte test does not exercise surrogate pairs; chaining tests only cover ×
- Task 4: minor (deferred): settings::load does not clamp hand-edited out-of-range sampling; save lacks sync_all
- Task 6-7: minor (deferred): 206 Content-Range offset unchecked; chunked early-EOF treated as complete; no initial progress on resume; no mid-stream cancel test; HTTP error body dropped; in-stream error payload ignored; partial text lost on mid-stream error
- Task 6: minor (deferred): test server 416 response uses multi-line literal (line endings depend on file EOL)
- Task 8: minor (deferred): free_port race; no Job Object so a hard-killed app orphans llama-server; engine.log never rotated/no separator; GPU→CPU fallback untested; fetch-llama temp dir not cleaned
- Task 9: minor (deferred): detection_does_not_panic asserts nothing (kept per Ruling 1)
- Task 10: minor (deferred): settings save errors ignored; repeated Retry queues restarts; Engine.gpu never read (dead-code warning)
- Task 10: minor (deferred): exit racing GPU→CPU retry can still spawn an orphan (needs shutdown flag); non-crash errors now delayed ~1 s; two crash restarts can run back to back; body_ending_without_done test uses literal newlines; write_all error path drops file unflushed; delete keeps status Ready while stopped
- Task 11-12: minor (deferred): Esc before ask registers is lost; late download-progress re-adds stuck %; sampling saves can resolve out of order / stale error; unhandled rejections in show/refresh/retry; non-crash ask errors overwrite result with Error; "(interrupted)" leading space when empty
- Task 13: minor (deferred): target/ cache grows (rust-cache would be tidier); workflow never run on GitHub yet
- Final: minor (deferred): stale Ready can flash while a model switch is queued; no-op select only when Ready, rapid clicks queue restarts; job test may leak ping ~30 s; delete_model can wait up to 180 s behind a background restart
