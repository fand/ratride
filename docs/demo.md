---
figlet_mobile: false
---
<!-- layout: center -->
<!-- figlet: ansi_shadow -->
<!-- image_max_width: 40% -->
<!-- line_height: 1.2 -->

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
<!-- bg_fill -->

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
<!-- bg_fill -->

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
<!-- bg_fill -->

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
<!-- line_height: 1.2 -->

# Slant

`<!-- figlet:slant -->` renders headings in custom figlet fonts.

Fonts depend on your `figlet` installation.

---

## Scrollable Content

Scroll: `↓`/`↑` or `j`/`k`. Half-page: `d`/`u`.

- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat
- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat
- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat
- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat
- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat
- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat
- This slide is long
- so long
- really long
- truly long, we need to scroll
- We can scroll with arrow keys
- and also with j/k keys
- cuz ratride is a tui app
- even it works on the web
- we still need to be able to navigate with keys
- and I repeat

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
