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

## Library Usage

`ratride` can also be used as a library. Disable default features to drop terminal-only dependencies:

```toml
[dependencies]
ratride = { version = "1", default-features = false }
```

### Parsing slides

```rust
use ratride::markdown::{parse_frontmatter, parse_slides};
use ratride::theme::Theme;

let md = std::fs::read_to_string("slides.md").unwrap();
let (frontmatter, body) = parse_frontmatter(&md);
let theme = Theme::catppuccin_mocha();
let slides = parse_slides(body, &theme, &frontmatter, None, false);
```

### Rendering

```rust
use ratride::render::draw_slide;

// Inside a ratatui draw callback:
let (images, hyperlinks) = draw_slide(&slide, scroll, &mut frame, area);
```

### HTML export

```rust
use ratride::export::export;

export("slides.md", "dist/", Some("latte")).unwrap();
```

### Key types

| Type | Module | Description |
|------|--------|-------------|
| `Slide` | `markdown` | Parsed slide with content, layout, theme |
| `SlideLayout` | `markdown` | `Default`, `Center`, `TwoColumn` |
| `TransitionKind` | `markdown` | `Fade`, `Dissolve`, `SweepIn`, etc. |
| `Theme` | `theme` | Color palette for rendering |
| `Frontmatter` | `markdown` | File-wide defaults from YAML header |

## License

MIT
