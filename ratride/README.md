# ratride

A markdown slide presenter for the terminal, built with [ratatui](https://github.com/ratatui/ratatui).

## Install

```
cargo install ratride
```

## Usage

```
ratride slides.md
```

Slides are separated by `---`. Use HTML comments for directives:

```markdown
<!-- layout: center -->
<!-- transition: fade -->
<!-- theme: latte -->
<!-- figlet -->
```

## Features

- Layouts: default, center, two-column (`|||`)
- Transitions: fade, dissolve, sweep, and more
- Image display (iTerm2 / Kitty / Sixel)
- FIGlet headings
- Catppuccin themes (mocha, macchiato, frappe, latte)

## License

MIT
