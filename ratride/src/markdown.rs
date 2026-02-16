use crate::theme::Theme;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::io::Write;
use std::process::{Command, Stdio};

/// File-wide defaults parsed from YAML frontmatter (`--- ... ---`).
#[derive(Clone, Debug, Default)]
pub struct Frontmatter {
    pub theme: Option<String>,
    pub layout: Option<SlideLayout>,
    pub transition: Option<TransitionKind>,
    pub image_max_width: Option<f64>,
    /// `Some(None)` = default figlet font, `Some(Some("slant"))` = named font.
    pub figlet: Option<Option<String>>,
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
    let close_pos = after_open
        .find("\n---")
        .map(|i| i + 1); // point to the `---` line start
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
    for line in yaml_block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
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
                "figlet" => {
                    if value.is_empty() || value == "true" {
                        fm.figlet = Some(None);
                    } else if value != "false" {
                        fm.figlet = Some(Some(value.to_string()));
                    }
                }
                _ => {}
            }
        }
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
}

const IMAGE_PLACEHOLDER_HEIGHT: u16 = 15;

/// Parse markdown into slides split by `---` (horizontal rule).
pub fn parse_slides(input: &str, theme: &Theme, frontmatter: &Frontmatter) -> Vec<Slide> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(input, options);
    let mut converter = MdConverter::new(theme.clone(), frontmatter);
    for event in parser {
        converter.process(event);
    }
    converter.finish_slides()
}

enum CommentDirective {
    Layout(SlideLayout),
    Transition(TransitionKind),
    Figlet(Option<String>),
    ImageMaxWidth(f64),
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
    if let Some(value) = inner.strip_prefix("image_max_width:") {
        let value = value.trim().trim_end_matches('%');
        if let Ok(pct) = value.parse::<f64>() {
            return Some(CommentDirective::ImageMaxWidth(pct / 100.0));
        }
    }
    None
}

struct MdConverter {
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
    pending_image_max_width: Option<f64>,
    // Frontmatter defaults
    default_layout: Option<SlideLayout>,
    default_transition: Option<TransitionKind>,
    default_image_max_width: Option<f64>,
    default_figlet: Option<Option<String>>,
}

#[derive(Clone)]
enum ListKind {
    Unordered,
    Ordered(u64),
}

impl MdConverter {
    fn new(theme: Theme, frontmatter: &Frontmatter) -> Self {
        let base_style = Style::default().fg(theme.fg);
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
            pending_image_max_width: None,
            default_layout: frontmatter.layout.clone(),
            default_transition: frontmatter.transition.clone(),
            default_image_max_width: frontmatter.image_max_width,
            default_figlet: frontmatter.figlet.clone(),
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
            self.lines.push(
                Line::from(spans).style(Style::default().bg(self.theme.surface)),
            );
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
            let mut slide = match layout {
                SlideLayout::TwoColumn => split_two_column(lines),
                _ => Slide {
                    layout,
                    content: Text::from(lines),
                    right_content: None,
                    images: Vec::new(),
                    transition: TransitionKind::default(),
                },
            };
            slide.images = images;
            slide.transition = transition;
            self.slides.push(slide);
        }
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
                Some(CommentDirective::ImageMaxWidth(pct)) => {
                    self.pending_image_max_width = Some(pct);
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
                let use_figlet = self.pending_figlet.is_some() || self.default_figlet.is_some();
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
                self.lines.push(Line::default());
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
            Event::Start(Tag::CodeBlock(_kind)) => {
                self.in_code_block = true;
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
                // Discard trailing whitespace-only span left by text.split('\n')
                self.current_spans.clear();
                self.lines
                    .push(Line::from("").style(Style::default().bg(self.theme.surface)));
                self.lines.push(Line::default());
            }

            // --- Lists ---
            Event::Start(Tag::List(start)) => {
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
                self.flush_line();
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

            // --- Text ---
            Event::Text(text) => {
                if self.in_heading {
                    self.heading_text_buf.push_str(&text);
                } else if self.in_image {
                    // Skip alt text of images
                } else if self.in_code_block {
                    let style = Style::default().fg(self.theme.fg).bg(self.theme.surface);
                    for line in text.split('\n') {
                        if !self.current_spans.is_empty() {
                            self.flush_line();
                        }
                        self.current_spans
                            .push(Span::styled(format!("  {line}"), style));
                    }
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

    fn render_figlet_heading(&mut self, text: &str, style: Style) {
        let style = style.remove_modifier(Modifier::UNDERLINED);
        let mut cmd = Command::new("figlet");
        if let Some(Some(font)) = &self.pending_figlet {
            cmd.args(["-f", font]);
        }
        let art = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait_with_output()
            })
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok());

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
        for line in &art_lines[..end] {
            self.lines
                .push(Line::from(Span::styled(line.to_string(), style)));
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
            }
        }
        None => Slide {
            layout: SlideLayout::TwoColumn,
            content: Text::from(lines),
            right_content: None,
            images: Vec::new(),
            transition: TransitionKind::default(),
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
        parse_slides(md, &test_theme(), &fm)
    }

    /// Helper: collect (text, has_bg) for each line in a slide.
    fn line_info(slide: &Slide) -> Vec<(String, bool)> {
        slide
            .content
            .lines
            .iter()
            .map(|l| {
                let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
                let has_bg = l.style.bg.is_some()
                    || l.spans.iter().any(|s| s.style.bg.is_some());
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
            let both_blank = w[0].0.trim().is_empty()
                && !w[0].1
                && w[1].0.trim().is_empty()
                && !w[1].1;
            assert!(
                !both_blank,
                "found double blank gap: {:?}",
                info
            );
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
