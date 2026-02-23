mod app;
mod backend;
mod overlay;

use app::WebApp;
use backend::CanvasBackend;
use overlay::DomOverlay;
use ratride::markdown::parse_frontmatter;
use ratride::theme;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent};

#[wasm_bindgen]
pub struct RatRide {
    #[allow(dead_code)]
    app: Rc<RefCell<WebApp>>,
}

#[wasm_bindgen]
impl RatRide {
    /// Create and start the slide presenter.
    ///
    /// - `md`: markdown source text
    /// - `canvas_id`: canvas element id (defaults to "ratride")
    /// - `theme_name`: theme name (defaults to "mocha")
    /// - `font_size`: font size in px (defaults to 16)
    #[wasm_bindgen]
    pub fn run(
        md: &str,
        canvas_id: Option<String>,
        theme_name: Option<String>,
        font_size: Option<f64>,
    ) -> RatRide {
        console_error_panic_hook::set_once();

        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");

        let id = canvas_id.as_deref().unwrap_or("ratride");
        let canvas: HtmlCanvasElement = document
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("no canvas#{id}"))
            .dyn_into()
            .expect("not a canvas");

        let (frontmatter, body) = parse_frontmatter(md);
        let resolved_theme = theme_name
            .as_deref()
            .and_then(theme::theme_from_name)
            .or_else(|| {
                frontmatter
                    .theme
                    .as_deref()
                    .and_then(theme::theme_from_name)
            })
            .unwrap_or_default();

        let fs = font_size.unwrap_or(16.0);
        let backend = CanvasBackend::new(canvas.clone(), fs);
        let overlay = DomOverlay::new(&format!("{id}-overlay"));
        let mut web_app = WebApp::new(backend, body, resolved_theme, &frontmatter, overlay);
        web_app.init();

        let app = Rc::new(RefCell::new(web_app));

        // Key event listener
        {
            let app = Rc::clone(&app);
            let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
                let key = event.key();
                app.borrow_mut().handle_key(&key);
            });
            document
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
                .expect("add keydown listener");
            closure.forget();
        }

        // requestAnimationFrame loop
        {
            let app = Rc::clone(&app);
            let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
            let g = Rc::clone(&f);

            *g.borrow_mut() = Some(Closure::new(move |timestamp: f64| {
                app.borrow_mut().tick(timestamp);

                // Schedule next frame
                let window = web_sys::window().unwrap();
                window
                    .request_animation_frame(
                        f.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                    )
                    .expect("rAF");
            }));

            let window = web_sys::window().unwrap();
            window
                .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .expect("initial rAF");
        }

        RatRide { app }
    }
}
