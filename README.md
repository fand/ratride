<div align="center">
    <img src="https://raw.githubusercontent.com/fand/ratride/refs/heads/main/docs/public/ratride_screenshot.webp" width="720" alt="Ratride screenshot"/>
</div>

# Ratride

Markdown slideshow on Terminal + Web.

Built with [Ratatui](https://github.com/ratatui/ratatui) + [TachyonFX](https://github.com/junkdog/tachyonfx).

## DEMO

https://amagi.dev/ratride

## Features

- Markdown to slide
- Animated transitions
- Image support
- Web export
- JS/Wasm binding

## Usage

```
cargo install ratride
ratride slides.md
```

You can write slides in Markdown syntax:

```md
# Hello

This is the first slide

---
<!-- transition: slide -->

## Hi again

- This is the second slide
- But with the transition!
```

For more detail, chek the demo slide: 
https://amagi.dev/ratride

## LICENSE

MIT

## Author

AMAGI (https://amagi.dev/)
