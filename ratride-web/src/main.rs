use std::cell::RefCell;
use std::rc::Rc;

use ratride_core::markdown::{parse_slides, Slide, TransitionKind};
use ratride_core::render::{self, ImagePlacement};
use ratride_core::theme::{self, Theme};
use ratzilla::ratatui::layout::{Constraint, Layout, Rect};
use ratzilla::{event::KeyCode, CanvasBackend, DomBackend, WebGl2Backend, WebRenderer};
use tachyonfx::{fx, Duration, Effect, EffectRenderer, Interpolation, Motion};

const MD: &str = include_str!(env!("RATRIDE_SLIDE_FILE"));

include!(concat!(env!("OUT_DIR"), "/embedded_images.rs"));
const FRAME_DURATION_MS: u32 = 16;

struct WebApp {
    slides: Vec<Slide>,
    current_page: usize,
    scroll_offsets: Vec<u16>,
    effect: Option<Effect>,
    theme: Theme,
}

impl WebApp {
    fn new() -> Self {
        let theme = theme_from_query()
            .and_then(|name| theme::theme_from_name(&name))
            .or_else(|| theme::theme_from_markdown(MD))
            .unwrap_or_default();
        let slides = parse_slides(MD, &theme);
        let len = slides.len().max(1);
        let mut app = Self {
            slides,
            current_page: 0,
            scroll_offsets: vec![0; len],
            effect: None,
            theme,
        };
        app.effect = Some(app.create_transition());
        app
    }

    fn total_pages(&self) -> usize {
        self.slides.len()
    }

    fn scroll_offset(&self) -> u16 {
        self.scroll_offsets[self.current_page]
    }

    fn goto_page(&mut self, page: usize) {
        if page < self.total_pages() && page != self.current_page {
            self.current_page = page;
            self.effect = Some(self.create_transition());
        }
    }

    fn next_page(&mut self) {
        let next = self.current_page + 1;
        self.goto_page(next);
    }

    fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.goto_page(self.current_page - 1);
        }
    }

    fn create_transition(&self) -> Effect {
        let slide = &self.slides[self.current_page];
        let bg = self.theme.bg;
        match slide.transition {
            TransitionKind::SlideIn => {
                fx::fade_from_fg(bg, (400, Interpolation::QuadOut))
            }
            TransitionKind::Fade => fx::fade_from_fg(bg, (600, Interpolation::SineOut)),
            TransitionKind::Dissolve => fx::dissolve((500, Interpolation::Linear)).reversed(),
            TransitionKind::Coalesce => fx::coalesce((500, Interpolation::QuadOut)),
            TransitionKind::SweepIn => fx::sweep_in(
                Motion::LeftToRight,
                15,
                0,
                bg,
                (600, Interpolation::QuadOut),
            ),
        }
    }
}

