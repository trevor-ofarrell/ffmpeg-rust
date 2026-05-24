//! FFmpeg named color inventory.

use std::fmt::Write as _;

use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedColor {
    name: &'static str,
    rgb: [u8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaColor {
    rgba: [u8; 4],
}

impl RgbaColor {
    pub const fn from_rgba(rgba: [u8; 4]) -> Self {
        Self { rgba }
    }

    pub const fn from_rgb(rgb: [u8; 3]) -> Self {
        Self {
            rgba: [rgb[0], rgb[1], rgb[2], 0xFF],
        }
    }

    pub const fn rgba(self) -> [u8; 4] {
        self.rgba
    }

    pub const fn rgb(self) -> [u8; 3] {
        [self.rgba[0], self.rgba[1], self.rgba[2]]
    }

    pub const fn alpha(self) -> u8 {
        self.rgba[3]
    }

    pub fn rgba_hex_lower(self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3]
        )
    }
}

impl NamedColor {
    pub const ALL: &'static [NamedColor] = &[
        NamedColor {
            name: "AliceBlue",
            rgb: [0xF0, 0xF8, 0xFF],
        },
        NamedColor {
            name: "AntiqueWhite",
            rgb: [0xFA, 0xEB, 0xD7],
        },
        NamedColor {
            name: "Aqua",
            rgb: [0x00, 0xFF, 0xFF],
        },
        NamedColor {
            name: "Aquamarine",
            rgb: [0x7F, 0xFF, 0xD4],
        },
        NamedColor {
            name: "Azure",
            rgb: [0xF0, 0xFF, 0xFF],
        },
        NamedColor {
            name: "Beige",
            rgb: [0xF5, 0xF5, 0xDC],
        },
        NamedColor {
            name: "Bisque",
            rgb: [0xFF, 0xE4, 0xC4],
        },
        NamedColor {
            name: "Black",
            rgb: [0x00, 0x00, 0x00],
        },
        NamedColor {
            name: "BlanchedAlmond",
            rgb: [0xFF, 0xEB, 0xCD],
        },
        NamedColor {
            name: "Blue",
            rgb: [0x00, 0x00, 0xFF],
        },
        NamedColor {
            name: "BlueViolet",
            rgb: [0x8A, 0x2B, 0xE2],
        },
        NamedColor {
            name: "Brown",
            rgb: [0xA5, 0x2A, 0x2A],
        },
        NamedColor {
            name: "BurlyWood",
            rgb: [0xDE, 0xB8, 0x87],
        },
        NamedColor {
            name: "CadetBlue",
            rgb: [0x5F, 0x9E, 0xA0],
        },
        NamedColor {
            name: "Chartreuse",
            rgb: [0x7F, 0xFF, 0x00],
        },
        NamedColor {
            name: "Chocolate",
            rgb: [0xD2, 0x69, 0x1E],
        },
        NamedColor {
            name: "Coral",
            rgb: [0xFF, 0x7F, 0x50],
        },
        NamedColor {
            name: "CornflowerBlue",
            rgb: [0x64, 0x95, 0xED],
        },
        NamedColor {
            name: "Cornsilk",
            rgb: [0xFF, 0xF8, 0xDC],
        },
        NamedColor {
            name: "Crimson",
            rgb: [0xDC, 0x14, 0x3C],
        },
        NamedColor {
            name: "Cyan",
            rgb: [0x00, 0xFF, 0xFF],
        },
        NamedColor {
            name: "DarkBlue",
            rgb: [0x00, 0x00, 0x8B],
        },
        NamedColor {
            name: "DarkCyan",
            rgb: [0x00, 0x8B, 0x8B],
        },
        NamedColor {
            name: "DarkGoldenRod",
            rgb: [0xB8, 0x86, 0x0B],
        },
        NamedColor {
            name: "DarkGray",
            rgb: [0xA9, 0xA9, 0xA9],
        },
        NamedColor {
            name: "DarkGreen",
            rgb: [0x00, 0x64, 0x00],
        },
        NamedColor {
            name: "DarkKhaki",
            rgb: [0xBD, 0xB7, 0x6B],
        },
        NamedColor {
            name: "DarkMagenta",
            rgb: [0x8B, 0x00, 0x8B],
        },
        NamedColor {
            name: "DarkOliveGreen",
            rgb: [0x55, 0x6B, 0x2F],
        },
        NamedColor {
            name: "Darkorange",
            rgb: [0xFF, 0x8C, 0x00],
        },
        NamedColor {
            name: "DarkOrchid",
            rgb: [0x99, 0x32, 0xCC],
        },
        NamedColor {
            name: "DarkRed",
            rgb: [0x8B, 0x00, 0x00],
        },
        NamedColor {
            name: "DarkSalmon",
            rgb: [0xE9, 0x96, 0x7A],
        },
        NamedColor {
            name: "DarkSeaGreen",
            rgb: [0x8F, 0xBC, 0x8F],
        },
        NamedColor {
            name: "DarkSlateBlue",
            rgb: [0x48, 0x3D, 0x8B],
        },
        NamedColor {
            name: "DarkSlateGray",
            rgb: [0x2F, 0x4F, 0x4F],
        },
        NamedColor {
            name: "DarkTurquoise",
            rgb: [0x00, 0xCE, 0xD1],
        },
        NamedColor {
            name: "DarkViolet",
            rgb: [0x94, 0x00, 0xD3],
        },
        NamedColor {
            name: "DeepPink",
            rgb: [0xFF, 0x14, 0x93],
        },
        NamedColor {
            name: "DeepSkyBlue",
            rgb: [0x00, 0xBF, 0xFF],
        },
        NamedColor {
            name: "DimGray",
            rgb: [0x69, 0x69, 0x69],
        },
        NamedColor {
            name: "DodgerBlue",
            rgb: [0x1E, 0x90, 0xFF],
        },
        NamedColor {
            name: "FireBrick",
            rgb: [0xB2, 0x22, 0x22],
        },
        NamedColor {
            name: "FloralWhite",
            rgb: [0xFF, 0xFA, 0xF0],
        },
        NamedColor {
            name: "ForestGreen",
            rgb: [0x22, 0x8B, 0x22],
        },
        NamedColor {
            name: "Fuchsia",
            rgb: [0xFF, 0x00, 0xFF],
        },
        NamedColor {
            name: "Gainsboro",
            rgb: [0xDC, 0xDC, 0xDC],
        },
        NamedColor {
            name: "GhostWhite",
            rgb: [0xF8, 0xF8, 0xFF],
        },
        NamedColor {
            name: "Gold",
            rgb: [0xFF, 0xD7, 0x00],
        },
        NamedColor {
            name: "GoldenRod",
            rgb: [0xDA, 0xA5, 0x20],
        },
        NamedColor {
            name: "Gray",
            rgb: [0x80, 0x80, 0x80],
        },
        NamedColor {
            name: "Green",
            rgb: [0x00, 0x80, 0x00],
        },
        NamedColor {
            name: "GreenYellow",
            rgb: [0xAD, 0xFF, 0x2F],
        },
        NamedColor {
            name: "HoneyDew",
            rgb: [0xF0, 0xFF, 0xF0],
        },
        NamedColor {
            name: "HotPink",
            rgb: [0xFF, 0x69, 0xB4],
        },
        NamedColor {
            name: "IndianRed",
            rgb: [0xCD, 0x5C, 0x5C],
        },
        NamedColor {
            name: "Indigo",
            rgb: [0x4B, 0x00, 0x82],
        },
        NamedColor {
            name: "Ivory",
            rgb: [0xFF, 0xFF, 0xF0],
        },
        NamedColor {
            name: "Khaki",
            rgb: [0xF0, 0xE6, 0x8C],
        },
        NamedColor {
            name: "Lavender",
            rgb: [0xE6, 0xE6, 0xFA],
        },
        NamedColor {
            name: "LavenderBlush",
            rgb: [0xFF, 0xF0, 0xF5],
        },
        NamedColor {
            name: "LawnGreen",
            rgb: [0x7C, 0xFC, 0x00],
        },
        NamedColor {
            name: "LemonChiffon",
            rgb: [0xFF, 0xFA, 0xCD],
        },
        NamedColor {
            name: "LightBlue",
            rgb: [0xAD, 0xD8, 0xE6],
        },
        NamedColor {
            name: "LightCoral",
            rgb: [0xF0, 0x80, 0x80],
        },
        NamedColor {
            name: "LightCyan",
            rgb: [0xE0, 0xFF, 0xFF],
        },
        NamedColor {
            name: "LightGoldenRodYellow",
            rgb: [0xFA, 0xFA, 0xD2],
        },
        NamedColor {
            name: "LightGreen",
            rgb: [0x90, 0xEE, 0x90],
        },
        NamedColor {
            name: "LightGrey",
            rgb: [0xD3, 0xD3, 0xD3],
        },
        NamedColor {
            name: "LightPink",
            rgb: [0xFF, 0xB6, 0xC1],
        },
        NamedColor {
            name: "LightSalmon",
            rgb: [0xFF, 0xA0, 0x7A],
        },
        NamedColor {
            name: "LightSeaGreen",
            rgb: [0x20, 0xB2, 0xAA],
        },
        NamedColor {
            name: "LightSkyBlue",
            rgb: [0x87, 0xCE, 0xFA],
        },
        NamedColor {
            name: "LightSlateGray",
            rgb: [0x77, 0x88, 0x99],
        },
        NamedColor {
            name: "LightSteelBlue",
            rgb: [0xB0, 0xC4, 0xDE],
        },
        NamedColor {
            name: "LightYellow",
            rgb: [0xFF, 0xFF, 0xE0],
        },
        NamedColor {
            name: "Lime",
            rgb: [0x00, 0xFF, 0x00],
        },
        NamedColor {
            name: "LimeGreen",
            rgb: [0x32, 0xCD, 0x32],
        },
        NamedColor {
            name: "Linen",
            rgb: [0xFA, 0xF0, 0xE6],
        },
        NamedColor {
            name: "Magenta",
            rgb: [0xFF, 0x00, 0xFF],
        },
        NamedColor {
            name: "Maroon",
            rgb: [0x80, 0x00, 0x00],
        },
        NamedColor {
            name: "MediumAquaMarine",
            rgb: [0x66, 0xCD, 0xAA],
        },
        NamedColor {
            name: "MediumBlue",
            rgb: [0x00, 0x00, 0xCD],
        },
        NamedColor {
            name: "MediumOrchid",
            rgb: [0xBA, 0x55, 0xD3],
        },
        NamedColor {
            name: "MediumPurple",
            rgb: [0x93, 0x70, 0xD8],
        },
        NamedColor {
            name: "MediumSeaGreen",
            rgb: [0x3C, 0xB3, 0x71],
        },
        NamedColor {
            name: "MediumSlateBlue",
            rgb: [0x7B, 0x68, 0xEE],
        },
        NamedColor {
            name: "MediumSpringGreen",
            rgb: [0x00, 0xFA, 0x9A],
        },
        NamedColor {
            name: "MediumTurquoise",
            rgb: [0x48, 0xD1, 0xCC],
        },
        NamedColor {
            name: "MediumVioletRed",
            rgb: [0xC7, 0x15, 0x85],
        },
        NamedColor {
            name: "MidnightBlue",
            rgb: [0x19, 0x19, 0x70],
        },
        NamedColor {
            name: "MintCream",
            rgb: [0xF5, 0xFF, 0xFA],
        },
        NamedColor {
            name: "MistyRose",
            rgb: [0xFF, 0xE4, 0xE1],
        },
        NamedColor {
            name: "Moccasin",
            rgb: [0xFF, 0xE4, 0xB5],
        },
        NamedColor {
            name: "NavajoWhite",
            rgb: [0xFF, 0xDE, 0xAD],
        },
        NamedColor {
            name: "Navy",
            rgb: [0x00, 0x00, 0x80],
        },
        NamedColor {
            name: "OldLace",
            rgb: [0xFD, 0xF5, 0xE6],
        },
        NamedColor {
            name: "Olive",
            rgb: [0x80, 0x80, 0x00],
        },
        NamedColor {
            name: "OliveDrab",
            rgb: [0x6B, 0x8E, 0x23],
        },
        NamedColor {
            name: "Orange",
            rgb: [0xFF, 0xA5, 0x00],
        },
        NamedColor {
            name: "OrangeRed",
            rgb: [0xFF, 0x45, 0x00],
        },
        NamedColor {
            name: "Orchid",
            rgb: [0xDA, 0x70, 0xD6],
        },
        NamedColor {
            name: "PaleGoldenRod",
            rgb: [0xEE, 0xE8, 0xAA],
        },
        NamedColor {
            name: "PaleGreen",
            rgb: [0x98, 0xFB, 0x98],
        },
        NamedColor {
            name: "PaleTurquoise",
            rgb: [0xAF, 0xEE, 0xEE],
        },
        NamedColor {
            name: "PaleVioletRed",
            rgb: [0xD8, 0x70, 0x93],
        },
        NamedColor {
            name: "PapayaWhip",
            rgb: [0xFF, 0xEF, 0xD5],
        },
        NamedColor {
            name: "PeachPuff",
            rgb: [0xFF, 0xDA, 0xB9],
        },
        NamedColor {
            name: "Peru",
            rgb: [0xCD, 0x85, 0x3F],
        },
        NamedColor {
            name: "Pink",
            rgb: [0xFF, 0xC0, 0xCB],
        },
        NamedColor {
            name: "Plum",
            rgb: [0xDD, 0xA0, 0xDD],
        },
        NamedColor {
            name: "PowderBlue",
            rgb: [0xB0, 0xE0, 0xE6],
        },
        NamedColor {
            name: "Purple",
            rgb: [0x80, 0x00, 0x80],
        },
        NamedColor {
            name: "Red",
            rgb: [0xFF, 0x00, 0x00],
        },
        NamedColor {
            name: "RosyBrown",
            rgb: [0xBC, 0x8F, 0x8F],
        },
        NamedColor {
            name: "RoyalBlue",
            rgb: [0x41, 0x69, 0xE1],
        },
        NamedColor {
            name: "SaddleBrown",
            rgb: [0x8B, 0x45, 0x13],
        },
        NamedColor {
            name: "Salmon",
            rgb: [0xFA, 0x80, 0x72],
        },
        NamedColor {
            name: "SandyBrown",
            rgb: [0xF4, 0xA4, 0x60],
        },
        NamedColor {
            name: "SeaGreen",
            rgb: [0x2E, 0x8B, 0x57],
        },
        NamedColor {
            name: "SeaShell",
            rgb: [0xFF, 0xF5, 0xEE],
        },
        NamedColor {
            name: "Sienna",
            rgb: [0xA0, 0x52, 0x2D],
        },
        NamedColor {
            name: "Silver",
            rgb: [0xC0, 0xC0, 0xC0],
        },
        NamedColor {
            name: "SkyBlue",
            rgb: [0x87, 0xCE, 0xEB],
        },
        NamedColor {
            name: "SlateBlue",
            rgb: [0x6A, 0x5A, 0xCD],
        },
        NamedColor {
            name: "SlateGray",
            rgb: [0x70, 0x80, 0x90],
        },
        NamedColor {
            name: "Snow",
            rgb: [0xFF, 0xFA, 0xFA],
        },
        NamedColor {
            name: "SpringGreen",
            rgb: [0x00, 0xFF, 0x7F],
        },
        NamedColor {
            name: "SteelBlue",
            rgb: [0x46, 0x82, 0xB4],
        },
        NamedColor {
            name: "Tan",
            rgb: [0xD2, 0xB4, 0x8C],
        },
        NamedColor {
            name: "Teal",
            rgb: [0x00, 0x80, 0x80],
        },
        NamedColor {
            name: "Thistle",
            rgb: [0xD8, 0xBF, 0xD8],
        },
        NamedColor {
            name: "Tomato",
            rgb: [0xFF, 0x63, 0x47],
        },
        NamedColor {
            name: "Turquoise",
            rgb: [0x40, 0xE0, 0xD0],
        },
        NamedColor {
            name: "Violet",
            rgb: [0xEE, 0x82, 0xEE],
        },
        NamedColor {
            name: "Wheat",
            rgb: [0xF5, 0xDE, 0xB3],
        },
        NamedColor {
            name: "White",
            rgb: [0xFF, 0xFF, 0xFF],
        },
        NamedColor {
            name: "WhiteSmoke",
            rgb: [0xF5, 0xF5, 0xF5],
        },
        NamedColor {
            name: "Yellow",
            rgb: [0xFF, 0xFF, 0x00],
        },
        NamedColor {
            name: "YellowGreen",
            rgb: [0x9A, 0xCD, 0x32],
        },
    ];

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn rgb(self) -> [u8; 3] {
        self.rgb
    }

    pub fn rgb_hex_lower(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.rgb[0], self.rgb[1], self.rgb[2])
    }
}

