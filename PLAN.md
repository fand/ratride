# ratride 実装プラン

Ratatui + tachyonFX によるMarkdownスライドツール。

## 技術スタック

```toml
ratatui = "0.30"           # TUI framework
crossterm = "0.28"         # Terminal backend
pulldown-cmark = "0.13"    # Markdown parser
tachyonfx = "0.11"         # Transition effects
ratatui-image = "10"       # 画像表示 (Kitty/Sixel/iTerm2)
image = "0.25"             # 画像読み込み
```

---

## Phase 1: Markdownをパースして表示

**Goal:** `cargo run -- main.md` でMarkdownをターミナルに描画。

### やること
- `cargo init` でプロジェクト作成
- ratatui + crossterm + pulldown-cmark を依存に追加
- pulldown-cmark の `Event` ストリームを ratatui の `Text` (`Vec<Line>`) に変換する `md_to_text()` を実装
- 対応要素:
  - 見出し (H1-H6): サイズに応じた色・Bold
  - 段落 / テキスト
  - **Bold**, *Italic*, ~~Strikethrough~~
  - インラインコード、コードブロック (背景色付き)
  - 箇条書き (ul / ol)
  - 引用ブロック (左にバー)
  - 水平線 (`---`)
- `Paragraph::new(text)` で描画

### 構成

```
src/
  main.rs       # CLI引数処理, Terminal初期化, イベントループ
  markdown.rs   # pulldown-cmark Event -> Vec<Line> 変換
```

---

## Phase 2: スクロール

**Goal:** 長いMarkdownをj/k/↑/↓でスクロール、スクロールバー表示。

### やること
- `AppState` に `scroll_offset`, `content_height` を持つ
- `Paragraph::scroll((offset, 0))` でスクロール
- `Scrollbar` ウィジェットを右端に描画
- キーバインド: `j`/`↓` = 下, `k`/`↑` = 上, `q` = 終了

---

## Phase 3: ページ送り

**Goal:** `---` でスライドを区切り、←/→でページ切替。

### やること
- `Event::Rule` でMarkdownイベントストリームを分割し `Vec<Slide>` を生成
- `Slide` = 1ページ分の `Text`
- `AppState` に `current_page`, `slides: Vec<Slide>` を追加
- キーバインド: `→`/`l`/`Space` = 次ページ, `←`/`h` = 前ページ
- ステータスバー (下端) に `[3/10]` のようなページ番号を表示
- 各ページ内でスクロールも維持

### データ構造

```rust
struct Slide {
    content: Text<'static>,
}

struct App {
    slides: Vec<Slide>,
    current_page: usize,
    scroll_offset: u16,
}
```

---

## Phase 4: ページごとのレイアウト

**Goal:** コメントでページ単位のレイアウト指定。

### フォーマット

```markdown
<!-- layout: center -->
# タイトルスライド

---

<!-- layout: two-column -->
左カラムの内容

|||

右カラムの内容

---

普通のスライド (デフォルトレイアウト)
```

### レイアウト種別

| layout | 説明 |
|---|---|
| `default` | 左揃え、上詰め |
| `center` | 上下左右中央 |
| `two-column` | `\|\|\|` で左右分割 |

### やること
- スライド分割時に先頭の `<!-- layout: xxx -->` をパース
- `Slide` に `layout: Layout` フィールド追加
- レイアウトごとの描画ロジック実装

---

## Phase 5: 画像対応

**Goal:** `![alt](path)` で画像をスライド内に表示。

### やること
- `ratatui-image` + `image` クレートを追加
- `Picker::from_query_stdio()` でプロトコル検出 (起動時1回)
- Markdownパース時に `Event::Start(Tag::Image { .. })` を検出
- 画像パスを解決し `image::open()` で読み込み
- `StatefulImage` で描画
- スライドデータに画像の配置情報を持たせる

### 構成追加

```
src/
  image.rs      # 画像の読み込み・プロトコル管理
```

---

## Phase 6: tachyonFXトランジション

**Goal:** ページ切替時にアニメーション効果。

### やること
- `tachyonfx` クレートを追加
- ページ切替時に効果を適用:
  - デフォルト: `slide_in` (方向に応じて左/右から)
  - コメントで指定可能: `<!-- transition: dissolve -->`
- 効果一覧:
  - `slide_in` / `slide_out`
  - `fade_from_fg` / `fade_to_fg`
  - `dissolve`
  - `coalesce`
  - `sweep_in`
- イベントループに `process()` を毎フレーム呼ぶタイマーを追加 (30-60fps)
- トランジション中はキー入力を無視 or キューイング

### 構成追加

```
src/
  transition.rs  # トランジション管理、Effect生成
```

---

## 最終的なファイル構成

```
src/
  main.rs         # CLI, Terminal, イベントループ
  app.rs          # AppState, キー入力ハンドリング
  markdown.rs     # Markdown -> Slide 変換
  slide.rs        # Slide構造体, レイアウト定義
  render.rs       # 各レイアウトの描画ロジック
  image.rs        # 画像読み込み・プロトコル管理
  transition.rs   # tachyonFXトランジション管理
```
