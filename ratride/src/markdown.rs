use crate::theme::Theme;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use syntect::parsing::SyntaxSet;

/// Default line-height multiplier when not specified in frontmatter or directives.
pub const DEFAULT_LINE_HEIGHT: f64 = 1.2;

/// A single header item, optionally linking to a URL.
#[derive(Clone, Debug)]
pub struct HeaderItem {
    pub text: String,
    pub url: Option<String>,
}

/// Parse a header item string, extracting `[text](url)` link syntax.
fn parse_header_item(s: &str) -> HeaderItem {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('[') {
        if let Some((text, rest)) = rest.split_once("](") {
            if let Some(url) = rest.strip_suffix(')') {
                return HeaderItem {
                    text: text.to_string(),
                    url: Some(url.to_string()),
                };
            }
        }
    }
    HeaderItem {
        text: s.to_string(),
        url: None,
    }
}

/// File-wide defaults parsed from YAML frontmatter (`--- ... ---`).
#[derive(Clone, Debug, Default)]
pub struct Frontmatter {
    pub theme: Option<String>,
    pub layout: Option<SlideLayout>,
    pub transition: Option<TransitionKind>,
    pub image_max_width: Option<f64>,
    pub line_height: Option<f64>,
    /// `Some(None)` = default figlet font, `Some(Some("slant"))` = named font.
    pub figlet: Option<Option<String>>,
    pub bg_fill: Option<bool>,
    /// Whether to enable figlet on mobile. Default: false (disabled on mobile).
    pub figlet_mobile: Option<bool>,
    /// Color argument for figrat. When set, `figrat --color "<value>"` is used
    /// instead of `figlet`.
    pub figlet_color: Option<String>,
    /// Header items displayed at top-right, overlaying the content area.
    pub header: Option<Vec<HeaderItem>>,
}

