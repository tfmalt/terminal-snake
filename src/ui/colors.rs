use ratatui::style::Color;

/// Converts a ratatui `Color` to its (R, G, B) components.
///
/// Named ANSI colors are mapped to standard terminal RGB approximations.
/// Indexed xterm-256 colors are resolved via `xterm_index_to_rgb`.
/// Returns `None` for `Color::Reset`.
pub fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    let rgb = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (229, 229, 229),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
        Color::Indexed(index) => xterm_index_to_rgb(index),
        Color::Reset => return None,
    };

    Some(rgb)
}

/// Cubic ease-out curve: fast start, gentle deceleration.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Linearly interpolates a single `u8` channel.
pub fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    (f32::from(from) + (f32::from(to) - f32::from(from)) * t).round() as u8
}

/// Linearly interpolates between two colors at factor `t` (0.0–1.0).
///
/// Both colors must be convertible to RGB via [`color_to_rgb`]; otherwise
/// `from` is returned unchanged (or `to` when `t >= 1.0`).
pub fn blend_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (color_to_rgb(from), color_to_rgb(to)) {
        (Some((fr, fg, fb)), Some((tr, tg, tb))) => {
            Color::Rgb(lerp_u8(fr, tr, t), lerp_u8(fg, tg, t), lerp_u8(fb, tb, t))
        }
        _ => {
            if t < 1.0 {
                from
            } else {
                to
            }
        }
    }
}

/// Brightens an RGB color by blending each channel toward 255.
///
/// `amount` is clamped to 0.0–1.0; 0.0 returns the original, 1.0 returns white.
/// Named (non-RGB) colors are returned unchanged.
pub fn brighten_color(color: Color, amount: f32) -> Color {
    let Some((r, g, b)) = color_to_rgb(color) else {
        return color;
    };

    let amount = amount.clamp(0.0, 1.0);
    let brighten_channel = |channel: u8| -> u8 {
        let remaining = 255.0 - f32::from(channel);
        (f32::from(channel) + (remaining * amount)).round() as u8
    };

    Color::Rgb(
        brighten_channel(r),
        brighten_channel(g),
        brighten_channel(b),
    )
}

/// Makes an RGB color appear redder by reducing green and blue channels.
///
/// `factor` is applied to green and blue (e.g. 0.9 = 10% reduction).
/// Named colors are returned unchanged.
pub fn redden_color(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r,
            (f32::from(g) * factor).round() as u8,
            (f32::from(b) * factor).round() as u8,
        ),
        other => other,
    }
}

/// Resolves an xterm-256 indexed color to its RGB components.
fn xterm_index_to_rgb(index: u8) -> (u8, u8, u8) {
    if index < 16 {
        const ANSI16: [(u8, u8, u8); 16] = [
            (0, 0, 0),
            (128, 0, 0),
            (0, 128, 0),
            (128, 128, 0),
            (0, 0, 128),
            (128, 0, 128),
            (0, 128, 128),
            (192, 192, 192),
            (128, 128, 128),
            (255, 0, 0),
            (0, 255, 0),
            (255, 255, 0),
            (0, 0, 255),
            (255, 0, 255),
            (0, 255, 255),
            (255, 255, 255),
        ];
        return ANSI16[index as usize];
    }

    if index <= 231 {
        let i = index - 16;
        let r = i / 36;
        let g = (i % 36) / 6;
        let b = i % 6;

        let level = |v: u8| -> u8 { if v == 0 { 0 } else { 55 + v * 40 } };

        return (level(r), level(g), level(b));
    }

    let gray = 8 + (index - 232) * 10;
    (gray, gray, gray)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_to_rgb_named_colors() {
        assert_eq!(color_to_rgb(Color::Black), Some((0, 0, 0)));
        assert_eq!(color_to_rgb(Color::White), Some((255, 255, 255)));
        assert_eq!(color_to_rgb(Color::Red), Some((205, 49, 49)));
    }

    #[test]
    fn color_to_rgb_direct_rgb() {
        assert_eq!(color_to_rgb(Color::Rgb(10, 20, 30)), Some((10, 20, 30)));
    }

    #[test]
    fn color_to_rgb_reset_returns_none() {
        assert_eq!(color_to_rgb(Color::Reset), None);
    }

    #[test]
    fn ease_out_cubic_boundaries() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ease_out_cubic_clamps() {
        assert!((ease_out_cubic(-0.5) - 0.0).abs() < f32::EPSILON);
        assert!((ease_out_cubic(1.5) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ease_out_cubic_midpoint_above_linear() {
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn blend_color_endpoints() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(255, 255, 255);
        assert_eq!(blend_color(a, b, 0.0), a);
        assert_eq!(blend_color(a, b, 1.0), b);
    }

    #[test]
    fn blend_color_midpoint() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(200, 100, 50);
        let mid = blend_color(a, b, 0.5);
        assert_eq!(mid, Color::Rgb(100, 50, 25));
    }

    #[test]
    fn brighten_zero_is_identity() {
        let c = Color::Rgb(100, 50, 25);
        assert_eq!(brighten_color(c, 0.0), c);
    }

    #[test]
    fn brighten_full_is_white() {
        let c = Color::Rgb(100, 50, 25);
        assert_eq!(brighten_color(c, 1.0), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn redden_preserves_red_channel() {
        let c = Color::Rgb(200, 100, 80);
        let r = redden_color(c, 0.5);
        assert_eq!(r, Color::Rgb(200, 50, 40));
    }

    #[test]
    fn xterm_indexed_color_resolves() {
        // Index 0 = black
        assert_eq!(color_to_rgb(Color::Indexed(0)), Some((0, 0, 0)));
        // Index 15 = white
        assert_eq!(color_to_rgb(Color::Indexed(15)), Some((255, 255, 255)));
    }
}
