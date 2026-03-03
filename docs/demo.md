---
line_height: 1.2
figlet_mobile: false
header:
  - [GitHub](https://github.com/fand/ratride)
---
<!-- layout: center -->
<!-- figlet -->
<!-- figlet_color: ff6666,ff66ff,33ffff -->
<!-- image_max_width: 35% -->

![](./ratride_logo_white.png)

# Ratride
A **Markdown slide tool** built with *Ratatui* + tachyonFX.

Press `→` to go to next slide.

---
<!-- transition: sweep-in -->

In Ratride, 

- You can write slides in Markdown
- Show it both in terminal & on Web!
- With animated transitions...

---
<!-- transition: dissolve -->
<!-- layout: center -->
<!-- figlet: standard -->

# It works
# on Terminal,

---
<!-- transition: lines -->
<!-- layout: center -->
<!-- figlet -->
# and on the
# Web!

---
<!-- transition: lines-rgb -->
<!-- layout: center -->
<!-- figlet -->

# with
# Animation!

---
<!-- transition: sweep-in -->

# Install

Install from [crates.io]:

```
$ cargo install ratride
```

Or you can install the JS-binding from [npm]:

```
$ npm install ratride
```

```js
import { run } from "ratride";

const md = await fetch("./slides.md").then((r) => r.text());
run(md);
```

---
<!-- transition: slide-in -->

## Features

- Basic Markdown features
- Navigation with `←→↓↑` (`hjkl`) keys 
- Catppuccin themes
- tachyonFX transitions
- Figlet headers
- Web export

---
<!-- transition: slide-in -->

# Markdown slides

You can write slides in Markdown, as always.
Slide separator is `---`.

## List

- list 1
- list 2 
  - Nested item
    - Deep nested
- Foo

1. First
2. Second
3. Third

## Text styling

- **Bold**, *Italic*, ~~Strikethrough~~
- `inline code` support
- [Link](https://github.com/fand/ratride)

> Blockquote

---

## Syntax Highlight

Of course you can!

``` js
const Counter = () => {
  const name = useName();  
  return <h1>Hello {name}</h1>;  
};
```

```rust
fn main() {
    let name = use_name();
    println!("Hello {}", name);
}
```

---
<!-- image_max_width: 50% -->

## Image support

Ratride supports image output for iTerm2 / Kitty protocols.

![](./demo.png)

This image is displayed with `<!-- image_max_width: 80% -->` slide option.

---

## Scrollable Content

Scroll: `↓`/`↑` (or `j`/`k`). Half-page: `d`/`u`.

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
- This is the end

---

## Options

Set global options via YAML frontmatter:

```yaml
---
theme: latte
transition: fade
figlet: slant
image_max_width: 80%
---
```

Or use HTML comment to set per-slide options:

```html
<!-- figlet: slant -->
<!-- layout: center -->
<!-- theme: latte -->
<!-- bg_fill -->
```

---
<!-- transition: sweep-in -->
<!-- figlet -->
<!-- figlet_color: ff8800,ff66ff -->

## Themes

Ratride ships with Catppuccin themes:

- **mocha** — dark (default)
- **macchiato** — dark
- **frappe** — dark
- **latte** — light

You can set the theme with `--theme latte` option, or in the frontmatter:

```md
---
theme: macchiato
---
```

---
<!-- theme: macchiato -->
<!-- transition: fade -->
<!-- bg_fill -->

### Macchiato

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

### Frappé

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

### Latte

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
<!-- transition: sweep-in -->

# Layout

For each slide, you can choose the layout from below:

- normal
- center
- two-column


---
<!-- layout: center -->

### Center

or place your contents at the center

with `<!-- layout: center -->`

---
<!-- layout: two-column -->

## Left Column

Or `two-column` layout, with `<!-- layout: two-column -->`.

- Foo
- Foo
- Foo

|||

## Right Column

1. Bar
2. Bar
3. Bar

---
<!-- transition: sweep-in -->
<!-- layout: center -->
<!-- figlet -->
<!-- figlet_color: ffff66,33ffff -->

# Figlet

With `figlet` option, turn the headers into ascii-art banners.

---
<!-- figlet: smslant -->

## Fonts

You can use figlet fonts like `<!-- figlet: slant -->`.
The default is **ANSI Shadow**.

The list of supported fonts:

- ANSI Shadow
- standard
- slant
- big
- small
- mini
- slant
- smslant
- block
- doom
- epic
- graffiti
- fraktur
- roman
- gothic
- speed
- script

---
<!-- figlet: small -->
<!-- figlet_color: ffff66,33ffff -->

## Figlet color

Ratride uses figrat, a fork of figlet with color support.
You can set text color or gradient like:

- `<!-- figlet_color: ff0000 -->`: single color
- `<!-- figlet_color: ffff66,33ffff -->`: horizontal gradient
- `<!-- figlet_color: ffff66,33ffff y -->`: vertical gradient

ref. https://github.com/fand/figrat


---
<!-- layout: center -->
<!-- figlet -->
<!-- transition: lines-rgb -->

# Transitions

Ratride supports tachyonFX-based transitions.

---

<!-- transition: slide-in -->

## Slide-In Transition

This slide uses the **slide-in** transition (default).

Content fades in from the background color.

---

<!-- transition: lines -->

## Lines Transition

This slide uses the **lines** transition.

- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.
- In `lines` transition, Each line is revealed left-to-right with staggered timing.

---

<!-- transition: lines-cross -->

## Lines-Cross Transition

This slide uses the **lines-cross** transition.

- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.
- In `lines-cross`, Even lines reveal left-to-right, odd lines right-to-left.

---

<!-- transition: slide-rgb -->

## Slide-RGB Transition

This slide uses the **slide-rgb** transition.

- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.
- In `slide-rgb`, color-cycling leading edge sweeps from left to right.

---
<!-- transition: lines-rgb -->

## Lines-RGB Transition

This slide uses the **lines-rgb** transition.

- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.
- Just like `Lines` transition, but with colorama-like color rotating effect.


---
<!-- transition: slide-in -->
<!-- theme: latte -->
<!-- bg_fill -->

# Web export

Ratride can export the slides to HTML, just like you see me right now!

You can export the slide to HTML:

```
$ ratride slide.md --export OUT_DIR
```

Ratride also comes with a live-reload server:

```
$ ratride slide.md --serve
```

---
<!-- layout: center -->
<!-- transition: dissolve -->
<!-- figlet: smslant -->

# Thank you!

That's all for the demo.

GitHub: [fand/ratride](https://github.com/fand/ratride)