pub fn known_color(index: usize) -> Option<NamedColor> {
    NamedColor::ALL.get(index).copied()
}

pub fn find_named_color(name: &str) -> Option<NamedColor> {
    NamedColor::ALL
        .iter()
        .copied()
        .find(|color| color.name.eq_ignore_ascii_case(name))
}

pub fn parse_color(input: &str) -> AvResult<RgbaColor> {
    let (color_text, alpha_text) = input.split_once('@').unwrap_or((input, ""));
    let has_alpha = input.contains('@');
    let (color_text, forced_hex) = if let Some(hex) = color_text.strip_prefix('#') {
        (hex, true)
    } else if let Some(hex) = color_text.strip_prefix("0x") {
        (hex, true)
    } else {
        (color_text, false)
    };

    let mut color = if color_text.eq_ignore_ascii_case("random")
        || color_text.eq_ignore_ascii_case("bikeshed")
    {
        return Err(AvError::unsupported(
            "random av_parse_color colors require a nondeterministic seed",
        ));
    } else if forced_hex || is_hex_text(color_text) {
        parse_hex_color(color_text)?
    } else {
        let named = find_named_color(color_text).ok_or_else(|| {
            AvError::invalid_argument(format!("unknown color name `{color_text}`"))
        })?;
        RgbaColor::from_rgb(named.rgb())
    };

    if has_alpha {
        color.rgba[3] = parse_alpha(alpha_text)?;
    }

    Ok(color)
}

