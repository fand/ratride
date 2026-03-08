<div align="center">
    <img src="./docs/public//ratride_logo_black.png" width="540" />
    <h1>
      <img src="./docs/public/ratride_titile.png" width="540" />
    </h1>
</div>

A tiny slide presenter built with [ratatui](https://github.com/ratatui/ratatui).

## Features

- Markdown-based slides (`---` delimiter)
- Layouts: default, center, two-column
- Slide transitions (fade, dissolve, sweep, etc.)
- Image display (iTerm2 / Kitty / Sixel)
- HTML export
- JS binding

## Install

```
cargo install ratride
```

## Usage

```
ratride slides.md
```

## Not supported

Ratride DOES NOT supported following fetures:

- Code execution
- Mermaid / LaTeX / Typst rendering
- Custom styling
- Speaker notes
- PDF export
- Tables
- Incremental reveal (`pause`)
- Hot reload

If you need any of them, I recommend using [presenterm](https://github.com/mfontanini/presenterm):


## LICENSE
MIT

## Author

AMAGI (https://amagi.dev/)
