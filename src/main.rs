mod markdown;

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use markdown::{Slide, SlideLayout};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    DefaultTerminal, Frame,
};

struct App {
    slides: Vec<Slide>,
    current_page: usize,
    scroll_offsets: Vec<u16>,
    quit: bool,
}

impl App {
    fn new(markdown: &str) -> Self {
        let slides = markdown::parse_slides(markdown);
        let len = slides.len().max(1);
        Self {
            slides,
            current_page: 0,
            scroll_offsets: vec![0; len],
            quit: false,
        }
    }

    fn total_pages(&self) -> usize {
        self.slides.len()
    }

    fn current_slide(&self) -> &Slide {
        &self.slides[self.current_page]
    }

    fn scroll_offset(&self) -> u16 {
        self.scroll_offsets[self.current_page]
    }

    fn scroll_offset_mut(&mut self) -> &mut u16 {
        &mut self.scroll_offsets[self.current_page]
    }

    fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages() {
            self.current_page += 1;
        }
    }

    fn prev_page(&mut self) {
        self.current_page = self.current_page.saturating_sub(1);
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_event()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

        let slide = self.current_slide();

        match slide.layout {
            SlideLayout::Default => self.draw_default(frame, main_area, slide),
            SlideLayout::Center => self.draw_center(frame, main_area, slide),
            SlideLayout::TwoColumn => self.draw_two_column(frame, main_area, slide),
        }

        // Status bar
        let layout_label = match slide.layout {
            SlideLayout::Default => "",
            SlideLayout::Center => " [center]",
            SlideLayout::TwoColumn => " [two-column]",
        };
        let status = format!(
            " ←/→:page  j/k:scroll  q:quit{}    [{}/{}]",
            layout_label,
            self.current_page + 1,
            self.total_pages()
        );
        frame.render_widget(
            Paragraph::new(status).style(
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::DarkGray)
                    .fg(ratatui::style::Color::White),
            ),
            status_area,
        );
    }

    fn draw_default(&self, frame: &mut Frame, area: Rect, slide: &Slide) {
        let content_area = area.inner(Margin::new(2, 1));

        let paragraph = Paragraph::new(slide.content.clone())
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(paragraph, content_area);

        self.draw_scrollbar(frame, area, slide.content.lines.len(), content_area.height);
    }

    fn draw_center(&self, frame: &mut Frame, area: Rect, slide: &Slide) {
        let content_height = slide.content.lines.len() as u16;
        let content_area = area.inner(Margin::new(2, 1));

        // Vertically center
        let [centered_area] = Layout::vertical([Constraint::Length(content_height)])
            .flex(Flex::Center)
            .areas(content_area);

        let paragraph = Paragraph::new(slide.content.clone())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(paragraph, centered_area);
    }

    fn draw_two_column(&self, frame: &mut Frame, area: Rect, slide: &Slide) {
        let content_area = area.inner(Margin::new(2, 1));

        let [left_area, _gap, right_area] = Layout::horizontal([
            Constraint::Percentage(48),
            Constraint::Percentage(4),
            Constraint::Percentage(48),
        ])
        .areas(content_area);

        // Left column
        let left_para = Paragraph::new(slide.content.clone())
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(left_para, left_area);

        // Right column
        if let Some(ref right) = slide.right_content {
            let right_para = Paragraph::new(right.clone())
                .wrap(Wrap { trim: false })
                .scroll((self.scroll_offset(), 0));
            frame.render_widget(right_para, right_area);
        }
    }

    fn draw_scrollbar(&self, frame: &mut Frame, area: Rect, content_len: usize, visible: u16) {
        let visible = visible as usize;
        if content_len > visible {
            let mut scrollbar_state = ScrollbarState::new(content_len.saturating_sub(visible))
                .position(self.scroll_offset() as usize);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut scrollbar_state,
            );
        }
    }

    fn handle_event(&mut self) -> io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
                // Page navigation
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => self.next_page(),
                KeyCode::Left | KeyCode::Char('h') => self.prev_page(),
                // Scroll
                KeyCode::Char('j') | KeyCode::Down => {
                    *self.scroll_offset_mut() = self.scroll_offset().saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(1);
                }
                KeyCode::Char('d') => {
                    *self.scroll_offset_mut() = self.scroll_offset().saturating_add(10);
                }
                KeyCode::Char('u') => {
                    *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(10);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("main.md");
    let markdown = std::fs::read_to_string(path)?;

    let terminal = ratatui::init();
    let result = App::new(&markdown).run(terminal);
    ratatui::restore();
    result
}
