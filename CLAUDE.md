# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                          # debug build
cargo build --release                # release build
cargo run -- examples/figlet.md      # run with example
cargo run -- file.md --theme latte   # run with theme
cargo install --path ratride         # install locally
```

No test suite or CI. No clippy/fmt config.

## Architecture

Rust workspace with single crate `ratride/`. Terminal slide presenter: parses markdown into slides, renders via ratatui with transitions.

### Key Files

- `ratride/src/main.rs` — CLI (clap), terminal event loop (~60fps), image backends (iTerm2 inline vs ratatui-image for Kitty/Sixel), transition effects (8 types including custom RGB line animations)
- `ratride/src/markdown.rs` — `MdConverter` processes pulldown-cmark events into `Vec<Slide>`. Handles HTML comment directives (`<!-- layout:center -->`, `<!-- transition:fade -->`, `<!-- figlet:font -->`), two-column splits (`|||`), figlet subprocess calls
- `ratride/src/render.rs` — `draw_slide()` dispatches by layout type (Default/Center/TwoColumn), scrollbar, status bar
- `ratride/src/theme.rs` — 4 Catppuccin themes (mocha default, macchiato, frappe, latte), theme resolution: CLI arg > markdown directive > default

### Data Flow

Markdown file → `MdConverter` (pulldown-cmark events → `Vec<Slide>`) → `App` event loop → `draw_slide()` renders current slide → tachyonfx handles transitions between slides → image backend flushes images after transition ends

### Markdown Directives

HTML comments control per-slide behavior: `<!-- layout: center -->`, `<!-- transition: fade -->`, `<!-- theme: macchiato -->`, `<!-- figlet -->`, `<!-- figlet:slant -->`. Slides split on `---`. Columns split on `|||`.