/// Extract YAML frontmatter from the beginning of a markdown string.
///
/// Returns the parsed `Frontmatter` and the remaining markdown body (with the
/// frontmatter block stripped). If no frontmatter is found the full input is
/// returned unchanged.
pub fn parse_frontmatter(input: &str) -> (Frontmatter, &str) {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return (Frontmatter::default(), input);
    }

    // Find the opening `---` line end
    let after_open = match trimmed.strip_prefix("---") {
        Some(rest) => {
            // Must be followed by a newline (or only whitespace before newline)
            let line_end = rest.find('\n');
            match line_end {
                Some(idx) if rest[..idx].trim().is_empty() => &rest[idx + 1..],
                _ => return (Frontmatter::default(), input),
            }
        }
        None => return (Frontmatter::default(), input),
    };

    // Find the closing `---`
    let close_pos = after_open.find("\n---").map(|i| i + 1); // point to the `---` line start
    let (yaml_block, body) = match close_pos {
        Some(pos) => {
            let yaml = &after_open[..pos];
            let rest = &after_open[pos..];
            // Skip the closing `---` line
            let after_close = rest.strip_prefix("---").unwrap_or(rest);
            let after_close = match after_close.strip_prefix('\n') {
                Some(s) => s,
                None if after_close.trim().is_empty() => after_close,
                None => after_close,
            };
            (yaml, after_close)
        }
        None => return (Frontmatter::default(), input),
    };

    let mut fm = Frontmatter::default();
    // Track whether we're collecting YAML list items for `header`
    let mut in_header_list = false;
    let mut header_items: Vec<HeaderItem> = Vec::new();

    for line in yaml_block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for YAML list item (e.g. "- item" or "  - [text](url)")
        if in_header_list {
            if let Some(item_text) = trimmed.strip_prefix("- ") {
                header_items.push(parse_header_item(item_text));
                continue;
            } else if trimmed.starts_with('-') && trimmed.len() == 1 {
                // bare `-` with no content, skip
                continue;
            } else {
                // End of list
                in_header_list = false;
                if !header_items.is_empty() {
                    fm.header = Some(std::mem::take(&mut header_items));
                }
            }
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "theme" => {
                    fm.theme = Some(value.to_string());
                }
                "layout" => {
                    fm.layout = Some(match value {
                        "center" => SlideLayout::Center,
                        "two-column" => SlideLayout::TwoColumn,
                        _ => SlideLayout::Default,
                    });
                }
                "transition" => {
                    fm.transition = Some(match value {
                        "fade" => TransitionKind::Fade,
                        "dissolve" => TransitionKind::Dissolve,
                        "coalesce" => TransitionKind::Coalesce,
                        "sweep" | "sweep-in" => TransitionKind::SweepIn,
                        "lines" => TransitionKind::Lines,
                        "lines-cross" => TransitionKind::LinesCross,
                        "lines-rgb" => TransitionKind::LinesRgb,
                        "slide-rgb" => TransitionKind::SlideRgb,
                        _ => TransitionKind::SlideIn,
                    });
                }
                "image_max_width" => {
                    let value = value.trim_end_matches('%');
                    if let Ok(pct) = value.parse::<f64>() {
                        fm.image_max_width = Some(pct / 100.0);
                    }
                }
                "line_height" => {
                    if let Ok(lh) = value.parse::<f64>() {
                        fm.line_height = Some(lh);
                    }
                }
                "figlet" => {
                    if value.is_empty() || value == "true" {
                        fm.figlet = Some(None);
                    } else if value != "false" {
                        fm.figlet = Some(Some(value.to_string()));
                    }
                }
                "bg_fill" => {
                    fm.bg_fill = Some(value == "true");
                }
                "figlet_mobile" => {
                    fm.figlet_mobile = Some(value == "true");
                }
                "figlet_color" => {
                    if !value.is_empty() {
                        fm.figlet_color = Some(value.to_string());
                    }
                }
                "header" => {
                    if value.is_empty() {
                        // Empty value means YAML list follows on next lines
                        in_header_list = true;
                        header_items.clear();
                    } else {
                        // Inline pipe-separated format: header: item1 | item2
                        let items: Vec<HeaderItem> = value
                            .split('|')
                            .map(|s| parse_header_item(s))
                            .filter(|item| !item.text.is_empty())
                            .collect();
                        if !items.is_empty() {
                            fm.header = Some(items);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Flush any remaining list items
    if in_header_list && !header_items.is_empty() {
        fm.header = Some(header_items);
    }

    (fm, body)
}

#[derive(Clone, Debug, Default)]
pub enum SlideLayout {
    #[default]
    Default,
    Center,
    TwoColumn,
}

#[derive(Clone, Debug, Default)]
pub enum TransitionKind {
    #[default]
    None,
    SlideIn,
    Fade,
    Dissolve,
    Coalesce,
    SweepIn,
    Lines,
    LinesCross,
    LinesRgb,
    SlideRgb,
}

/// Semantic element for a11y overlay in web builds.
#[derive(Clone, Debug)]
pub enum SemanticElement {
    Heading {
        level: u8,
        text: String,
        line_index: usize,
    },
    Link {
        url: String,
        text: String,
        line_index: usize,
        start_col: usize,
        end_col: usize,
    },
}

/// Metadata for a figlet heading that was rendered into ASCII art.
/// Used by the web layer to render figlet headings as images.
#[derive(Clone, Debug)]
pub struct FigletHeadingMeta {
    /// Line index in `content.lines` where the figlet art starts.
    pub line_index: usize,
    /// Number of lines the figlet art occupies.
    pub line_count: usize,
    /// The rendered ASCII art lines (with colors), saved for image rendering.
    pub styled_lines: Vec<Line<'static>>,
}

/// Image reference found in a slide.
#[derive(Clone, Debug)]
pub struct SlideImage {
    pub path: String,
    /// Line index in content where placeholder starts.
    pub line_index: usize,
    /// Number of placeholder lines reserved.
    pub height: u16,
    /// Original pixel dimensions (filled after image loading).
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Max display width as percentage of content area (0.0–1.0).
    pub max_width_percent: Option<f64>,
}

#[derive(Clone)]
pub struct Slide {
    pub layout: SlideLayout,
    pub content: Text<'static>,
    /// Right column content (only for TwoColumn layout)
    pub right_content: Option<Text<'static>>,
    /// Images in this slide.
    pub images: Vec<SlideImage>,
    /// Transition effect for entering this slide.
    pub transition: TransitionKind,
    /// Semantic elements for a11y overlay (headings, links).
    pub semantics: Vec<SemanticElement>,
    /// Per-slide theme (defaults to the presentation theme).
    pub theme: Theme,
    /// Line-height multiplier for web rendering (default 1.2).
    pub line_height: f64,
    /// Whether to fill entire screen with theme bg color.
    pub bg_fill: bool,
    /// Header items displayed at top-right, overlaying the content area.
    pub header: Vec<HeaderItem>,
    /// Figlet heading metadata for web image rendering.
    pub figlet_headings: Vec<FigletHeadingMeta>,
}

const IMAGE_PLACEHOLDER_HEIGHT: u16 = 15;

/// Parse markdown into slides split by `---` (horizontal rule).
/// Figlet rendering callback: `(text, font_name, color) -> Option<ascii_art>`.
/// When `color` is `Some(...)`, the renderer should use `figrat --color` instead
/// of plain `figlet`.
pub type FigletFn = dyn Fn(&str, Option<&str>, Option<&str>) -> Option<String>;

pub fn parse_slides(
    input: &str,
    theme: &Theme,
    frontmatter: &Frontmatter,
    figlet_fn: Option<&FigletFn>,
    is_mobile: bool,
) -> Vec<Slide> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(input, options);
    let mut converter = MdConverter::new(theme.clone(), frontmatter, figlet_fn, is_mobile);
    for (event, range) in parser.into_offset_iter() {
        if matches!(event, Event::Rule) {
            if input[range].contains('-') {
                // Only dash-based rules (`---`) act as slide separators
                converter.process(event);
            } else {
                // `___` / `***` render as a visible horizontal rule
                converter.process_horizontal_rule();
            }
        } else {
            converter.process(event);
        }
    }
    converter.finish_slides()
}

enum CommentDirective {
    Layout(SlideLayout),
    Transition(TransitionKind),
    Figlet(Option<String>),
    FigletMobile(bool),
    FigletColor(String),
    ImageMaxWidth(f64),
    LineHeight(f64),
    Theme(Theme),
    BgFill(bool),
    Header(Vec<HeaderItem>),
}

fn parse_comment(html: &str) -> Option<CommentDirective> {
    let trimmed = html.trim();
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?;
    let inner = inner.trim();

    if let Some(value) = inner.strip_prefix("layout:") {
        let layout = match value.trim() {
            "center" => SlideLayout::Center,
            "two-column" => SlideLayout::TwoColumn,
            _ => SlideLayout::Default,
        };
        return Some(CommentDirective::Layout(layout));
    }
    if let Some(value) = inner.strip_prefix("transition:") {
        let transition = match value.trim() {
            "fade" => TransitionKind::Fade,
            "dissolve" => TransitionKind::Dissolve,
            "coalesce" => TransitionKind::Coalesce,
            "sweep" | "sweep-in" => TransitionKind::SweepIn,
            "lines" => TransitionKind::Lines,
            "lines-cross" => TransitionKind::LinesCross,
            "lines-rgb" => TransitionKind::LinesRgb,
            "slide-rgb" => TransitionKind::SlideRgb,
            _ => TransitionKind::SlideIn,
        };
        return Some(CommentDirective::Transition(transition));
    }
    if inner == "figlet" {
        return Some(CommentDirective::Figlet(None));
    }
    if let Some(font) = inner.strip_prefix("figlet:") {
        return Some(CommentDirective::Figlet(Some(font.trim().to_string())));
    }
    if let Some(value) = inner.strip_prefix("figlet_mobile:") {
        return Some(CommentDirective::FigletMobile(value.trim() == "true"));
    }
    if let Some(value) = inner.strip_prefix("figlet_color:") {
        let value = value.trim();
        if !value.is_empty() {
            return Some(CommentDirective::FigletColor(value.to_string()));
        }
    }
    if let Some(value) = inner.strip_prefix("image_max_width:") {
        let value = value.trim().trim_end_matches('%');
        if let Ok(pct) = value.parse::<f64>() {
            return Some(CommentDirective::ImageMaxWidth(pct / 100.0));
        }
    }
    if let Some(value) = inner.strip_prefix("line_height:") {
        if let Ok(lh) = value.trim().parse::<f64>() {
            return Some(CommentDirective::LineHeight(lh));
        }
    }
    if let Some(value) = inner.strip_prefix("theme:") {
        if let Some(t) = crate::theme::theme_from_name(value.trim()) {
            return Some(CommentDirective::Theme(t));
        }
    }
    if inner == "bg_fill" {
        return Some(CommentDirective::BgFill(true));
    }
    if let Some(value) = inner.strip_prefix("bg_fill:") {
        return Some(CommentDirective::BgFill(value.trim() == "true"));
    }
    if let Some(value) = inner.strip_prefix("header:") {
        let items: Vec<HeaderItem> = value
            .split('|')
            .map(|s| parse_header_item(s))
            .filter(|item| !item.text.is_empty())
            .collect();
        if !items.is_empty() {
            return Some(CommentDirective::Header(items));
        }
    }
    None
}

/// Parse a single line containing ANSI true-color escape codes (`\x1b[38;2;R;G;Bm`
/// and `\x1b[0m`) into a ratatui `Line` with per-segment colors.
fn parse_ansi_line(input: &str, base_style: Style) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = base_style;
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Flush accumulated text
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), current_style));
            }
            // Parse escape sequence: expect '[' then params then 'm'
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut seq = String::new();
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                    seq.push(c);
                }
                if seq == "0" {
                    current_style = base_style;
                } else if seq.starts_with("38;2;") {
                    let parts: Vec<&str> = seq.splitn(5, ';').collect();
                    if parts.len() == 5 {
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            parts[2].parse::<u8>(),
                            parts[3].parse::<u8>(),
                            parts[4].parse::<u8>(),
                        ) {
                            current_style = base_style.fg(Color::Rgb(r, g, b));
                        }
                    }
                }
            }
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, current_style));
    }
    Line::from(spans)
}

