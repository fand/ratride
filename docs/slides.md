<!-- layout: center -->
<!-- figlet -->
<!-- image_max_width: 40% -->

![](./ratride_logo_white.png)

# Ratride

A **Markdown slide tool** built with *Ratatui* + tachyonFX.

Press `→` to go to next slide.

---
<!-- transition: fade -->

## Features

- Parse Markdown and render in terminal
- Scroll with `j`/`k` keys
- **Bold**, *Italic*, ~~Strikethrough~~
- `inline code` support
- Page navigation with `←`/`→`
- Per-slide layouts & transitions

---
<!-- transition: slide-in -->

## Markdown Demo

### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

- Unordered list
  - Nested item
    - Deep nested
- Top level

1. First
2. Second
3. Third

___

> Blockquote: **bold**, *italic*, `code`, [link](https://github.com/fand/ratride)

---

## Themes

4 Catppuccin themes:

- **mocha** — dark (default)
- **macchiato** — dark
- **frappe** — dark
- **latte** — light

Try: `ratride slides.md --theme latte`

---
<!-- theme: macchiato -->
<!-- transition: fade -->

## Macchiato

This slide uses **Catppuccin Macchiato** theme via `<!-- theme: macchiato -->`.

- `inline code` and **bold**
- *italic* and ~~strikethrough~~

> Blockquote in macchiato colors.

```rust
fn main() {
    println!("macchiato!");
}
```

---
<!-- theme: frappe -->
<!-- transition: fade -->

## Frappé

This slide uses **Catppuccin Frappé** theme via `<!-- theme: frappe -->`.

- `inline code` and **bold**
- *italic* and ~~strikethrough~~

> Blockquote in frappé colors.

```rust
fn main() {
    println!("frappé!");
}
```

---
<!-- theme: latte -->
<!-- transition: fade -->

## Latte

This slide uses **Catppuccin Latte** theme via `<!-- theme: latte -->`.

- `inline code` and **bold**
- *italic* and ~~strikethrough~~

> Blockquote in latte colors.

```rust
fn main() {
    println!("latte!");
}
```

---

## Image support

Ratride supports image output for iTerm2 / Kitty protocols.

![](./demo.png)

---

<!-- layout: two-column -->
<!-- transition: sweep-in -->

## Left Column

- Item A
- Item B
- Item C

|||

## Right Column

1. First
2. Second
3. Third

---

<!-- transition: coalesce -->

## Code Block

```rust
fn main() {
    println!("Hello, world!");
}
```

> This is a blockquote.

```jsx
function test() {
  const name = "world";
  console.log(`Hello ${}!`);
  return <div>{`Hello ${name}`}</div>;
}
```

---

## Frontmatter

Set global defaults via YAML frontmatter:

```yaml
---
theme: latte
layout: center
transition: fade
figlet: slant
image_max_width: 80%
---
```

Per-slide HTML comments override frontmatter defaults.

---
<!-- figlet:slant -->
<!-- layout: center -->

# Slant

`<!-- figlet:slant -->` renders headings in custom figlet fonts.

Fonts depend on your `figlet` installation.

---

## Scrollable Content

Scroll: `j`/`k` or `↓`/`↑`. Half-page: `d`/`u`.

- Lorem ipsum dolor sit amet
- Consectetur adipiscing elit
- Sed do eiusmod tempor incididunt
- Ut labore et dolore magna aliqua
- Ut enim ad minim veniam
- Quis nostrud exercitation ullamco
- Laboris nisi ut aliquip ex ea
- Commodo consequat duis aute irure
- Dolor in reprehenderit in voluptate
- Velit esse cillum dolore eu fugiat
- Nulla pariatur excepteur sint
- Occaecat cupidatat non proident
- Sunt in culpa qui officia deserunt
- Mollit anim id est laborum
- Sed ut perspiciatis unde omnis
- Iste natus error sit voluptatem
- Accusantium doloremque laudantium
- Totam rem aperiam eaque ipsa
- Quae ab illo inventore veritatis
- Et quasi architecto beatae vitae

---

<!-- transition: slide-in -->

## Slide-In Transition

This slide uses the **slide-in** transition (default).

Content fades in from the background color.

---

<!-- transition: lines -->

## Lines Transition

This slide uses the **lines** transition.

Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.
Each line is revealed left-to-right with staggered timing.

---

<!-- transition: lines-cross -->

## Lines-Cross Transition

This slide uses the **lines-cross** transition.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.
Even lines reveal left-to-right, odd lines right-to-left.

---

<!-- transition: slide-rgb -->

## Slide-RGB Transition

This slide uses the **slide-rgb** transition.

A color-cycling leading edge sweeps from left to right.

---

<!-- transition: lines-rgb -->

## Lines-RGB Transition

This slide uses the **lines-rgb** transition.

Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.
Just like `Lines` transition, but with color rotating effect.


---

<!-- layout: center -->
<!-- transition: dissolve -->

# Thank you!

That's all for the demo.

GitHub: [fand/ratride](https://github.com/fand/ratride)
