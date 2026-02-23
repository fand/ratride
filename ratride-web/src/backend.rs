use ratride::render::ImagePlacement;
use ratatui::{
    backend::{Backend, ClearType, WindowSize},
    buffer::{Buffer, Cell},
    layout::{Position, Size},
    style::{Color, Modifier},
};
use std::io;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

pub struct CanvasBackend {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    cols: u16,
    rows: u16,
    cell_width: f64,
    cell_height: f64,
    font_size: f64,
    dpr: f64,
}

impl CanvasBackend {
    pub fn new(canvas: HtmlCanvasElement, font_size: f64) -> Self {
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();

        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);

        // Scale context for high-DPI displays
        let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);

        // Measure cell size in CSS pixels (after DPR scaling is applied)
        let scaled_font_size = font_size;
        let font = format!("{scaled_font_size}px monospace");
        ctx.set_font(&font);
        let metrics = ctx.measure_text("W").unwrap();
        let cell_width = metrics.width();
        let cell_height = scaled_font_size * 1.2;

        // Canvas size is in physical pixels; grid is in CSS pixels
        let css_w = canvas.width() as f64 / dpr;
        let css_h = canvas.height() as f64 / dpr;
        let cols = (css_w / cell_width) as u16;
        let rows = (css_h / cell_height) as u16;

        Self {
            canvas,
            ctx,
            cols,
            rows,
            cell_width,
            cell_height,
            font_size,
            dpr,
        }
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cell_width(&self) -> f64 {
        self.cell_width
    }

    pub fn cell_height(&self) -> f64 {
        self.cell_height
    }

    pub fn resize(&mut self) {
        let css_w = self.canvas.width() as f64 / self.dpr;
        let css_h = self.canvas.height() as f64 / self.dpr;
        self.cols = (css_w / self.cell_width) as u16;
        self.rows = (css_h / self.cell_height) as u16;

        // Re-apply DPR scale + font (setTransform is absolute, won't compound)
        let _ = self.ctx.set_transform(self.dpr, 0.0, 0.0, self.dpr, 0.0, 0.0);
        let font = format!("{}px monospace", self.font_size);
        self.ctx.set_font(&font);
    }

    /// Clear a rectangular region on the canvas (cell coordinates).
    pub fn clear_cell_rect(&self, x: u16, y: u16, w: u16, h: u16) {
        let px = x as f64 * self.cell_width;
        let py = y as f64 * self.cell_height;
        let pw = w as f64 * self.cell_width;
        let ph = h as f64 * self.cell_height;
        self.ctx.clear_rect(px, py, pw, ph);
    }

    /// Redraw cells from a buffer for a rectangular region.
    pub fn redraw_region(&mut self, buf: &Buffer, x: u16, y: u16, w: u16, h: u16) {
        let cells: Vec<_> = (y..y + h)
            .flat_map(|row| {
                (x..x + w).filter_map(move |col| {
                    buf.cell(Position::new(col, row)).map(|c| (col, row, c))
                })
            })
            .collect();
        let _ = <Self as Backend>::draw(self, cells.into_iter());
    }

    /// Draw an image on the canvas with optional clipping when partially off-screen.
    pub fn draw_image(&self, img: &HtmlImageElement, placement: &ImagePlacement) {
        if !img.complete() || img.natural_width() == 0 {
            return;
        }
        let nat_w = img.natural_width() as f64;
        let nat_h = img.natural_height() as f64;

        let box_px = placement.x as f64 * self.cell_width;
        let box_py = placement.y as f64 * self.cell_height;
        let box_pw = placement.width as f64 * self.cell_width;
        let full_ph = placement.full_height as f64 * self.cell_height;
        let vis_ph = placement.height as f64 * self.cell_height;

        // Scale to fit within the FULL box (width × full_height)
        let scale = (box_pw / nat_w).min(full_ph / nat_h);
        let draw_w = nat_w * scale;
        let draw_h = nat_h * scale;

        if placement.full_height > placement.height {
            // Image partially off-screen: crop via source rect
            let center_x = (box_pw - draw_w) / 2.0;
            let center_y = (full_ph - draw_h) / 2.0;

            // Visible window within the full box
            let vis_y0 = if placement.clip_top {
                (placement.full_height - placement.height) as f64 * self.cell_height
            } else {
                0.0
            };
            let vis_y1 = vis_y0 + vis_ph;

            // Image rect within full box
            let img_y0 = center_y;
            let img_y1 = center_y + draw_h;

            // Intersection
            let int_y0 = vis_y0.max(img_y0);
            let int_y1 = vis_y1.min(img_y1);
            if int_y1 <= int_y0 {
                return;
            }

            // Source crop in original image pixels
            let src_y = (int_y0 - img_y0) / draw_h * nat_h;
            let src_h = (int_y1 - int_y0) / draw_h * nat_h;

            // Destination on canvas
            let dst_x = box_px + center_x;
            let dst_y = box_py + (int_y0 - vis_y0);
            let dst_w = draw_w;
            let dst_h = int_y1 - int_y0;

            let _ = self.ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0.0, src_y, nat_w, src_h, dst_x, dst_y, dst_w, dst_h,
            );
        } else {
            // Normal: fit and center within the visible box
            let center_x = box_px + (box_pw - draw_w) / 2.0;
            let center_y = box_py + (vis_ph - draw_h) / 2.0;

            let _ = self
                .ctx
                .draw_image_with_html_image_element_and_dw_and_dh(img, center_x, center_y, draw_w, draw_h);
        }
    }

    fn color_to_css(color: Color, fallback: &str) -> String {
        match color {
            Color::Rgb(r, g, b) => format!("rgb({r},{g},{b})"),
            Color::Black => "#000000".into(),
            Color::White => "#ffffff".into(),
            Color::Red => "#ff0000".into(),
            Color::Green => "#00ff00".into(),
            Color::Blue => "#0000ff".into(),
            Color::Yellow => "#ffff00".into(),
            Color::Cyan => "#00ffff".into(),
            Color::Magenta => "#ff00ff".into(),
            Color::Gray => "#808080".into(),
            Color::DarkGray => "#404040".into(),
            Color::LightRed => "#ff8080".into(),
            Color::LightGreen => "#80ff80".into(),
            Color::LightBlue => "#8080ff".into(),
            Color::LightYellow => "#ffff80".into(),
            Color::LightCyan => "#80ffff".into(),
            Color::LightMagenta => "#ff80ff".into(),
            _ => fallback.into(),
        }
    }
}

