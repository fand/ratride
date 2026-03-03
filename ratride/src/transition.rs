use ratatui::buffer::Buffer;
use ratatui::style::Color;
use tachyonfx::{Effect, Interpolation, Motion, fx};

use crate::color::{anim_color, blend_color, hue_to_rgb};
use crate::markdown::TransitionKind;

pub fn create_transition(
    kind: &TransitionKind,
    bg: Color,
    prev_buf: Option<Buffer>,
    rows: u16,
    content_lines: usize,
    line_dur_ms: f32,
    stagger_ms: f32,
) -> Option<Effect> {
    Some(match kind {
        TransitionKind::None => return None,
        TransitionKind::SlideIn => fx::fade_from_fg(bg, (400, Interpolation::QuadOut)),
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
        TransitionKind::Lines => {
            let prev = prev_buf.clone();
            let approx_lines = rows as f32;
            let duration_ms = line_dur_ms + stagger_ms * (approx_lines - 1.0).max(0.0);
            fx::effect_fn_buf(
                (),
                (duration_ms as u32, Interpolation::Linear),
                move |_state, ctx, buf| {
                    let elapsed = ctx.alpha() * duration_ms;
                    let area = ctx.area;
                    let width = area.width;

                    for y in area.y..area.y + area.height {
                        let line_index = (y - area.y) as f32;
                        let line_start = line_index * stagger_ms;
                        let local_alpha =
                            ((elapsed - line_start) / line_dur_ms).clamp(0.0, 1.0);
                        let local_alpha = Interpolation::QuadOut.alpha(local_alpha);
                        let shift = ((1.0 - local_alpha) * width as f32) as u16;

                        let original: Vec<_> = (area.x..area.x + width)
                            .map(|x| buf[(x, y)].clone())
                            .collect();

                        for x in area.x..area.x + width {
                            let col = x - area.x;
                            let src_col = col + shift;
                            let cell = &mut buf[(x, y)];
                            if src_col < width {
                                *cell = original[src_col as usize].clone();
                            } else {
                                let d = (src_col - width) as f32;
                                let fade = (d * 2.0 / width as f32).clamp(0.0, 1.0);
                                if fade > 0.0 {
                                    if let Some(old) =
                                        prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                    {
                                        cell.set_char(
                                            old.symbol().chars().next().unwrap_or(' '),
                                        );
                                        cell.set_fg(blend_color(bg, old.fg, fade));
                                        cell.set_bg(blend_color(bg, old.bg, fade));
                                    }
                                } else {
                                    cell.reset();
                                }
                            }
                        }
                    }
                },
            )
        }
        TransitionKind::LinesCross => {
            let prev = prev_buf.clone();
            let approx_lines = rows as f32;
            let duration_ms = line_dur_ms + stagger_ms * (approx_lines - 1.0).max(0.0);
            fx::effect_fn_buf(
                (),
                (duration_ms as u32, Interpolation::Linear),
                move |_state, ctx, buf| {
                    let elapsed = ctx.alpha() * duration_ms;
                    let area = ctx.area;
                    let width = area.width as f32;

                    for y in area.y..area.y + area.height {
                        let line_index = (y - area.y) as f32;
                        let line_start = line_index * stagger_ms;
                        let local_alpha =
                            ((elapsed - line_start) / line_dur_ms).clamp(0.0, 1.0);
                        let local_alpha = Interpolation::QuadOut.alpha(local_alpha);
                        let visible_cols = (local_alpha * width) as u16;
                        let is_odd = (y - area.y) % 2 == 1;

                        for x in area.x..area.x + area.width {
                            let col_offset = x - area.x;
                            let should_blank = if is_odd {
                                col_offset < area.width - visible_cols
                            } else {
                                col_offset >= visible_cols
                            };
                            if should_blank {
                                let cell = &mut buf[(x, y)];
                                let d = if is_odd {
                                    (area.width - visible_cols - 1 - col_offset) as f32
                                } else {
                                    (col_offset - visible_cols) as f32
                                };
                                let fade = (d * 2.0 / area.width as f32).clamp(0.0, 1.0);
                                if fade > 0.0 {
                                    if let Some(old) =
                                        prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                    {
                                        cell.set_char(
                                            old.symbol().chars().next().unwrap_or(' '),
                                        );
                                        cell.set_fg(blend_color(bg, old.fg, fade));
                                        cell.set_bg(blend_color(bg, old.bg, fade));
                                    }
                                } else {
                                    cell.reset();
                                }
                            }
                        }
                    }
                },
            )
        }
        TransitionKind::LinesRgb => {
            let prev = prev_buf.clone();
            let cl = content_lines as f32;
            let duration_ms = line_dur_ms + stagger_ms * (cl - 1.0).max(0.0);
            fx::effect_fn_buf(
                (false, 0u16, 0u16),
                (duration_ms as u32, Interpolation::Linear),
                move |state, ctx, buf| {
                    let area = ctx.area;
                    let width = area.width;

                    if !state.0 {
                        state.0 = true;
                        let mut first: u16 = area.y + area.height;
                        let mut last: u16 = area.y;
                        for y in area.y..area.y + area.height {
                            for x in area.x..area.x + width {
                                let sym = buf[(x, y)].symbol().chars().next().unwrap_or(' ');
                                if sym != ' ' {
                                    if y < first { first = y; }
                                    if y > last { last = y; }
                                    break;
                                }
                            }
                        }
                        if first > last {
                            first = area.y;
                            last = area.y + area.height - 1;
                        }
                        state.1 = first;
                        state.2 = last;
                    }

                    let first_content = state.1;
                    let last_content = state.2;
                    let elapsed = ctx.alpha() * duration_ms;
                    let global_fade = 1.0 - ctx.alpha();

                    for y in area.y..area.y + area.height {
                        if y < first_content || y > last_content {
                            continue;
                        }

                        let line_index = (y - first_content) as f32;
                        let line_start = line_index * stagger_ms;
                        let local_alpha =
                            ((elapsed - line_start) / line_dur_ms).clamp(0.0, 1.0);
                        let local_alpha = Interpolation::QuadOut.alpha(local_alpha);
                        let shift = ((1.0 - local_alpha) * width as f32) as u16;

                        let original: Vec<_> = (area.x..area.x + width)
                            .map(|x| buf[(x, y)].clone())
                            .collect();

                        let color = anim_color(local_alpha);

                        for x in area.x..area.x + width {
                            let col = x - area.x;
                            let cell = &mut buf[(x, y)];

                            let (in_range, src_col) = {
                                let sc = col + shift;
                                if sc < width { (true, sc) } else { (false, 0) }
                            };

                            if in_range {
                                *cell = original[src_col as usize].clone();
                                if local_alpha < 1.0 {
                                    cell.set_fg(color);
                                }
                            } else {
                                if let Some(old) =
                                    prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                {
                                    cell.set_char(
                                        old.symbol().chars().next().unwrap_or(' '),
                                    );
                                    cell.set_fg(blend_color(bg, old.fg, global_fade));
                                    cell.set_bg(blend_color(bg, old.bg, global_fade));
                                } else {
                                    cell.reset();
                                }
                            }
                        }
                    }
                },
            )
        }
        TransitionKind::SlideRgb => {
            let prev = prev_buf.clone();
            let band_width = 24_u16;
            fx::effect_fn_buf(
                (),
                (800, Interpolation::QuadOut),
                move |_state, ctx, buf| {
                    let alpha = ctx.alpha();
                    let area = ctx.area;
                    let width = area.width as f32;
                    let edge_col = (alpha * (width + band_width as f32)) as u16;

                    for y in area.y..area.y + area.height {
                        for x in area.x..area.x + area.width {
                            let col_offset = x - area.x;
                            if col_offset >= edge_col {
                                let cell = &mut buf[(x, y)];
                                if let Some(old) =
                                    prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                {
                                    *cell = old.clone();
                                }
                            } else if col_offset + band_width >= edge_col {
                                let d = edge_col - col_offset;
                                let t = d as f32 / band_width as f32;
                                let hue = t * 300.0;
                                let color = hue_to_rgb(hue);
                                let cell = &mut buf[(x, y)];
                                cell.set_fg(color);
                            }
                        }
                    }
                },
            )
        }
    })
}