pub fn colors_table_string() -> String {
    let mut output = format!("{:<32} #RRGGBB\n", "name");
    for color in NamedColor::ALL {
        let _ = writeln!(output, "{:<32} {}", color.name(), color.rgb_hex_lower());
    }
    output
}

fn parse_hex_color(hex: &str) -> AvResult<RgbaColor> {
    if !matches!(hex.len(), 6 | 8) || !is_hex_text(hex) {
        return Err(AvError::invalid_argument(
            "expected color hex value in RRGGBB or RRGGBBAA form",
        ));
    }

    let mut value = u32::from_str_radix(hex, 16)
        .map_err(|_| AvError::invalid_argument("invalid color hex value"))?;
    let alpha = if hex.len() == 8 {
        let alpha = value as u8;
        value >>= 8;
        alpha
    } else {
        0xFF
    };

    Ok(RgbaColor::from_rgba([
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
        alpha,
    ]))
}

fn parse_alpha(alpha: &str) -> AvResult<u8> {
    if let Some(hex) = alpha.strip_prefix("0x") {
        return parse_hex_alpha(hex);
    }

    let normalized = alpha
        .parse::<f64>()
        .map_err(|_| AvError::invalid_argument("invalid alpha value"))?;
    if !normalized.is_finite() || !(0.0..=1.0).contains(&normalized) {
        return Err(AvError::invalid_argument(
            "alpha value must be a finite value between 0 and 1",
        ));
    }

    let alpha = 255.0 * normalized;
    Ok(alpha.trunc() as u8)
}