impl Backend for CanvasBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let cw = self.cell_width;
        let ch = self.cell_height;
        let baseline_offset = self.font_size * 0.85;

        for (x, y, cell) in content {
            let px = x as f64 * cw;
            let py = y as f64 * ch;

            // Draw background
            let bg_css = Self::color_to_css(cell.bg, "transparent");
            if bg_css != "transparent" {
                self.ctx.set_fill_style_str(&bg_css);
                self.ctx.fill_rect(px, py, cw, ch);
            } else {
                self.ctx.clear_rect(px, py, cw, ch);
            }

            // Draw character
            let symbol = cell.symbol();
            if !symbol.is_empty() && symbol != " " {
                let mods = cell.modifier;
                let bold = mods.contains(Modifier::BOLD);
                let italic = mods.contains(Modifier::ITALIC);
                let font = match (bold, italic) {
                    (true, true) => format!("bold italic {}px monospace", self.font_size),
                    (true, false) => format!("bold {}px monospace", self.font_size),
                    (false, true) => format!("italic {}px monospace", self.font_size),
                    (false, false) => format!("{}px monospace", self.font_size),
                };
                self.ctx.set_font(&font);

                let fg_css = Self::color_to_css(cell.fg, "#cccccc");
                self.ctx.set_fill_style_str(&fg_css);
                let _ = self.ctx.fill_text(symbol, px, py + baseline_offset);

                // Underline
                if mods.contains(Modifier::UNDERLINED) {
                    self.ctx.set_stroke_style_str(&fg_css);
                    self.ctx.begin_path();
                    self.ctx.move_to(px, py + ch - 1.0);
                    self.ctx.line_to(px + cw, py + ch - 1.0);
                    self.ctx.stroke();
                }

                // Strikethrough
                if mods.contains(Modifier::CROSSED_OUT) {
                    self.ctx.set_stroke_style_str(&fg_css);
                    self.ctx.begin_path();
                    self.ctx.move_to(px, py + ch / 2.0);
                    self.ctx.line_to(px + cw, py + ch / 2.0);
                    self.ctx.stroke();
                }

                // Reset font if modified
                if bold || italic {
                    let base_font = format!("{}px monospace", self.font_size);
                    self.ctx.set_font(&base_font);
                }
            }
        }

        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        Ok(Position::new(0, 0))
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, _pos: P) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        self.ctx.clear_rect(0.0, 0.0, w, h);
        Ok(())
    }

    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
        match clear_type {
            ClearType::All => self.clear(),
            _ => Ok(()),
        }
    }

    fn size(&self) -> Result<Size, Self::Error> {
        Ok(Size::new(self.cols, self.rows))
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        Ok(WindowSize {
            columns_rows: Size::new(self.cols, self.rows),
            pixels: Size::new(self.canvas.width() as u16, self.canvas.height() as u16),
        })
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