struct MdConverter<'a> {
    theme: Theme,
    slides: Vec<Slide>,
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    list_stack: Vec<ListKind>,
    in_code_block: bool,
    in_blockquote: bool,
    in_image: bool,
    pending_layout: Option<SlideLayout>,
    pending_transition: Option<TransitionKind>,
    pending_figlet: Option<Option<String>>,
    in_heading: bool,
    heading_text_buf: String,
    images: Vec<SlideImage>,
    figlet_headings: Vec<FigletHeadingMeta>,
    pending_image_max_width: Option<f64>,
    // Semantic elements for a11y
    semantics: Vec<SemanticElement>,
    semantic_heading_level: u8,
    semantic_heading_buf: String,
    semantic_heading_line: usize,
    in_semantic_heading: bool,
    in_link: bool,
    link_url: String,
    link_text_buf: String,
    link_start_line: usize,
    link_start_col: usize,
    // Syntax highlighting
    code_block_lang: Option<String>,
    code_block_buf: String,
    syntax_set: SyntaxSet,
    syntect_theme: syntect::highlighting::Theme,
    // Frontmatter defaults
    default_layout: Option<SlideLayout>,
    default_transition: Option<TransitionKind>,
    default_image_max_width: Option<f64>,
    default_line_height: Option<f64>,
    pending_line_height: Option<f64>,
    default_figlet: Option<Option<String>>,
    default_bg_fill: Option<bool>,
    pending_bg_fill: Option<bool>,
    // External figlet renderer
    figlet_fn: Option<&'a FigletFn>,
    // Default theme for resetting after each slide
    default_theme: Theme,
    // Mobile detection
    is_mobile: bool,
    default_figlet_mobile: bool,
    pending_figlet_mobile: Option<bool>,
    // Figrat color
    default_figlet_color: Option<String>,
    pending_figlet_color: Option<String>,
    // Header
    default_header: Option<Vec<HeaderItem>>,
    pending_header: Option<Vec<HeaderItem>>,
}

