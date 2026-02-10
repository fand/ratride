mod markdown;

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use markdown::Slide;
use ratatui::{
    layout::{Constraint, Layout, Margin},
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
        let content = &slide.content;

        // Render slide content
        let paragraph = Paragraph::new(content.clone())
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(paragraph, main_area.inner(Margin::new(2, 1)));

        // Scrollbar
        let content_len = content.lines.len();
        let visible = main_area.height.saturating_sub(2) as usize;
        if content_len > visible {
            let mut scrollbar_state =
                ScrollbarState::new(content_len.saturating_sub(visible))
                    .position(self.scroll_offset() as usize);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                main_area,
                &mut scrollbar_state,
            );
        }

        // Status bar
        let status = format!(
            " ←/→:page  j/k:scroll  q:quit    [{}/{}]",
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
