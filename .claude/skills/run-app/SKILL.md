---
name: run-app
description: Start the Dioxus web client (crates/app) locally, verify it in the browser, and stop it. Use when asked to run, serve, or screenshot the app, or to confirm a UI change works for real rather than just compiling.
---

# Run the web client

## Start

Run in the background (it never exits on its own) from `crates/app`:

```sh
cd crates/app && dx serve --port 8090 --open false
```

`dx` is the Dioxus CLI pinned in `README.md` (0.7.x); `dx --version` to confirm. The first
build takes ~30s, later ones are incremental and hot-reload on save. Wait for readiness with a
poll rather than a fixed sleep:

```sh
for i in $(seq 1 60); do curl -s -o /dev/null --max-time 2 http://127.0.0.1:8090/ && break; sleep 1; done
```

If port 8090 is busy, pick another; nothing depends on the number. `dx build` in the same
directory does a one-off build (output under `target/dx/app/debug/web/public`).

## Verify in the browser

Prefer actually exercising the board over trusting a compile. With the Chrome tools:

- Open `http://127.0.0.1:8090/` in a new tab, screenshot, and check the console for errors.
- The board is a square 8x8 grid; get its bounding box from a screenshot (or
  `document.querySelector('.board').getBoundingClientRect()` via the JavaScript tool) and use
  `x = left + (file + 0.5) * size/8`, `y = top + (7 - rank + 0.5) * size/8` for White
  orientation (files a..h = 0..7, ranks 1..8 = 0..7).
- The sidebar grows as moves are added, which shifts the buttons down. Re-screenshot before
  clicking a button after the move list has changed.
- The board only accepts clicks when viewing the latest position of an unfinished game. If
  clicks do nothing, check whether you're in history (use "⏭" / ArrowDown to return).
- A quick smoke test: `e2-e4 e7-e5 g1-f3`, confirm the move list reads `1. e4 e5 2. Nf3`,
  press ArrowLeft with the board focused, confirm the status flips to "White to move".
- Promotion: `a2a4 b7b5 a4b5 a7a6 b5a6 b8c6 a6a7 c6b8 a7b8` opens the picker.

Known false alarm: the developer's Chrome runs Dark Reader
(`document.documentElement.dataset.darkreaderMode === "dynamic"`). It repaints the light
squares near-black and hides the semi-transparent hint dots. Check computed styles or the
served CSS before concluding the board CSS is broken.

## Stop

```sh
pkill -f 'dx serve --port 8090'
```

Close any browser tabs you opened.
