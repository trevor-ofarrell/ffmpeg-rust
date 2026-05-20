use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Gray8,
    Ya8,
    Ya16Le,
    Ya16Be,
    Gray16Le,
    Gray16Be,
    Gray32Le,
    Gray32Be,
    GrayF16Le,
    GrayF16Be,
    GrayF32Le,
    GrayF32Be,
    Rgb24,
    Bgr24,
    Rgb48Le,
    Rgb48Be,
    Bgr48Le,
    Bgr48Be,
    Rgba64Le,
    Rgba64Be,
    Bgra64Le,
    Bgra64Be,
    Rgba,
    Bgra,
    Argb,
    Abgr,
    ZeroRgb,
    Rgb0,
    ZeroBgr,
    Bgr0,
    Gbrp,
    Yuv420p,
    Yuv422p,
    Yuv410p,
    Yuv411p,
    Yuv440p,
    Yuv444p,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormatClass {
    Gray,
    Rgb,
    Yuv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelFormatDescriptor {
    pub format: PixelFormat,
    pub name: &'static str,
    pub class: PixelFormatClass,
    pub component_count: usize,
    pub bits_per_component: u8,
    pub bits_per_pixel: u8,
    pub plane_count: usize,
    pub is_planar: bool,
    pub has_alpha: bool,
    pub is_float: bool,
    pub packed_bytes_per_pixel: Option<usize>,
    pub log2_chroma_w: u8,
    pub log2_chroma_h: u8,
}

impl PixelFormat {
    pub const ALL: &'static [Self] = &[
        Self::Gray8,
        Self::Ya8,
        Self::Ya16Le,
        Self::Ya16Be,
        Self::Gray16Le,
        Self::Gray16Be,
        Self::Gray32Le,
        Self::Gray32Be,
        Self::GrayF16Le,
        Self::GrayF16Be,
        Self::GrayF32Le,
        Self::GrayF32Be,
        Self::Rgb24,
        Self::Bgr24,
        Self::Rgb48Le,
        Self::Rgb48Be,
        Self::Bgr48Le,
        Self::Bgr48Be,
        Self::Rgba64Le,
        Self::Rgba64Be,
        Self::Bgra64Le,
        Self::Bgra64Be,
        Self::Rgba,
        Self::Bgra,
        Self::Argb,
        Self::Abgr,
        Self::ZeroRgb,
        Self::Rgb0,
        Self::ZeroBgr,
        Self::Bgr0,
        Self::Gbrp,
        Self::Yuv420p,
        Self::Yuv422p,
        Self::Yuv410p,
        Self::Yuv411p,
        Self::Yuv440p,
        Self::Yuv444p,
    ];

    pub fn name(self) -> &'static str {
        self.descriptor().name
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "gray" | "gray8" => Some(Self::Gray8),
            "ya8" | "gray8a" | "y400a" => Some(Self::Ya8),
            "ya16le" => Some(Self::Ya16Le),
            "ya16be" => Some(Self::Ya16Be),
            "gray16le" => Some(Self::Gray16Le),
            "gray16be" => Some(Self::Gray16Be),
            "gray32le" | "y32le" => Some(Self::Gray32Le),
            "gray32be" | "y32be" => Some(Self::Gray32Be),
            "grayf16le" => Some(Self::GrayF16Le),
            "grayf16be" => Some(Self::GrayF16Be),
            "grayf32le" | "yf32le" => Some(Self::GrayF32Le),
            "grayf32be" | "yf32be" => Some(Self::GrayF32Be),
            "rgb24" => Some(Self::Rgb24),
            "bgr24" => Some(Self::Bgr24),
            "rgb48le" => Some(Self::Rgb48Le),
            "rgb48be" => Some(Self::Rgb48Be),
            "bgr48le" => Some(Self::Bgr48Le),
            "bgr48be" => Some(Self::Bgr48Be),
            "rgba64le" => Some(Self::Rgba64Le),
            "rgba64be" => Some(Self::Rgba64Be),
            "bgra64le" => Some(Self::Bgra64Le),
            "bgra64be" => Some(Self::Bgra64Be),
            "rgba" => Some(Self::Rgba),
            "bgra" => Some(Self::Bgra),
            "argb" => Some(Self::Argb),
            "abgr" => Some(Self::Abgr),
            "0rgb" => Some(Self::ZeroRgb),
            "rgb0" => Some(Self::Rgb0),
            "0bgr" => Some(Self::ZeroBgr),
            "bgr0" => Some(Self::Bgr0),
            "gbrp" => Some(Self::Gbrp),
            "yuv420p" => Some(Self::Yuv420p),
            "yuv422p" => Some(Self::Yuv422p),
            "yuv410p" => Some(Self::Yuv410p),
            "yuv411p" => Some(Self::Yuv411p),
            "yuv440p" => Some(Self::Yuv440p),
            "yuv444p" => Some(Self::Yuv444p),
            _ => None,
        }
    }

    pub fn descriptor(self) -> PixelFormatDescriptor {
        let (
            name,
            class,
            component_count,
            bits_per_pixel,
            plane_count,
            is_planar,
            has_alpha,
            packed_bytes_per_pixel,
            log2_chroma_w,
            log2_chroma_h,
        ) = match self {
            Self::Gray8 => (
                "gray",
                PixelFormatClass::Gray,
                1,
                8,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::Ya8 => (
                "ya8",
                PixelFormatClass::Gray,
                2,
                16,
                1,
                false,
                true,
                Some(2),
                0,
                0,
            ),
            Self::Ya16Le => (
                "ya16le",
                PixelFormatClass::Gray,
                2,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Ya16Be => (
                "ya16be",
                PixelFormatClass::Gray,
                2,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Gray16Le => (
                "gray16le",
                PixelFormatClass::Gray,
                1,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray16Be => (
                "gray16be",
                PixelFormatClass::Gray,
                1,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray32Le => (
                "gray32le",
                PixelFormatClass::Gray,
                1,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Gray32Be => (
                "gray32be",
                PixelFormatClass::Gray,
                1,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::GrayF16Le => (
                "grayf16le",
                PixelFormatClass::Gray,
                1,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::GrayF16Be => (
                "grayf16be",
                PixelFormatClass::Gray,
                1,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::GrayF32Le => (
                "grayf32le",
                PixelFormatClass::Gray,
                1,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::GrayF32Be => (
                "grayf32be",
                PixelFormatClass::Gray,
                1,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Rgb24 => (
                "rgb24",
                PixelFormatClass::Rgb,
                3,
                24,
                1,
                false,
                false,
                Some(3),
                0,
                0,
            ),
            Self::Bgr24 => (
                "bgr24",
                PixelFormatClass::Rgb,
                3,
                24,
                1,
                false,
                false,
                Some(3),
                0,
                0,
            ),
            Self::Rgb48Le => (
                "rgb48le",
                PixelFormatClass::Rgb,
                3,
                48,
                1,
                false,
                false,
                Some(6),
                0,
                0,
            ),
            Self::Rgb48Be => (
                "rgb48be",
                PixelFormatClass::Rgb,
                3,
                48,
                1,
                false,
                false,
                Some(6),
                0,
                0,
            ),
            Self::Bgr48Le => (
                "bgr48le",
                PixelFormatClass::Rgb,
                3,
                48,
                1,
                false,
                false,
                Some(6),
                0,
                0,
            ),
            Self::Bgr48Be => (
                "bgr48be",
                PixelFormatClass::Rgb,
                3,
                48,
                1,
                false,
                false,
                Some(6),
                0,
                0,
            ),
            Self::Rgba64Le => (
                "rgba64le",
                PixelFormatClass::Rgb,
                4,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Rgba64Be => (
                "rgba64be",
                PixelFormatClass::Rgb,
                4,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Bgra64Le => (
                "bgra64le",
                PixelFormatClass::Rgb,
                4,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Bgra64Be => (
                "bgra64be",
                PixelFormatClass::Rgb,
                4,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Rgba => (
                "rgba",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Bgra => (
                "bgra",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Argb => (
                "argb",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Abgr => (
                "abgr",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::ZeroRgb => (
                "0rgb",
                PixelFormatClass::Rgb,
                3,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Rgb0 => (
                "rgb0",
                PixelFormatClass::Rgb,
                3,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::ZeroBgr => (
                "0bgr",
                PixelFormatClass::Rgb,
                3,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Bgr0 => (
                "bgr0",
                PixelFormatClass::Rgb,
                3,
                32,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Gbrp => (
                "gbrp",
                PixelFormatClass::Rgb,
                3,
                24,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv420p => (
                "yuv420p",
                PixelFormatClass::Yuv,
                3,
                12,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p => (
                "yuv422p",
                PixelFormatClass::Yuv,
                3,
                16,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv411p => (
                "yuv411p",
                PixelFormatClass::Yuv,
                3,
                12,
                3,
                true,
                false,
                None,
                2,
                0,
            ),
            Self::Yuv410p => (
                "yuv410p",
                PixelFormatClass::Yuv,
                3,
                9,
                3,
                true,
                false,
                None,
                2,
                2,
            ),
            Self::Yuv440p => (
                "yuv440p",
                PixelFormatClass::Yuv,
                3,
                16,
                3,
                true,
                false,
                None,
                0,
                1,
            ),
            Self::Yuv444p => (
                "yuv444p",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
        };

        PixelFormatDescriptor {
            format: self,
            name,
            class,
            component_count,
            bits_per_component: if matches!(
                self,
                Self::Ya16Le
                    | Self::Ya16Be
                    | Self::Gray16Le
                    | Self::Gray16Be
                    | Self::GrayF16Le
                    | Self::GrayF16Be
                    | Self::Rgb48Le
                    | Self::Rgb48Be
                    | Self::Bgr48Le
                    | Self::Bgr48Be
                    | Self::Rgba64Le
                    | Self::Rgba64Be
                    | Self::Bgra64Le
                    | Self::Bgra64Be
            ) {
                16
            } else if matches!(
                self,
                Self::Gray32Le | Self::Gray32Be | Self::GrayF32Le | Self::GrayF32Be
            ) {
                32
            } else {
                8
            },
            bits_per_pixel,
            plane_count,
            is_planar,
            has_alpha,
            is_float: matches!(
                self,
                Self::GrayF16Le | Self::GrayF16Be | Self::GrayF32Le | Self::GrayF32Be
            ),
            packed_bytes_per_pixel,
            log2_chroma_w,
            log2_chroma_h,
        }
    }

    pub fn class(self) -> PixelFormatClass {
        self.descriptor().class
    }

    pub fn is_gray(self) -> bool {
        self.class() == PixelFormatClass::Gray
    }

    pub fn is_rgb(self) -> bool {
        self.class() == PixelFormatClass::Rgb
    }

    pub fn is_yuv(self) -> bool {
        self.class() == PixelFormatClass::Yuv
    }

    pub fn component_count(self) -> usize {
        self.descriptor().component_count
    }

    pub fn bits_per_component(self) -> u8 {
        self.descriptor().bits_per_component
    }

    pub fn bits_per_pixel(self) -> u8 {
        self.descriptor().bits_per_pixel
    }

    pub fn log2_chroma(self) -> (u8, u8) {
        let descriptor = self.descriptor();
        (descriptor.log2_chroma_w, descriptor.log2_chroma_h)
    }

    pub fn has_chroma_subsampling(self) -> bool {
        let (log2_chroma_w, log2_chroma_h) = self.log2_chroma();
        log2_chroma_w != 0 || log2_chroma_h != 0
    }

    pub fn plane_count(self) -> usize {
        self.descriptor().plane_count
    }

    pub fn is_planar(self) -> bool {
        self.descriptor().is_planar
    }

    pub fn is_packed(self) -> bool {
        !self.is_planar()
    }

    pub fn has_alpha(self) -> bool {
        self.descriptor().has_alpha
    }

    pub fn is_float(self) -> bool {
        self.descriptor().is_float
    }

    pub fn packed_bytes_per_pixel(self) -> Option<usize> {
        self.descriptor().packed_bytes_per_pixel
    }

    pub fn plane_sizes(self, width: usize, height: usize) -> AvResult<Vec<usize>> {
        validate_dimensions(width, height, "pixel format")?;
        let pixels = checked_area(width, height, "pixel format frame area")?;

        match self {
            Self::Gray8 => Ok(vec![pixels]),
            Self::Ya8 => Ok(vec![checked_mul(
                pixels,
                2,
                "8-bit gray-alpha pixel format frame size",
            )?]),
            Self::Ya16Le | Self::Ya16Be => Ok(vec![checked_mul(
                pixels,
                4,
                "16-bit gray-alpha pixel format frame size",
            )?]),
            Self::Gray16Le | Self::Gray16Be => Ok(vec![checked_mul(
                pixels,
                2,
                "16-bit gray pixel format frame size",
            )?]),
            Self::Gray32Le | Self::Gray32Be => Ok(vec![checked_mul(
                pixels,
                4,
                "32-bit gray pixel format frame size",
            )?]),
            Self::GrayF16Le | Self::GrayF16Be => Ok(vec![checked_mul(
                pixels,
                2,
                "16-bit floating gray pixel format frame size",
            )?]),
            Self::GrayF32Le | Self::GrayF32Be => Ok(vec![checked_mul(
                pixels,
                4,
                "32-bit floating gray pixel format frame size",
            )?]),
            Self::Rgb24 | Self::Bgr24 => Ok(vec![checked_mul(
                pixels,
                3,
                "24-bit packed pixel format frame size",
            )?]),
            Self::Rgb48Le | Self::Rgb48Be | Self::Bgr48Le | Self::Bgr48Be => Ok(vec![checked_mul(
                pixels,
                6,
                "48-bit packed pixel format frame size",
            )?]),
            Self::Rgba64Le | Self::Rgba64Be | Self::Bgra64Le | Self::Bgra64Be => {
                Ok(vec![checked_mul(
                    pixels,
                    8,
                    "64-bit packed pixel format frame size",
                )?])
            }
            Self::Rgba
            | Self::Bgra
            | Self::Argb
            | Self::Abgr
            | Self::ZeroRgb
            | Self::Rgb0
            | Self::ZeroBgr
            | Self::Bgr0 => Ok(vec![checked_mul(
                pixels,
                4,
                "32-bit packed pixel format frame size",
            )?]),
            Self::Gbrp => Ok(vec![pixels, pixels, pixels]),
            Self::Yuv420p
            | Self::Yuv422p
            | Self::Yuv410p
            | Self::Yuv411p
            | Self::Yuv440p
            | Self::Yuv444p => {
                let descriptor = self.descriptor();
                let chroma_w = 1_usize << descriptor.log2_chroma_w;
                let chroma_h = 1_usize << descriptor.log2_chroma_h;
                if descriptor.log2_chroma_w != 0 && width % chroma_w != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by {}",
                        self.name(),
                        chroma_w
                    )));
                }
                if descriptor.log2_chroma_h != 0 && height % chroma_h != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format height must be divisible by {}",
                        self.name(),
                        chroma_h
                    )));
                }
                let chroma = checked_area(
                    width >> descriptor.log2_chroma_w,
                    height >> descriptor.log2_chroma_h,
                    "planar YUV chroma plane area",
                )?;
                Ok(vec![pixels, chroma, chroma])
            }
        }
    }

    pub fn frame_size(self, width: usize, height: usize) -> AvResult<usize> {
        sum_checked(
            self.plane_sizes(width, height)?,
            "pixel format frame size overflow",
        )
    }

    pub fn split_planes(self, data: &[u8], width: usize, height: usize) -> AvResult<Vec<Vec<u8>>> {
        let plane_sizes = self.plane_sizes(width, height)?;
        let expected = sum_checked(
            plane_sizes.iter().copied(),
            "pixel format frame size overflow",
        )?;
        if data.len() != expected {
            return Err(AvError::invalid_data(format!(
                "{} frame has {} bytes, expected {expected}",
                self.name(),
                data.len()
            )));
        }

        let mut start = 0;
        let mut planes = Vec::with_capacity(plane_sizes.len());
        for size in plane_sizes {
            let end = start + size;
            planes.push(data[start..end].to_vec());
            start = end;
        }
        Ok(planes)
    }
}

fn validate_dimensions(width: usize, height: usize, context: &str) -> AvResult<()> {
    if width == 0 || height == 0 {
        return Err(AvError::invalid_argument(format!(
            "{context} dimensions must be non-zero"
        )));
    }
    Ok(())
}

fn checked_area(width: usize, height: usize, context: &str) -> AvResult<usize> {
    width
        .checked_mul(height)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} overflow")))
}

fn checked_mul(value: usize, factor: usize, context: &str) -> AvResult<usize> {
    value
        .checked_mul(factor)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} overflow")))
}

fn sum_checked(
    values: impl IntoIterator<Item = usize>,
    overflow_message: &'static str,
) -> AvResult<usize> {
    values.into_iter().try_fold(0_usize, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| AvError::invalid_argument(overflow_message))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn pixel_formats_report_ffmpeg_names_and_layout() {
        assert_eq!(PixelFormat::from_name("gray"), Some(PixelFormat::Gray8));
        assert_eq!(PixelFormat::from_name("gray8"), Some(PixelFormat::Gray8));
        assert_eq!(PixelFormat::from_name("ya8"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("gray8a"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("y400a"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("ya16le"), Some(PixelFormat::Ya16Le));
        assert_eq!(PixelFormat::from_name("ya16be"), Some(PixelFormat::Ya16Be));
        assert_eq!(
            PixelFormat::from_name("gray16le"),
            Some(PixelFormat::Gray16Le)
        );
        assert_eq!(
            PixelFormat::from_name("gray16be"),
            Some(PixelFormat::Gray16Be)
        );
        assert_eq!(
            PixelFormat::from_name("gray32le"),
            Some(PixelFormat::Gray32Le)
        );
        assert_eq!(
            PixelFormat::from_name("gray32be"),
            Some(PixelFormat::Gray32Be)
        );
        assert_eq!(PixelFormat::from_name("y32le"), Some(PixelFormat::Gray32Le));
        assert_eq!(PixelFormat::from_name("y32be"), Some(PixelFormat::Gray32Be));
        assert_eq!(
            PixelFormat::from_name("grayf16le"),
            Some(PixelFormat::GrayF16Le)
        );
        assert_eq!(
            PixelFormat::from_name("grayf16be"),
            Some(PixelFormat::GrayF16Be)
        );
        assert_eq!(
            PixelFormat::from_name("grayf32le"),
            Some(PixelFormat::GrayF32Le)
        );
        assert_eq!(
            PixelFormat::from_name("grayf32be"),
            Some(PixelFormat::GrayF32Be)
        );
        assert_eq!(
            PixelFormat::from_name("yf32le"),
            Some(PixelFormat::GrayF32Le)
        );
        assert_eq!(
            PixelFormat::from_name("yf32be"),
            Some(PixelFormat::GrayF32Be)
        );
        assert_eq!(PixelFormat::Rgb24.name(), "rgb24");
        assert_eq!(PixelFormat::from_name("bgr24"), Some(PixelFormat::Bgr24));
        assert_eq!(
            PixelFormat::from_name("rgb48le"),
            Some(PixelFormat::Rgb48Le)
        );
        assert_eq!(
            PixelFormat::from_name("rgb48be"),
            Some(PixelFormat::Rgb48Be)
        );
        assert_eq!(
            PixelFormat::from_name("bgr48le"),
            Some(PixelFormat::Bgr48Le)
        );
        assert_eq!(
            PixelFormat::from_name("bgr48be"),
            Some(PixelFormat::Bgr48Be)
        );
        assert_eq!(
            PixelFormat::from_name("rgba64le"),
            Some(PixelFormat::Rgba64Le)
        );
        assert_eq!(
            PixelFormat::from_name("rgba64be"),
            Some(PixelFormat::Rgba64Be)
        );
        assert_eq!(
            PixelFormat::from_name("bgra64le"),
            Some(PixelFormat::Bgra64Le)
        );
        assert_eq!(
            PixelFormat::from_name("bgra64be"),
            Some(PixelFormat::Bgra64Be)
        );
        assert_eq!(PixelFormat::from_name("bgra"), Some(PixelFormat::Bgra));
        assert_eq!(PixelFormat::from_name("argb"), Some(PixelFormat::Argb));
        assert_eq!(PixelFormat::from_name("abgr"), Some(PixelFormat::Abgr));
        assert_eq!(PixelFormat::from_name("0rgb"), Some(PixelFormat::ZeroRgb));
        assert_eq!(PixelFormat::from_name("rgb0"), Some(PixelFormat::Rgb0));
        assert_eq!(PixelFormat::from_name("0bgr"), Some(PixelFormat::ZeroBgr));
        assert_eq!(PixelFormat::from_name("bgr0"), Some(PixelFormat::Bgr0));
        assert_eq!(PixelFormat::from_name("gbrp"), Some(PixelFormat::Gbrp));
        assert_eq!(
            PixelFormat::from_name("yuv422p"),
            Some(PixelFormat::Yuv422p)
        );
        assert_eq!(
            PixelFormat::from_name("yuv411p"),
            Some(PixelFormat::Yuv411p)
        );
        assert_eq!(
            PixelFormat::from_name("yuv410p"),
            Some(PixelFormat::Yuv410p)
        );
        assert_eq!(
            PixelFormat::from_name("yuv440p"),
            Some(PixelFormat::Yuv440p)
        );
        assert_eq!(
            PixelFormat::from_name("yuv444p"),
            Some(PixelFormat::Yuv444p)
        );
        assert_eq!(PixelFormat::ALL.len(), 37);
        assert_eq!(PixelFormat::Ya8.plane_count(), 1);
        assert_eq!(PixelFormat::Ya16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray32Le.plane_count(), 1);
        assert_eq!(PixelFormat::GrayF16Le.plane_count(), 1);
        assert_eq!(PixelFormat::GrayF32Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgba.plane_count(), 1);
        assert_eq!(PixelFormat::Rgb48Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgba64Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgb0.plane_count(), 1);
        assert_eq!(PixelFormat::Yuv420p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv422p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv410p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv411p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv440p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv444p.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp.plane_count(), 3);
        assert!(!PixelFormat::Rgb24.is_planar());
        assert!(PixelFormat::Rgb24.is_packed());
        assert!(PixelFormat::Ya8.is_packed());
        assert!(PixelFormat::Ya16Be.is_packed());
        assert!(PixelFormat::Gray16Be.is_packed());
        assert!(PixelFormat::Gray32Be.is_packed());
        assert!(PixelFormat::GrayF16Be.is_packed());
        assert!(PixelFormat::GrayF32Be.is_packed());
        assert!(PixelFormat::Bgr48Be.is_packed());
        assert!(PixelFormat::Bgra64Be.is_packed());
        assert!(PixelFormat::Rgb0.is_packed());
        assert!(PixelFormat::Yuv420p.is_planar());
        assert!(PixelFormat::Yuv422p.is_planar());
        assert!(PixelFormat::Yuv410p.is_planar());
        assert!(PixelFormat::Yuv411p.is_planar());
        assert!(PixelFormat::Yuv440p.is_planar());
        assert!(PixelFormat::Yuv444p.is_planar());
        assert!(PixelFormat::Gbrp.is_planar());
        assert!(!PixelFormat::Yuv420p.is_packed());
        assert!(!PixelFormat::Gbrp.is_packed());
        assert!(!PixelFormat::Rgb24.has_alpha());
        assert!(PixelFormat::Ya8.has_alpha());
        assert!(PixelFormat::Ya16Le.has_alpha());
        assert!(PixelFormat::Bgra.has_alpha());
        assert!(PixelFormat::Rgba64Le.has_alpha());
        assert!(!PixelFormat::ZeroRgb.has_alpha());
        assert!(!PixelFormat::Gray16Le.is_float());
        assert!(PixelFormat::GrayF16Le.is_float());
        assert!(PixelFormat::GrayF32Be.is_float());
        assert_eq!(PixelFormat::Bgr24.packed_bytes_per_pixel(), Some(3));
        assert_eq!(PixelFormat::Ya8.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::Ya16Be.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Gray16Le.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::Gray32Le.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::GrayF16Le.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::GrayF32Le.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Rgb48Le.packed_bytes_per_pixel(), Some(6));
        assert_eq!(PixelFormat::Bgra64Be.packed_bytes_per_pixel(), Some(8));
        assert_eq!(PixelFormat::Argb.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Bgr0.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Gbrp.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Yuv420p.packed_bytes_per_pixel(), None);
    }

    #[test]
    fn pixel_formats_report_descriptor_metadata() {
        let gray = PixelFormat::Gray8.descriptor();
        assert_eq!(gray.format, PixelFormat::Gray8);
        assert_eq!(gray.name, "gray");
        assert_eq!(gray.class, PixelFormatClass::Gray);
        assert_eq!(gray.component_count, 1);
        assert_eq!(gray.bits_per_component, 8);
        assert_eq!(gray.bits_per_pixel, 8);
        assert_eq!(gray.plane_count, 1);
        assert!(!gray.is_planar);
        assert!(!gray.has_alpha);
        assert!(!gray.is_float);
        assert_eq!(gray.packed_bytes_per_pixel, Some(1));
        assert_eq!((gray.log2_chroma_w, gray.log2_chroma_h), (0, 0));

        let ya8 = PixelFormat::Ya8.descriptor();
        assert_eq!(ya8.format, PixelFormat::Ya8);
        assert_eq!(ya8.name, "ya8");
        assert_eq!(PixelFormat::from_name(ya8.name), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("gray8a"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("y400a"), Some(PixelFormat::Ya8));
        assert_eq!(ya8.class, PixelFormatClass::Gray);
        assert!(PixelFormat::Ya8.is_gray());
        assert!(!PixelFormat::Ya8.is_rgb());
        assert!(!PixelFormat::Ya8.is_yuv());
        assert_eq!(ya8.component_count, 2);
        assert_eq!(ya8.bits_per_component, 8);
        assert_eq!(ya8.bits_per_pixel, 16);
        assert_eq!(ya8.plane_count, 1);
        assert!(!ya8.is_planar);
        assert!(ya8.has_alpha);
        assert!(!ya8.is_float);
        assert_eq!(ya8.packed_bytes_per_pixel, Some(2));
        assert_eq!(PixelFormat::Ya8.log2_chroma(), (0, 0));

        for (format, expected_name) in [
            (PixelFormat::Ya16Le, "ya16le"),
            (PixelFormat::Ya16Be, "ya16be"),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Gray);
            assert!(format.is_gray());
            assert_eq!(descriptor.component_count, 2);
            assert_eq!(descriptor.bits_per_component, 16);
            assert_eq!(descriptor.bits_per_pixel, 32);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(4));
            assert_eq!(format.log2_chroma(), (0, 0));
        }

        for (format, expected_name) in [
            (PixelFormat::Gray16Le, "gray16le"),
            (PixelFormat::Gray16Be, "gray16be"),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Gray);
            assert!(format.is_gray());
            assert_eq!(descriptor.component_count, 1);
            assert_eq!(descriptor.bits_per_component, 16);
            assert_eq!(descriptor.bits_per_pixel, 16);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(2));
            assert_eq!(format.log2_chroma(), (0, 0));
        }

        for (format, expected_name, expected_alias) in [
            (PixelFormat::Gray32Le, "gray32le", "y32le"),
            (PixelFormat::Gray32Be, "gray32be", "y32be"),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(PixelFormat::from_name(expected_alias), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Gray);
            assert!(format.is_gray());
            assert_eq!(descriptor.component_count, 1);
            assert_eq!(descriptor.bits_per_component, 32);
            assert_eq!(descriptor.bits_per_pixel, 32);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(4));
            assert_eq!(format.log2_chroma(), (0, 0));
        }

        for (format, expected_name, expected_alias, expected_bits_per_component) in [
            (PixelFormat::GrayF16Le, "grayf16le", None, 16),
            (PixelFormat::GrayF16Be, "grayf16be", None, 16),
            (PixelFormat::GrayF32Le, "grayf32le", Some("yf32le"), 32),
            (PixelFormat::GrayF32Be, "grayf32be", Some("yf32be"), 32),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            if let Some(alias) = expected_alias {
                assert_eq!(PixelFormat::from_name(alias), Some(format));
            }
            assert_eq!(descriptor.class, PixelFormatClass::Gray);
            assert!(format.is_gray());
            assert_eq!(descriptor.component_count, 1);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_component);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(descriptor.is_float);
            assert_eq!(
                descriptor.packed_bytes_per_pixel,
                Some(usize::from(expected_bits_per_component / 8))
            );
            assert_eq!(format.log2_chroma(), (0, 0));
        }

        for (format, expected_bits_per_component) in [
            (PixelFormat::Rgb24, 8),
            (PixelFormat::Bgr24, 8),
            (PixelFormat::Rgb48Le, 16),
            (PixelFormat::Rgb48Be, 16),
            (PixelFormat::Bgr48Le, 16),
            (PixelFormat::Bgr48Be, 16),
            (PixelFormat::Rgba64Le, 16),
            (PixelFormat::Rgba64Be, 16),
            (PixelFormat::Bgra64Le, 16),
            (PixelFormat::Bgra64Be, 16),
            (PixelFormat::Rgba, 8),
            (PixelFormat::Bgra, 8),
            (PixelFormat::Argb, 8),
            (PixelFormat::Abgr, 8),
            (PixelFormat::ZeroRgb, 8),
            (PixelFormat::Rgb0, 8),
            (PixelFormat::ZeroBgr, 8),
            (PixelFormat::Bgr0, 8),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert!(format.is_rgb());
            assert!(!format.is_gray());
            assert!(!format.is_yuv());
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert_eq!(descriptor.has_alpha, format.has_alpha());
            assert!(!descriptor.is_float);
            assert_eq!(
                descriptor.packed_bytes_per_pixel,
                format.packed_bytes_per_pixel()
            );
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }

        let gbrp = PixelFormat::Gbrp.descriptor();
        assert_eq!(gbrp.format, PixelFormat::Gbrp);
        assert_eq!(gbrp.name, "gbrp");
        assert_eq!(PixelFormat::from_name(gbrp.name), Some(PixelFormat::Gbrp));
        assert_eq!(gbrp.class, PixelFormatClass::Rgb);
        assert!(PixelFormat::Gbrp.is_rgb());
        assert!(!PixelFormat::Gbrp.is_yuv());
        assert_eq!(gbrp.component_count, 3);
        assert_eq!(gbrp.bits_per_component, 8);
        assert_eq!(gbrp.bits_per_pixel, 24);
        assert_eq!(gbrp.plane_count, 3);
        assert!(gbrp.is_planar);
        assert!(!gbrp.has_alpha);
        assert!(!gbrp.is_float);
        assert_eq!(gbrp.packed_bytes_per_pixel, None);
        assert_eq!(PixelFormat::Gbrp.log2_chroma(), (0, 0));
        assert!(!PixelFormat::Gbrp.has_chroma_subsampling());

        assert_eq!(PixelFormat::Rgb24.component_count(), 3);
        assert_eq!(PixelFormat::Rgb24.bits_per_pixel(), 24);
        assert_eq!(PixelFormat::Gbrp.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp.bits_per_component(), 8);
        assert_eq!(PixelFormat::Gbrp.bits_per_pixel(), 24);
        assert_eq!(PixelFormat::Ya16Le.component_count(), 2);
        assert_eq!(PixelFormat::Ya16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Ya16Le.bits_per_pixel(), 32);
        assert_eq!(PixelFormat::Gray32Le.component_count(), 1);
        assert_eq!(PixelFormat::Gray32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::Gray32Le.bits_per_pixel(), 32);
        assert_eq!(PixelFormat::GrayF16Le.component_count(), 1);
        assert_eq!(PixelFormat::GrayF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::GrayF16Le.bits_per_pixel(), 16);
        assert_eq!(PixelFormat::GrayF32Le.component_count(), 1);
        assert_eq!(PixelFormat::GrayF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::GrayF32Le.bits_per_pixel(), 32);
        assert_eq!(PixelFormat::Rgb48Le.component_count(), 3);
        assert_eq!(PixelFormat::Rgb48Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Rgb48Le.bits_per_pixel(), 48);
        assert_eq!(PixelFormat::Bgr48Be.bits_per_component(), 16);
        assert_eq!(PixelFormat::Bgr48Be.bits_per_pixel(), 48);
        assert_eq!(PixelFormat::Rgba64Le.component_count(), 4);
        assert_eq!(PixelFormat::Rgba64Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Rgba64Le.bits_per_pixel(), 64);
        assert_eq!(PixelFormat::Bgra64Be.component_count(), 4);
        assert_eq!(PixelFormat::Bgra64Be.bits_per_component(), 16);
        assert_eq!(PixelFormat::Bgra64Be.bits_per_pixel(), 64);
        assert_eq!(PixelFormat::Rgba.component_count(), 4);
        assert_eq!(PixelFormat::Rgba.bits_per_pixel(), 32);
        assert_eq!(PixelFormat::ZeroRgb.component_count(), 3);
        assert_eq!(PixelFormat::ZeroRgb.bits_per_pixel(), 32);

        for (format, expected_name, expected_bits_per_pixel, expected_log2_chroma) in [
            (PixelFormat::Yuv420p, "yuv420p", 12, (1, 1)),
            (PixelFormat::Yuv422p, "yuv422p", 16, (1, 0)),
            (PixelFormat::Yuv410p, "yuv410p", 9, (2, 2)),
            (PixelFormat::Yuv411p, "yuv411p", 12, (2, 0)),
            (PixelFormat::Yuv440p, "yuv440p", 16, (0, 1)),
            (PixelFormat::Yuv444p, "yuv444p", 24, (0, 0)),
        ] {
            let yuv = format.descriptor();
            assert_eq!(yuv.format, format);
            assert_eq!(yuv.name, expected_name);
            assert_eq!(yuv.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert_eq!(yuv.component_count, 3);
            assert_eq!(yuv.bits_per_component, 8);
            assert_eq!(yuv.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(yuv.plane_count, 3);
            assert!(yuv.is_planar);
            assert!(!yuv.has_alpha);
            assert!(!yuv.is_float);
            assert_eq!(yuv.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), expected_log2_chroma);
            assert_eq!(
                format.has_chroma_subsampling(),
                expected_log2_chroma != (0, 0)
            );
        }
        assert_eq!(PixelFormat::Yuv420p.log2_chroma(), (1, 1));
    }

    #[test]
    fn pixel_formats_compute_plane_and_frame_sizes() {
        assert_eq!(PixelFormat::Gray8.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Ya8.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Ya8.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Ya16Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::Ya16Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Gray16Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Gray16Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Gray32Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::Gray32Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::GrayF16Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::GrayF16Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::GrayF32Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::GrayF32Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Rgb24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Bgr24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Rgb48Le.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Rgb48Be.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Bgr48Le.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Bgr48Be.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Rgba64Le.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Rgba64Be.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Bgra64Le.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Bgra64Be.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Rgba.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Bgra.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Argb.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Abgr.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::ZeroRgb.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Rgb0.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::ZeroBgr.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Bgr0.frame_size(2, 2).unwrap(), 16);
        assert_eq!(
            PixelFormat::Yuv420p.plane_sizes(4, 2).unwrap(),
            vec![8, 2, 2]
        );
        assert_eq!(PixelFormat::Yuv420p.frame_size(4, 2).unwrap(), 12);
        assert_eq!(
            PixelFormat::Yuv422p.plane_sizes(4, 3).unwrap(),
            vec![12, 6, 6]
        );
        assert_eq!(PixelFormat::Yuv422p.frame_size(4, 3).unwrap(), 24);
        assert_eq!(
            PixelFormat::Yuv411p.plane_sizes(4, 3).unwrap(),
            vec![12, 3, 3]
        );
        assert_eq!(PixelFormat::Yuv411p.frame_size(4, 3).unwrap(), 18);
        assert_eq!(
            PixelFormat::Yuv410p.plane_sizes(4, 4).unwrap(),
            vec![16, 1, 1]
        );
        assert_eq!(PixelFormat::Yuv410p.frame_size(4, 4).unwrap(), 18);
        assert_eq!(
            PixelFormat::Yuv440p.plane_sizes(3, 2).unwrap(),
            vec![6, 3, 3]
        );
        assert_eq!(PixelFormat::Yuv440p.frame_size(3, 2).unwrap(), 12);
        assert_eq!(
            PixelFormat::Yuv444p.plane_sizes(3, 2).unwrap(),
            vec![6, 6, 6]
        );
        assert_eq!(PixelFormat::Yuv444p.frame_size(3, 2).unwrap(), 18);
        assert_eq!(PixelFormat::Gbrp.plane_sizes(3, 2).unwrap(), vec![6, 6, 6]);
        assert_eq!(PixelFormat::Gbrp.frame_size(3, 2).unwrap(), 18);
    }

    #[test]
    fn pixel_formats_split_frame_payloads_by_plane() {
        let planes = PixelFormat::Yuv420p
            .split_planes(&[0, 1, 2, 3, 4, 5], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4], vec![5]]);

        let planes = PixelFormat::Yuv422p
            .split_planes(&[0, 1, 2, 3, 4, 5, 6, 7], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4, 5], vec![6, 7]]);

        let planes = PixelFormat::Yuv411p
            .split_planes(&[0, 1, 2, 3, 4, 5], 4, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4], vec![5]]);

        let planes = PixelFormat::Yuv410p
            .split_planes(&(0..18).collect::<Vec<_>>(), 4, 4)
            .unwrap();

        assert_eq!(
            planes,
            vec![(0..16).collect::<Vec<_>>(), vec![16], vec![17]]
        );

        let planes = PixelFormat::Yuv440p
            .split_planes(&(0..12).collect::<Vec<_>>(), 3, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..6).collect::<Vec<_>>(),
                (6..9).collect::<Vec<_>>(),
                (9..12).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv444p
            .split_planes(&[0, 1, 2, 3, 4, 5], 1, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1], vec![2, 3], vec![4, 5]]);

        let planes = PixelFormat::Gbrp
            .split_planes(&[0, 1, 2, 3, 4, 5], 1, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1], vec![2, 3], vec![4, 5]]);

        let planes = PixelFormat::Rgb48Le
            .split_planes(&[0, 1, 2, 3, 4, 5], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3, 4, 5]]);

        let planes = PixelFormat::Rgba64Le
            .split_planes(&[0, 1, 2, 3, 4, 5, 6, 7], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3, 4, 5, 6, 7]]);

        let planes = PixelFormat::Ya8.split_planes(&[0, 255], 1, 1).unwrap();

        assert_eq!(planes, vec![vec![0, 255]]);

        let planes = PixelFormat::Ya16Le
            .split_planes(&[0, 1, 255, 254], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 255, 254]]);

        let planes = PixelFormat::Gray32Le
            .split_planes(&[0, 1, 2, 3], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::GrayF16Le.split_planes(&[0, 1], 1, 1).unwrap();

        assert_eq!(planes, vec![vec![0, 1]]);

        let planes = PixelFormat::GrayF32Le
            .split_planes(&[0, 1, 2, 3], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);
    }

    #[test]
    fn pixel_formats_reject_invalid_dimensions_and_payload_sizes() {
        assert_eq!(
            PixelFormat::Gray8.frame_size(0, 1).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv420p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv422p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv422p.frame_size(2, 3).unwrap(), 12);
        assert_eq!(
            PixelFormat::Yuv411p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv411p.frame_size(4, 3).unwrap(), 18);
        assert_eq!(
            PixelFormat::Yuv410p.frame_size(4, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv410p.frame_size(2, 4).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv410p.frame_size(4, 4).unwrap(), 18);
        assert_eq!(
            PixelFormat::Yuv440p.frame_size(3, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv440p.frame_size(3, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Yuv444p.frame_size(3, 2).unwrap(), 18);
        assert_eq!(
            PixelFormat::Rgb24
                .split_planes(&[0; 5], 1, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }
}
