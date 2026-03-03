//! Built-in FIGlet font renderer.
//!
//! Parses .flf (FIGlet font) files and renders text as ASCII art.
//! Bundled fonts are embedded at compile time via `include_str!`.

const ANSI_SHADOW_FLF: &str = include_str!("../fonts/ANSI Shadow.flf");
const STANDARD_FLF: &str = include_str!("../fonts/standard.flf");
const BIG_FLF: &str = include_str!("../fonts/big.flf");
const SMALL_FLF: &str = include_str!("../fonts/small.flf");
const MINI_FLF: &str = include_str!("../fonts/mini.flf");
const SLANT_FLF: &str = include_str!("../fonts/slant.flf");
const SMSLANT_FLF: &str = include_str!("../fonts/smslant.flf");
const BLOCK_FLF: &str = include_str!("../fonts/block.flf");
const DOOM_FLF: &str = include_str!("../fonts/doom.flf");
const EPIC_FLF: &str = include_str!("../fonts/epic.flf");
const GRAFFITI_FLF: &str = include_str!("../fonts/graffiti.flf");
const FRAKTUR_FLF: &str = include_str!("../fonts/fraktur.flf");
const ROMAN_FLF: &str = include_str!("../fonts/roman.flf");
const GOTHIC_FLF: &str = include_str!("../fonts/gothic.flf");
const SPEED_FLF: &str = include_str!("../fonts/speed.flf");
const SCRIPT_FLF: &str = include_str!("../fonts/script.flf");

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
    let font_name = font.unwrap_or("ANSI Shadow");
    let flf = match font_name {
        "ANSI Shadow" | "ansi_shadow" | "ansi-shadow" => ANSI_SHADOW_FLF,
        "standard" => STANDARD_FLF,
        "big" => BIG_FLF,
        "small" => SMALL_FLF,
        "mini" => MINI_FLF,
        "slant" => SLANT_FLF,
        "smslant" => SMSLANT_FLF,
        "block" => BLOCK_FLF,
        "doom" => DOOM_FLF,
        "epic" => EPIC_FLF,
        "graffiti" => GRAFFITI_FLF,
        "fraktur" => FRAKTUR_FLF,
        "roman" => ROMAN_FLF,
        "gothic" => GOTHIC_FLF,
        "speed" => SPEED_FLF,
        "script" => SCRIPT_FLF,
        _ => return None,
    };
    let fig = FigFont::parse(flf)?;
    Some(fig.render(text))
}

/// List of built-in font names.
pub fn builtin_fonts() -> &'static [&'static str] {
    &[
        "ANSI Shadow",
        "standard",
        "big",
        "small",
        "mini",
        "slant",
        "smslant",
        "block",
        "doom",
        "epic",
        "graffiti",
        "fraktur",
        "roman",
        "gothic",
        "speed",
        "script",
    ]
}

/// Render text using figrat with color. Returns ANSI-colored string.
///
/// `color_spec` is the color argument (e.g. `"ff0000,00ffff x"`).
pub fn render_figrat(text: &str, font: Option<&str>, color_spec: &str) -> Option<String> {
    let font_data = load_font_data(font)?;
    let fig = figrat::font::parser::FigFont::parse(&font_data).ok()?;
    let canvas = figrat::render::layout::render(&fig, text, None);

    let (gradient, direction) = parse_color_spec(color_spec);
    let rules = vec![figrat::color::colorize::ColorRule {
        chars: None,
        gradient,
        direction,
    }];
    let colored = figrat::color::colorize::colorize(&canvas, &rules);
    Some(figrat::output::ansi::render_ansi(
        &canvas,
        &colored,
        fig.header.hardblank,
    ))
}

/// Load .flf font data by name. Checks built-in fonts first, then system paths.
fn load_font_data(font: Option<&str>) -> Option<String> {
    let font_name = font.unwrap_or("ANSI Shadow");
    // Check built-in fonts first
    let builtin = match font_name {
        "ANSI Shadow" | "ansi_shadow" | "ansi-shadow" => Some(ANSI_SHADOW_FLF),
        "standard" => Some(STANDARD_FLF),
        "big" => Some(BIG_FLF),
        "small" => Some(SMALL_FLF),
        "mini" => Some(MINI_FLF),
        "slant" => Some(SLANT_FLF),
        "smslant" => Some(SMSLANT_FLF),
        "block" => Some(BLOCK_FLF),
        "doom" => Some(DOOM_FLF),
        "epic" => Some(EPIC_FLF),
        "graffiti" => Some(GRAFFITI_FLF),
        "fraktur" => Some(FRAKTUR_FLF),
        "roman" => Some(ROMAN_FLF),
        "gothic" => Some(GOTHIC_FLF),
        "speed" => Some(SPEED_FLF),
        "script" => Some(SCRIPT_FLF),
        _ => None,
    };
    if let Some(data) = builtin {
        return Some(data.to_string());
    }
    // Try common system figlet font directories
    for dir in &[
        "/usr/share/figlet",
        "/usr/share/figlet/fonts",
        "/usr/local/share/figlet",
        "/usr/local/share/figlet/fonts",
    ] {
        let path = format!("{}/{}.flf", dir, font_name);
        if let Ok(data) = std::fs::read_to_string(&path) {
            return Some(data);
        }
    }
    // Try the font name as a direct path
    std::fs::read_to_string(font_name).ok()
}

fn parse_hex(s: &str) -> figrat::color::palette::Rgba {
    let s = s.trim_start_matches('#');
    if s.len() < 6 {
        return figrat::color::palette::Rgba::rgb(255, 255, 255);
    }
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
    figrat::color::palette::Rgba::rgb(r, g, b)
}

fn parse_color_spec(
    s: &str,
) -> (
    figrat::color::gradient::Gradient,
    figrat::color::colorize::GradientDirection,
) {
    use figrat::color::colorize::GradientDirection;
    use figrat::color::gradient::Gradient;
    use figrat::color::palette::Rgba;

    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.is_empty() {
        return (
            Gradient::solid(Rgba::rgb(255, 255, 255)),
            GradientDirection::Horizontal,
        );
    }

    let (color_str, direction) = match tokens.last() {
        Some(&"y") => (
            tokens[..tokens.len() - 1].join(" "),
            GradientDirection::Vertical,
        ),
        Some(&"x") => (
            tokens[..tokens.len() - 1].join(" "),
            GradientDirection::Horizontal,
        ),
        _ => (tokens.join(" "), GradientDirection::Horizontal),
    };

    let colors: Vec<Rgba> = color_str.split(',').map(|c| parse_hex(c.trim())).collect();
    (Gradient::multi(&colors), direction)
}
