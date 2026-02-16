use ratatui::style::Color;

#[derive(Clone, Debug)]
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub h1: Color,
    pub h2: Color,
    pub h3: Color,
    pub h4: Color,
    pub inline_code_fg: Color,
    pub surface: Color,
    pub block_quote_prefix: Color,
    pub list_bullet: Color,
    pub status_fg: Color,
    pub status_bg: Color,
}

fn hex(s: &str) -> Color {
    let r = u8::from_str_radix(&s[0..2], 16).unwrap();
    let g = u8::from_str_radix(&s[2..4], 16).unwrap();
    let b = u8::from_str_radix(&s[4..6], 16).unwrap();
    Color::Rgb(r, g, b)
}

impl Theme {
    pub fn catppuccin_mocha() -> Self {
        Self {
            fg: hex("cdd6f4"),
            bg: hex("1e1e2e"),
            h1: hex("94e2d5"),
            h2: hex("cba6f7"),
            h3: hex("89b4fa"),
            h4: hex("f38ba8"),
            inline_code_fg: hex("a6e3a1"),
            surface: hex("313244"),
            block_quote_prefix: hex("f9e2af"),
            list_bullet: hex("6c7086"),
            status_fg: hex("cdd6f4"),
            status_bg: hex("313244"),
        }
    }

    pub fn catppuccin_macchiato() -> Self {
        Self {
            fg: hex("cad3f5"),
            bg: hex("24273a"),
            h1: hex("8bd5ca"),
            h2: hex("c6a0f6"),
            h3: hex("8aadf4"),
            h4: hex("ed8796"),
            inline_code_fg: hex("a6da95"),
            surface: hex("363a4f"),
            block_quote_prefix: hex("eed49f"),
            list_bullet: hex("6e738d"),
            status_fg: hex("cad3f5"),
            status_bg: hex("363a4f"),
        }
    }

    pub fn catppuccin_frappe() -> Self {
        Self {
            fg: hex("c6d0f5"),
            bg: hex("303446"),
            h1: hex("81c8be"),
            h2: hex("ca9ee6"),
            h3: hex("8caaee"),
            h4: hex("e78284"),
            inline_code_fg: hex("a6d189"),
            surface: hex("414559"),
            block_quote_prefix: hex("e5c890"),
            list_bullet: hex("737994"),
            status_fg: hex("c6d0f5"),
            status_bg: hex("414559"),
        }
    }

    pub fn catppuccin_latte() -> Self {
        Self {
            fg: hex("4c4f69"),
            bg: hex("eff1f5"),
            h1: hex("179299"),
            h2: hex("8839ef"),
            h3: hex("1e66f5"),
            h4: hex("d20f39"),
            inline_code_fg: hex("40a02b"),
            surface: hex("ccd0da"),
            block_quote_prefix: hex("df8e1d"),
            list_bullet: hex("9ca0b0"),
            status_fg: hex("4c4f69"),
            status_bg: hex("ccd0da"),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

impl Theme {
    pub fn syntect_theme(&self) -> syntect::highlighting::Theme {
        let bytes: &[u8] = match (self.bg, self.fg) {
            // Match by bg color to identify which Catppuccin flavor
            (ratatui::style::Color::Rgb(0x1e, 0x1e, 0x2e), _) => {
                include_bytes!("../themes/Catppuccin Mocha.tmTheme")
            }
            (ratatui::style::Color::Rgb(0x24, 0x27, 0x3a), _) => {
                include_bytes!("../themes/Catppuccin Macchiato.tmTheme")
            }
            (ratatui::style::Color::Rgb(0x30, 0x34, 0x46), _) => {
                include_bytes!("../themes/Catppuccin Frappe.tmTheme")
            }
            (ratatui::style::Color::Rgb(0xef, 0xf1, 0xf5), _) => {
                include_bytes!("../themes/Catppuccin Latte.tmTheme")
            }
            _ => include_bytes!("../themes/Catppuccin Mocha.tmTheme"),
        };
        let cursor = std::io::Cursor::new(bytes);
        syntect::highlighting::ThemeSet::load_from_reader(&mut std::io::BufReader::new(cursor))
            .expect("valid tmTheme")
    }
}

/// Resolve a theme name to a Theme.
/// Accepts both "catppuccin-mocha" and "mocha" forms.
pub fn theme_from_name(name: &str) -> Option<Theme> {
    let normalized = name.trim().to_lowercase();
    let short = normalized
        .strip_prefix("catppuccin-")
        .unwrap_or(&normalized);
    match short {
        "mocha" => Some(Theme::catppuccin_mocha()),
        "macchiato" => Some(Theme::catppuccin_macchiato()),
        "frappe" | "frappé" => Some(Theme::catppuccin_frappe()),
        "latte" => Some(Theme::catppuccin_latte()),
        _ => None,
    }
}

