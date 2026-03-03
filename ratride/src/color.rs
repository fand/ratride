use ratatui::style::Color;

/// Linearly blend two colors. At t=0 returns `a`, at t=1 returns `b`.
/// Non-RGB colors (e.g. Color::Reset) are returned as-is to avoid
/// introducing explicit background colors where the terminal default is used.
pub fn blend_color(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let inv = 1.0 - t;
            Color::Rgb(
                (ar as f32 * inv + br as f32 * t) as u8,
                (ag as f32 * inv + bg as f32 * t) as u8,
                (ab as f32 * inv + bb as f32 * t) as u8,
            )
        }
        _ => b,
    }
}

/// Convert a hue (0-360) to an RGB color (full saturation & value).
pub fn hue_to_rgb(hue: f32) -> Color {
    let h = (hue % 360.0) / 60.0;
    let i = h.floor() as u8;
    let f = h - h.floor();
    let q = (255.0 * (1.0 - f)) as u8;
    let t = (255.0 * f) as u8;
    match i {
        0 => Color::Rgb(255, t, 0),
        1 => Color::Rgb(q, 255, 0),
        2 => Color::Rgb(0, 255, t),
        3 => Color::Rgb(0, q, 255),
        4 => Color::Rgb(t, 0, 255),
        _ => Color::Rgb(255, 0, q),
    }
}

/// Animated color gradient: blue → cyan → magenta → white → red.
pub fn anim_color(progress: f32) -> Color {
    let lerp_rgb = |a: (u8, u8, u8), b: (u8, u8, u8), t: f32| -> Color {
        let inv = 1.0 - t;
        Color::Rgb(
            (a.0 as f32 * inv + b.0 as f32 * t) as u8,
            (a.1 as f32 * inv + b.1 as f32 * t) as u8,
            (a.2 as f32 * inv + b.2 as f32 * t) as u8,
        )
    };
    let blue = (80, 80, 255);
    let cyan = (100, 255, 255);
    let white = (255, 255, 255);
    let red = (255, 100, 100);

    if progress < 0.8 {
        lerp_rgb(blue, cyan, progress * 1.25)
    } else if progress < 0.9 {
        lerp_rgb(cyan, red, progress * 10.0 - 8.0)
    } else {
        lerp_rgb(red, white, progress * 10.0 - 9.0)
    }
}