/// Read `?theme=...` from the URL query string.
fn theme_from_query() -> Option<String> {
    let href = ratzilla::web_sys::window()
        .and_then(|w| w.location().href().ok())
        .unwrap_or_default();
    if let Some(q) = href.split('?').nth(1) {
        for pair in q.split('&') {
            if let Some(val) = pair.strip_prefix("theme=") {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Read `?backend=dom|canvas|webgl2` from the URL query string.
fn backend_from_query() -> &'static str {
    let href = ratzilla::web_sys::window()
        .and_then(|w| w.location().href().ok())
        .unwrap_or_default();
    if let Some(q) = href.split('?').nth(1) {
        for pair in q.split('&') {
            if let Some(val) = pair.strip_prefix("backend=") {
                return match val {
                    "canvas" => "canvas",
                    "webgl2" => "webgl2",
                    _ => "dom",
                };
            }
        }
    }
    "dom"
}

/// Run the app with the given terminal + backend.
fn run<B: ratzilla::ratatui::backend::Backend + 'static>(
    terminal: ratzilla::ratatui::Terminal<B>,
    app: Rc<RefCell<WebApp>>,
) {
    terminal.on_key_event({
        let app = app.clone();
        move |key_event| {
            let mut app = app.borrow_mut();
            let page = app.current_page;
            match key_event.code {
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => app.next_page(),
                KeyCode::Left | KeyCode::Char('h') => app.prev_page(),
                KeyCode::Char('j') | KeyCode::Down => {
                    app.scroll_offsets[page] = app.scroll_offsets[page].saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.scroll_offsets[page] = app.scroll_offsets[page].saturating_sub(1);
                }
                KeyCode::Char('d') => {
                    app.scroll_offsets[page] = app.scroll_offsets[page].saturating_add(10);
                }
                KeyCode::Char('u') => {
                    app.scroll_offsets[page] = app.scroll_offsets[page].saturating_sub(10);
                }
                _ => {}
            }
        }
    });

    terminal.draw_web({
        let app = app.clone();
        move |frame| {
            let mut app = app.borrow_mut();
            let area = frame.area();

            let [main_area, status_area] =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

            let slide = &app.slides[app.current_page];
            let layout = slide.layout.clone();
            let scroll = app.scroll_offset();

            let placements = render::draw_slide(slide, scroll, frame, main_area);

            let is_transitioning = if let Some(ref mut effect) = app.effect {
                let delta = Duration::from_millis(FRAME_DURATION_MS);
                frame.render_effect(effect, main_area, delta);
                if effect.done() {
                    app.effect = None;
                    false
                } else {
                    true
                }
            } else {
                false
            };

            let total = app.total_pages();
            render::draw_status_bar(&layout, app.current_page, total, frame, status_area, &app.theme);

            update_image_overlay(&placements, area, is_transitioning);
        }
    });
}

fn main() -> std::io::Result<()> {
    let app = Rc::new(RefCell::new(WebApp::new()));

    // Set body background to theme color
    if let Some(body) = ratzilla::web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    {
        let bg = app.borrow().theme.bg_hex();
        let _ = body.set_attribute("style", &format!("background-color:{bg}"));
    }

    match backend_from_query() {
        "canvas" => {
            let terminal = ratzilla::ratatui::Terminal::new(CanvasBackend::new()?)?;
            run(terminal, app);
        }
        "webgl2" => {
            let terminal = ratzilla::ratatui::Terminal::new(WebGl2Backend::new()?)?;
            run(terminal, app);
        }
        _ => {
            let terminal = ratzilla::ratatui::Terminal::new(DomBackend::new()?)?;
            run(terminal, app);
        }
    }

    Ok(())
}

/// Overlay `<img>` elements on top of the ratzilla grid for each image placement.
fn update_image_overlay(placements: &[ImagePlacement], terminal_area: Rect, hide: bool) {
    let window = match ratzilla::web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    // Get or create the overlay container
    let overlay = match document.get_element_by_id("ratride-image-overlay") {
        Some(el) => el,
        None => {
            let el = document.create_element("div").unwrap();
            el.set_id("ratride-image-overlay");
            el.set_attribute(
                "style",
                "position:fixed;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:10",
            )
            .unwrap();
            document.body().unwrap().append_child(&el).unwrap();
            el
        }
    };

    // Clear previous frame's images
    overlay.set_inner_html("");

    if hide || placements.is_empty() {
        return;
    }

    // Find the ratzilla grid element (pre for DOM backend, canvas for canvas/webgl2)
    let grid_el = document
        .query_selector("pre")
        .ok()
        .flatten()
        .or_else(|| document.query_selector("canvas").ok().flatten());

    let grid_el = match grid_el {
        Some(el) => el,
        None => return,
    };

    let rect = grid_el.get_bounding_client_rect();
    let grid_x = rect.left();
    let grid_y = rect.top();
    let grid_w = rect.width();
    let grid_h = rect.height();

    let cols = terminal_area.width as f64;
    let rows = terminal_area.height as f64;
    if cols == 0.0 || rows == 0.0 {
        return;
    }

    let cell_w = grid_w / cols;
    let cell_h = grid_h / rows;

    for p in placements {
        let src = match get_embedded_image(&p.path) {
            Some(s) => s,
            None => continue,
        };

        let px_x = grid_x + p.x as f64 * cell_w;
        let px_y = grid_y + p.y as f64 * cell_h;
        let px_w = p.width as f64 * cell_w;
        let px_h = p.height as f64 * cell_h;

        let img = document.create_element("img").unwrap();
        img.set_attribute("src", src).unwrap();
        img.set_attribute(
            "style",
            &format!(
                "position:fixed;left:{px_x}px;top:{px_y}px;width:{px_w}px;height:{px_h}px;\
                 object-fit:contain;pointer-events:none"
            ),
        )
        .unwrap();

        overlay.append_child(&img).unwrap();
    }
}
