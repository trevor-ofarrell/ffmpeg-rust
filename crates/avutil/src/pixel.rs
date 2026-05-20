use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Gray8,
    MonoWhite,
    MonoBlack,
    Ya8,
    Ya16Le,
    Ya16Be,
    Gray9Le,
    Gray9Be,
    Gray10Le,
    Gray10Be,
    Gray12Le,
    Gray12Be,
    Gray14Le,
    Gray14Be,
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
    Rgb8,
    Bgr8,
    Rgb4,
    Bgr4,
    Rgb4Byte,
    Bgr4Byte,
    Rgb565Be,
    Rgb565Le,
    Rgb555Be,
    Rgb555Le,
    Bgr565Be,
    Bgr565Le,
    Bgr555Be,
    Bgr555Le,
    Rgb444Le,
    Rgb444Be,
    Bgr444Le,
    Bgr444Be,
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
    Gbrp9Le,
    Gbrp9Be,
    Gbrp10Le,
    Gbrp10Be,
    Gbrp12Le,
    Gbrp12Be,
    Gbrp14Le,
    Gbrp14Be,
    Gbrp16Le,
    Gbrp16Be,
    Gbrap,
    Gbrap10Le,
    Gbrap10Be,
    Gbrap12Le,
    Gbrap12Be,
    Gbrap14Le,
    Gbrap14Be,
    Gbrap16Le,
    Gbrap16Be,
    Gbrap32Le,
    Gbrap32Be,
    GbrapF16Le,
    GbrapF16Be,
    GbrapF32Le,
    GbrapF32Be,
    Yuyv422,
    Uyvy422,
    Yvyu422,
    Nv12,
    Nv21,
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
        Self::MonoWhite,
        Self::MonoBlack,
        Self::Ya8,
        Self::Ya16Le,
        Self::Ya16Be,
        Self::Gray9Le,
        Self::Gray9Be,
        Self::Gray10Le,
        Self::Gray10Be,
        Self::Gray12Le,
        Self::Gray12Be,
        Self::Gray14Le,
        Self::Gray14Be,
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
        Self::Rgb8,
        Self::Bgr8,
        Self::Rgb4,
        Self::Bgr4,
        Self::Rgb4Byte,
        Self::Bgr4Byte,
        Self::Rgb565Be,
        Self::Rgb565Le,
        Self::Rgb555Be,
        Self::Rgb555Le,
        Self::Bgr565Be,
        Self::Bgr565Le,
        Self::Bgr555Be,
        Self::Bgr555Le,
        Self::Rgb444Le,
        Self::Rgb444Be,
        Self::Bgr444Le,
        Self::Bgr444Be,
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
        Self::Gbrp9Le,
        Self::Gbrp9Be,
        Self::Gbrp10Le,
        Self::Gbrp10Be,
        Self::Gbrp12Le,
        Self::Gbrp12Be,
        Self::Gbrp14Le,
        Self::Gbrp14Be,
        Self::Gbrp16Le,
        Self::Gbrp16Be,
        Self::Gbrap,
        Self::Gbrap10Le,
        Self::Gbrap10Be,
        Self::Gbrap12Le,
        Self::Gbrap12Be,
        Self::Gbrap14Le,
        Self::Gbrap14Be,
        Self::Gbrap16Le,
        Self::Gbrap16Be,
        Self::Gbrap32Le,
        Self::Gbrap32Be,
        Self::GbrapF16Le,
        Self::GbrapF16Be,
        Self::GbrapF32Le,
        Self::GbrapF32Be,
        Self::Yuyv422,
        Self::Uyvy422,
        Self::Yvyu422,
        Self::Nv12,
        Self::Nv21,
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
            "monow" => Some(Self::MonoWhite),
            "monob" => Some(Self::MonoBlack),
            "ya8" | "gray8a" | "y400a" => Some(Self::Ya8),
            "ya16le" => Some(Self::Ya16Le),
            "ya16be" => Some(Self::Ya16Be),
            "gray9le" | "y9le" => Some(Self::Gray9Le),
            "gray9be" | "y9be" => Some(Self::Gray9Be),
            "gray10le" | "y10le" => Some(Self::Gray10Le),
            "gray10be" | "y10be" => Some(Self::Gray10Be),
            "gray12le" | "y12le" => Some(Self::Gray12Le),
            "gray12be" | "y12be" => Some(Self::Gray12Be),
            "gray14le" | "y14le" => Some(Self::Gray14Le),
            "gray14be" | "y14be" => Some(Self::Gray14Be),
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
            "rgb8" => Some(Self::Rgb8),
            "bgr8" => Some(Self::Bgr8),
            "rgb4" => Some(Self::Rgb4),
            "bgr4" => Some(Self::Bgr4),
            "rgb4_byte" => Some(Self::Rgb4Byte),
            "bgr4_byte" => Some(Self::Bgr4Byte),
            "rgb565be" => Some(Self::Rgb565Be),
            "rgb565le" => Some(Self::Rgb565Le),
            "rgb555be" => Some(Self::Rgb555Be),
            "rgb555le" => Some(Self::Rgb555Le),
            "bgr565be" => Some(Self::Bgr565Be),
            "bgr565le" => Some(Self::Bgr565Le),
            "bgr555be" => Some(Self::Bgr555Be),
            "bgr555le" => Some(Self::Bgr555Le),
            "rgb444le" => Some(Self::Rgb444Le),
            "rgb444be" => Some(Self::Rgb444Be),
            "bgr444le" => Some(Self::Bgr444Le),
            "bgr444be" => Some(Self::Bgr444Be),
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
            "gbrp9le" => Some(Self::Gbrp9Le),
            "gbrp9be" => Some(Self::Gbrp9Be),
            "gbrp10le" => Some(Self::Gbrp10Le),
            "gbrp10be" => Some(Self::Gbrp10Be),
            "gbrp12le" => Some(Self::Gbrp12Le),
            "gbrp12be" => Some(Self::Gbrp12Be),
            "gbrp14le" => Some(Self::Gbrp14Le),
            "gbrp14be" => Some(Self::Gbrp14Be),
            "gbrp16le" => Some(Self::Gbrp16Le),
            "gbrp16be" => Some(Self::Gbrp16Be),
            "gbrap" => Some(Self::Gbrap),
            "gbrap10le" => Some(Self::Gbrap10Le),
            "gbrap10be" => Some(Self::Gbrap10Be),
            "gbrap12le" => Some(Self::Gbrap12Le),
            "gbrap12be" => Some(Self::Gbrap12Be),
            "gbrap14le" => Some(Self::Gbrap14Le),
            "gbrap14be" => Some(Self::Gbrap14Be),
            "gbrap16le" => Some(Self::Gbrap16Le),
            "gbrap16be" => Some(Self::Gbrap16Be),
            "gbrap32le" => Some(Self::Gbrap32Le),
            "gbrap32be" => Some(Self::Gbrap32Be),
            "gbrapf16le" => Some(Self::GbrapF16Le),
            "gbrapf16be" => Some(Self::GbrapF16Be),
            "gbrapf32le" => Some(Self::GbrapF32Le),
            "gbrapf32be" => Some(Self::GbrapF32Be),
            "yuyv422" => Some(Self::Yuyv422),
            "uyvy422" => Some(Self::Uyvy422),
            "yvyu422" => Some(Self::Yvyu422),
            "nv12" => Some(Self::Nv12),
            "nv21" => Some(Self::Nv21),
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
            Self::MonoWhite => (
                "monow",
                PixelFormatClass::Gray,
                1,
                1,
                1,
                false,
                false,
                None,
                0,
                0,
            ),
            Self::MonoBlack => (
                "monob",
                PixelFormatClass::Gray,
                1,
                1,
                1,
                false,
                false,
                None,
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
            Self::Gray9Le => (
                "gray9le",
                PixelFormatClass::Gray,
                1,
                9,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray9Be => (
                "gray9be",
                PixelFormatClass::Gray,
                1,
                9,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray10Le => (
                "gray10le",
                PixelFormatClass::Gray,
                1,
                10,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray10Be => (
                "gray10be",
                PixelFormatClass::Gray,
                1,
                10,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray12Le => (
                "gray12le",
                PixelFormatClass::Gray,
                1,
                12,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray12Be => (
                "gray12be",
                PixelFormatClass::Gray,
                1,
                12,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray14Le => (
                "gray14le",
                PixelFormatClass::Gray,
                1,
                14,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Gray14Be => (
                "gray14be",
                PixelFormatClass::Gray,
                1,
                14,
                1,
                false,
                false,
                Some(2),
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
            Self::Rgb8 => (
                "rgb8",
                PixelFormatClass::Rgb,
                3,
                8,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::Bgr8 => (
                "bgr8",
                PixelFormatClass::Rgb,
                3,
                8,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::Rgb4 => (
                "rgb4",
                PixelFormatClass::Rgb,
                3,
                4,
                1,
                false,
                false,
                None,
                0,
                0,
            ),
            Self::Bgr4 => (
                "bgr4",
                PixelFormatClass::Rgb,
                3,
                4,
                1,
                false,
                false,
                None,
                0,
                0,
            ),
            Self::Rgb4Byte => (
                "rgb4_byte",
                PixelFormatClass::Rgb,
                3,
                8,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::Bgr4Byte => (
                "bgr4_byte",
                PixelFormatClass::Rgb,
                3,
                8,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::Rgb565Be => (
                "rgb565be",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Rgb565Le => (
                "rgb565le",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Rgb555Be => (
                "rgb555be",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Rgb555Le => (
                "rgb555le",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Bgr565Be => (
                "bgr565be",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Bgr565Le => (
                "bgr565le",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Bgr555Be => (
                "bgr555be",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Bgr555Le => (
                "bgr555le",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Rgb444Le => (
                "rgb444le",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Rgb444Be => (
                "rgb444be",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Bgr444Le => (
                "bgr444le",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                0,
                0,
            ),
            Self::Bgr444Be => (
                "bgr444be",
                PixelFormatClass::Rgb,
                3,
                16,
                1,
                false,
                false,
                Some(2),
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
            Self::Gbrp9Le => (
                "gbrp9le",
                PixelFormatClass::Rgb,
                3,
                27,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp9Be => (
                "gbrp9be",
                PixelFormatClass::Rgb,
                3,
                27,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp10Le => (
                "gbrp10le",
                PixelFormatClass::Rgb,
                3,
                30,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp10Be => (
                "gbrp10be",
                PixelFormatClass::Rgb,
                3,
                30,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp12Le => (
                "gbrp12le",
                PixelFormatClass::Rgb,
                3,
                36,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp12Be => (
                "gbrp12be",
                PixelFormatClass::Rgb,
                3,
                36,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp14Le => (
                "gbrp14le",
                PixelFormatClass::Rgb,
                3,
                42,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp14Be => (
                "gbrp14be",
                PixelFormatClass::Rgb,
                3,
                42,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp16Le => (
                "gbrp16le",
                PixelFormatClass::Rgb,
                3,
                48,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrp16Be => (
                "gbrp16be",
                PixelFormatClass::Rgb,
                3,
                48,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Gbrap => (
                "gbrap",
                PixelFormatClass::Rgb,
                4,
                32,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap10Le => (
                "gbrap10le",
                PixelFormatClass::Rgb,
                4,
                40,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap10Be => (
                "gbrap10be",
                PixelFormatClass::Rgb,
                4,
                40,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap12Le => (
                "gbrap12le",
                PixelFormatClass::Rgb,
                4,
                48,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap12Be => (
                "gbrap12be",
                PixelFormatClass::Rgb,
                4,
                48,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap14Le => (
                "gbrap14le",
                PixelFormatClass::Rgb,
                4,
                56,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap14Be => (
                "gbrap14be",
                PixelFormatClass::Rgb,
                4,
                56,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap16Le => (
                "gbrap16le",
                PixelFormatClass::Rgb,
                4,
                64,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap16Be => (
                "gbrap16be",
                PixelFormatClass::Rgb,
                4,
                64,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap32Le => (
                "gbrap32le",
                PixelFormatClass::Rgb,
                4,
                128,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Gbrap32Be => (
                "gbrap32be",
                PixelFormatClass::Rgb,
                4,
                128,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::GbrapF16Le => (
                "gbrapf16le",
                PixelFormatClass::Rgb,
                4,
                64,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::GbrapF16Be => (
                "gbrapf16be",
                PixelFormatClass::Rgb,
                4,
                64,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::GbrapF32Le => (
                "gbrapf32le",
                PixelFormatClass::Rgb,
                4,
                128,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::GbrapF32Be => (
                "gbrapf32be",
                PixelFormatClass::Rgb,
                4,
                128,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuyv422 => (
                "yuyv422",
                PixelFormatClass::Yuv,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                1,
                0,
            ),
            Self::Uyvy422 => (
                "uyvy422",
                PixelFormatClass::Yuv,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                1,
                0,
            ),
            Self::Yvyu422 => (
                "yvyu422",
                PixelFormatClass::Yuv,
                3,
                16,
                1,
                false,
                false,
                Some(2),
                1,
                0,
            ),
            Self::Nv12 => (
                "nv12",
                PixelFormatClass::Yuv,
                3,
                12,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Nv21 => (
                "nv21",
                PixelFormatClass::Yuv,
                3,
                12,
                2,
                true,
                false,
                None,
                1,
                1,
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
            bits_per_component: if matches!(self, Self::MonoWhite | Self::MonoBlack) {
                1
            } else if matches!(
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
                    | Self::Gbrp16Le
                    | Self::Gbrp16Be
                    | Self::Gbrap16Le
                    | Self::Gbrap16Be
                    | Self::GbrapF16Le
                    | Self::GbrapF16Be
            ) {
                16
            } else if matches!(self, Self::Rgb8 | Self::Bgr8) {
                3
            } else if matches!(
                self,
                Self::Rgb4 | Self::Bgr4 | Self::Rgb4Byte | Self::Bgr4Byte
            ) {
                2
            } else if matches!(
                self,
                Self::Rgb565Be | Self::Rgb565Le | Self::Bgr565Be | Self::Bgr565Le
            ) {
                6
            } else if matches!(
                self,
                Self::Rgb555Be | Self::Rgb555Le | Self::Bgr555Be | Self::Bgr555Le
            ) {
                5
            } else if matches!(
                self,
                Self::Rgb444Le | Self::Rgb444Be | Self::Bgr444Le | Self::Bgr444Be
            ) {
                4
            } else if matches!(
                self,
                Self::Gray9Le | Self::Gray9Be | Self::Gbrp9Le | Self::Gbrp9Be
            ) {
                9
            } else if matches!(
                self,
                Self::Gray10Le
                    | Self::Gray10Be
                    | Self::Gbrp10Le
                    | Self::Gbrp10Be
                    | Self::Gbrap10Le
                    | Self::Gbrap10Be
            ) {
                10
            } else if matches!(
                self,
                Self::Gray12Le
                    | Self::Gray12Be
                    | Self::Gbrp12Le
                    | Self::Gbrp12Be
                    | Self::Gbrap12Le
                    | Self::Gbrap12Be
            ) {
                12
            } else if matches!(
                self,
                Self::Gray14Le
                    | Self::Gray14Be
                    | Self::Gbrp14Le
                    | Self::Gbrp14Be
                    | Self::Gbrap14Le
                    | Self::Gbrap14Be
            ) {
                14
            } else if matches!(
                self,
                Self::Gray32Le
                    | Self::Gray32Be
                    | Self::GrayF32Le
                    | Self::GrayF32Be
                    | Self::Gbrap32Le
                    | Self::Gbrap32Be
                    | Self::GbrapF32Le
                    | Self::GbrapF32Be
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
                Self::GrayF16Le
                    | Self::GrayF16Be
                    | Self::GrayF32Le
                    | Self::GrayF32Be
                    | Self::GbrapF16Le
                    | Self::GbrapF16Be
                    | Self::GbrapF32Le
                    | Self::GbrapF32Be
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
            Self::MonoWhite | Self::MonoBlack => Ok(vec![checked_mul(
                one_bit_line_size(width),
                height,
                "1-bit monochrome bitstream pixel format frame size",
            )?]),
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
            Self::Gray9Le
            | Self::Gray9Be
            | Self::Gray10Le
            | Self::Gray10Be
            | Self::Gray12Le
            | Self::Gray12Be
            | Self::Gray14Le
            | Self::Gray14Be => Ok(vec![checked_mul(
                pixels,
                2,
                "high bit-depth gray pixel format frame size",
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
            Self::Rgb8 | Self::Bgr8 | Self::Rgb4Byte | Self::Bgr4Byte => Ok(vec![pixels]),
            Self::Rgb4 | Self::Bgr4 => Ok(vec![checked_mul(
                nibble_line_size(width),
                height,
                "4-bit RGB bitstream pixel format frame size",
            )?]),
            Self::Rgb565Be
            | Self::Rgb565Le
            | Self::Rgb555Be
            | Self::Rgb555Le
            | Self::Bgr565Be
            | Self::Bgr565Le
            | Self::Bgr555Be
            | Self::Bgr555Le
            | Self::Rgb444Le
            | Self::Rgb444Be
            | Self::Bgr444Le
            | Self::Bgr444Be => Ok(vec![checked_mul(
                pixels,
                2,
                "16-bit packed RGB pixel format frame size",
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
            Self::Gbrp9Le
            | Self::Gbrp9Be
            | Self::Gbrp10Le
            | Self::Gbrp10Be
            | Self::Gbrp12Le
            | Self::Gbrp12Be
            | Self::Gbrp14Le
            | Self::Gbrp14Be
            | Self::Gbrp16Le
            | Self::Gbrp16Be => {
                let plane = checked_mul(
                    pixels,
                    2,
                    "high bit-depth planar GBR pixel format plane size",
                )?;
                Ok(vec![plane, plane, plane])
            }
            Self::Gbrap => Ok(vec![pixels, pixels, pixels, pixels]),
            Self::Gbrap10Le
            | Self::Gbrap10Be
            | Self::Gbrap12Le
            | Self::Gbrap12Be
            | Self::Gbrap14Le
            | Self::Gbrap14Be
            | Self::Gbrap16Le
            | Self::Gbrap16Be
            | Self::GbrapF16Le
            | Self::GbrapF16Be => {
                let plane = checked_mul(
                    pixels,
                    2,
                    "high bit-depth planar GBRA pixel format plane size",
                )?;
                Ok(vec![plane, plane, plane, plane])
            }
            Self::Gbrap32Le | Self::Gbrap32Be | Self::GbrapF32Le | Self::GbrapF32Be => {
                let plane = checked_mul(pixels, 4, "32-bit planar GBRA pixel format plane size")?;
                Ok(vec![plane, plane, plane, plane])
            }
            Self::Yuyv422 | Self::Uyvy422 | Self::Yvyu422 => {
                if width % 2 != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by 2",
                        self.name()
                    )));
                }
                Ok(vec![checked_mul(
                    pixels,
                    2,
                    "packed YUV 4:2:2 pixel format frame size",
                )?])
            }
            Self::Nv12 | Self::Nv21 => {
                if width % 2 != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by 2",
                        self.name()
                    )));
                }
                if height % 2 != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format height must be divisible by 2",
                        self.name()
                    )));
                }
                let chroma = checked_area(
                    width,
                    height >> 1,
                    "semi-planar 4:2:0 YUV chroma plane area",
                )?;
                Ok(vec![pixels, chroma])
            }
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

fn nibble_line_size(width: usize) -> usize {
    (width / 2) + (width % 2)
}

fn one_bit_line_size(width: usize) -> usize {
    width.div_ceil(8)
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
        for (name, format) in [
            ("monow", PixelFormat::MonoWhite),
            ("monob", PixelFormat::MonoBlack),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_gray());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), None);
        }
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
        for (name, format) in [
            ("rgb8", PixelFormat::Rgb8),
            ("bgr8", PixelFormat::Bgr8),
            ("rgb4_byte", PixelFormat::Rgb4Byte),
            ("bgr4_byte", PixelFormat::Bgr4Byte),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_rgb());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(1));
        }
        for (name, format) in [("rgb4", PixelFormat::Rgb4), ("bgr4", PixelFormat::Bgr4)] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_rgb());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), None);
        }
        for (name, format) in [
            ("rgb565be", PixelFormat::Rgb565Be),
            ("rgb565le", PixelFormat::Rgb565Le),
            ("rgb555be", PixelFormat::Rgb555Be),
            ("rgb555le", PixelFormat::Rgb555Le),
            ("bgr565be", PixelFormat::Bgr565Be),
            ("bgr565le", PixelFormat::Bgr565Le),
            ("bgr555be", PixelFormat::Bgr555Be),
            ("bgr555le", PixelFormat::Bgr555Le),
            ("rgb444le", PixelFormat::Rgb444Le),
            ("rgb444be", PixelFormat::Rgb444Be),
            ("bgr444le", PixelFormat::Bgr444Le),
            ("bgr444be", PixelFormat::Bgr444Be),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(2));
        }
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
            PixelFormat::from_name("gbrp9le"),
            Some(PixelFormat::Gbrp9Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp9be"),
            Some(PixelFormat::Gbrp9Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp10le"),
            Some(PixelFormat::Gbrp10Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp10be"),
            Some(PixelFormat::Gbrp10Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp12le"),
            Some(PixelFormat::Gbrp12Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp12be"),
            Some(PixelFormat::Gbrp12Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp14le"),
            Some(PixelFormat::Gbrp14Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp14be"),
            Some(PixelFormat::Gbrp14Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp16le"),
            Some(PixelFormat::Gbrp16Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp16be"),
            Some(PixelFormat::Gbrp16Be)
        );
        assert_eq!(PixelFormat::from_name("gbrap"), Some(PixelFormat::Gbrap));
        assert_eq!(
            PixelFormat::from_name("gbrap10le"),
            Some(PixelFormat::Gbrap10Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap10be"),
            Some(PixelFormat::Gbrap10Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap12le"),
            Some(PixelFormat::Gbrap12Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap12be"),
            Some(PixelFormat::Gbrap12Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap14le"),
            Some(PixelFormat::Gbrap14Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap14be"),
            Some(PixelFormat::Gbrap14Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap16le"),
            Some(PixelFormat::Gbrap16Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap16be"),
            Some(PixelFormat::Gbrap16Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap32le"),
            Some(PixelFormat::Gbrap32Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrap32be"),
            Some(PixelFormat::Gbrap32Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrapf16le"),
            Some(PixelFormat::GbrapF16Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrapf16be"),
            Some(PixelFormat::GbrapF16Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrapf32le"),
            Some(PixelFormat::GbrapF32Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrapf32be"),
            Some(PixelFormat::GbrapF32Be)
        );
        for (name, format) in [
            ("yuyv422", PixelFormat::Yuyv422),
            ("uyvy422", PixelFormat::Uyvy422),
            ("yvyu422", PixelFormat::Yvyu422),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_yuv());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(2));
        }
        for (name, format) in [("nv12", PixelFormat::Nv12), ("nv21", PixelFormat::Nv21)] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 2);
            assert!(format.is_yuv());
            assert!(format.is_planar());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), None);
        }
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
        assert_eq!(PixelFormat::ALL.len(), 95);
        assert_eq!(PixelFormat::Ya8.plane_count(), 1);
        assert_eq!(PixelFormat::Ya16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray10Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray32Le.plane_count(), 1);
        assert_eq!(PixelFormat::GrayF16Le.plane_count(), 1);
        assert_eq!(PixelFormat::GrayF32Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgba.plane_count(), 1);
        assert_eq!(PixelFormat::Rgb48Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgba64Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgb0.plane_count(), 1);
        assert_eq!(PixelFormat::Yuyv422.plane_count(), 1);
        assert_eq!(PixelFormat::Uyvy422.plane_count(), 1);
        assert_eq!(PixelFormat::Yvyu422.plane_count(), 1);
        assert_eq!(PixelFormat::Nv12.plane_count(), 2);
        assert_eq!(PixelFormat::Nv21.plane_count(), 2);
        assert_eq!(PixelFormat::Yuv420p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv422p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv410p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv411p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv440p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv444p.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp9Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp9Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp10Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp10Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp12Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp12Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp14Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp14Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp16Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp16Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrap.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap10Le.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap10Be.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap12Le.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap12Be.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap14Le.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap14Be.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap16Le.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap16Be.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap32Le.plane_count(), 4);
        assert_eq!(PixelFormat::Gbrap32Be.plane_count(), 4);
        assert_eq!(PixelFormat::GbrapF16Le.plane_count(), 4);
        assert_eq!(PixelFormat::GbrapF16Be.plane_count(), 4);
        assert_eq!(PixelFormat::GbrapF32Le.plane_count(), 4);
        assert_eq!(PixelFormat::GbrapF32Be.plane_count(), 4);
        assert!(!PixelFormat::Rgb24.is_planar());
        assert!(PixelFormat::Rgb24.is_packed());
        assert!(PixelFormat::MonoWhite.is_packed());
        assert!(PixelFormat::Rgb8.is_packed());
        assert!(PixelFormat::Rgb4.is_packed());
        assert!(PixelFormat::Bgr4Byte.is_packed());
        assert!(PixelFormat::Ya8.is_packed());
        assert!(PixelFormat::Ya16Be.is_packed());
        assert!(PixelFormat::Gray16Be.is_packed());
        assert!(PixelFormat::Gray32Be.is_packed());
        assert!(PixelFormat::GrayF16Be.is_packed());
        assert!(PixelFormat::GrayF32Be.is_packed());
        assert!(PixelFormat::Bgr48Be.is_packed());
        assert!(PixelFormat::Bgra64Be.is_packed());
        assert!(PixelFormat::Rgb0.is_packed());
        assert!(PixelFormat::Yuyv422.is_packed());
        assert!(PixelFormat::Uyvy422.is_packed());
        assert!(PixelFormat::Yvyu422.is_packed());
        assert!(PixelFormat::Nv12.is_planar());
        assert!(PixelFormat::Nv21.is_planar());
        assert!(PixelFormat::Yuv420p.is_planar());
        assert!(PixelFormat::Yuv422p.is_planar());
        assert!(PixelFormat::Yuv410p.is_planar());
        assert!(PixelFormat::Yuv411p.is_planar());
        assert!(PixelFormat::Yuv440p.is_planar());
        assert!(PixelFormat::Yuv444p.is_planar());
        assert!(PixelFormat::Gbrp.is_planar());
        assert!(PixelFormat::Gbrp9Le.is_planar());
        assert!(PixelFormat::Gbrp10Le.is_planar());
        assert!(PixelFormat::Gbrp12Le.is_planar());
        assert!(PixelFormat::Gbrp14Le.is_planar());
        assert!(PixelFormat::Gbrp16Le.is_planar());
        assert!(PixelFormat::Gbrap.is_planar());
        assert!(PixelFormat::Gbrap16Le.is_planar());
        assert!(PixelFormat::Gbrap32Le.is_planar());
        assert!(PixelFormat::GbrapF16Le.is_planar());
        assert!(PixelFormat::GbrapF32Le.is_planar());
        assert!(!PixelFormat::Yuv420p.is_packed());
        assert!(!PixelFormat::Gbrp.is_packed());
        assert!(!PixelFormat::Gbrp9Le.is_packed());
        assert!(!PixelFormat::Gbrp10Le.is_packed());
        assert!(!PixelFormat::Gbrp12Le.is_packed());
        assert!(!PixelFormat::Gbrp14Le.is_packed());
        assert!(!PixelFormat::Gbrp16Le.is_packed());
        assert!(!PixelFormat::Gbrap.is_packed());
        assert!(!PixelFormat::Gbrap16Le.is_packed());
        assert!(!PixelFormat::Gbrap32Le.is_packed());
        assert!(!PixelFormat::GbrapF16Le.is_packed());
        assert!(!PixelFormat::GbrapF32Le.is_packed());
        assert!(!PixelFormat::Rgb24.has_alpha());
        assert!(PixelFormat::Ya8.has_alpha());
        assert!(PixelFormat::Ya16Le.has_alpha());
        assert!(PixelFormat::Bgra.has_alpha());
        assert!(PixelFormat::Rgba64Le.has_alpha());
        assert!(PixelFormat::Gbrap.has_alpha());
        assert!(PixelFormat::Gbrap16Le.has_alpha());
        assert!(PixelFormat::GbrapF32Le.has_alpha());
        assert!(!PixelFormat::ZeroRgb.has_alpha());
        assert!(!PixelFormat::Gray16Le.is_float());
        assert!(PixelFormat::GrayF16Le.is_float());
        assert!(PixelFormat::GrayF32Be.is_float());
        assert!(!PixelFormat::Gbrap16Le.is_float());
        assert!(PixelFormat::GbrapF16Le.is_float());
        assert!(PixelFormat::GbrapF32Be.is_float());
        assert_eq!(PixelFormat::MonoWhite.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Bgr24.packed_bytes_per_pixel(), Some(3));
        assert_eq!(PixelFormat::Rgb8.packed_bytes_per_pixel(), Some(1));
        assert_eq!(PixelFormat::Rgb4.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Bgr4Byte.packed_bytes_per_pixel(), Some(1));
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
        assert_eq!(PixelFormat::Yuyv422.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::Uyvy422.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::Yvyu422.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::Nv12.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv21.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrp.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrp9Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrp10Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrp12Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrp14Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrp16Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrap.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrap16Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Gbrap32Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::GbrapF16Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::GbrapF32Le.packed_bytes_per_pixel(), None);
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

        for format in [PixelFormat::MonoWhite, PixelFormat::MonoBlack] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Gray);
            assert!(format.is_gray());
            assert_eq!(descriptor.component_count, 1);
            assert_eq!(descriptor.bits_per_component, 1);
            assert_eq!(descriptor.bits_per_pixel, 1);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), (0, 0));
        }

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

        for (format, expected_name, expected_alias, expected_bits_per_component) in [
            (PixelFormat::Gray9Le, "gray9le", Some("y9le"), 9),
            (PixelFormat::Gray9Be, "gray9be", Some("y9be"), 9),
            (PixelFormat::Gray10Le, "gray10le", Some("y10le"), 10),
            (PixelFormat::Gray10Be, "gray10be", Some("y10be"), 10),
            (PixelFormat::Gray12Le, "gray12le", Some("y12le"), 12),
            (PixelFormat::Gray12Be, "gray12be", Some("y12be"), 12),
            (PixelFormat::Gray14Le, "gray14le", Some("y14le"), 14),
            (PixelFormat::Gray14Be, "gray14be", Some("y14be"), 14),
            (PixelFormat::Gray16Le, "gray16le", None, 16),
            (PixelFormat::Gray16Be, "gray16be", None, 16),
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
            (PixelFormat::Rgb8, 3),
            (PixelFormat::Bgr8, 3),
            (PixelFormat::Rgb4, 2),
            (PixelFormat::Bgr4, 2),
            (PixelFormat::Rgb4Byte, 2),
            (PixelFormat::Bgr4Byte, 2),
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

        for (format, expected_name, expected_bits_per_component) in [
            (PixelFormat::Rgb565Be, "rgb565be", 6),
            (PixelFormat::Rgb565Le, "rgb565le", 6),
            (PixelFormat::Rgb555Be, "rgb555be", 5),
            (PixelFormat::Rgb555Le, "rgb555le", 5),
            (PixelFormat::Bgr565Be, "bgr565be", 6),
            (PixelFormat::Bgr565Le, "bgr565le", 6),
            (PixelFormat::Bgr555Be, "bgr555be", 5),
            (PixelFormat::Bgr555Le, "bgr555le", 5),
            (PixelFormat::Rgb444Le, "rgb444le", 4),
            (PixelFormat::Rgb444Be, "rgb444be", 4),
            (PixelFormat::Bgr444Le, "bgr444le", 4),
            (PixelFormat::Bgr444Be, "bgr444be", 4),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert!(format.is_rgb());
            assert!(!format.is_gray());
            assert!(!format.is_yuv());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, 16);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(2));
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

        for (format, name, expected_bits_per_component, expected_bits_per_pixel) in [
            (PixelFormat::Gbrp9Le, "gbrp9le", 9, 27),
            (PixelFormat::Gbrp9Be, "gbrp9be", 9, 27),
            (PixelFormat::Gbrp10Le, "gbrp10le", 10, 30),
            (PixelFormat::Gbrp10Be, "gbrp10be", 10, 30),
            (PixelFormat::Gbrp12Le, "gbrp12le", 12, 36),
            (PixelFormat::Gbrp12Be, "gbrp12be", 12, 36),
            (PixelFormat::Gbrp14Le, "gbrp14le", 14, 42),
            (PixelFormat::Gbrp14Be, "gbrp14be", 14, 42),
            (PixelFormat::Gbrp16Le, "gbrp16le", 16, 48),
            (PixelFormat::Gbrp16Be, "gbrp16be", 16, 48),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert!(format.is_rgb());
            assert!(!format.is_yuv());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(descriptor.plane_count, 3);
            assert!(descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }

        let gbrap = PixelFormat::Gbrap.descriptor();
        assert_eq!(gbrap.format, PixelFormat::Gbrap);
        assert_eq!(gbrap.name, "gbrap");
        assert_eq!(PixelFormat::from_name(gbrap.name), Some(PixelFormat::Gbrap));
        assert_eq!(gbrap.class, PixelFormatClass::Rgb);
        assert!(PixelFormat::Gbrap.is_rgb());
        assert_eq!(gbrap.component_count, 4);
        assert_eq!(gbrap.bits_per_component, 8);
        assert_eq!(gbrap.bits_per_pixel, 32);
        assert_eq!(gbrap.plane_count, 4);
        assert!(gbrap.is_planar);
        assert!(gbrap.has_alpha);
        assert!(!gbrap.is_float);
        assert_eq!(gbrap.packed_bytes_per_pixel, None);
        assert_eq!(PixelFormat::Gbrap.log2_chroma(), (0, 0));
        assert!(!PixelFormat::Gbrap.has_chroma_subsampling());

        for (format, name, expected_bits_per_component, expected_bits_per_pixel, is_float) in [
            (PixelFormat::Gbrap10Le, "gbrap10le", 10, 40, false),
            (PixelFormat::Gbrap10Be, "gbrap10be", 10, 40, false),
            (PixelFormat::Gbrap12Le, "gbrap12le", 12, 48, false),
            (PixelFormat::Gbrap12Be, "gbrap12be", 12, 48, false),
            (PixelFormat::Gbrap14Le, "gbrap14le", 14, 56, false),
            (PixelFormat::Gbrap14Be, "gbrap14be", 14, 56, false),
            (PixelFormat::Gbrap16Le, "gbrap16le", 16, 64, false),
            (PixelFormat::Gbrap16Be, "gbrap16be", 16, 64, false),
            (PixelFormat::Gbrap32Le, "gbrap32le", 32, 128, false),
            (PixelFormat::Gbrap32Be, "gbrap32be", 32, 128, false),
            (PixelFormat::GbrapF16Le, "gbrapf16le", 16, 64, true),
            (PixelFormat::GbrapF16Be, "gbrapf16be", 16, 64, true),
            (PixelFormat::GbrapF32Le, "gbrapf32le", 32, 128, true),
            (PixelFormat::GbrapF32Be, "gbrapf32be", 32, 128, true),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert!(format.is_rgb());
            assert!(!format.is_yuv());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 4);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(descriptor.plane_count, 4);
            assert!(descriptor.is_planar);
            assert!(descriptor.has_alpha);
            assert_eq!(descriptor.is_float, is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }

        for format in [
            PixelFormat::Yuyv422,
            PixelFormat::Uyvy422,
            PixelFormat::Yvyu422,
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, 8);
            assert_eq!(descriptor.bits_per_pixel, 16);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(2));
            assert_eq!(format.log2_chroma(), (1, 0));
            assert!(format.has_chroma_subsampling());
        }

        for format in [PixelFormat::Nv12, PixelFormat::Nv21] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, 8);
            assert_eq!(descriptor.bits_per_pixel, 12);
            assert_eq!(descriptor.plane_count, 2);
            assert!(descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), (1, 1));
            assert!(format.has_chroma_subsampling());
        }

        assert_eq!(PixelFormat::Rgb24.component_count(), 3);
        assert_eq!(PixelFormat::Rgb24.bits_per_pixel(), 24);
        assert_eq!(PixelFormat::Rgb8.component_count(), 3);
        assert_eq!(PixelFormat::Rgb8.bits_per_component(), 3);
        assert_eq!(PixelFormat::Bgr8.bits_per_pixel(), 8);
        assert_eq!(PixelFormat::Rgb4.bits_per_component(), 2);
        assert_eq!(PixelFormat::Bgr4.bits_per_pixel(), 4);
        assert_eq!(PixelFormat::Rgb4Byte.bits_per_component(), 2);
        assert_eq!(PixelFormat::Bgr4Byte.bits_per_pixel(), 8);
        assert_eq!(PixelFormat::Gbrp.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp.bits_per_component(), 8);
        assert_eq!(PixelFormat::Gbrp.bits_per_pixel(), 24);
        assert_eq!(PixelFormat::Gbrp9Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp9Le.bits_per_component(), 9);
        assert_eq!(PixelFormat::Gbrp9Le.bits_per_pixel(), 27);
        assert_eq!(PixelFormat::Gbrp10Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp10Le.bits_per_component(), 10);
        assert_eq!(PixelFormat::Gbrp10Le.bits_per_pixel(), 30);
        assert_eq!(PixelFormat::Gbrp12Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp12Le.bits_per_component(), 12);
        assert_eq!(PixelFormat::Gbrp12Le.bits_per_pixel(), 36);
        assert_eq!(PixelFormat::Gbrp14Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp14Le.bits_per_component(), 14);
        assert_eq!(PixelFormat::Gbrp14Le.bits_per_pixel(), 42);
        assert_eq!(PixelFormat::Gbrp16Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Gbrp16Le.bits_per_pixel(), 48);
        assert_eq!(PixelFormat::Gbrap.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap.bits_per_component(), 8);
        assert_eq!(PixelFormat::Gbrap.bits_per_pixel(), 32);
        assert_eq!(PixelFormat::Gbrap10Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap10Le.bits_per_component(), 10);
        assert_eq!(PixelFormat::Gbrap10Le.bits_per_pixel(), 40);
        assert_eq!(PixelFormat::Gbrap12Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap12Le.bits_per_component(), 12);
        assert_eq!(PixelFormat::Gbrap12Le.bits_per_pixel(), 48);
        assert_eq!(PixelFormat::Gbrap14Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap14Le.bits_per_component(), 14);
        assert_eq!(PixelFormat::Gbrap14Le.bits_per_pixel(), 56);
        assert_eq!(PixelFormat::Gbrap16Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Gbrap16Le.bits_per_pixel(), 64);
        assert_eq!(PixelFormat::Gbrap32Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::Gbrap32Le.bits_per_pixel(), 128);
        assert_eq!(PixelFormat::GbrapF16Le.component_count(), 4);
        assert_eq!(PixelFormat::GbrapF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::GbrapF16Le.bits_per_pixel(), 64);
        assert_eq!(PixelFormat::GbrapF32Le.component_count(), 4);
        assert_eq!(PixelFormat::GbrapF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::GbrapF32Le.bits_per_pixel(), 128);
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
        assert_eq!(PixelFormat::Rgb565Le.component_count(), 3);
        assert_eq!(PixelFormat::Rgb565Le.bits_per_component(), 6);
        assert_eq!(PixelFormat::Rgb565Le.bits_per_pixel(), 16);
        assert_eq!(PixelFormat::Bgr555Be.bits_per_component(), 5);
        assert_eq!(PixelFormat::Bgr555Be.bits_per_pixel(), 16);
        assert_eq!(PixelFormat::Rgb444Le.bits_per_component(), 4);
        assert_eq!(PixelFormat::Bgr444Be.bits_per_component(), 4);
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
        assert_eq!(PixelFormat::Gray9Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Gray9Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Gray10Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Gray10Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Gray12Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Gray12Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Gray14Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Gray14Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Gray16Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Gray16Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Gray32Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::Gray32Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::GrayF16Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::GrayF16Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::GrayF32Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::GrayF32Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::MonoWhite.plane_sizes(9, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::MonoBlack.frame_size(16, 2).unwrap(), 4);
        assert_eq!(PixelFormat::Rgb24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Bgr24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Rgb8.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Bgr8.frame_size(2, 2).unwrap(), 4);
        assert_eq!(PixelFormat::Rgb4.plane_sizes(3, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Bgr4.frame_size(4, 1).unwrap(), 2);
        assert_eq!(PixelFormat::Rgb4Byte.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Bgr4Byte.frame_size(2, 2).unwrap(), 4);
        assert_eq!(PixelFormat::Rgb565Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Rgb565Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Bgr555Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Rgb444Le.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Bgr444Be.frame_size(2, 2).unwrap(), 8);
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
        assert_eq!(PixelFormat::Yuyv422.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Uyvy422.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Yvyu422.frame_size(4, 1).unwrap(), 8);
        assert_eq!(PixelFormat::Nv12.plane_sizes(4, 2).unwrap(), vec![8, 4]);
        assert_eq!(PixelFormat::Nv12.frame_size(4, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Nv21.plane_sizes(2, 4).unwrap(), vec![8, 4]);
        assert_eq!(PixelFormat::Nv21.frame_size(2, 4).unwrap(), 12);
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
        assert_eq!(
            PixelFormat::Gbrp9Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp9Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrp10Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp10Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrp12Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp12Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrp14Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp14Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrp16Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp16Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrap.plane_sizes(3, 2).unwrap(),
            vec![6, 6, 6, 6]
        );
        assert_eq!(PixelFormat::Gbrap.frame_size(3, 2).unwrap(), 24);
        assert_eq!(
            PixelFormat::Gbrap10Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrap10Le.frame_size(3, 2).unwrap(), 48);
        assert_eq!(
            PixelFormat::Gbrap12Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrap12Le.frame_size(3, 2).unwrap(), 48);
        assert_eq!(
            PixelFormat::Gbrap14Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrap14Le.frame_size(3, 2).unwrap(), 48);
        assert_eq!(
            PixelFormat::Gbrap16Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrap16Le.frame_size(3, 2).unwrap(), 48);
        assert_eq!(
            PixelFormat::Gbrap32Le.plane_sizes(3, 2).unwrap(),
            vec![24, 24, 24, 24]
        );
        assert_eq!(PixelFormat::Gbrap32Le.frame_size(3, 2).unwrap(), 96);
        assert_eq!(
            PixelFormat::GbrapF16Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12, 12]
        );
        assert_eq!(PixelFormat::GbrapF16Le.frame_size(3, 2).unwrap(), 48);
        assert_eq!(
            PixelFormat::GbrapF32Le.plane_sizes(3, 2).unwrap(),
            vec![24, 24, 24, 24]
        );
        assert_eq!(PixelFormat::GbrapF32Le.frame_size(3, 2).unwrap(), 96);
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

        let planes = PixelFormat::Gbrp9Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
        );

        let planes = PixelFormat::Gbrp10Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
        );

        let planes = PixelFormat::Gbrp12Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
        );

        let planes = PixelFormat::Gbrp14Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
        );

        let planes = PixelFormat::Gbrp16Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
        );

        let planes = PixelFormat::Gbrap
            .split_planes(&(0..8).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1], vec![2, 3], vec![4, 5], vec![6, 7]]);

        let planes = PixelFormat::Gbrap16Le
            .split_planes(&(0..16).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                vec![0, 1, 2, 3],
                vec![4, 5, 6, 7],
                vec![8, 9, 10, 11],
                vec![12, 13, 14, 15]
            ]
        );

        let planes = PixelFormat::GbrapF32Le
            .split_planes(&(0..32).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..16).collect::<Vec<_>>(),
                (16..24).collect::<Vec<_>>(),
                (24..32).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Rgb48Le
            .split_planes(&[0, 1, 2, 3, 4, 5], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3, 4, 5]]);

        let planes = PixelFormat::Rgb565Le
            .split_planes(&[0, 1, 2, 3], 2, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Rgb8.split_planes(&[0, 1, 2, 3], 2, 2).unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Rgb4.split_planes(&[0, 1, 2, 3], 3, 2).unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Bgr4.split_planes(&[4, 5], 4, 1).unwrap();

        assert_eq!(planes, vec![vec![4, 5]]);

        let planes = PixelFormat::MonoWhite
            .split_planes(&[0x80, 0x01, 0xff, 0x00], 9, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0x80, 0x01, 0xff, 0x00]]);

        let planes = PixelFormat::MonoBlack
            .split_planes(&[0xaa, 0x55], 16, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0xaa, 0x55]]);

        let planes = PixelFormat::Bgr4Byte
            .split_planes(&[4, 5, 6, 7], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![4, 5, 6, 7]]);

        let planes = PixelFormat::Bgr444Be.split_planes(&[0, 1], 1, 1).unwrap();

        assert_eq!(planes, vec![vec![0, 1]]);

        let planes = PixelFormat::Yuyv422
            .split_planes(&[0, 1, 2, 3], 2, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Yvyu422
            .split_planes(&[4, 5, 6, 7], 2, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![4, 5, 6, 7]]);

        let planes = PixelFormat::Nv12
            .split_planes(&(0..12).collect::<Vec<_>>(), 4, 2)
            .unwrap();

        assert_eq!(planes, vec![(0..8).collect::<Vec<_>>(), (8..12).collect()]);

        let planes = PixelFormat::Nv21
            .split_planes(&(12..24).collect::<Vec<_>>(), 4, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![(12..20).collect::<Vec<_>>(), (20..24).collect()]
        );

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

        let planes = PixelFormat::Gray10Le
            .split_planes(&[0x01, 0x02, 0x03, 0x04], 2, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0x01, 0x02, 0x03, 0x04]]);

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
            PixelFormat::Yuyv422.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Uyvy422.frame_size(2, 3).unwrap(), 12);
        assert_eq!(
            PixelFormat::Yvyu422
                .split_planes(&[0; 3], 2, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Nv12.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Nv21.frame_size(2, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Nv12
                .split_planes(&[0; 11], 4, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Rgb24
                .split_planes(&[0; 5], 1, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }
}
