use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

#[derive(Clone, Debug, Default)]
pub enum SlideLayout {
    #[default]
    Default,
    Center,
    TwoColumn,
}

#[derive(Clone)]
pub struct Slide {
    pub layout: SlideLayout,
    pub content: Text<'static>,
    /// Right column content (only for TwoColumn layout)
    pub right_content: Option<Text<'static>>,
}

/// Parse markdown into slides split by `---` (horizontal rule).
pub fn parse_slides(input: &str) -> Vec<Slide> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(input, options);
    let mut converter = MdConverter::new();
    for event in parser {
        converter.process(event);
    }
    converter.finish_slides()
}

fn parse_layout_comment(html: &str) -> Option<SlideLayout> {
    let trimmed = html.trim();
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?;
    let inner = inner.trim();
    let value = inner.strip_prefix("layout:")?.trim();
    match value {
        "center" => Some(SlideLayout::Center),
        "two-column" => Some(SlideLayout::TwoColumn),
        "default" => Some(SlideLayout::Default),
        _ => None,
    }
}

struct MdConverter {
    slides: Vec<Slide>,
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    list_stack: Vec<ListKind>,
    in_code_block: bool,
    in_blockquote: bool,
    pending_layout: Option<SlideLayout>,
}

#[derive(Clone)]
enum ListKind {
    Unordered,
    Ordered(u64),
}

impl MdConverter {
    fn new() -> Self {
        Self {
            slides: Vec::new(),
            lines: Vec::new(),
            current_spans: Vec::new(),
            style_stack: vec![Style::default()],
            list_stack: Vec::new(),
            in_code_block: false,
            in_blockquote: false,
            pending_layout: None,
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
            let mut bq_spans = vec![Span::styled("│ ", Style::default().fg(Color::DarkGray))];
            bq_spans.extend(spans);
            self.lines.push(Line::from(bq_spans));
        } else {
            self.lines.push(Line::from(spans));
        }
    }

    fn flush_slide(&mut self) {
        if !self.current_spans.is_empty() {
            self.flush_line();
        }
        // Trim trailing blank lines
        while self.lines.last().is_some_and(|l| l.spans.is_empty()) {
            self.lines.pop();
        }
        let lines = std::mem::take(&mut self.lines);
        if !lines.is_empty() {
            let layout = self.pending_layout.take().unwrap_or_default();
            let slide = match layout {
                SlideLayout::TwoColumn => split_two_column(lines),
                _ => Slide {
                    layout,
                    content: Text::from(lines),
                    right_content: None,
                },
            };
            self.slides.push(slide);
        }
    }

    fn list_indent(&self) -> String {
        "  ".repeat(self.list_stack.len().saturating_sub(1))
    }

    fn process(&mut self, event: Event) {
        match event {
            // --- HTML comments (layout directives) ---
            Event::Html(html) | Event::InlineHtml(html) => {
                if let Some(layout) = parse_layout_comment(&html) {
                    self.pending_layout = Some(layout);
                }
                // Otherwise ignore HTML
            }

            // --- Headings ---
            Event::Start(Tag::Heading { level, .. }) => {
                let style = match level {
                    HeadingLevel::H1 => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    HeadingLevel::H2 => Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H3 => Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default().add_modifier(Modifier::BOLD),
                };
                self.push_style(|_| style);
            }
            Event::End(TagEnd::Heading(_)) => {
                self.flush_line();
                self.lines.push(Line::default());
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
                let style = Style::default().fg(Color::Red).bg(Color::DarkGray);
                self.current_spans
                    .push(Span::styled(format!(" {code} "), style));
            }

            // --- Code Block ---
            Event::Start(Tag::CodeBlock(_kind)) => {
                self.in_code_block = true;
                self.flush_line();
            }
            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
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
                self.current_spans
                    .push(Span::styled(bullet, Style::default().fg(Color::DarkGray)));
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
                if self.in_code_block {
                    let style = Style::default().fg(Color::White).bg(Color::DarkGray);
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

    fn finish_slides(mut self) -> Vec<Slide> {
        self.flush_slide();
        if self.slides.is_empty() && !self.lines.is_empty() {
            self.slides.push(Slide {
                layout: SlideLayout::Default,
                content: Text::from(self.lines),
                right_content: None,
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
            }
        }
        None => Slide {
            layout: SlideLayout::TwoColumn,
            content: Text::from(lines),
            right_content: None,
        },
    }
}
