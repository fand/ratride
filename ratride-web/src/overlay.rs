use ratride::markdown::{HeaderItem, SemanticElement};
use ratatui::text::Text;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

pub struct DomOverlay {
    container: HtmlElement,
    document: Document,
}

impl DomOverlay {
    pub fn new(overlay_id: &str) -> Self {
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");
        let container: HtmlElement = document
            .get_element_by_id(overlay_id)
            .unwrap_or_else(|| panic!("no #{overlay_id}"))
            .dyn_into()
            .expect("not an HtmlElement");
        Self {
            container,
            document,
        }
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self
            .container
            .style()
            .set_property("display", if visible { "" } else { "none" });
    }

    pub fn update(
        &self,
        semantics: &[SemanticElement],
        scroll: u16,
        content_offset_x: f64,
        content_offset_y: f64,
        cell_width: f64,
        cell_height: f64,
        visible_rows: u16,
        is_center: bool,
        content: &Text<'_>,
        content_area_width: u16,
    ) {
        self.container.set_inner_html("");

        for elem in semantics {
            match elem {
                SemanticElement::Heading {
                    level,
                    text,
                    line_index,
                } => {
                    let y_offset = *line_index as i32 - scroll as i32;
                    if y_offset < 0 || y_offset >= visible_rows as i32 {
                        continue;
                    }
                    let tag = match level {
                        1 => "h1",
                        2 => "h2",
                        3 => "h3",
                        4 => "h4",
                        5 => "h5",
                        _ => "h6",
                    };
                    let el = self.document.create_element(tag).expect("create heading");
                    el.set_text_content(Some(text));
                    // sr-only: visually hidden but accessible to screen readers
                    let html_el = el.dyn_ref::<HtmlElement>().unwrap();
                    let s = html_el.style();
                    let _ = s.set_property("position", "absolute");
                    let _ = s.set_property("width", "1px");
                    let _ = s.set_property("height", "1px");
                    let _ = s.set_property("padding", "0");
                    let _ = s.set_property("margin", "-1px");
                    let _ = s.set_property("overflow", "hidden");
                    let _ = s.set_property("clip", "rect(0,0,0,0)");
                    let _ = s.set_property("white-space", "nowrap");
                    let _ = s.set_property("border", "0");
                    let _ = self.container.append_child(&el);
                }
                SemanticElement::Link {
                    url,
                    text,
                    line_index,
                    start_col,
                    end_col,
                } => {
                    let y_offset = *line_index as i32 - scroll as i32;
                    if y_offset < 0 || y_offset >= visible_rows as i32 {
                        continue;
                    }

                    // For center layout, add horizontal centering offset
                    let center_x = if is_center {
                        if let Some(line) = content.lines.get(*line_index) {
                            let line_w = line.width();
                            (content_area_width as usize).saturating_sub(line_w) as f64
                                / 2.0
                                * cell_width
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    let px_x =
                        content_offset_x + center_x + (*start_col as f64) * cell_width;
                    let px_y = content_offset_y + (y_offset as f64) * cell_height;
                    let px_w = ((*end_col - *start_col) as f64) * cell_width;
                    let px_h = cell_height;

                    let a = self.document.create_element("a").expect("create anchor");
                    let _ = a.set_attribute("href", url);
                    let _ = a.set_attribute("target", "_blank");
                    let _ = a.set_attribute("rel", "noopener noreferrer");
                    a.set_text_content(Some(text));
                    let style = format!(
                        "position:absolute;left:{px_x}px;top:{px_y}px;\
                         width:{px_w}px;height:{px_h}px;\
                         color:transparent;pointer-events:auto;cursor:pointer;\
                         text-decoration:none;font-size:0;display:block"
                    );
                    let _ = a.set_attribute("style", &style);
                    let _ = self.container.append_child(&a);
                }
            }
        }
    }

    /// Add clickable overlays for header links.
    /// `header_x` / `header_y` are the pixel coordinates of the header area origin.
    pub fn update_header_links(
        &self,
        header: &[HeaderItem],
        header_x: f64,
        header_y: f64,
        cell_width: f64,
        cell_height: f64,
        area_width: u16,
    ) {
        if header.is_empty() {
            return;
        }

        let separator = " │ ";
        // Compute total character width (mirrors render::draw_header)
        let mut total_chars: usize = 1; // leading padding " "
        for (i, item) in header.iter().enumerate() {
            if i > 0 {
                total_chars += separator.len();
            }
            total_chars += item.text.len();
        }
        total_chars += 1; // trailing padding " "

        // Header is right-aligned with 1-cell margin from right edge
        let start_x = area_width.saturating_sub(total_chars as u16 + 1);

        let mut col: usize = 1; // skip leading padding
        for (i, item) in header.iter().enumerate() {
            if i > 0 {
                col += separator.len();
            }
            if let Some(url) = &item.url {
                let px_x = header_x + (start_x as usize + col) as f64 * cell_width;
                let px_y = header_y;
                let px_w = item.text.len() as f64 * cell_width;
                let px_h = cell_height;

                let a = self.document.create_element("a").expect("create anchor");
                let _ = a.set_attribute("href", url);
                let _ = a.set_attribute("target", "_blank");
                let _ = a.set_attribute("rel", "noopener noreferrer");
                a.set_text_content(Some(&item.text));
                let style = format!(
                    "position:absolute;left:{px_x}px;top:{px_y}px;\
                     width:{px_w}px;height:{px_h}px;\
                     color:transparent;pointer-events:auto;cursor:pointer;\
                     text-decoration:none;font-size:0;display:block"
                );
                let _ = a.set_attribute("style", &style);
                let _ = self.container.append_child(&a);
            }
            col += item.text.len();
        }
    }
}