#[derive(Clone)]
enum ListKind {
    Unordered,
    Ordered(u64),
}

impl<'a> MdConverter<'a> {
    fn new(
        theme: Theme,
        frontmatter: &Frontmatter,
        figlet_fn: Option<&'a FigletFn>,
        is_mobile: bool,
    ) -> Self {
        let base_style = Style::default().fg(theme.fg);
        let syntect_theme = theme.syntect_theme();
        let default_theme = theme.clone();
        Self {
            theme,
            slides: Vec::new(),
            lines: Vec::new(),
            current_spans: Vec::new(),
            style_stack: vec![base_style],
            list_stack: Vec::new(),
            in_code_block: false,
            in_blockquote: false,
            in_image: false,
            pending_layout: None,
            pending_transition: None,
            pending_figlet: None,
            in_heading: false,
            heading_text_buf: String::new(),
            images: Vec::new(),
            figlet_headings: Vec::new(),
            pending_image_max_width: None,
            semantics: Vec::new(),
            semantic_heading_level: 0,
            semantic_heading_buf: String::new(),
            semantic_heading_line: 0,
            in_semantic_heading: false,
            in_link: false,
            link_url: String::new(),
            link_text_buf: String::new(),
            link_start_line: 0,
            link_start_col: 0,
            code_block_lang: None,
            code_block_buf: String::new(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            syntect_theme,
            default_layout: frontmatter.layout.clone(),
            default_transition: frontmatter.transition.clone(),
            default_image_max_width: frontmatter.image_max_width,
            default_line_height: frontmatter.line_height,
            pending_line_height: None,
            default_figlet: frontmatter.figlet.clone(),
            default_bg_fill: frontmatter.bg_fill,
            pending_bg_fill: None,
            figlet_fn,
            default_theme,
            is_mobile,
            default_figlet_mobile: frontmatter.figlet_mobile.unwrap_or(false),
            pending_figlet_mobile: None,
            default_figlet_color: frontmatter.figlet_color.clone(),
            pending_figlet_color: None,
            default_header: frontmatter.header.clone(),
            pending_header: None,
        }
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn push_style(&mut self, modifier: impl FnOnce(Style) -> Style) {
        let base = self.current_style();
        self.style_stack.push(modifier(base));
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn flush_line(&mut self) {
        let spans = std::mem::take(&mut self.current_spans);
        if self.in_blockquote {
            let mut bq_spans = vec![Span::styled(
                "│ ",
                Style::default().fg(self.theme.block_quote_prefix),
            )];
            bq_spans.extend(spans);
            self.lines.push(Line::from(bq_spans));
        } else if self.in_code_block {
            self.lines
                .push(Line::from(spans).style(Style::default().bg(self.theme.surface)));
        } else {
            self.lines.push(Line::from(spans));
        }
    }

    fn flush_slide(&mut self) {
        if !self.current_spans.is_empty() {
            self.flush_line();
        }
        // Trim trailing blank lines (but keep bg-styled padding lines)
        while self
            .lines
            .last()
            .is_some_and(|l| l.spans.is_empty() && l.style.bg.is_none())
        {
            self.lines.pop();
        }
        let lines = std::mem::take(&mut self.lines);
        let images = std::mem::take(&mut self.images);
        self.pending_figlet = None;
        self.pending_figlet_mobile = None;
        self.pending_figlet_color = None;
        let transition = self
            .pending_transition
            .take()
            .or_else(|| self.default_transition.clone())
            .unwrap_or_default();
        if !lines.is_empty() {
            let layout = self
                .pending_layout
                .take()
                .or_else(|| self.default_layout.clone())
                .unwrap_or_default();
            let semantics = std::mem::take(&mut self.semantics);
            let figlet_headings = std::mem::take(&mut self.figlet_headings);
            let mut slide = match layout {
                SlideLayout::TwoColumn => split_two_column(lines),
                _ => Slide {
                    layout,
                    content: Text::from(lines),
                    right_content: None,
                    images: Vec::new(),
                    transition: TransitionKind::default(),
                    semantics: Vec::new(),
                    theme: Theme::default(),
                    line_height: DEFAULT_LINE_HEIGHT,
                    bg_fill: false,
                    header: Vec::new(),
                    figlet_headings: Vec::new(),
                },
            };
            slide.images = images;
            slide.transition = transition;
            slide.semantics = semantics;
            slide.figlet_headings = figlet_headings;
            slide.theme = self.theme.clone();
            slide.line_height = self
                .pending_line_height
                .take()
                .or(self.default_line_height)
                .unwrap_or(DEFAULT_LINE_HEIGHT);
            slide.bg_fill = self
                .pending_bg_fill
                .take()
                .or(self.default_bg_fill)
                .unwrap_or(false);
            slide.header = self
                .pending_header
                .take()
                .or_else(|| self.default_header.clone())
                .unwrap_or_default();
            self.slides.push(slide);
        }
        // Reset theme to default for next slide
        self.syntect_theme = self.default_theme.syntect_theme();
        self.style_stack[0] = Style::default().fg(self.default_theme.fg);
        self.theme = self.default_theme.clone();
    }

    fn list_indent(&self) -> String {
        "  ".repeat(self.list_stack.len().saturating_sub(1))
    }

    fn process(&mut self, event: Event) {
        match event {
            // --- Images ---
            Event::Start(Tag::Image { dest_url, .. }) => {
                self.in_image = true;
                if !self.current_spans.is_empty() {
                    self.flush_line();
                }
                let line_index = self.lines.len();
                self.images.push(SlideImage {
                    path: dest_url.to_string(),
                    line_index,
                    height: IMAGE_PLACEHOLDER_HEIGHT,
                    pixel_width: 0,
                    pixel_height: 0,
                    max_width_percent: self
                        .pending_image_max_width
                        .take()
                        .or(self.default_image_max_width),
                });
                // Insert placeholder lines
                for _ in 0..IMAGE_PLACEHOLDER_HEIGHT {
                    self.lines.push(Line::default());
                }
            }
            Event::End(TagEnd::Image) => {
                self.in_image = false;
            }

            // --- HTML comments (directives) ---
            Event::Html(html) | Event::InlineHtml(html) => match parse_comment(&html) {
                Some(CommentDirective::Layout(layout)) => {
                    self.pending_layout = Some(layout);
                }
                Some(CommentDirective::Transition(transition)) => {
                    self.pending_transition = Some(transition);
                }
                Some(CommentDirective::Figlet(font)) => {
                    self.pending_figlet = Some(font);
                }
                Some(CommentDirective::FigletMobile(enabled)) => {
                    self.pending_figlet_mobile = Some(enabled);
                }
                Some(CommentDirective::FigletColor(color)) => {
                    self.pending_figlet_color = Some(color);
                }
                Some(CommentDirective::ImageMaxWidth(pct)) => {
                    self.pending_image_max_width = Some(pct);
                }
                Some(CommentDirective::LineHeight(lh)) => {
                    self.pending_line_height = Some(lh);
                }
                Some(CommentDirective::Theme(t)) => {
                    self.syntect_theme = t.syntect_theme();
                    self.style_stack[0] = Style::default().fg(t.fg);
                    self.theme = t;
                }
                Some(CommentDirective::BgFill(v)) => {
                    self.pending_bg_fill = Some(v);
                }
                Some(CommentDirective::Header(items)) => {
                    self.pending_header = Some(items);
                }
                None => {}
            },

            // --- Headings ---
            Event::Start(Tag::Heading { level, .. }) => {
                let style = match level {
                    HeadingLevel::H1 => Style::default()
                        .fg(self.theme.h1)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H2 => Style::default()
                        .fg(self.theme.h2)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H3 => Style::default()
                        .fg(self.theme.h3)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(self.theme.h4)
                        .add_modifier(Modifier::BOLD),
                };
                self.push_style(|_| style);
                // Track heading for semantic overlay
                self.in_semantic_heading = true;
                self.semantic_heading_buf.clear();
                self.semantic_heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                self.semantic_heading_line = self.lines.len();
                let has_figlet = self.pending_figlet.is_some() || self.default_figlet.is_some();
                let figlet_mobile = self
                    .pending_figlet_mobile
                    .unwrap_or(self.default_figlet_mobile);
                let use_figlet = has_figlet && !(self.is_mobile && !figlet_mobile);
                if use_figlet {
                    // Apply default figlet if no per-slide directive
                    if self.pending_figlet.is_none() {
                        self.pending_figlet = self.default_figlet.clone();
                    }
                    self.in_heading = true;
                    self.heading_text_buf.clear();
                } else if !matches!(
                    self.pending_layout
                        .as_ref()
                        .or(self.default_layout.as_ref()),
                    Some(SlideLayout::Center)
                ) {
                    self.current_spans
                        .push(Span::styled("# ", self.current_style()));
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                // Emit semantic heading
                if self.in_semantic_heading {
                    self.in_semantic_heading = false;
                    self.semantics.push(SemanticElement::Heading {
                        level: self.semantic_heading_level,
                        text: self.semantic_heading_buf.clone(),
                        line_index: self.semantic_heading_line,
                    });
                }
                if self.in_heading {
                    self.in_heading = false;
                    let style = self.current_style();
                    self.current_spans.clear();
                    self.render_figlet_heading(&self.heading_text_buf.clone(), style);
                    self.lines.push(Line::default());
                } else {
                    self.flush_line();
                    self.lines.push(Line::default());
                }
                self.pop_style();
            }

            // --- Paragraph ---
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                self.flush_line();
                // Suppress blank line between list items (loose lists wrap items in paragraphs)
                if self.list_stack.is_empty() {
                    self.lines.push(Line::default());
                }
            }

            // --- Emphasis / Strong / Strikethrough ---
            Event::Start(Tag::Emphasis) => {
                self.push_style(|s| s.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => self.pop_style(),

            Event::Start(Tag::Strong) => {
                self.push_style(|s| s.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => self.pop_style(),

            Event::Start(Tag::Strikethrough) => {
                self.push_style(|s| s.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => self.pop_style(),

            // --- Code ---
            Event::Code(code) => {
                let style = Style::default()
                    .fg(self.theme.inline_code_fg)
                    .bg(self.theme.surface);
                self.current_spans
                    .push(Span::styled(format!(" {code} "), style));
            }

            // --- Code Block ---
            Event::Start(Tag::CodeBlock(kind)) => {
                self.in_code_block = true;
                self.code_block_buf.clear();
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang = lang.split(',').next().unwrap_or("").trim().to_string();
                        if lang.is_empty() { None } else { Some(lang) }
                    }
                    CodeBlockKind::Indented => None,
                };
                self.flush_line();
                // Replace preceding blank line (from paragraph end) with bg-colored padding,
                // but keep the gap when following another code block.
                if self.lines.last().is_some_and(|l| l.spans.is_empty()) {
                    let prev_has_bg = self.lines.len() >= 2
                        && self.lines[self.lines.len() - 2].style.bg.is_some();
                    if !prev_has_bg {
                        self.lines.pop();
                    }
                }
                self.lines
                    .push(Line::from("").style(Style::default().bg(self.theme.surface)));
            }
            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
                self.current_spans.clear();
                self.flush_code_block();
                self.lines
                    .push(Line::from("").style(Style::default().bg(self.theme.surface)));
                self.lines.push(Line::default());
            }

            // --- Lists ---
            Event::Start(Tag::List(start)) => {
                if !self.current_spans.is_empty() {
                    self.flush_line();
                }
                let kind = match start {
                    Some(n) => ListKind::Ordered(n),
                    None => ListKind::Unordered,
                };
                self.list_stack.push(kind);
            }
            Event::End(TagEnd::List(_)) => {
                self.list_stack.pop();
                if self.list_stack.is_empty() {
                    self.lines.push(Line::default());
                }
            }

            Event::Start(Tag::Item) => {
                let indent = self.list_indent();
                let bullet = match self.list_stack.last() {
                    Some(ListKind::Unordered) => format!("{indent}• "),
                    Some(ListKind::Ordered(n)) => {
                        let s = format!("{indent}{}. ", n);
                        if let Some(ListKind::Ordered(num)) = self.list_stack.last_mut() {
                            *num += 1;
                        }
                        s
                    }
                    None => String::new(),
                };
                self.current_spans.push(Span::styled(
                    bullet,
                    Style::default().fg(self.theme.list_bullet),
                ));
            }
            Event::End(TagEnd::Item) => {
                if !self.current_spans.is_empty() {
                    self.flush_line();
                }
            }

            // --- Blockquote ---
            Event::Start(Tag::BlockQuote(_)) => {
                self.in_blockquote = true;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                self.in_blockquote = false;
                self.lines.push(Line::default());
            }

            // --- Horizontal Rule = Slide separator ---
            Event::Rule => {
                self.flush_slide();
            }

            // --- Links ---
            Event::Start(Tag::Link { dest_url, .. }) => {
                self.in_link = true;
                self.link_url = dest_url.to_string();
                self.link_text_buf.clear();
                self.link_start_line = self.lines.len();
                self.link_start_col = self.current_spans.iter().map(|s| s.width()).sum::<usize>()
                    + if self.in_blockquote { 2 } else { 0 }; // "│ " prefix
                let link_color = self.theme.link;
                self.push_style(|s| s.fg(link_color).add_modifier(Modifier::UNDERLINED));
            }
            Event::End(TagEnd::Link) => {
                let end_col: usize = self.link_start_col + Span::raw(&self.link_text_buf).width();
                self.semantics.push(SemanticElement::Link {
                    url: std::mem::take(&mut self.link_url),
                    text: std::mem::take(&mut self.link_text_buf),
                    line_index: self.link_start_line,
                    start_col: self.link_start_col,
                    end_col,
                });
                self.in_link = false;
                self.pop_style();
            }

            // --- Text ---
            Event::Text(text) => {
                // Accumulate into semantic buffers
                if self.in_semantic_heading {
                    self.semantic_heading_buf.push_str(&text);
                }
                if self.in_link {
                    self.link_text_buf.push_str(&text);
                }

                if self.in_heading {
                    self.heading_text_buf.push_str(&text);
                } else if self.in_image {
                    // Skip alt text of images
                } else if self.in_code_block {
                    self.code_block_buf.push_str(&text);
                } else {
                    self.current_spans
                        .push(Span::styled(text.to_string(), self.current_style()));
                }
            }

            Event::SoftBreak => {
                self.current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                self.flush_line();
            }

            _ => {}
        }
    }

    fn process_horizontal_rule(&mut self) {
        if !self.current_spans.is_empty() {
            self.flush_line();
        }
        self.lines.push(Line::from(Span::styled(
            "─".repeat(40),
            Style::default().fg(self.theme.fg),
        )));
        self.lines.push(Line::default());
    }

    fn flush_code_block(&mut self) {
        let buf = std::mem::take(&mut self.code_block_buf);
        let lang = self.code_block_lang.take();
        let bg = self.theme.surface;
        let code = buf.trim_end_matches('\n');

        let syntax = lang.as_deref().and_then(|l| {
            self.syntax_set.find_syntax_by_token(l).or_else(|| {
                // Fallback: map common tokens missing from syntect defaults
                let fallback = match l {
                    "jsx" | "tsx" | "ts" | "typescript" => Some("js"),
                    _ => None,
                };
                fallback.and_then(|f| self.syntax_set.find_syntax_by_token(f))
            })
        });

        if let Some(syntax) = syntax {
            let mut h = syntect::easy::HighlightLines::new(syntax, &self.syntect_theme);
            for line in code.split('\n') {
                let regions = h.highlight_line(line, &self.syntax_set).unwrap_or_default();
                let mut spans: Vec<Span<'static>> =
                    vec![Span::styled("\u{00a0}\u{00a0}", Style::default().bg(bg))];
                for (syn_style, text) in regions {
                    let fg_color = Color::Rgb(
                        syn_style.foreground.r,
                        syn_style.foreground.g,
                        syn_style.foreground.b,
                    );
                    let mut style = Style::default().fg(fg_color).bg(bg);
                    if syn_style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::BOLD)
                    {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if syn_style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::ITALIC)
                    {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if syn_style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::UNDERLINE)
                    {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    // Use NBSP so word-wrapper falls back to character-based
                    // wrapping, matching wrapped_line_height calculation.
                    spans.push(Span::styled(text.replace(' ', "\u{00a0}"), style));
                }
                self.lines
                    .push(Line::from(spans).style(Style::default().bg(bg)));
            }
        } else {
            // Fallback: uniform style (no language or unknown language)
            let style = Style::default().fg(self.theme.fg).bg(bg);
            for line in code.split('\n') {
                // Use NBSP so word-wrapper falls back to character-based wrapping
                let text = format!("\u{00a0}\u{00a0}{}", line.replace(' ', "\u{00a0}"));
                self.lines.push(
                    Line::from(vec![Span::styled(text, style)])
                        .style(Style::default().bg(bg)),
                );
            }
        }
    }

    fn render_figlet_heading(&mut self, text: &str, style: Style) {
        let style = style.remove_modifier(Modifier::UNDERLINED);
        let font = self.pending_figlet.as_ref().and_then(|f| f.as_deref());
        let color = self
            .pending_figlet_color
            .as_deref()
            .or(self.default_figlet_color.as_deref());
        let has_color = color.is_some();
        let art = self.figlet_fn.and_then(|f| f(text, font, color));

        let Some(art) = art else {
            self.current_spans
                .push(Span::styled(text.to_string(), style));
            self.flush_line();
            return;
        };
        // Trim trailing all-whitespace lines
        let art_lines: Vec<&str> = art.split('\n').collect();
        let end = art_lines
            .iter()
            .rposition(|l| l.chars().any(|c| !c.is_whitespace()))
            .map_or(0, |i| i + 1);

        let line_index = self.lines.len();
        let mut styled_lines = Vec::new();
        if has_color {
            // Parse ANSI escape codes into colored Spans
            for line in &art_lines[..end] {
                let l = parse_ansi_line(line, style);
                styled_lines.push(l.clone());
                self.lines.push(l);
            }
        } else {
            for line in &art_lines[..end] {
                let l = Line::from(Span::styled(line.to_string(), style));
                styled_lines.push(l.clone());
                self.lines.push(l);
            }
        }
        let line_count = styled_lines.len();
        if line_count > 0 {
            self.figlet_headings.push(FigletHeadingMeta {
                line_index,
                line_count,
                styled_lines,
            });
        }
    }

    fn finish_slides(mut self) -> Vec<Slide> {
        self.flush_slide();
        if self.slides.is_empty() && !self.lines.is_empty() {
            let layout = self
                .pending_layout
                .take()
                .or_else(|| self.default_layout.clone())
                .unwrap_or_default();
            let transition = self
                .pending_transition
                .take()
                .or_else(|| self.default_transition.clone())
                .unwrap_or_default();
            self.slides.push(Slide {
                layout,
                content: Text::from(self.lines),
                right_content: None,
                images: std::mem::take(&mut self.images),
                transition,
                semantics: std::mem::take(&mut self.semantics),
                theme: self.theme.clone(),
                line_height: self
                    .pending_line_height
                    .take()
                    .or(self.default_line_height)
                    .unwrap_or(1.2),
                bg_fill: self
                    .pending_bg_fill
                    .take()
                    .or(self.default_bg_fill)
                    .unwrap_or(false),
                header: self
                    .pending_header
                    .take()
                    .or_else(|| self.default_header.clone())
                    .unwrap_or_default(),
                figlet_headings: std::mem::take(&mut self.figlet_headings),
            });
        }
        self.slides
    }
}

/// Split lines at `|||` marker into left/right columns for TwoColumn layout.
fn split_two_column(lines: Vec<Line<'static>>) -> Slide {
    let sep_idx = lines.iter().position(|line| {
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        text.trim() == "|||"
    });

    match sep_idx {
        Some(idx) => {
            let mut left: Vec<Line<'static>> = lines[..idx].to_vec();
            let mut right: Vec<Line<'static>> = lines[idx + 1..].to_vec();
            // Trim trailing blanks
            while left.last().is_some_and(|l| l.spans.is_empty()) {
                left.pop();
            }
            while right.last().is_some_and(|l| l.spans.is_empty()) {
                right.pop();
            }
            // Trim leading blanks from right
            while right.first().is_some_and(|l| l.spans.is_empty()) {
                right.remove(0);
            }
            Slide {
                layout: SlideLayout::TwoColumn,
                content: Text::from(left),
                right_content: Some(Text::from(right)),
                images: Vec::new(),
                transition: TransitionKind::default(),
                semantics: Vec::new(),
                theme: Theme::default(),
                line_height: 1.2,
                bg_fill: false,
                header: Vec::new(),
                figlet_headings: Vec::new(),
            }
        }
        None => Slide {
            layout: SlideLayout::TwoColumn,
            content: Text::from(lines),
            right_content: None,
            images: Vec::new(),
            transition: TransitionKind::default(),
            semantics: Vec::new(),
            theme: Theme::default(),
            line_height: 1.2,
            bg_fill: false,
            header: Vec::new(),
            figlet_headings: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_theme() -> Theme {
        Theme::default()
    }

    fn parse(md: &str) -> Vec<Slide> {
        let fm = Frontmatter::default();
        parse_slides(md, &test_theme(), &fm, None, false)
    }

    /// Helper: collect (text, has_bg) for each line in a slide.
    fn line_info(slide: &Slide) -> Vec<(String, bool)> {
        slide
            .content
            .lines
            .iter()
            .map(|l| {
                let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
                let has_bg = l.style.bg.is_some() || l.spans.iter().any(|s| s.style.bg.is_some());
                (text, has_bg)
            })
            .collect()
    }

    #[test]
    fn single_code_block() {
        let md = "```\nhello\n```\n";
        let slides = parse(md);
        assert_eq!(slides.len(), 1);
        let info = line_info(&slides[0]);

        // Expected: bg_pad, "  hello"(bg)
        // (trailing bg_pad is trimmed by flush_slide since Line::from("") has empty spans)
        assert!(info.len() >= 2, "got {} lines: {:?}", info.len(), info);
        // First line is bg padding
        assert!(info[0].1, "first line should have bg");
        // Content line
        assert!(info[1].0.contains("hello"), "content line: {:?}", info[1]);
        assert!(info[1].1, "content should have bg");
    }

    #[test]
    fn consecutive_code_blocks_have_gap() {
        let md = "```\nfirst\n```\n\n```\nsecond\n```\n";
        let slides = parse(md);
        assert_eq!(slides.len(), 1);
        let info = line_info(&slides[0]);

        // Find the two content lines
        let first_idx = info.iter().position(|(t, _)| t.contains("first")).unwrap();
        let second_idx = info.iter().position(|(t, _)| t.contains("second")).unwrap();

        // There should be a non-bg (blank) line between the two blocks
        let between = &info[first_idx + 1..second_idx];
        let has_blank = between.iter().any(|(_, bg)| !bg);
        assert!(
            has_blank,
            "expected a blank (non-bg) gap between code blocks, got: {:?}",
            between
        );
    }

    #[test]
    fn consecutive_code_blocks_no_stale_spans() {
        let md = "```\naaa\n```\n\n```\nbbb\n```\n";
        let slides = parse(md);
        let info = line_info(&slides[0]);

        // No line should be just whitespace with bg (stale span artifact)
        for (text, has_bg) in &info {
            if text.trim().is_empty() && *has_bg {
                // Only bg-padding lines (empty string) are allowed, not "  " leftover
                assert!(
                    text.is_empty(),
                    "found stale whitespace-only bg line: {:?}",
                    text
                );
            }
        }
    }

    #[test]
    fn code_block_after_paragraph_no_double_blank() {
        let md = "some text\n\n```\ncode\n```\n";
        let slides = parse(md);
        let info = line_info(&slides[0]);

        // The paragraph text should exist
        assert!(info.iter().any(|(t, _)| t.contains("some text")));
        // The code content should exist
        assert!(info.iter().any(|(t, _)| t.contains("code")));

        // No two consecutive blank non-bg lines (would show as double gap)
        for w in info.windows(2) {
            let both_blank =
                w[0].0.trim().is_empty() && !w[0].1 && w[1].0.trim().is_empty() && !w[1].1;
            assert!(!both_blank, "found double blank gap: {:?}", info);
        }
    }

    #[test]
    fn code_block_at_slide_end_has_bottom_padding() {
        let md = "# Title\n\n```\ncode\n```\n";
        let slides = parse(md);
        assert_eq!(slides.len(), 1);
        let info = line_info(&slides[0]);

        let code_idx = info.iter().position(|(t, _)| t.contains("code")).unwrap();
        // There should be a bg-colored padding line after the code content
        assert!(
            info.len() > code_idx + 1,
            "missing bottom padding after code block at slide end: {:?}",
            info
        );
        assert!(
            info[code_idx + 1].1,
            "bottom padding should have bg: {:?}",
            info
        );
    }

    #[test]
    fn three_consecutive_code_blocks() {
        let md = "```\na\n```\n\n```\nb\n```\n\n```\nc\n```\n";
        let slides = parse(md);
        assert_eq!(slides.len(), 1);
        let info = line_info(&slides[0]);

        let a_idx = info.iter().position(|(t, _)| t.contains("a")).unwrap();
        let b_idx = info.iter().position(|(t, _)| t.contains("b")).unwrap();
        let c_idx = info.iter().position(|(t, _)| t.contains("c")).unwrap();

        // Gap between block 1 and 2
        let gap1 = &info[a_idx + 1..b_idx];
        assert!(
            gap1.iter().any(|(_, bg)| !bg),
            "expected gap between block 1 and 2: {:?}",
            gap1
        );

        // Gap between block 2 and 3
        let gap2 = &info[b_idx + 1..c_idx];
        assert!(
            gap2.iter().any(|(_, bg)| !bg),
            "expected gap between block 2 and 3: {:?}",
            gap2
        );
    }
}
