use ratatui::{
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
    style::{Color, Modifier},
};
use std::io;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub struct CanvasBackend {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    cols: u16,
    rows: u16,
    cell_width: f64,
    cell_height: f64,
    font_size: f64,
}

impl CanvasBackend {
    pub fn new(canvas: HtmlCanvasElement, font_size: f64) -> Self {
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();

        let font = format!("{font_size}px monospace");
        ctx.set_font(&font);
        let metrics = ctx.measure_text("W").unwrap();
        let cell_width = metrics.width();
        let cell_height = font_size * 1.2;

        let canvas_w = canvas.width() as f64;
        let canvas_h = canvas.height() as f64;
        let cols = (canvas_w / cell_width) as u16;
        let rows = (canvas_h / cell_height) as u16;

        Self {
            canvas,
            ctx,
            cols,
            rows,
            cell_width,
            cell_height,
            font_size,
        }
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn resize(&mut self) {
        let canvas_w = self.canvas.width() as f64;
        let canvas_h = self.canvas.height() as f64;
        self.cols = (canvas_w / self.cell_width) as u16;
        self.rows = (canvas_h / self.cell_height) as u16;

        // Re-set font after resize (canvas resets context state)
        let font = format!("{}px monospace", self.font_size);
        self.ctx.set_font(&font);
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
