//! Built-in FIGlet font renderer.
//!
//! Parses .flf (FIGlet font) files and renders text as ASCII art.
//! Bundled fonts are embedded at compile time via `include_str!`.

const ANSI_SHADOW_FLF: &str = include_str!("../fonts/ANSI Shadow.flf");

/// A parsed FIGlet font.
struct FigFont {
    height: usize,
    hardblank: char,
    /// Characters indexed by `(ascii_code - 32)`.
    chars: Vec<Vec<String>>,
}

impl FigFont {
    fn parse(input: &str) -> Option<Self> {
        let mut lines = input.lines();
        let header = lines.next()?;

        // Header: "flf2a<hardblank> height baseline max_length old_layout comment_lines ..."
        if !header.starts_with("flf2a") {
            return None;
        }
        let hardblank = header.chars().nth(5)?;
        let params: Vec<&str> = header[6..].split_whitespace().collect();
        if params.len() < 5 {
            return None;
        }
        let height: usize = params[0].parse().ok()?;
        let comment_lines: usize = params[4].parse().ok()?;

        // Skip comment lines
        for _ in 0..comment_lines {
            lines.next();
        }

        // Parse characters starting from ASCII 32 (space)
        let mut chars: Vec<Vec<String>> = Vec::new();
        let mut current_char: Vec<String> = Vec::new();

        for line in lines {
            let is_last = line.ends_with("@@");
            // Strip trailing @/@@
            let stripped = line.trim_end_matches('@');
            current_char.push(stripped.to_string());

            if is_last || current_char.len() == height {
                // Pad to height if needed
                while current_char.len() < height {
                    current_char.push(String::new());
                }
                chars.push(current_char);
                current_char = Vec::new();
            }
        }

        Some(FigFont {
            height,
            hardblank,
            chars,
        })
    }

    fn render(&self, text: &str) -> String {
        let mut rows: Vec<String> = vec![String::new(); self.height];

        for ch in text.chars() {
            let idx = ch as usize;
            if idx < 32 || idx - 32 >= self.chars.len() {
                for row in &mut rows {
                    row.push(' ');
                }
                continue;
            }
            let glyph = &self.chars[idx - 32];
            // Find max width (in chars) across all lines of this glyph
            let max_width = glyph.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            for (i, row) in rows.iter_mut().enumerate() {
                if i < glyph.len() {
                    let line = glyph[i].replace(self.hardblank, " ");
                    let char_count = line.chars().count();
                    row.push_str(&line);
                    // Pad to max width so all rows align
                    for _ in char_count..max_width {
                        row.push(' ');
                    }
                }
            }
        }

        // Ensure all rows have the same width, then trim trailing empty lines
        let max_len = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let mut result: Vec<String> = rows
            .iter()
            .map(|r| {
                let pad = max_len - r.chars().count();
                let mut s = r.clone();
                for _ in 0..pad {
                    s.push(' ');
                }
                s
            })
            .collect();
        // Drop trailing blank rows
        while result.last().is_some_and(|r| r.trim().is_empty()) {
            result.pop();
        }
        result.join("\n")
    }
}

/// Render text using a built-in font. Returns `None` if the font is not bundled.
pub fn render_builtin(text: &str, font: Option<&str>) -> Option<String> {
    let font_name = font.unwrap_or("");
    let flf = match font_name {
        "ANSI Shadow" | "ansi_shadow" | "ansi-shadow" => ANSI_SHADOW_FLF,
        _ => return None,
    };
    let fig = FigFont::parse(flf)?;
    Some(fig.render(text))
}

/// List of built-in font names.
pub fn builtin_fonts() -> &'static [&'static str] {
    &["ANSI Shadow"]
}