fn parse_hex_alpha(hex: &str) -> AvResult<u8> {
    if hex.is_empty() || !is_hex_text(hex) {
        return Err(AvError::invalid_argument("invalid hexadecimal alpha value"));
    }

    let mut value = 0u16;
    for byte in hex.bytes() {
        let digit = hex_digit(byte).expect("validated hexadecimal digit");
        value = value
            .checked_mul(16)
            .and_then(|current| current.checked_add(u16::from(digit)))
            .ok_or_else(|| AvError::invalid_argument("alpha value exceeds 255"))?;
        if value > 255 {
            return Err(AvError::invalid_argument("alpha value exceeds 255"));
        }
    }

    Ok(value as u8)
}

fn is_hex_text(text: &str) -> bool {
    text.bytes().all(|byte| hex_digit(byte).is_some())
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn color_table_matches_source_checked_inventory_boundaries() {
        assert_eq!(NamedColor::ALL.len(), 140);
        assert_eq!(
            known_color(0),
            Some(NamedColor {
                name: "AliceBlue",
                rgb: [0xF0, 0xF8, 0xFF]
            })
        );
        assert_eq!(
            known_color(139),
            Some(NamedColor {
                name: "YellowGreen",
                rgb: [0x9A, 0xCD, 0x32]
            })
        );
        assert_eq!(known_color(140), None);
    }

    #[test]
    fn named_color_lookup_is_ascii_case_insensitive() {
        assert_eq!(
            find_named_color("darkorange").map(NamedColor::rgb),
            Some([0xFF, 0x8C, 0x00])
        );
        assert_eq!(
            find_named_color("LIGHTGOLDENRODYELLOW").map(NamedColor::rgb),
            Some([0xFA, 0xFA, 0xD2])
        );
        assert_eq!(find_named_color("Transparent"), None);
    }

    #[test]
    fn color_inventory_table_uses_ffmpeg_colors_shape() {
        let table = colors_table_string();
        let rows = table.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 141);
        assert_eq!(rows[0], format!("{:<32} #RRGGBB", "name"));
        assert_eq!(rows[1], format!("{:<32} #f0f8ff", "AliceBlue"));
        assert_eq!(rows[30], format!("{:<32} #ff8c00", "Darkorange"));
        assert_eq!(rows[140], format!("{:<32} #9acd32", "YellowGreen"));
    }

    #[test]
    fn parse_color_accepts_named_colors_case_insensitively() {
        assert_eq!(parse_color("red").unwrap().rgba(), [0xFF, 0x00, 0x00, 0xFF]);
        assert_eq!(
            parse_color("Darkorange").unwrap().rgba(),
            [0xFF, 0x8C, 0x00, 0xFF]
        );
        assert_eq!(
            parse_color("LIGHTGOLDENRODYELLOW").unwrap().rgba(),
            [0xFA, 0xFA, 0xD2, 0xFF]
        );
    }

    #[test]
    fn parse_color_accepts_ffmpeg_hex_forms() {
        assert_eq!(
            parse_color("#112233").unwrap().rgba(),
            [0x11, 0x22, 0x33, 0xFF]
        );
        assert_eq!(
            parse_color("0x11223344").unwrap().rgba(),
            [0x11, 0x22, 0x33, 0x44]
        );
        assert_eq!(
            parse_color("112233").unwrap().rgba(),
            [0x11, 0x22, 0x33, 0xFF]
        );
        assert_eq!(
            parse_color("11223344").unwrap().rgba(),
            [0x11, 0x22, 0x33, 0x44]
        );
        assert_eq!(
            parse_color("AABBCCDD").unwrap().rgba_hex_lower(),
            "#aabbccdd"
        );
    }

    #[test]
    fn parse_color_alpha_suffix_overrides_named_and_embedded_alpha() {
        assert_eq!(
            parse_color("red@0x80").unwrap().rgba(),
            [0xFF, 0x00, 0x00, 0x80]
        );
        assert_eq!(
            parse_color("Blue@0.5").unwrap().rgba(),
            [0x00, 0x00, 0xFF, 0x7F]
        );
        assert_eq!(parse_color("white@1").unwrap().alpha(), 0xFF);
        assert_eq!(parse_color("red@0.999").unwrap().alpha(), 0xFE);
        assert_eq!(
            parse_color("#01020304@0x05").unwrap().rgba(),
            [0x01, 0x02, 0x03, 0x05]
        );
    }

    #[test]
    fn parse_color_rejects_invalid_inputs_with_typed_errors() {
        for input in [
            "",
            "#12345",
            "#11223z",
            "0X112233",
            "transparent",
            "white@",
            "red@0x100",
            "red@0x",
            "red@@0.5",
        ] {
            assert_eq!(
                parse_color(input).unwrap_err().kind(),
                AvErrorKind::InvalidArgument,
                "{input}"
            );
        }
        assert_eq!(
            parse_color("random").unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            parse_color("bikeshed").unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
    }
}
