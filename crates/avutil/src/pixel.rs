use crate::{AvError, AvResult, Rational};

pub const AVPALETTE_COUNT: usize = 256;
pub const AVPALETTE_SIZE: usize = AVPALETTE_COUNT * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Gray8,
    MonoWhite,
    MonoBlack,
    Pal8,
    Ya8,
    Ya16Le,
    Ya16Be,
    Yaf16Le,
    Yaf16Be,
    Yaf32Le,
    Yaf32Be,
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
    BayerBggr8,
    BayerRggb8,
    BayerGbrg8,
    BayerGrbg8,
    BayerBggr16Le,
    BayerBggr16Be,
    BayerRggb16Le,
    BayerRggb16Be,
    BayerGbrg16Le,
    BayerGbrg16Be,
    BayerGrbg16Le,
    BayerGrbg16Be,
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
    RgbF16Le,
    RgbF16Be,
    RgbF32Le,
    RgbF32Be,
    Rgb96Le,
    Rgb96Be,
    RgbaF16Le,
    RgbaF16Be,
    RgbaF32Le,
    RgbaF32Be,
    Rgba128Le,
    Rgba128Be,
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
    X2Rgb10Le,
    X2Rgb10Be,
    X2Bgr10Le,
    X2Bgr10Be,
    Gbrp,
    Gbrp9Le,
    Gbrp9Be,
    Gbrp10Le,
    Gbrp10Be,
    Gbrp10MsbLe,
    Gbrp10MsbBe,
    Gbrp12Le,
    Gbrp12Be,
    Gbrp12MsbLe,
    Gbrp12MsbBe,
    Gbrp14Le,
    Gbrp14Be,
    Gbrp16Le,
    Gbrp16Be,
    GbrpF16Le,
    GbrpF16Be,
    GbrpF32Le,
    GbrpF32Be,
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
    Ayuv64Le,
    Ayuv64Be,
    Vuya,
    Vuyx,
    Xv30Le,
    Xv30Be,
    Xv36Le,
    Xv36Be,
    Xv48Le,
    Xv48Be,
    V30xLe,
    V30xBe,
    Ayuv,
    Uyva,
    Vyu444,
    Xyz12Le,
    Xyz12Be,
    Yuyv422,
    Uyvy422,
    Yvyu422,
    Uyyvyy411,
    Y210Le,
    Y210Be,
    Y212Le,
    Y212Be,
    Y216Le,
    Y216Be,
    Nv12,
    Nv21,
    Nv16,
    Nv20Le,
    Nv20Be,
    Nv24,
    Nv42,
    P010Le,
    P010Be,
    P012Le,
    P012Be,
    P016Le,
    P016Be,
    P210Le,
    P210Be,
    P212Le,
    P212Be,
    P216Le,
    P216Be,
    P410Le,
    P410Be,
    P412Le,
    P412Be,
    P416Le,
    P416Be,
    Yuv420p,
    YuvJ420p,
    Yuv422p,
    YuvJ422p,
    Yuv410p,
    Yuv411p,
    YuvJ411p,
    Yuv440p,
    YuvJ440p,
    Yuv444p,
    YuvJ444p,
    Yuva420p,
    Yuva422p,
    Yuva444p,
    Yuva420p9Le,
    Yuva420p9Be,
    Yuva422p9Le,
    Yuva422p9Be,
    Yuva444p9Le,
    Yuva444p9Be,
    Yuva420p10Le,
    Yuva420p10Be,
    Yuva422p10Le,
    Yuva422p10Be,
    Yuva444p10Le,
    Yuva444p10Be,
    Yuva422p12Le,
    Yuva422p12Be,
    Yuva444p12Le,
    Yuva444p12Be,
    Yuva420p16Le,
    Yuva420p16Be,
    Yuva422p16Le,
    Yuva422p16Be,
    Yuva444p16Le,
    Yuva444p16Be,
    Yuv440p10Le,
    Yuv440p10Be,
    Yuv440p12Le,
    Yuv440p12Be,
    Yuv420p9Le,
    Yuv420p9Be,
    Yuv422p9Le,
    Yuv422p9Be,
    Yuv444p9Le,
    Yuv444p9Be,
    Yuv420p10Le,
    Yuv420p10Be,
    Yuv422p10Le,
    Yuv422p10Be,
    Yuv444p10Le,
    Yuv444p10Be,
    Yuv444p10MsbLe,
    Yuv444p10MsbBe,
    Yuv420p12Le,
    Yuv420p12Be,
    Yuv422p12Le,
    Yuv422p12Be,
    Yuv444p12Le,
    Yuv444p12Be,
    Yuv444p12MsbLe,
    Yuv444p12MsbBe,
    Yuv420p14Le,
    Yuv420p14Be,
    Yuv422p14Le,
    Yuv422p14Be,
    Yuv444p14Le,
    Yuv444p14Be,
    Yuv420p16Le,
    Yuv420p16Be,
    Yuv422p16Le,
    Yuv422p16Be,
    Yuv444p16Le,
    Yuv444p16Be,
    Vaapi,
    Dxva2Vld,
    Vdpau,
    Qsv,
    Mmal,
    D3d11VaVld,
    Cuda,
    VideoToolboxVld,
    MediaCodec,
    D3d11,
    DrmPrime,
    OpenCl,
    Vulkan,
    D3d12,
    Amf,
    OhCodec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormatClass {
    Gray,
    Rgb,
    Xyz,
    Yuv,
    Hardware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelFormatDescriptor {
    pub format: PixelFormat,
    pub name: &'static str,
    pub class: PixelFormatClass,
    pub component_count: usize,
    pub bits_per_component: u8,
    pub bits_per_pixel: Rational,
    pub plane_count: usize,
    pub is_planar: bool,
    pub has_alpha: bool,
    pub is_float: bool,
    pub is_paletted: bool,
    pub packed_bytes_per_pixel: Option<usize>,
    pub log2_chroma_w: u8,
    pub log2_chroma_h: u8,
}

impl PixelFormatDescriptor {
    pub fn bits_per_pixel_integer(self) -> Option<u8> {
        if self.bits_per_pixel.den() == 1 {
            u8::try_from(self.bits_per_pixel.num()).ok()
        } else {
            None
        }
    }
}

impl PixelFormat {
    pub const ALL: &'static [Self] = &[
        Self::Gray8,
        Self::MonoWhite,
        Self::MonoBlack,
        Self::Pal8,
        Self::Ya8,
        Self::Ya16Le,
        Self::Ya16Be,
        Self::Yaf16Le,
        Self::Yaf16Be,
        Self::Yaf32Le,
        Self::Yaf32Be,
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
        Self::BayerBggr8,
        Self::BayerRggb8,
        Self::BayerGbrg8,
        Self::BayerGrbg8,
        Self::BayerBggr16Le,
        Self::BayerBggr16Be,
        Self::BayerRggb16Le,
        Self::BayerRggb16Be,
        Self::BayerGbrg16Le,
        Self::BayerGbrg16Be,
        Self::BayerGrbg16Le,
        Self::BayerGrbg16Be,
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
        Self::RgbF16Le,
        Self::RgbF16Be,
        Self::RgbF32Le,
        Self::RgbF32Be,
        Self::Rgb96Le,
        Self::Rgb96Be,
        Self::RgbaF16Le,
        Self::RgbaF16Be,
        Self::RgbaF32Le,
        Self::RgbaF32Be,
        Self::Rgba128Le,
        Self::Rgba128Be,
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
        Self::X2Rgb10Le,
        Self::X2Rgb10Be,
        Self::X2Bgr10Le,
        Self::X2Bgr10Be,
        Self::Gbrp,
        Self::Gbrp9Le,
        Self::Gbrp9Be,
        Self::Gbrp10Le,
        Self::Gbrp10Be,
        Self::Gbrp10MsbLe,
        Self::Gbrp10MsbBe,
        Self::Gbrp12Le,
        Self::Gbrp12Be,
        Self::Gbrp12MsbLe,
        Self::Gbrp12MsbBe,
        Self::Gbrp14Le,
        Self::Gbrp14Be,
        Self::Gbrp16Le,
        Self::Gbrp16Be,
        Self::GbrpF16Le,
        Self::GbrpF16Be,
        Self::GbrpF32Le,
        Self::GbrpF32Be,
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
        Self::Ayuv64Le,
        Self::Ayuv64Be,
        Self::Vuya,
        Self::Vuyx,
        Self::Xv30Le,
        Self::Xv30Be,
        Self::Xv36Le,
        Self::Xv36Be,
        Self::Xv48Le,
        Self::Xv48Be,
        Self::V30xLe,
        Self::V30xBe,
        Self::Ayuv,
        Self::Uyva,
        Self::Vyu444,
        Self::Xyz12Le,
        Self::Xyz12Be,
        Self::Yuyv422,
        Self::Uyvy422,
        Self::Yvyu422,
        Self::Uyyvyy411,
        Self::Y210Le,
        Self::Y210Be,
        Self::Y212Le,
        Self::Y212Be,
        Self::Y216Le,
        Self::Y216Be,
        Self::Nv12,
        Self::Nv21,
        Self::Nv16,
        Self::Nv20Le,
        Self::Nv20Be,
        Self::Nv24,
        Self::Nv42,
        Self::P010Le,
        Self::P010Be,
        Self::P012Le,
        Self::P012Be,
        Self::P016Le,
        Self::P016Be,
        Self::P210Le,
        Self::P210Be,
        Self::P212Le,
        Self::P212Be,
        Self::P216Le,
        Self::P216Be,
        Self::P410Le,
        Self::P410Be,
        Self::P412Le,
        Self::P412Be,
        Self::P416Le,
        Self::P416Be,
        Self::Yuv420p,
        Self::YuvJ420p,
        Self::Yuv422p,
        Self::YuvJ422p,
        Self::Yuv410p,
        Self::Yuv411p,
        Self::YuvJ411p,
        Self::Yuv440p,
        Self::YuvJ440p,
        Self::Yuv444p,
        Self::YuvJ444p,
        Self::Yuva420p,
        Self::Yuva422p,
        Self::Yuva444p,
        Self::Yuva420p9Le,
        Self::Yuva420p9Be,
        Self::Yuva422p9Le,
        Self::Yuva422p9Be,
        Self::Yuva444p9Le,
        Self::Yuva444p9Be,
        Self::Yuva420p10Le,
        Self::Yuva420p10Be,
        Self::Yuva422p10Le,
        Self::Yuva422p10Be,
        Self::Yuva444p10Le,
        Self::Yuva444p10Be,
        Self::Yuva422p12Le,
        Self::Yuva422p12Be,
        Self::Yuva444p12Le,
        Self::Yuva444p12Be,
        Self::Yuva420p16Le,
        Self::Yuva420p16Be,
        Self::Yuva422p16Le,
        Self::Yuva422p16Be,
        Self::Yuva444p16Le,
        Self::Yuva444p16Be,
        Self::Yuv440p10Le,
        Self::Yuv440p10Be,
        Self::Yuv440p12Le,
        Self::Yuv440p12Be,
        Self::Yuv420p9Le,
        Self::Yuv420p9Be,
        Self::Yuv422p9Le,
        Self::Yuv422p9Be,
        Self::Yuv444p9Le,
        Self::Yuv444p9Be,
        Self::Yuv420p10Le,
        Self::Yuv420p10Be,
        Self::Yuv422p10Le,
        Self::Yuv422p10Be,
        Self::Yuv444p10Le,
        Self::Yuv444p10Be,
        Self::Yuv444p10MsbLe,
        Self::Yuv444p10MsbBe,
        Self::Yuv420p12Le,
        Self::Yuv420p12Be,
        Self::Yuv422p12Le,
        Self::Yuv422p12Be,
        Self::Yuv444p12Le,
        Self::Yuv444p12Be,
        Self::Yuv444p12MsbLe,
        Self::Yuv444p12MsbBe,
        Self::Yuv420p14Le,
        Self::Yuv420p14Be,
        Self::Yuv422p14Le,
        Self::Yuv422p14Be,
        Self::Yuv444p14Le,
        Self::Yuv444p14Be,
        Self::Yuv420p16Le,
        Self::Yuv420p16Be,
        Self::Yuv422p16Le,
        Self::Yuv422p16Be,
        Self::Yuv444p16Le,
        Self::Yuv444p16Be,
    ];

    pub const HARDWARE: &'static [Self] = &[
        Self::Vaapi,
        Self::Dxva2Vld,
        Self::Vdpau,
        Self::Qsv,
        Self::Mmal,
        Self::D3d11VaVld,
        Self::Cuda,
        Self::VideoToolboxVld,
        Self::MediaCodec,
        Self::D3d11,
        Self::DrmPrime,
        Self::OpenCl,
        Self::Vulkan,
        Self::D3d12,
        Self::Amf,
        Self::OhCodec,
    ];

    pub fn name(self) -> &'static str {
        self.descriptor().name
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "gray" | "gray8" => Some(Self::Gray8),
            "monow" => Some(Self::MonoWhite),
            "monob" => Some(Self::MonoBlack),
            "pal8" => Some(Self::Pal8),
            "ya8" | "gray8a" | "y400a" => Some(Self::Ya8),
            "ya16le" => Some(Self::Ya16Le),
            "ya16be" => Some(Self::Ya16Be),
            "yaf16le" => Some(Self::Yaf16Le),
            "yaf16be" => Some(Self::Yaf16Be),
            "yaf32le" => Some(Self::Yaf32Le),
            "yaf32be" => Some(Self::Yaf32Be),
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
            "bayer_bggr8" => Some(Self::BayerBggr8),
            "bayer_rggb8" => Some(Self::BayerRggb8),
            "bayer_gbrg8" => Some(Self::BayerGbrg8),
            "bayer_grbg8" => Some(Self::BayerGrbg8),
            "bayer_bggr16le" => Some(Self::BayerBggr16Le),
            "bayer_bggr16be" => Some(Self::BayerBggr16Be),
            "bayer_rggb16le" => Some(Self::BayerRggb16Le),
            "bayer_rggb16be" => Some(Self::BayerRggb16Be),
            "bayer_gbrg16le" => Some(Self::BayerGbrg16Le),
            "bayer_gbrg16be" => Some(Self::BayerGbrg16Be),
            "bayer_grbg16le" => Some(Self::BayerGrbg16Le),
            "bayer_grbg16be" => Some(Self::BayerGrbg16Be),
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
            "rgbf16le" => Some(Self::RgbF16Le),
            "rgbf16be" => Some(Self::RgbF16Be),
            "rgbf32le" => Some(Self::RgbF32Le),
            "rgbf32be" => Some(Self::RgbF32Be),
            "rgb96le" => Some(Self::Rgb96Le),
            "rgb96be" => Some(Self::Rgb96Be),
            "rgbaf16le" => Some(Self::RgbaF16Le),
            "rgbaf16be" => Some(Self::RgbaF16Be),
            "rgbaf32le" => Some(Self::RgbaF32Le),
            "rgbaf32be" => Some(Self::RgbaF32Be),
            "rgba128le" => Some(Self::Rgba128Le),
            "rgba128be" => Some(Self::Rgba128Be),
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
            "x2rgb10le" => Some(Self::X2Rgb10Le),
            "x2rgb10be" => Some(Self::X2Rgb10Be),
            "x2bgr10le" => Some(Self::X2Bgr10Le),
            "x2bgr10be" => Some(Self::X2Bgr10Be),
            "gbrp" => Some(Self::Gbrp),
            "gbrp9le" => Some(Self::Gbrp9Le),
            "gbrp9be" => Some(Self::Gbrp9Be),
            "gbrp10le" => Some(Self::Gbrp10Le),
            "gbrp10be" => Some(Self::Gbrp10Be),
            "gbrp10msble" => Some(Self::Gbrp10MsbLe),
            "gbrp10msbbe" => Some(Self::Gbrp10MsbBe),
            "gbrp12le" => Some(Self::Gbrp12Le),
            "gbrp12be" => Some(Self::Gbrp12Be),
            "gbrp12msble" => Some(Self::Gbrp12MsbLe),
            "gbrp12msbbe" => Some(Self::Gbrp12MsbBe),
            "gbrp14le" => Some(Self::Gbrp14Le),
            "gbrp14be" => Some(Self::Gbrp14Be),
            "gbrp16le" => Some(Self::Gbrp16Le),
            "gbrp16be" => Some(Self::Gbrp16Be),
            "gbrpf16le" => Some(Self::GbrpF16Le),
            "gbrpf16be" => Some(Self::GbrpF16Be),
            "gbrpf32le" => Some(Self::GbrpF32Le),
            "gbrpf32be" => Some(Self::GbrpF32Be),
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
            "ayuv64le" => Some(Self::Ayuv64Le),
            "ayuv64be" => Some(Self::Ayuv64Be),
            "vuya" => Some(Self::Vuya),
            "vuyx" => Some(Self::Vuyx),
            "xv30le" => Some(Self::Xv30Le),
            "xv30be" => Some(Self::Xv30Be),
            "xv36le" => Some(Self::Xv36Le),
            "xv36be" => Some(Self::Xv36Be),
            "xv48le" => Some(Self::Xv48Le),
            "xv48be" => Some(Self::Xv48Be),
            "v30xle" => Some(Self::V30xLe),
            "v30xbe" => Some(Self::V30xBe),
            "ayuv" => Some(Self::Ayuv),
            "uyva" => Some(Self::Uyva),
            "vyu444" => Some(Self::Vyu444),
            "xyz12le" => Some(Self::Xyz12Le),
            "xyz12be" => Some(Self::Xyz12Be),
            "yuyv422" => Some(Self::Yuyv422),
            "uyvy422" => Some(Self::Uyvy422),
            "yvyu422" => Some(Self::Yvyu422),
            "uyyvyy411" => Some(Self::Uyyvyy411),
            "y210le" => Some(Self::Y210Le),
            "y210be" => Some(Self::Y210Be),
            "y212le" => Some(Self::Y212Le),
            "y212be" => Some(Self::Y212Be),
            "y216le" => Some(Self::Y216Le),
            "y216be" => Some(Self::Y216Be),
            "nv12" => Some(Self::Nv12),
            "nv21" => Some(Self::Nv21),
            "nv16" => Some(Self::Nv16),
            "nv20le" => Some(Self::Nv20Le),
            "nv20be" => Some(Self::Nv20Be),
            "nv24" => Some(Self::Nv24),
            "nv42" => Some(Self::Nv42),
            "p010le" => Some(Self::P010Le),
            "p010be" => Some(Self::P010Be),
            "p012le" => Some(Self::P012Le),
            "p012be" => Some(Self::P012Be),
            "p016le" => Some(Self::P016Le),
            "p016be" => Some(Self::P016Be),
            "p210le" => Some(Self::P210Le),
            "p210be" => Some(Self::P210Be),
            "p212le" => Some(Self::P212Le),
            "p212be" => Some(Self::P212Be),
            "p216le" => Some(Self::P216Le),
            "p216be" => Some(Self::P216Be),
            "p410le" => Some(Self::P410Le),
            "p410be" => Some(Self::P410Be),
            "p412le" => Some(Self::P412Le),
            "p412be" => Some(Self::P412Be),
            "p416le" => Some(Self::P416Le),
            "p416be" => Some(Self::P416Be),
            "yuv420p" => Some(Self::Yuv420p),
            "yuvj420p" => Some(Self::YuvJ420p),
            "yuv422p" => Some(Self::Yuv422p),
            "yuvj422p" => Some(Self::YuvJ422p),
            "yuv410p" => Some(Self::Yuv410p),
            "yuv411p" => Some(Self::Yuv411p),
            "yuvj411p" => Some(Self::YuvJ411p),
            "yuv440p" => Some(Self::Yuv440p),
            "yuvj440p" => Some(Self::YuvJ440p),
            "yuv444p" => Some(Self::Yuv444p),
            "yuvj444p" => Some(Self::YuvJ444p),
            "yuva420p" => Some(Self::Yuva420p),
            "yuva422p" => Some(Self::Yuva422p),
            "yuva444p" => Some(Self::Yuva444p),
            "yuva420p9le" => Some(Self::Yuva420p9Le),
            "yuva420p9be" => Some(Self::Yuva420p9Be),
            "yuva422p9le" => Some(Self::Yuva422p9Le),
            "yuva422p9be" => Some(Self::Yuva422p9Be),
            "yuva444p9le" => Some(Self::Yuva444p9Le),
            "yuva444p9be" => Some(Self::Yuva444p9Be),
            "yuva420p10le" => Some(Self::Yuva420p10Le),
            "yuva420p10be" => Some(Self::Yuva420p10Be),
            "yuva422p10le" => Some(Self::Yuva422p10Le),
            "yuva422p10be" => Some(Self::Yuva422p10Be),
            "yuva444p10le" => Some(Self::Yuva444p10Le),
            "yuva444p10be" => Some(Self::Yuva444p10Be),
            "yuva422p12le" => Some(Self::Yuva422p12Le),
            "yuva422p12be" => Some(Self::Yuva422p12Be),
            "yuva444p12le" => Some(Self::Yuva444p12Le),
            "yuva444p12be" => Some(Self::Yuva444p12Be),
            "yuva420p16le" => Some(Self::Yuva420p16Le),
            "yuva420p16be" => Some(Self::Yuva420p16Be),
            "yuva422p16le" => Some(Self::Yuva422p16Le),
            "yuva422p16be" => Some(Self::Yuva422p16Be),
            "yuva444p16le" => Some(Self::Yuva444p16Le),
            "yuva444p16be" => Some(Self::Yuva444p16Be),
            "yuv440p10le" => Some(Self::Yuv440p10Le),
            "yuv440p10be" => Some(Self::Yuv440p10Be),
            "yuv440p12le" => Some(Self::Yuv440p12Le),
            "yuv440p12be" => Some(Self::Yuv440p12Be),
            "yuv420p9le" => Some(Self::Yuv420p9Le),
            "yuv420p9be" => Some(Self::Yuv420p9Be),
            "yuv422p9le" => Some(Self::Yuv422p9Le),
            "yuv422p9be" => Some(Self::Yuv422p9Be),
            "yuv444p9le" => Some(Self::Yuv444p9Le),
            "yuv444p9be" => Some(Self::Yuv444p9Be),
            "yuv420p10le" => Some(Self::Yuv420p10Le),
            "yuv420p10be" => Some(Self::Yuv420p10Be),
            "yuv422p10le" => Some(Self::Yuv422p10Le),
            "yuv422p10be" => Some(Self::Yuv422p10Be),
            "yuv444p10le" => Some(Self::Yuv444p10Le),
            "yuv444p10be" => Some(Self::Yuv444p10Be),
            "yuv444p10msble" => Some(Self::Yuv444p10MsbLe),
            "yuv444p10msbbe" => Some(Self::Yuv444p10MsbBe),
            "yuv420p12le" => Some(Self::Yuv420p12Le),
            "yuv420p12be" => Some(Self::Yuv420p12Be),
            "yuv422p12le" => Some(Self::Yuv422p12Le),
            "yuv422p12be" => Some(Self::Yuv422p12Be),
            "yuv444p12le" => Some(Self::Yuv444p12Le),
            "yuv444p12be" => Some(Self::Yuv444p12Be),
            "yuv444p12msble" => Some(Self::Yuv444p12MsbLe),
            "yuv444p12msbbe" => Some(Self::Yuv444p12MsbBe),
            "yuv420p14le" => Some(Self::Yuv420p14Le),
            "yuv420p14be" => Some(Self::Yuv420p14Be),
            "yuv422p14le" => Some(Self::Yuv422p14Le),
            "yuv422p14be" => Some(Self::Yuv422p14Be),
            "yuv444p14le" => Some(Self::Yuv444p14Le),
            "yuv444p14be" => Some(Self::Yuv444p14Be),
            "yuv420p16le" => Some(Self::Yuv420p16Le),
            "yuv420p16be" => Some(Self::Yuv420p16Be),
            "yuv422p16le" => Some(Self::Yuv422p16Le),
            "yuv422p16be" => Some(Self::Yuv422p16Be),
            "yuv444p16le" => Some(Self::Yuv444p16Le),
            "yuv444p16be" => Some(Self::Yuv444p16Be),
            "vaapi" => Some(Self::Vaapi),
            "dxva2_vld" => Some(Self::Dxva2Vld),
            "vdpau" => Some(Self::Vdpau),
            "qsv" => Some(Self::Qsv),
            "mmal" => Some(Self::Mmal),
            "d3d11va_vld" => Some(Self::D3d11VaVld),
            "cuda" => Some(Self::Cuda),
            "videotoolbox_vld" => Some(Self::VideoToolboxVld),
            "mediacodec" => Some(Self::MediaCodec),
            "d3d11" => Some(Self::D3d11),
            "drm_prime" => Some(Self::DrmPrime),
            "opencl" => Some(Self::OpenCl),
            "vulkan" => Some(Self::Vulkan),
            "d3d12" => Some(Self::D3d12),
            "amf" => Some(Self::Amf),
            "ohcodec" => Some(Self::OhCodec),
            _ => None,
        }
    }

    fn hardware_name(self) -> Option<&'static str> {
        match self {
            Self::Vaapi => Some("vaapi"),
            Self::Dxva2Vld => Some("dxva2_vld"),
            Self::Vdpau => Some("vdpau"),
            Self::Qsv => Some("qsv"),
            Self::Mmal => Some("mmal"),
            Self::D3d11VaVld => Some("d3d11va_vld"),
            Self::Cuda => Some("cuda"),
            Self::VideoToolboxVld => Some("videotoolbox_vld"),
            Self::MediaCodec => Some("mediacodec"),
            Self::D3d11 => Some("d3d11"),
            Self::DrmPrime => Some("drm_prime"),
            Self::OpenCl => Some("opencl"),
            Self::Vulkan => Some("vulkan"),
            Self::D3d12 => Some("d3d12"),
            Self::Amf => Some("amf"),
            Self::OhCodec => Some("ohcodec"),
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
            Self::Vaapi
            | Self::Dxva2Vld
            | Self::Vdpau
            | Self::Qsv
            | Self::Mmal
            | Self::D3d11VaVld
            | Self::Cuda
            | Self::VideoToolboxVld
            | Self::MediaCodec
            | Self::D3d11
            | Self::DrmPrime
            | Self::OpenCl
            | Self::Vulkan
            | Self::D3d12
            | Self::Amf
            | Self::OhCodec => (
                self.hardware_name()
                    .expect("hardware pixel format has a pinned FFmpeg name"),
                PixelFormatClass::Hardware,
                0,
                0,
                0,
                false,
                false,
                None,
                0,
                0,
            ),
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
            Self::Pal8 => (
                "pal8",
                PixelFormatClass::Rgb,
                1,
                8,
                1,
                false,
                true,
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
            Self::Yaf16Le => (
                "yaf16le",
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
            Self::Yaf16Be => (
                "yaf16be",
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
            Self::Yaf32Le => (
                "yaf32le",
                PixelFormatClass::Gray,
                2,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Yaf32Be => (
                "yaf32be",
                PixelFormatClass::Gray,
                2,
                64,
                1,
                false,
                true,
                Some(8),
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
                4,
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
                4,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::BayerBggr8 => (
                "bayer_bggr8",
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
            Self::BayerRggb8 => (
                "bayer_rggb8",
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
            Self::BayerGbrg8 => (
                "bayer_gbrg8",
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
            Self::BayerGrbg8 => (
                "bayer_grbg8",
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
            Self::BayerBggr16Le => (
                "bayer_bggr16le",
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
            Self::BayerBggr16Be => (
                "bayer_bggr16be",
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
            Self::BayerRggb16Le => (
                "bayer_rggb16le",
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
            Self::BayerRggb16Be => (
                "bayer_rggb16be",
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
            Self::BayerGbrg16Le => (
                "bayer_gbrg16le",
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
            Self::BayerGbrg16Be => (
                "bayer_gbrg16be",
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
            Self::BayerGrbg16Le => (
                "bayer_grbg16le",
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
            Self::BayerGrbg16Be => (
                "bayer_grbg16be",
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
                15,
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
                15,
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
                15,
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
                15,
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
                12,
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
                12,
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
                12,
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
                12,
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
            Self::RgbF16Le => (
                "rgbf16le",
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
            Self::RgbF16Be => (
                "rgbf16be",
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
            Self::RgbF32Le => (
                "rgbf32le",
                PixelFormatClass::Rgb,
                3,
                96,
                1,
                false,
                false,
                Some(12),
                0,
                0,
            ),
            Self::RgbF32Be => (
                "rgbf32be",
                PixelFormatClass::Rgb,
                3,
                96,
                1,
                false,
                false,
                Some(12),
                0,
                0,
            ),
            Self::Rgb96Le => (
                "rgb96le",
                PixelFormatClass::Rgb,
                3,
                96,
                1,
                false,
                false,
                Some(12),
                0,
                0,
            ),
            Self::Rgb96Be => (
                "rgb96be",
                PixelFormatClass::Rgb,
                3,
                96,
                1,
                false,
                false,
                Some(12),
                0,
                0,
            ),
            Self::RgbaF16Le => (
                "rgbaf16le",
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
            Self::RgbaF16Be => (
                "rgbaf16be",
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
            Self::RgbaF32Le => (
                "rgbaf32le",
                PixelFormatClass::Rgb,
                4,
                128,
                1,
                false,
                true,
                Some(16),
                0,
                0,
            ),
            Self::RgbaF32Be => (
                "rgbaf32be",
                PixelFormatClass::Rgb,
                4,
                128,
                1,
                false,
                true,
                Some(16),
                0,
                0,
            ),
            Self::Rgba128Le => (
                "rgba128le",
                PixelFormatClass::Rgb,
                4,
                128,
                1,
                false,
                true,
                Some(16),
                0,
                0,
            ),
            Self::Rgba128Be => (
                "rgba128be",
                PixelFormatClass::Rgb,
                4,
                128,
                1,
                false,
                true,
                Some(16),
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
                24,
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
                24,
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
                24,
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
                24,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::X2Rgb10Le => (
                "x2rgb10le",
                PixelFormatClass::Rgb,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::X2Rgb10Be => (
                "x2rgb10be",
                PixelFormatClass::Rgb,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::X2Bgr10Le => (
                "x2bgr10le",
                PixelFormatClass::Rgb,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::X2Bgr10Be => (
                "x2bgr10be",
                PixelFormatClass::Rgb,
                3,
                30,
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
            Self::Gbrp10MsbLe => (
                "gbrp10msble",
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
            Self::Gbrp10MsbBe => (
                "gbrp10msbbe",
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
            Self::Gbrp12MsbLe => (
                "gbrp12msble",
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
            Self::Gbrp12MsbBe => (
                "gbrp12msbbe",
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
            Self::GbrpF16Le => (
                "gbrpf16le",
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
            Self::GbrpF16Be => (
                "gbrpf16be",
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
            Self::GbrpF32Le => (
                "gbrpf32le",
                PixelFormatClass::Rgb,
                3,
                96,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::GbrpF32Be => (
                "gbrpf32be",
                PixelFormatClass::Rgb,
                3,
                96,
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
            Self::Ayuv64Le => (
                "ayuv64le",
                PixelFormatClass::Yuv,
                4,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Ayuv64Be => (
                "ayuv64be",
                PixelFormatClass::Yuv,
                4,
                64,
                1,
                false,
                true,
                Some(8),
                0,
                0,
            ),
            Self::Vuya => (
                "vuya",
                PixelFormatClass::Yuv,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Vuyx => (
                "vuyx",
                PixelFormatClass::Yuv,
                3,
                24,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Xv30Le => (
                "xv30le",
                PixelFormatClass::Yuv,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Xv30Be => (
                "xv30be",
                PixelFormatClass::Yuv,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Xv36Le => (
                "xv36le",
                PixelFormatClass::Yuv,
                3,
                36,
                1,
                false,
                false,
                Some(8),
                0,
                0,
            ),
            Self::Xv36Be => (
                "xv36be",
                PixelFormatClass::Yuv,
                3,
                36,
                1,
                false,
                false,
                Some(8),
                0,
                0,
            ),
            Self::Xv48Le => (
                "xv48le",
                PixelFormatClass::Yuv,
                3,
                48,
                1,
                false,
                false,
                Some(8),
                0,
                0,
            ),
            Self::Xv48Be => (
                "xv48be",
                PixelFormatClass::Yuv,
                3,
                48,
                1,
                false,
                false,
                Some(8),
                0,
                0,
            ),
            Self::V30xLe => (
                "v30xle",
                PixelFormatClass::Yuv,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::V30xBe => (
                "v30xbe",
                PixelFormatClass::Yuv,
                3,
                30,
                1,
                false,
                false,
                Some(4),
                0,
                0,
            ),
            Self::Ayuv => (
                "ayuv",
                PixelFormatClass::Yuv,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Uyva => (
                "uyva",
                PixelFormatClass::Yuv,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Vyu444 => (
                "vyu444",
                PixelFormatClass::Yuv,
                3,
                24,
                1,
                false,
                false,
                Some(3),
                0,
                0,
            ),
            Self::Xyz12Le => (
                "xyz12le",
                PixelFormatClass::Xyz,
                3,
                36,
                1,
                false,
                false,
                Some(6),
                0,
                0,
            ),
            Self::Xyz12Be => (
                "xyz12be",
                PixelFormatClass::Xyz,
                3,
                36,
                1,
                false,
                false,
                Some(6),
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
            Self::Uyyvyy411 => (
                "uyyvyy411",
                PixelFormatClass::Yuv,
                3,
                12,
                1,
                false,
                false,
                None,
                2,
                0,
            ),
            Self::Y210Le => (
                "y210le",
                PixelFormatClass::Yuv,
                3,
                20,
                1,
                false,
                false,
                Some(4),
                1,
                0,
            ),
            Self::Y210Be => (
                "y210be",
                PixelFormatClass::Yuv,
                3,
                20,
                1,
                false,
                false,
                Some(4),
                1,
                0,
            ),
            Self::Y212Le => (
                "y212le",
                PixelFormatClass::Yuv,
                3,
                24,
                1,
                false,
                false,
                Some(4),
                1,
                0,
            ),
            Self::Y212Be => (
                "y212be",
                PixelFormatClass::Yuv,
                3,
                24,
                1,
                false,
                false,
                Some(4),
                1,
                0,
            ),
            Self::Y216Le => (
                "y216le",
                PixelFormatClass::Yuv,
                3,
                32,
                1,
                false,
                false,
                Some(4),
                1,
                0,
            ),
            Self::Y216Be => (
                "y216be",
                PixelFormatClass::Yuv,
                3,
                32,
                1,
                false,
                false,
                Some(4),
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
            Self::Nv16 => (
                "nv16",
                PixelFormatClass::Yuv,
                3,
                16,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Nv20Le => (
                "nv20le",
                PixelFormatClass::Yuv,
                3,
                20,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Nv20Be => (
                "nv20be",
                PixelFormatClass::Yuv,
                3,
                20,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Nv24 => (
                "nv24",
                PixelFormatClass::Yuv,
                3,
                24,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Nv42 => (
                "nv42",
                PixelFormatClass::Yuv,
                3,
                24,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::P010Le => (
                "p010le",
                PixelFormatClass::Yuv,
                3,
                15,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::P010Be => (
                "p010be",
                PixelFormatClass::Yuv,
                3,
                15,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::P012Le => (
                "p012le",
                PixelFormatClass::Yuv,
                3,
                18,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::P012Be => (
                "p012be",
                PixelFormatClass::Yuv,
                3,
                18,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::P016Le => (
                "p016le",
                PixelFormatClass::Yuv,
                3,
                24,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::P016Be => (
                "p016be",
                PixelFormatClass::Yuv,
                3,
                24,
                2,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::P210Le => (
                "p210le",
                PixelFormatClass::Yuv,
                3,
                20,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::P210Be => (
                "p210be",
                PixelFormatClass::Yuv,
                3,
                20,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::P212Le => (
                "p212le",
                PixelFormatClass::Yuv,
                3,
                24,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::P212Be => (
                "p212be",
                PixelFormatClass::Yuv,
                3,
                24,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::P216Le => (
                "p216le",
                PixelFormatClass::Yuv,
                3,
                32,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::P216Be => (
                "p216be",
                PixelFormatClass::Yuv,
                3,
                32,
                2,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::P410Le => (
                "p410le",
                PixelFormatClass::Yuv,
                3,
                30,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::P410Be => (
                "p410be",
                PixelFormatClass::Yuv,
                3,
                30,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::P412Le => (
                "p412le",
                PixelFormatClass::Yuv,
                3,
                36,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::P412Be => (
                "p412be",
                PixelFormatClass::Yuv,
                3,
                36,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::P416Le => (
                "p416le",
                PixelFormatClass::Yuv,
                3,
                48,
                2,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::P416Be => (
                "p416be",
                PixelFormatClass::Yuv,
                3,
                48,
                2,
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
            Self::YuvJ420p => (
                "yuvj420p",
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
            Self::YuvJ422p => (
                "yuvj422p",
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
            Self::YuvJ411p => (
                "yuvj411p",
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
            Self::YuvJ440p => (
                "yuvj440p",
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
            Self::YuvJ444p => (
                "yuvj444p",
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
            Self::Yuva420p => (
                "yuva420p",
                PixelFormatClass::Yuv,
                4,
                20,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva422p => (
                "yuva422p",
                PixelFormatClass::Yuv,
                4,
                24,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva444p => (
                "yuva444p",
                PixelFormatClass::Yuv,
                4,
                32,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva420p9Le => (
                "yuva420p9le",
                PixelFormatClass::Yuv,
                4,
                22,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva420p9Be => (
                "yuva420p9be",
                PixelFormatClass::Yuv,
                4,
                22,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva422p9Le => (
                "yuva422p9le",
                PixelFormatClass::Yuv,
                4,
                27,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva422p9Be => (
                "yuva422p9be",
                PixelFormatClass::Yuv,
                4,
                27,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva444p9Le => (
                "yuva444p9le",
                PixelFormatClass::Yuv,
                4,
                36,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva444p9Be => (
                "yuva444p9be",
                PixelFormatClass::Yuv,
                4,
                36,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva420p10Le => (
                "yuva420p10le",
                PixelFormatClass::Yuv,
                4,
                25,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva420p10Be => (
                "yuva420p10be",
                PixelFormatClass::Yuv,
                4,
                25,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva422p10Le => (
                "yuva422p10le",
                PixelFormatClass::Yuv,
                4,
                30,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva422p10Be => (
                "yuva422p10be",
                PixelFormatClass::Yuv,
                4,
                30,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva444p10Le => (
                "yuva444p10le",
                PixelFormatClass::Yuv,
                4,
                40,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva444p10Be => (
                "yuva444p10be",
                PixelFormatClass::Yuv,
                4,
                40,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva422p12Le => (
                "yuva422p12le",
                PixelFormatClass::Yuv,
                4,
                36,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva422p12Be => (
                "yuva422p12be",
                PixelFormatClass::Yuv,
                4,
                36,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva444p12Le => (
                "yuva444p12le",
                PixelFormatClass::Yuv,
                4,
                48,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva444p12Be => (
                "yuva444p12be",
                PixelFormatClass::Yuv,
                4,
                48,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva420p16Le => (
                "yuva420p16le",
                PixelFormatClass::Yuv,
                4,
                40,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva420p16Be => (
                "yuva420p16be",
                PixelFormatClass::Yuv,
                4,
                40,
                4,
                true,
                true,
                None,
                1,
                1,
            ),
            Self::Yuva422p16Le => (
                "yuva422p16le",
                PixelFormatClass::Yuv,
                4,
                48,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva422p16Be => (
                "yuva422p16be",
                PixelFormatClass::Yuv,
                4,
                48,
                4,
                true,
                true,
                None,
                1,
                0,
            ),
            Self::Yuva444p16Le => (
                "yuva444p16le",
                PixelFormatClass::Yuv,
                4,
                64,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuva444p16Be => (
                "yuva444p16be",
                PixelFormatClass::Yuv,
                4,
                64,
                4,
                true,
                true,
                None,
                0,
                0,
            ),
            Self::Yuv440p10Le => (
                "yuv440p10le",
                PixelFormatClass::Yuv,
                3,
                20,
                3,
                true,
                false,
                None,
                0,
                1,
            ),
            Self::Yuv440p10Be => (
                "yuv440p10be",
                PixelFormatClass::Yuv,
                3,
                20,
                3,
                true,
                false,
                None,
                0,
                1,
            ),
            Self::Yuv440p12Le => (
                "yuv440p12le",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                0,
                1,
            ),
            Self::Yuv440p12Be => (
                "yuv440p12be",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                0,
                1,
            ),
            Self::Yuv420p9Le => (
                "yuv420p9le",
                PixelFormatClass::Yuv,
                3,
                13,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv420p9Be => (
                "yuv420p9be",
                PixelFormatClass::Yuv,
                3,
                13,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p9Le => (
                "yuv422p9le",
                PixelFormatClass::Yuv,
                3,
                18,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv422p9Be => (
                "yuv422p9be",
                PixelFormatClass::Yuv,
                3,
                18,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv444p9Le => (
                "yuv444p9le",
                PixelFormatClass::Yuv,
                3,
                27,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p9Be => (
                "yuv444p9be",
                PixelFormatClass::Yuv,
                3,
                27,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv420p10Le => (
                "yuv420p10le",
                PixelFormatClass::Yuv,
                3,
                15,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv420p10Be => (
                "yuv420p10be",
                PixelFormatClass::Yuv,
                3,
                15,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p10Le => (
                "yuv422p10le",
                PixelFormatClass::Yuv,
                3,
                20,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv422p10Be => (
                "yuv422p10be",
                PixelFormatClass::Yuv,
                3,
                20,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv444p10Le => (
                "yuv444p10le",
                PixelFormatClass::Yuv,
                3,
                30,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p10Be => (
                "yuv444p10be",
                PixelFormatClass::Yuv,
                3,
                30,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p10MsbLe => (
                "yuv444p10msble",
                PixelFormatClass::Yuv,
                3,
                30,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p10MsbBe => (
                "yuv444p10msbbe",
                PixelFormatClass::Yuv,
                3,
                30,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv420p12Le => (
                "yuv420p12le",
                PixelFormatClass::Yuv,
                3,
                18,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv420p12Be => (
                "yuv420p12be",
                PixelFormatClass::Yuv,
                3,
                18,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p12Le => (
                "yuv422p12le",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv422p12Be => (
                "yuv422p12be",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv444p12Le => (
                "yuv444p12le",
                PixelFormatClass::Yuv,
                3,
                36,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p12Be => (
                "yuv444p12be",
                PixelFormatClass::Yuv,
                3,
                36,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p12MsbLe => (
                "yuv444p12msble",
                PixelFormatClass::Yuv,
                3,
                36,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p12MsbBe => (
                "yuv444p12msbbe",
                PixelFormatClass::Yuv,
                3,
                36,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv420p14Le => (
                "yuv420p14le",
                PixelFormatClass::Yuv,
                3,
                21,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv420p14Be => (
                "yuv420p14be",
                PixelFormatClass::Yuv,
                3,
                21,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p14Le => (
                "yuv422p14le",
                PixelFormatClass::Yuv,
                3,
                28,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv422p14Be => (
                "yuv422p14be",
                PixelFormatClass::Yuv,
                3,
                28,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv444p14Le => (
                "yuv444p14le",
                PixelFormatClass::Yuv,
                3,
                42,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p14Be => (
                "yuv444p14be",
                PixelFormatClass::Yuv,
                3,
                42,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv420p16Le => (
                "yuv420p16le",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv420p16Be => (
                "yuv420p16be",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p16Le => (
                "yuv422p16le",
                PixelFormatClass::Yuv,
                3,
                32,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv422p16Be => (
                "yuv422p16be",
                PixelFormatClass::Yuv,
                3,
                32,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv444p16Le => (
                "yuv444p16le",
                PixelFormatClass::Yuv,
                3,
                48,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
            Self::Yuv444p16Be => (
                "yuv444p16be",
                PixelFormatClass::Yuv,
                3,
                48,
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
            bits_per_component: if self.is_hardware() {
                0
            } else if matches!(self, Self::MonoWhite | Self::MonoBlack) {
                1
            } else if matches!(
                self,
                Self::Ya16Le
                    | Self::Ya16Be
                    | Self::Yaf16Le
                    | Self::Yaf16Be
                    | Self::Gray16Le
                    | Self::Gray16Be
                    | Self::GrayF16Le
                    | Self::GrayF16Be
                    | Self::Rgb48Le
                    | Self::Rgb48Be
                    | Self::Bgr48Le
                    | Self::Bgr48Be
                    | Self::RgbF16Le
                    | Self::RgbF16Be
                    | Self::RgbaF16Le
                    | Self::RgbaF16Be
                    | Self::Rgba64Le
                    | Self::Rgba64Be
                    | Self::Bgra64Le
                    | Self::Bgra64Be
                    | Self::BayerBggr16Le
                    | Self::BayerBggr16Be
                    | Self::BayerRggb16Le
                    | Self::BayerRggb16Be
                    | Self::BayerGbrg16Le
                    | Self::BayerGbrg16Be
                    | Self::BayerGrbg16Le
                    | Self::BayerGrbg16Be
                    | Self::Ayuv64Le
                    | Self::Ayuv64Be
                    | Self::Xv48Le
                    | Self::Xv48Be
                    | Self::Gbrp16Le
                    | Self::Gbrp16Be
                    | Self::GbrpF16Le
                    | Self::GbrpF16Be
                    | Self::Gbrap16Le
                    | Self::Gbrap16Be
                    | Self::GbrapF16Le
                    | Self::GbrapF16Be
                    | Self::P016Le
                    | Self::P016Be
                    | Self::P216Le
                    | Self::P216Be
                    | Self::P416Le
                    | Self::P416Be
                    | Self::Y216Le
                    | Self::Y216Be
                    | Self::Yuva420p16Le
                    | Self::Yuva420p16Be
                    | Self::Yuva422p16Le
                    | Self::Yuva422p16Be
                    | Self::Yuva444p16Le
                    | Self::Yuva444p16Be
                    | Self::Yuv420p16Le
                    | Self::Yuv420p16Be
                    | Self::Yuv422p16Le
                    | Self::Yuv422p16Be
                    | Self::Yuv444p16Le
                    | Self::Yuv444p16Be
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
                Self::Gray9Le
                    | Self::Gray9Be
                    | Self::Gbrp9Le
                    | Self::Gbrp9Be
                    | Self::Yuva420p9Le
                    | Self::Yuva420p9Be
                    | Self::Yuva422p9Le
                    | Self::Yuva422p9Be
                    | Self::Yuva444p9Le
                    | Self::Yuva444p9Be
                    | Self::Yuv420p9Le
                    | Self::Yuv420p9Be
                    | Self::Yuv422p9Le
                    | Self::Yuv422p9Be
                    | Self::Yuv444p9Le
                    | Self::Yuv444p9Be
            ) {
                9
            } else if matches!(
                self,
                Self::Gray10Le
                    | Self::Gray10Be
                    | Self::X2Rgb10Le
                    | Self::X2Rgb10Be
                    | Self::X2Bgr10Le
                    | Self::X2Bgr10Be
                    | Self::Xv30Le
                    | Self::Xv30Be
                    | Self::V30xLe
                    | Self::V30xBe
                    | Self::Gbrp10Le
                    | Self::Gbrp10Be
                    | Self::Gbrp10MsbLe
                    | Self::Gbrp10MsbBe
                    | Self::Gbrap10Le
                    | Self::Gbrap10Be
                    | Self::Nv20Le
                    | Self::Nv20Be
                    | Self::Y210Le
                    | Self::Y210Be
                    | Self::P010Le
                    | Self::P010Be
                    | Self::P210Le
                    | Self::P210Be
                    | Self::P410Le
                    | Self::P410Be
                    | Self::Yuva420p10Le
                    | Self::Yuva420p10Be
                    | Self::Yuva422p10Le
                    | Self::Yuva422p10Be
                    | Self::Yuva444p10Le
                    | Self::Yuva444p10Be
                    | Self::Yuv420p10Le
                    | Self::Yuv420p10Be
                    | Self::Yuv422p10Le
                    | Self::Yuv422p10Be
                    | Self::Yuv440p10Le
                    | Self::Yuv440p10Be
                    | Self::Yuv444p10Le
                    | Self::Yuv444p10Be
                    | Self::Yuv444p10MsbLe
                    | Self::Yuv444p10MsbBe
            ) {
                10
            } else if matches!(
                self,
                Self::Gray12Le
                    | Self::Gray12Be
                    | Self::Gbrp12Le
                    | Self::Gbrp12Be
                    | Self::Gbrp12MsbLe
                    | Self::Gbrp12MsbBe
                    | Self::Gbrap12Le
                    | Self::Gbrap12Be
                    | Self::Xyz12Le
                    | Self::Xyz12Be
                    | Self::Xv36Le
                    | Self::Xv36Be
                    | Self::Y212Le
                    | Self::Y212Be
                    | Self::P012Le
                    | Self::P012Be
                    | Self::P212Le
                    | Self::P212Be
                    | Self::P412Le
                    | Self::P412Be
                    | Self::Yuva422p12Le
                    | Self::Yuva422p12Be
                    | Self::Yuva444p12Le
                    | Self::Yuva444p12Be
                    | Self::Yuv420p12Le
                    | Self::Yuv420p12Be
                    | Self::Yuv422p12Le
                    | Self::Yuv422p12Be
                    | Self::Yuv440p12Le
                    | Self::Yuv440p12Be
                    | Self::Yuv444p12Le
                    | Self::Yuv444p12Be
                    | Self::Yuv444p12MsbLe
                    | Self::Yuv444p12MsbBe
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
                    | Self::Yuv420p14Le
                    | Self::Yuv420p14Be
                    | Self::Yuv422p14Le
                    | Self::Yuv422p14Be
                    | Self::Yuv444p14Le
                    | Self::Yuv444p14Be
            ) {
                14
            } else if matches!(
                self,
                Self::Gray32Le
                    | Self::Gray32Be
                    | Self::Yaf32Le
                    | Self::Yaf32Be
                    | Self::GrayF32Le
                    | Self::GrayF32Be
                    | Self::RgbF32Le
                    | Self::RgbF32Be
                    | Self::RgbaF32Le
                    | Self::RgbaF32Be
                    | Self::Rgb96Le
                    | Self::Rgb96Be
                    | Self::Rgba128Le
                    | Self::Rgba128Be
                    | Self::GbrpF32Le
                    | Self::GbrpF32Be
                    | Self::Gbrap32Le
                    | Self::Gbrap32Be
                    | Self::GbrapF32Le
                    | Self::GbrapF32Be
            ) {
                32
            } else {
                8
            },
            bits_per_pixel: if matches!(self, Self::Yuv420p9Le | Self::Yuv420p9Be) {
                Rational::from_raw(27, 2)
            } else if matches!(self, Self::Yuva420p9Le | Self::Yuva420p9Be) {
                Rational::from_raw(45, 2)
            } else {
                Rational::from_raw(bits_per_pixel, 1)
            },
            plane_count,
            is_planar,
            has_alpha,
            is_float: matches!(
                self,
                Self::GrayF16Le
                    | Self::GrayF16Be
                    | Self::GrayF32Le
                    | Self::GrayF32Be
                    | Self::Yaf16Le
                    | Self::Yaf16Be
                    | Self::Yaf32Le
                    | Self::Yaf32Be
                    | Self::RgbF16Le
                    | Self::RgbF16Be
                    | Self::RgbF32Le
                    | Self::RgbF32Be
                    | Self::RgbaF16Le
                    | Self::RgbaF16Be
                    | Self::RgbaF32Le
                    | Self::RgbaF32Be
                    | Self::GbrpF16Le
                    | Self::GbrpF16Be
                    | Self::GbrpF32Le
                    | Self::GbrpF32Be
                    | Self::GbrapF16Le
                    | Self::GbrapF16Be
                    | Self::GbrapF32Le
                    | Self::GbrapF32Be
            ),
            is_paletted: matches!(self, Self::Pal8),
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

    pub fn is_xyz(self) -> bool {
        self.class() == PixelFormatClass::Xyz
    }

    pub fn is_yuv(self) -> bool {
        self.class() == PixelFormatClass::Yuv
    }

    pub fn is_hardware(self) -> bool {
        matches!(
            self,
            Self::Vaapi
                | Self::Dxva2Vld
                | Self::Vdpau
                | Self::Qsv
                | Self::Mmal
                | Self::D3d11VaVld
                | Self::Cuda
                | Self::VideoToolboxVld
                | Self::MediaCodec
                | Self::D3d11
                | Self::DrmPrime
                | Self::OpenCl
                | Self::Vulkan
                | Self::D3d12
                | Self::Amf
                | Self::OhCodec
        )
    }

    pub fn component_count(self) -> usize {
        self.descriptor().component_count
    }

    pub fn bits_per_component(self) -> u8 {
        self.descriptor().bits_per_component
    }

    pub fn component_bit_depths(self) -> Vec<u8> {
        if self.is_hardware() {
            return vec![0];
        }

        match self {
            Self::Rgb8 | Self::Bgr8 => vec![3, 3, 2],
            Self::Rgb4 | Self::Bgr4 | Self::Rgb4Byte | Self::Bgr4Byte => vec![1, 2, 1],
            Self::Rgb565Be | Self::Rgb565Le | Self::Bgr565Be | Self::Bgr565Le => {
                vec![5, 6, 5]
            }
            Self::BayerBggr8 | Self::BayerRggb8 | Self::BayerGbrg8 | Self::BayerGrbg8 => {
                vec![2, 4, 2]
            }
            Self::BayerBggr16Le
            | Self::BayerBggr16Be
            | Self::BayerRggb16Le
            | Self::BayerRggb16Be
            | Self::BayerGbrg16Le
            | Self::BayerGbrg16Be
            | Self::BayerGrbg16Le
            | Self::BayerGrbg16Be => vec![4, 8, 4],
            _ => vec![self.bits_per_component(); self.component_count()],
        }
    }

    pub fn bits_per_pixel(self) -> Rational {
        self.descriptor().bits_per_pixel
    }

    pub fn bits_per_pixel_integer(self) -> Option<u8> {
        self.descriptor().bits_per_pixel_integer()
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

    pub fn is_paletted(self) -> bool {
        self.descriptor().is_paletted
    }

    pub fn is_bayer(self) -> bool {
        matches!(
            self,
            Self::BayerBggr8
                | Self::BayerRggb8
                | Self::BayerGbrg8
                | Self::BayerGrbg8
                | Self::BayerBggr16Le
                | Self::BayerBggr16Be
                | Self::BayerRggb16Le
                | Self::BayerRggb16Be
                | Self::BayerGbrg16Le
                | Self::BayerGbrg16Be
                | Self::BayerGrbg16Le
                | Self::BayerGrbg16Be
        )
    }

    pub fn packed_bytes_per_pixel(self) -> Option<usize> {
        self.descriptor().packed_bytes_per_pixel
    }

    pub fn plane_sizes(self, width: usize, height: usize) -> AvResult<Vec<usize>> {
        if self.is_hardware() {
            return Err(AvError::unsupported(format!(
                "hardware pixel format `{}` does not expose Rust-owned frame geometry",
                self.name()
            )));
        }

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
            Self::Ya16Le | Self::Ya16Be | Self::Yaf16Le | Self::Yaf16Be => Ok(vec![checked_mul(
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
            Self::Yaf32Le | Self::Yaf32Be => Ok(vec![checked_mul(
                pixels,
                8,
                "32-bit floating gray-alpha pixel format frame size",
            )?]),
            Self::Rgb24 | Self::Bgr24 => Ok(vec![checked_mul(
                pixels,
                3,
                "24-bit packed pixel format frame size",
            )?]),
            Self::Pal8
            | Self::Rgb8
            | Self::Bgr8
            | Self::Rgb4Byte
            | Self::Bgr4Byte
            | Self::BayerBggr8
            | Self::BayerRggb8
            | Self::BayerGbrg8
            | Self::BayerGrbg8 => Ok(vec![pixels]),
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
            | Self::Bgr444Be
            | Self::BayerBggr16Le
            | Self::BayerBggr16Be
            | Self::BayerRggb16Le
            | Self::BayerRggb16Be
            | Self::BayerGbrg16Le
            | Self::BayerGbrg16Be
            | Self::BayerGrbg16Le
            | Self::BayerGrbg16Be => Ok(vec![checked_mul(
                pixels,
                2,
                "16-bit packed RGB pixel format frame size",
            )?]),
            Self::Rgb48Le
            | Self::Rgb48Be
            | Self::Bgr48Le
            | Self::Bgr48Be
            | Self::RgbF16Le
            | Self::RgbF16Be => Ok(vec![checked_mul(
                pixels,
                6,
                "48-bit packed pixel format frame size",
            )?]),
            Self::RgbF32Le | Self::RgbF32Be | Self::Rgb96Le | Self::Rgb96Be => {
                Ok(vec![checked_mul(
                    pixels,
                    12,
                    "96-bit packed RGB pixel format frame size",
                )?])
            }
            Self::RgbaF16Le
            | Self::RgbaF16Be
            | Self::Rgba64Le
            | Self::Rgba64Be
            | Self::Bgra64Le
            | Self::Bgra64Be => Ok(vec![checked_mul(
                pixels,
                8,
                "64-bit packed pixel format frame size",
            )?]),
            Self::RgbaF32Le | Self::RgbaF32Be | Self::Rgba128Le | Self::Rgba128Be => {
                Ok(vec![checked_mul(
                    pixels,
                    16,
                    "128-bit packed RGBA pixel format frame size",
                )?])
            }
            Self::Rgba
            | Self::Bgra
            | Self::Argb
            | Self::Abgr
            | Self::ZeroRgb
            | Self::Rgb0
            | Self::ZeroBgr
            | Self::Bgr0
            | Self::X2Rgb10Le
            | Self::X2Rgb10Be
            | Self::X2Bgr10Le
            | Self::X2Bgr10Be
            | Self::Xv30Le
            | Self::Xv30Be
            | Self::V30xLe
            | Self::V30xBe => Ok(vec![checked_mul(
                pixels,
                4,
                "32-bit packed pixel format frame size",
            )?]),
            Self::Gbrp => Ok(vec![pixels, pixels, pixels]),
            Self::Gbrp9Le
            | Self::Gbrp9Be
            | Self::Gbrp10Le
            | Self::Gbrp10Be
            | Self::Gbrp10MsbLe
            | Self::Gbrp10MsbBe
            | Self::Gbrp12Le
            | Self::Gbrp12Be
            | Self::Gbrp12MsbLe
            | Self::Gbrp12MsbBe
            | Self::Gbrp14Le
            | Self::Gbrp14Be
            | Self::Gbrp16Le
            | Self::Gbrp16Be
            | Self::GbrpF16Le
            | Self::GbrpF16Be => {
                let plane = checked_mul(
                    pixels,
                    2,
                    "high bit-depth planar GBR pixel format plane size",
                )?;
                Ok(vec![plane, plane, plane])
            }
            Self::GbrpF32Le | Self::GbrpF32Be => {
                let plane = checked_mul(pixels, 4, "32-bit planar GBR pixel format plane size")?;
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
            Self::Ayuv64Le | Self::Ayuv64Be | Self::Xv48Le | Self::Xv48Be => Ok(vec![checked_mul(
                pixels,
                8,
                "packed 64-bit YUV pixel format frame size",
            )?]),
            Self::Vuya | Self::Vuyx | Self::Ayuv | Self::Uyva => Ok(vec![checked_mul(
                pixels,
                4,
                "packed 8-bit YUV 4:4:4:4 pixel format frame size",
            )?]),
            Self::Vyu444 => Ok(vec![checked_mul(
                pixels,
                3,
                "packed 8-bit VYU 4:4:4 pixel format frame size",
            )?]),
            Self::Xyz12Le | Self::Xyz12Be => Ok(vec![checked_mul(
                pixels,
                6,
                "packed six-byte pixel format frame size",
            )?]),
            Self::Xv36Le | Self::Xv36Be => Ok(vec![checked_mul(
                pixels,
                8,
                "packed xv36 pixel format frame size",
            )?]),
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
            Self::Uyyvyy411 => {
                let row = checked_mul(
                    width.div_ceil(4),
                    6,
                    "packed YUV 4:1:1 pixel format line size",
                )?;
                Ok(vec![checked_mul(
                    row,
                    height,
                    "packed YUV 4:1:1 pixel format frame size",
                )?])
            }
            Self::Y210Le
            | Self::Y210Be
            | Self::Y212Le
            | Self::Y212Be
            | Self::Y216Le
            | Self::Y216Be => {
                if width % 2 != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by 2",
                        self.name()
                    )));
                }
                Ok(vec![checked_mul(
                    pixels,
                    4,
                    "high bit-depth packed YUV 4:2:2 pixel format frame size",
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
            Self::Nv16 => {
                if width % 2 != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by 2",
                        self.name()
                    )));
                }
                Ok(vec![pixels, pixels])
            }
            Self::Nv20Le | Self::Nv20Be => {
                if width % 2 != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by 2",
                        self.name()
                    )));
                }
                let plane = checked_mul(
                    pixels,
                    2,
                    "semi-planar 10-bit 4:2:2 YUV pixel format plane size",
                )?;
                Ok(vec![plane, plane])
            }
            Self::Nv24 | Self::Nv42 => {
                let chroma = checked_mul(pixels, 2, "semi-planar 4:4:4 YUV chroma plane size")?;
                Ok(vec![pixels, chroma])
            }
            Self::P010Le
            | Self::P010Be
            | Self::P012Le
            | Self::P012Be
            | Self::P016Le
            | Self::P016Be
            | Self::P210Le
            | Self::P210Be
            | Self::P212Le
            | Self::P212Be
            | Self::P216Le
            | Self::P216Be
            | Self::P410Le
            | Self::P410Be
            | Self::P412Le
            | Self::P412Be
            | Self::P416Le
            | Self::P416Be => {
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
                let luma = checked_mul(pixels, 2, "semi-planar high-bit YUV luma plane size")?;
                let chroma_samples = checked_area(
                    width >> descriptor.log2_chroma_w,
                    height >> descriptor.log2_chroma_h,
                    "semi-planar high-bit YUV chroma sample area",
                )?;
                let chroma_components = checked_mul(
                    chroma_samples,
                    2,
                    "semi-planar high-bit YUV chroma component count",
                )?;
                let chroma = checked_mul(
                    chroma_components,
                    2,
                    "semi-planar high-bit YUV chroma plane size",
                )?;
                Ok(vec![luma, chroma])
            }
            Self::Yuv420p
            | Self::YuvJ420p
            | Self::Yuv422p
            | Self::YuvJ422p
            | Self::Yuv410p
            | Self::Yuv411p
            | Self::YuvJ411p
            | Self::Yuv440p
            | Self::YuvJ440p
            | Self::Yuv444p
            | Self::YuvJ444p
            | Self::Yuv440p10Le
            | Self::Yuv440p10Be
            | Self::Yuv440p12Le
            | Self::Yuv440p12Be
            | Self::Yuv420p9Le
            | Self::Yuv420p9Be
            | Self::Yuv422p9Le
            | Self::Yuv422p9Be
            | Self::Yuv444p9Le
            | Self::Yuv444p9Be
            | Self::Yuv420p10Le
            | Self::Yuv420p10Be
            | Self::Yuv422p10Le
            | Self::Yuv422p10Be
            | Self::Yuv444p10Le
            | Self::Yuv444p10Be
            | Self::Yuv444p10MsbLe
            | Self::Yuv444p10MsbBe
            | Self::Yuv420p12Le
            | Self::Yuv420p12Be
            | Self::Yuv422p12Le
            | Self::Yuv422p12Be
            | Self::Yuv444p12Le
            | Self::Yuv444p12Be
            | Self::Yuv444p12MsbLe
            | Self::Yuv444p12MsbBe
            | Self::Yuv420p14Le
            | Self::Yuv420p14Be
            | Self::Yuv422p14Le
            | Self::Yuv422p14Be
            | Self::Yuv444p14Le
            | Self::Yuv444p14Be
            | Self::Yuv420p16Le
            | Self::Yuv420p16Be
            | Self::Yuv422p16Le
            | Self::Yuv422p16Be
            | Self::Yuv444p16Le
            | Self::Yuv444p16Be => {
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
                let bytes_per_sample = if matches!(
                    self,
                    Self::Yuv420p9Le
                        | Self::Yuv420p9Be
                        | Self::Yuv422p9Le
                        | Self::Yuv422p9Be
                        | Self::Yuv444p9Le
                        | Self::Yuv444p9Be
                        | Self::Yuv420p10Le
                        | Self::Yuv420p10Be
                        | Self::Yuv422p10Le
                        | Self::Yuv422p10Be
                        | Self::Yuv440p10Le
                        | Self::Yuv440p10Be
                        | Self::Yuv444p10Le
                        | Self::Yuv444p10Be
                        | Self::Yuv444p10MsbLe
                        | Self::Yuv444p10MsbBe
                        | Self::Yuv420p12Le
                        | Self::Yuv420p12Be
                        | Self::Yuv422p12Le
                        | Self::Yuv422p12Be
                        | Self::Yuv440p12Le
                        | Self::Yuv440p12Be
                        | Self::Yuv444p12Le
                        | Self::Yuv444p12Be
                        | Self::Yuv444p12MsbLe
                        | Self::Yuv444p12MsbBe
                        | Self::Yuv420p14Le
                        | Self::Yuv420p14Be
                        | Self::Yuv422p14Le
                        | Self::Yuv422p14Be
                        | Self::Yuv444p14Le
                        | Self::Yuv444p14Be
                        | Self::Yuv420p16Le
                        | Self::Yuv420p16Be
                        | Self::Yuv422p16Le
                        | Self::Yuv422p16Be
                        | Self::Yuv444p16Le
                        | Self::Yuv444p16Be
                ) {
                    2
                } else {
                    1
                };
                let luma = checked_mul(pixels, bytes_per_sample, "planar YUV luma plane size")?;
                let chroma = checked_mul(chroma, bytes_per_sample, "planar YUV chroma plane size")?;
                Ok(vec![luma, chroma, chroma])
            }
            Self::Yuva420p
            | Self::Yuva422p
            | Self::Yuva444p
            | Self::Yuva420p9Le
            | Self::Yuva420p9Be
            | Self::Yuva422p9Le
            | Self::Yuva422p9Be
            | Self::Yuva444p9Le
            | Self::Yuva444p9Be
            | Self::Yuva420p10Le
            | Self::Yuva420p10Be
            | Self::Yuva422p10Le
            | Self::Yuva422p10Be
            | Self::Yuva444p10Le
            | Self::Yuva444p10Be
            | Self::Yuva422p12Le
            | Self::Yuva422p12Be
            | Self::Yuva444p12Le
            | Self::Yuva444p12Be
            | Self::Yuva420p16Le
            | Self::Yuva420p16Be
            | Self::Yuva422p16Le
            | Self::Yuva422p16Be
            | Self::Yuva444p16Le
            | Self::Yuva444p16Be => {
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
                    "planar YUVA chroma plane area",
                )?;
                let bytes_per_sample = if descriptor.bits_per_component > 8 {
                    2
                } else {
                    1
                };
                let luma = checked_mul(pixels, bytes_per_sample, "planar YUVA luma plane size")?;
                let chroma =
                    checked_mul(chroma, bytes_per_sample, "planar YUVA chroma plane size")?;
                let alpha = checked_mul(pixels, bytes_per_sample, "planar YUVA alpha plane size")?;
                Ok(vec![luma, chroma, chroma, alpha])
            }
            Self::Vaapi
            | Self::Dxva2Vld
            | Self::Vdpau
            | Self::Qsv
            | Self::Mmal
            | Self::D3d11VaVld
            | Self::Cuda
            | Self::VideoToolboxVld
            | Self::MediaCodec
            | Self::D3d11
            | Self::DrmPrime
            | Self::OpenCl
            | Self::Vulkan
            | Self::D3d12
            | Self::Amf
            | Self::OhCodec => Err(AvError::unsupported(format!(
                "hardware pixel format `{}` does not expose Rust-owned frame geometry",
                self.name()
            ))),
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

    fn bpp(bits: u8) -> Rational {
        Rational::from_raw(i32::from(bits), 1)
    }

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
            assert!(!format.is_paletted());
            assert_eq!(format.packed_bytes_per_pixel(), None);
        }
        assert_eq!(PixelFormat::Pal8.name(), "pal8");
        assert_eq!(PixelFormat::from_name("pal8"), Some(PixelFormat::Pal8));
        assert_eq!(PixelFormat::Pal8.plane_count(), 1);
        assert!(PixelFormat::Pal8.is_rgb());
        assert!(PixelFormat::Pal8.is_packed());
        assert!(PixelFormat::Pal8.has_alpha());
        assert!(PixelFormat::Pal8.is_paletted());
        assert_eq!(PixelFormat::Pal8.packed_bytes_per_pixel(), Some(1));
        assert_eq!(AVPALETTE_COUNT, 256);
        assert_eq!(AVPALETTE_SIZE, 1024);
        assert_eq!(PixelFormat::from_name("ya8"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("gray8a"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("y400a"), Some(PixelFormat::Ya8));
        assert_eq!(PixelFormat::from_name("ya16le"), Some(PixelFormat::Ya16Le));
        assert_eq!(PixelFormat::from_name("ya16be"), Some(PixelFormat::Ya16Be));
        assert_eq!(
            PixelFormat::from_name("yaf16le"),
            Some(PixelFormat::Yaf16Le)
        );
        assert_eq!(
            PixelFormat::from_name("yaf16be"),
            Some(PixelFormat::Yaf16Be)
        );
        assert_eq!(
            PixelFormat::from_name("yaf32le"),
            Some(PixelFormat::Yaf32Le)
        );
        assert_eq!(
            PixelFormat::from_name("yaf32be"),
            Some(PixelFormat::Yaf32Be)
        );
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
        for (name, format, bytes_per_pixel) in [
            ("bayer_bggr8", PixelFormat::BayerBggr8, 1),
            ("bayer_rggb8", PixelFormat::BayerRggb8, 1),
            ("bayer_gbrg8", PixelFormat::BayerGbrg8, 1),
            ("bayer_grbg8", PixelFormat::BayerGrbg8, 1),
            ("bayer_bggr16le", PixelFormat::BayerBggr16Le, 2),
            ("bayer_bggr16be", PixelFormat::BayerBggr16Be, 2),
            ("bayer_rggb16le", PixelFormat::BayerRggb16Le, 2),
            ("bayer_rggb16be", PixelFormat::BayerRggb16Be, 2),
            ("bayer_gbrg16le", PixelFormat::BayerGbrg16Le, 2),
            ("bayer_gbrg16be", PixelFormat::BayerGbrg16Be, 2),
            ("bayer_grbg16le", PixelFormat::BayerGrbg16Le, 2),
            ("bayer_grbg16be", PixelFormat::BayerGrbg16Be, 2),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_rgb());
            assert!(format.is_bayer());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(bytes_per_pixel));
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
            PixelFormat::from_name("rgbf16le"),
            Some(PixelFormat::RgbF16Le)
        );
        assert_eq!(
            PixelFormat::from_name("rgbf16be"),
            Some(PixelFormat::RgbF16Be)
        );
        assert_eq!(
            PixelFormat::from_name("rgbf32le"),
            Some(PixelFormat::RgbF32Le)
        );
        assert_eq!(
            PixelFormat::from_name("rgbf32be"),
            Some(PixelFormat::RgbF32Be)
        );
        assert_eq!(
            PixelFormat::from_name("rgbaf16le"),
            Some(PixelFormat::RgbaF16Le)
        );
        assert_eq!(
            PixelFormat::from_name("rgbaf16be"),
            Some(PixelFormat::RgbaF16Be)
        );
        assert_eq!(
            PixelFormat::from_name("rgbaf32le"),
            Some(PixelFormat::RgbaF32Le)
        );
        assert_eq!(
            PixelFormat::from_name("rgbaf32be"),
            Some(PixelFormat::RgbaF32Be)
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
        for (name, format) in [
            ("x2rgb10le", PixelFormat::X2Rgb10Le),
            ("x2rgb10be", PixelFormat::X2Rgb10Be),
            ("x2bgr10le", PixelFormat::X2Bgr10Le),
            ("x2bgr10be", PixelFormat::X2Bgr10Be),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_rgb());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(4));
        }
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
            PixelFormat::from_name("gbrp10msble"),
            Some(PixelFormat::Gbrp10MsbLe)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp10msbbe"),
            Some(PixelFormat::Gbrp10MsbBe)
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
            PixelFormat::from_name("gbrp12msble"),
            Some(PixelFormat::Gbrp12MsbLe)
        );
        assert_eq!(
            PixelFormat::from_name("gbrp12msbbe"),
            Some(PixelFormat::Gbrp12MsbBe)
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
        assert_eq!(
            PixelFormat::from_name("gbrpf16le"),
            Some(PixelFormat::GbrpF16Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrpf16be"),
            Some(PixelFormat::GbrpF16Be)
        );
        assert_eq!(
            PixelFormat::from_name("gbrpf32le"),
            Some(PixelFormat::GbrpF32Le)
        );
        assert_eq!(
            PixelFormat::from_name("gbrpf32be"),
            Some(PixelFormat::GbrpF32Be)
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
            ("ayuv64le", PixelFormat::Ayuv64Le),
            ("ayuv64be", PixelFormat::Ayuv64Be),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_yuv());
            assert!(format.is_packed());
            assert!(format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(8));
        }
        for (name, format, has_alpha, bytes_per_pixel) in [
            ("vuya", PixelFormat::Vuya, true, 4),
            ("vuyx", PixelFormat::Vuyx, false, 4),
            ("ayuv", PixelFormat::Ayuv, true, 4),
            ("uyva", PixelFormat::Uyva, true, 4),
            ("vyu444", PixelFormat::Vyu444, false, 3),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_yuv());
            assert!(format.is_packed());
            assert_eq!(format.has_alpha(), has_alpha);
            assert_eq!(format.packed_bytes_per_pixel(), Some(bytes_per_pixel));
        }
        for (name, format, bytes_per_pixel) in [
            ("xv30le", PixelFormat::Xv30Le, 4),
            ("xv30be", PixelFormat::Xv30Be, 4),
            ("xv36le", PixelFormat::Xv36Le, 8),
            ("xv36be", PixelFormat::Xv36Be, 8),
            ("xv48le", PixelFormat::Xv48Le, 8),
            ("xv48be", PixelFormat::Xv48Be, 8),
            ("v30xle", PixelFormat::V30xLe, 4),
            ("v30xbe", PixelFormat::V30xBe, 4),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_yuv());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(bytes_per_pixel));
        }
        for (name, format) in [
            ("xyz12le", PixelFormat::Xyz12Le),
            ("xyz12be", PixelFormat::Xyz12Be),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
            assert_eq!(format.plane_count(), 1);
            assert!(format.is_xyz());
            assert!(format.is_packed());
            assert!(!format.has_alpha());
            assert_eq!(format.packed_bytes_per_pixel(), Some(6));
        }
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
        assert_eq!(PixelFormat::Uyyvyy411.name(), "uyyvyy411");
        assert_eq!(
            PixelFormat::from_name("uyyvyy411"),
            Some(PixelFormat::Uyyvyy411)
        );
        assert_eq!(PixelFormat::Uyyvyy411.plane_count(), 1);
        assert!(PixelFormat::Uyyvyy411.is_yuv());
        assert!(PixelFormat::Uyyvyy411.is_packed());
        assert!(!PixelFormat::Uyyvyy411.has_alpha());
        assert_eq!(PixelFormat::Uyyvyy411.packed_bytes_per_pixel(), None);
        for (name, format) in [
            ("nv12", PixelFormat::Nv12),
            ("nv21", PixelFormat::Nv21),
            ("nv16", PixelFormat::Nv16),
            ("nv20le", PixelFormat::Nv20Le),
            ("nv20be", PixelFormat::Nv20Be),
            ("nv24", PixelFormat::Nv24),
            ("nv42", PixelFormat::Nv42),
            ("p010le", PixelFormat::P010Le),
            ("p010be", PixelFormat::P010Be),
            ("p012le", PixelFormat::P012Le),
            ("p012be", PixelFormat::P012Be),
            ("p016le", PixelFormat::P016Le),
            ("p016be", PixelFormat::P016Be),
            ("p210le", PixelFormat::P210Le),
            ("p210be", PixelFormat::P210Be),
            ("p212le", PixelFormat::P212Le),
            ("p212be", PixelFormat::P212Be),
            ("p216le", PixelFormat::P216Le),
            ("p216be", PixelFormat::P216Be),
            ("p410le", PixelFormat::P410Le),
            ("p410be", PixelFormat::P410Be),
            ("p412le", PixelFormat::P412Le),
            ("p412be", PixelFormat::P412Be),
            ("p416le", PixelFormat::P416Le),
            ("p416be", PixelFormat::P416Be),
        ] {
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
        for (name, format) in [
            ("yuvj420p", PixelFormat::YuvJ420p),
            ("yuvj422p", PixelFormat::YuvJ422p),
            ("yuvj411p", PixelFormat::YuvJ411p),
            ("yuvj440p", PixelFormat::YuvJ440p),
            ("yuvj444p", PixelFormat::YuvJ444p),
            ("yuva420p", PixelFormat::Yuva420p),
            ("yuva422p", PixelFormat::Yuva422p),
            ("yuva444p", PixelFormat::Yuva444p),
            ("yuva420p9le", PixelFormat::Yuva420p9Le),
            ("yuva420p9be", PixelFormat::Yuva420p9Be),
            ("yuva422p9le", PixelFormat::Yuva422p9Le),
            ("yuva422p9be", PixelFormat::Yuva422p9Be),
            ("yuva444p9le", PixelFormat::Yuva444p9Le),
            ("yuva444p9be", PixelFormat::Yuva444p9Be),
            ("yuva420p10le", PixelFormat::Yuva420p10Le),
            ("yuva420p10be", PixelFormat::Yuva420p10Be),
            ("yuva422p10le", PixelFormat::Yuva422p10Le),
            ("yuva422p10be", PixelFormat::Yuva422p10Be),
            ("yuva444p10le", PixelFormat::Yuva444p10Le),
            ("yuva444p10be", PixelFormat::Yuva444p10Be),
            ("yuva422p12le", PixelFormat::Yuva422p12Le),
            ("yuva422p12be", PixelFormat::Yuva422p12Be),
            ("yuva444p12le", PixelFormat::Yuva444p12Le),
            ("yuva444p12be", PixelFormat::Yuva444p12Be),
            ("yuva420p16le", PixelFormat::Yuva420p16Le),
            ("yuva420p16be", PixelFormat::Yuva420p16Be),
            ("yuva422p16le", PixelFormat::Yuva422p16Le),
            ("yuva422p16be", PixelFormat::Yuva422p16Be),
            ("yuva444p16le", PixelFormat::Yuva444p16Le),
            ("yuva444p16be", PixelFormat::Yuva444p16Be),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
        }
        for (name, format) in [
            ("yuv440p10le", PixelFormat::Yuv440p10Le),
            ("yuv440p10be", PixelFormat::Yuv440p10Be),
            ("yuv440p12le", PixelFormat::Yuv440p12Le),
            ("yuv440p12be", PixelFormat::Yuv440p12Be),
            ("yuv420p9le", PixelFormat::Yuv420p9Le),
            ("yuv420p9be", PixelFormat::Yuv420p9Be),
            ("yuv422p9le", PixelFormat::Yuv422p9Le),
            ("yuv422p9be", PixelFormat::Yuv422p9Be),
            ("yuv444p9le", PixelFormat::Yuv444p9Le),
            ("yuv444p9be", PixelFormat::Yuv444p9Be),
            ("yuv420p10le", PixelFormat::Yuv420p10Le),
            ("yuv420p10be", PixelFormat::Yuv420p10Be),
            ("yuv422p10le", PixelFormat::Yuv422p10Le),
            ("yuv422p10be", PixelFormat::Yuv422p10Be),
            ("yuv444p10le", PixelFormat::Yuv444p10Le),
            ("yuv444p10be", PixelFormat::Yuv444p10Be),
            ("yuv444p10msble", PixelFormat::Yuv444p10MsbLe),
            ("yuv444p10msbbe", PixelFormat::Yuv444p10MsbBe),
            ("yuv420p12le", PixelFormat::Yuv420p12Le),
            ("yuv420p12be", PixelFormat::Yuv420p12Be),
            ("yuv422p12le", PixelFormat::Yuv422p12Le),
            ("yuv422p12be", PixelFormat::Yuv422p12Be),
            ("yuv444p12le", PixelFormat::Yuv444p12Le),
            ("yuv444p12be", PixelFormat::Yuv444p12Be),
            ("yuv444p12msble", PixelFormat::Yuv444p12MsbLe),
            ("yuv444p12msbbe", PixelFormat::Yuv444p12MsbBe),
            ("yuv420p14le", PixelFormat::Yuv420p14Le),
            ("yuv420p14be", PixelFormat::Yuv420p14Be),
            ("yuv422p14le", PixelFormat::Yuv422p14Le),
            ("yuv422p14be", PixelFormat::Yuv422p14Be),
            ("yuv444p14le", PixelFormat::Yuv444p14Le),
            ("yuv444p14be", PixelFormat::Yuv444p14Be),
            ("yuv420p16le", PixelFormat::Yuv420p16Le),
            ("yuv420p16be", PixelFormat::Yuv420p16Be),
            ("yuv422p16le", PixelFormat::Yuv422p16Le),
            ("yuv422p16be", PixelFormat::Yuv422p16Be),
            ("yuv444p16le", PixelFormat::Yuv444p16Le),
            ("yuv444p16be", PixelFormat::Yuv444p16Be),
        ] {
            assert_eq!(format.name(), name);
            assert_eq!(PixelFormat::from_name(name), Some(format));
        }
        for format in PixelFormat::HARDWARE {
            let descriptor = format.descriptor();
            assert_eq!(PixelFormat::from_name(format.name()), Some(*format));
            assert_eq!(descriptor.class, PixelFormatClass::Hardware);
            assert!(format.is_hardware());
            assert_eq!(format.component_count(), 0);
            assert_eq!(format.bits_per_component(), 0);
            assert_eq!(format.bits_per_pixel_integer(), Some(0));
            assert_eq!(format.component_bit_depths(), vec![0]);
            assert_eq!(format.plane_count(), 0);
            assert_eq!(format.packed_bytes_per_pixel(), None);
            assert_eq!(
                format.plane_sizes(1, 1).unwrap_err().kind(),
                AvErrorKind::Unsupported
            );
        }
        assert_eq!(PixelFormat::ALL.len(), 251);
        assert_eq!(PixelFormat::HARDWARE.len(), 16);
        assert_eq!(PixelFormat::Ya8.plane_count(), 1);
        assert_eq!(PixelFormat::Ya16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Yaf16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Yaf32Le.plane_count(), 1);
        assert_eq!(PixelFormat::BayerBggr8.plane_count(), 1);
        assert_eq!(PixelFormat::BayerGrbg16Be.plane_count(), 1);
        assert_eq!(PixelFormat::Gray10Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray16Le.plane_count(), 1);
        assert_eq!(PixelFormat::Gray32Le.plane_count(), 1);
        assert_eq!(PixelFormat::GrayF16Le.plane_count(), 1);
        assert_eq!(PixelFormat::GrayF32Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgba.plane_count(), 1);
        assert_eq!(PixelFormat::Rgb48Le.plane_count(), 1);
        assert_eq!(PixelFormat::RgbaF16Le.plane_count(), 1);
        assert_eq!(PixelFormat::RgbaF32Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgba64Le.plane_count(), 1);
        assert_eq!(PixelFormat::Rgb0.plane_count(), 1);
        assert_eq!(PixelFormat::Yuyv422.plane_count(), 1);
        assert_eq!(PixelFormat::Uyvy422.plane_count(), 1);
        assert_eq!(PixelFormat::Yvyu422.plane_count(), 1);
        assert_eq!(PixelFormat::Uyyvyy411.plane_count(), 1);
        assert_eq!(PixelFormat::Nv12.plane_count(), 2);
        assert_eq!(PixelFormat::Nv21.plane_count(), 2);
        assert_eq!(PixelFormat::Nv16.plane_count(), 2);
        assert_eq!(PixelFormat::Nv20Le.plane_count(), 2);
        assert_eq!(PixelFormat::Nv20Be.plane_count(), 2);
        assert_eq!(PixelFormat::Nv24.plane_count(), 2);
        assert_eq!(PixelFormat::Nv42.plane_count(), 2);
        assert_eq!(PixelFormat::Yuv420p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv422p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv410p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv411p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv440p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv444p.plane_count(), 3);
        for format in [
            PixelFormat::YuvJ420p,
            PixelFormat::YuvJ422p,
            PixelFormat::YuvJ411p,
            PixelFormat::YuvJ440p,
            PixelFormat::YuvJ444p,
            PixelFormat::Yuv440p10Le,
            PixelFormat::Yuv440p10Be,
            PixelFormat::Yuv440p12Le,
            PixelFormat::Yuv440p12Be,
            PixelFormat::Yuv420p9Le,
            PixelFormat::Yuv420p9Be,
            PixelFormat::Yuv422p9Le,
            PixelFormat::Yuv422p9Be,
            PixelFormat::Yuv444p9Le,
            PixelFormat::Yuv444p9Be,
            PixelFormat::Yuv420p10Le,
            PixelFormat::Yuv420p10Be,
            PixelFormat::Yuv422p10Le,
            PixelFormat::Yuv422p10Be,
            PixelFormat::Yuv444p10Le,
            PixelFormat::Yuv444p10Be,
            PixelFormat::Yuv444p10MsbLe,
            PixelFormat::Yuv444p10MsbBe,
            PixelFormat::Yuv420p12Le,
            PixelFormat::Yuv420p12Be,
            PixelFormat::Yuv422p12Le,
            PixelFormat::Yuv422p12Be,
            PixelFormat::Yuv444p12Le,
            PixelFormat::Yuv444p12Be,
            PixelFormat::Yuv444p12MsbLe,
            PixelFormat::Yuv444p12MsbBe,
            PixelFormat::Yuv420p14Le,
            PixelFormat::Yuv420p14Be,
            PixelFormat::Yuv422p14Le,
            PixelFormat::Yuv422p14Be,
            PixelFormat::Yuv444p14Le,
            PixelFormat::Yuv444p14Be,
            PixelFormat::Yuv420p16Le,
            PixelFormat::Yuv420p16Be,
            PixelFormat::Yuv422p16Le,
            PixelFormat::Yuv422p16Be,
            PixelFormat::Yuv444p16Le,
            PixelFormat::Yuv444p16Be,
        ] {
            assert_eq!(format.plane_count(), 3);
        }
        assert_eq!(PixelFormat::Gbrp.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp9Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp9Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp10Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp10Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp10MsbLe.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp10MsbBe.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp12Le.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp12Be.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp12MsbLe.plane_count(), 3);
        assert_eq!(PixelFormat::Gbrp12MsbBe.plane_count(), 3);
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
        assert!(PixelFormat::Pal8.is_packed());
        assert!(PixelFormat::Rgb8.is_packed());
        assert!(PixelFormat::Rgb4.is_packed());
        assert!(PixelFormat::Bgr4Byte.is_packed());
        assert!(PixelFormat::Ya8.is_packed());
        assert!(PixelFormat::Ya16Be.is_packed());
        assert!(PixelFormat::Yaf16Be.is_packed());
        assert!(PixelFormat::Yaf32Be.is_packed());
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
        assert!(PixelFormat::Uyyvyy411.is_packed());
        assert!(PixelFormat::Ayuv64Le.is_packed());
        assert!(PixelFormat::Ayuv64Be.is_packed());
        assert!(PixelFormat::Nv12.is_planar());
        assert!(PixelFormat::Nv21.is_planar());
        assert!(PixelFormat::Nv16.is_planar());
        assert!(PixelFormat::Nv20Le.is_planar());
        assert!(PixelFormat::Nv20Be.is_planar());
        assert!(PixelFormat::Nv24.is_planar());
        assert!(PixelFormat::Nv42.is_planar());
        assert!(PixelFormat::Yuv420p.is_planar());
        assert!(PixelFormat::Yuv422p.is_planar());
        assert!(PixelFormat::Yuv410p.is_planar());
        assert!(PixelFormat::Yuv411p.is_planar());
        assert!(PixelFormat::Yuv440p.is_planar());
        assert!(PixelFormat::Yuv444p.is_planar());
        for format in [
            PixelFormat::YuvJ420p,
            PixelFormat::YuvJ422p,
            PixelFormat::YuvJ411p,
            PixelFormat::YuvJ440p,
            PixelFormat::YuvJ444p,
            PixelFormat::Yuv440p10Le,
            PixelFormat::Yuv440p10Be,
            PixelFormat::Yuv440p12Le,
            PixelFormat::Yuv440p12Be,
            PixelFormat::Yuv420p9Le,
            PixelFormat::Yuv420p9Be,
            PixelFormat::Yuv422p9Le,
            PixelFormat::Yuv422p9Be,
            PixelFormat::Yuv444p9Le,
            PixelFormat::Yuv444p9Be,
            PixelFormat::Yuv420p10Le,
            PixelFormat::Yuv420p10Be,
            PixelFormat::Yuv422p10Le,
            PixelFormat::Yuv422p10Be,
            PixelFormat::Yuv444p10Le,
            PixelFormat::Yuv444p10Be,
            PixelFormat::Yuv444p10MsbLe,
            PixelFormat::Yuv444p10MsbBe,
            PixelFormat::Yuv420p12Le,
            PixelFormat::Yuv420p12Be,
            PixelFormat::Yuv422p12Le,
            PixelFormat::Yuv422p12Be,
            PixelFormat::Yuv444p12Le,
            PixelFormat::Yuv444p12Be,
            PixelFormat::Yuv444p12MsbLe,
            PixelFormat::Yuv444p12MsbBe,
        ] {
            assert!(format.is_planar());
            assert!(!format.is_packed());
            assert_eq!(format.packed_bytes_per_pixel(), None);
        }
        assert!(PixelFormat::Gbrp.is_planar());
        assert!(PixelFormat::Gbrp9Le.is_planar());
        assert!(PixelFormat::Gbrp10Le.is_planar());
        assert!(PixelFormat::Gbrp10MsbLe.is_planar());
        assert!(PixelFormat::Gbrp12Le.is_planar());
        assert!(PixelFormat::Gbrp12MsbLe.is_planar());
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
        assert!(!PixelFormat::Gbrp10MsbLe.is_packed());
        assert!(!PixelFormat::Gbrp12Le.is_packed());
        assert!(!PixelFormat::Gbrp12MsbLe.is_packed());
        assert!(!PixelFormat::Gbrp14Le.is_packed());
        assert!(!PixelFormat::Gbrp16Le.is_packed());
        assert!(!PixelFormat::Gbrap.is_packed());
        assert!(!PixelFormat::Gbrap16Le.is_packed());
        assert!(!PixelFormat::Gbrap32Le.is_packed());
        assert!(!PixelFormat::GbrapF16Le.is_packed());
        assert!(!PixelFormat::GbrapF32Le.is_packed());
        assert!(!PixelFormat::Rgb24.has_alpha());
        assert!(PixelFormat::Pal8.has_alpha());
        assert!(PixelFormat::Ya8.has_alpha());
        assert!(PixelFormat::Ya16Le.has_alpha());
        assert!(PixelFormat::Yaf16Le.has_alpha());
        assert!(PixelFormat::Yaf32Be.has_alpha());
        assert!(PixelFormat::Bgra.has_alpha());
        assert!(PixelFormat::Rgba64Le.has_alpha());
        assert!(PixelFormat::Gbrap.has_alpha());
        assert!(PixelFormat::Gbrap16Le.has_alpha());
        assert!(PixelFormat::GbrapF32Le.has_alpha());
        assert!(PixelFormat::Yuva420p.has_alpha());
        assert!(PixelFormat::Yuva444p.has_alpha());
        assert!(PixelFormat::Yuva420p9Le.has_alpha());
        assert!(PixelFormat::Yuva422p12Be.has_alpha());
        assert!(PixelFormat::Yuva444p16Be.has_alpha());
        assert!(PixelFormat::Ayuv64Le.has_alpha());
        assert!(PixelFormat::Ayuv64Be.has_alpha());
        assert!(!PixelFormat::ZeroRgb.has_alpha());
        assert!(!PixelFormat::Gray16Le.is_float());
        assert!(PixelFormat::GrayF16Le.is_float());
        assert!(PixelFormat::GrayF32Be.is_float());
        assert!(PixelFormat::Yaf16Le.is_float());
        assert!(PixelFormat::Yaf32Be.is_float());
        assert!(!PixelFormat::Gbrap16Le.is_float());
        assert!(PixelFormat::GbrapF16Le.is_float());
        assert!(PixelFormat::GbrapF32Be.is_float());
        assert_eq!(PixelFormat::MonoWhite.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Pal8.packed_bytes_per_pixel(), Some(1));
        assert_eq!(PixelFormat::Bgr24.packed_bytes_per_pixel(), Some(3));
        assert_eq!(PixelFormat::Rgb8.packed_bytes_per_pixel(), Some(1));
        assert_eq!(PixelFormat::Rgb4.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Bgr4Byte.packed_bytes_per_pixel(), Some(1));
        assert_eq!(PixelFormat::Ya8.packed_bytes_per_pixel(), Some(2));
        assert_eq!(PixelFormat::Ya16Be.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Yaf16Le.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Yaf32Le.packed_bytes_per_pixel(), Some(8));
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
        assert_eq!(PixelFormat::Uyyvyy411.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Y210Le.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Y212Be.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Y216Le.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Ayuv64Le.packed_bytes_per_pixel(), Some(8));
        assert_eq!(PixelFormat::Ayuv64Be.packed_bytes_per_pixel(), Some(8));
        assert_eq!(PixelFormat::Nv12.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv21.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv16.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv20Le.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv20Be.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv24.packed_bytes_per_pixel(), None);
        assert_eq!(PixelFormat::Nv42.packed_bytes_per_pixel(), None);
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
        assert_eq!(gray.bits_per_pixel, bpp(8));
        assert_eq!(gray.plane_count, 1);
        assert!(!gray.is_planar);
        assert!(!gray.has_alpha);
        assert!(!gray.is_float);
        assert!(!gray.is_paletted);
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
            assert_eq!(descriptor.bits_per_pixel, bpp(1));
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert!(!descriptor.is_paletted);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), (0, 0));
        }

        let pal8 = PixelFormat::Pal8.descriptor();
        assert_eq!(pal8.format, PixelFormat::Pal8);
        assert_eq!(pal8.name, "pal8");
        assert_eq!(PixelFormat::from_name(pal8.name), Some(PixelFormat::Pal8));
        assert_eq!(pal8.class, PixelFormatClass::Rgb);
        assert!(PixelFormat::Pal8.is_rgb());
        assert!(!PixelFormat::Pal8.is_gray());
        assert!(!PixelFormat::Pal8.is_yuv());
        assert_eq!(pal8.component_count, 1);
        assert_eq!(pal8.bits_per_component, 8);
        assert_eq!(pal8.bits_per_pixel, bpp(8));
        assert_eq!(pal8.plane_count, 1);
        assert!(!pal8.is_planar);
        assert!(pal8.has_alpha);
        assert!(!pal8.is_float);
        assert!(pal8.is_paletted);
        assert_eq!(pal8.packed_bytes_per_pixel, Some(1));
        assert_eq!(PixelFormat::Pal8.log2_chroma(), (0, 0));
        assert!(!PixelFormat::Pal8.has_chroma_subsampling());

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
        assert_eq!(ya8.bits_per_pixel, bpp(16));
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
            assert_eq!(descriptor.bits_per_pixel, bpp(32));
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(4));
            assert_eq!(format.log2_chroma(), (0, 0));
        }

        for (
            format,
            expected_name,
            expected_bits_per_component,
            expected_bits_per_pixel,
            expected_packed_bytes,
        ) in [
            (PixelFormat::Yaf16Le, "yaf16le", 16, bpp(32), 4),
            (PixelFormat::Yaf16Be, "yaf16be", 16, bpp(32), 4),
            (PixelFormat::Yaf32Le, "yaf32le", 32, bpp(64), 8),
            (PixelFormat::Yaf32Be, "yaf32be", 32, bpp(64), 8),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Gray);
            assert!(format.is_gray());
            assert_eq!(descriptor.component_count, 2);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(descriptor.has_alpha);
            assert!(descriptor.is_float);
            assert_eq!(
                descriptor.packed_bytes_per_pixel,
                Some(expected_packed_bytes)
            );
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
            assert_eq!(descriptor.bits_per_pixel, bpp(expected_bits_per_component));
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
            assert_eq!(descriptor.bits_per_pixel, bpp(32));
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
            assert_eq!(descriptor.bits_per_pixel, bpp(expected_bits_per_component));
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

        for (format, expected_bits_per_component, expected_is_float) in [
            (PixelFormat::Rgb24, 8, false),
            (PixelFormat::Bgr24, 8, false),
            (PixelFormat::Rgb8, 3, false),
            (PixelFormat::Bgr8, 3, false),
            (PixelFormat::Rgb4, 2, false),
            (PixelFormat::Bgr4, 2, false),
            (PixelFormat::Rgb4Byte, 2, false),
            (PixelFormat::Bgr4Byte, 2, false),
            (PixelFormat::Rgb48Le, 16, false),
            (PixelFormat::Rgb48Be, 16, false),
            (PixelFormat::Bgr48Le, 16, false),
            (PixelFormat::Bgr48Be, 16, false),
            (PixelFormat::RgbF16Le, 16, true),
            (PixelFormat::RgbF16Be, 16, true),
            (PixelFormat::RgbF32Le, 32, true),
            (PixelFormat::RgbF32Be, 32, true),
            (PixelFormat::Rgb96Le, 32, false),
            (PixelFormat::Rgb96Be, 32, false),
            (PixelFormat::RgbaF16Le, 16, true),
            (PixelFormat::RgbaF16Be, 16, true),
            (PixelFormat::RgbaF32Le, 32, true),
            (PixelFormat::RgbaF32Be, 32, true),
            (PixelFormat::Rgba128Le, 32, false),
            (PixelFormat::Rgba128Be, 32, false),
            (PixelFormat::Rgba64Le, 16, false),
            (PixelFormat::Rgba64Be, 16, false),
            (PixelFormat::Bgra64Le, 16, false),
            (PixelFormat::Bgra64Be, 16, false),
            (PixelFormat::Rgba, 8, false),
            (PixelFormat::Bgra, 8, false),
            (PixelFormat::Argb, 8, false),
            (PixelFormat::Abgr, 8, false),
            (PixelFormat::ZeroRgb, 8, false),
            (PixelFormat::Rgb0, 8, false),
            (PixelFormat::ZeroBgr, 8, false),
            (PixelFormat::Bgr0, 8, false),
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
            assert_eq!(descriptor.is_float, expected_is_float);
            assert_eq!(
                descriptor.packed_bytes_per_pixel,
                format.packed_bytes_per_pixel()
            );
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }

        for (format, expected_name, expected_bits_per_component, expected_bits_per_pixel) in [
            (PixelFormat::Rgb565Be, "rgb565be", 6, bpp(16)),
            (PixelFormat::Rgb565Le, "rgb565le", 6, bpp(16)),
            (PixelFormat::Rgb555Be, "rgb555be", 5, bpp(15)),
            (PixelFormat::Rgb555Le, "rgb555le", 5, bpp(15)),
            (PixelFormat::Bgr565Be, "bgr565be", 6, bpp(16)),
            (PixelFormat::Bgr565Le, "bgr565le", 6, bpp(16)),
            (PixelFormat::Bgr555Be, "bgr555be", 5, bpp(15)),
            (PixelFormat::Bgr555Le, "bgr555le", 5, bpp(15)),
            (PixelFormat::Rgb444Le, "rgb444le", 4, bpp(12)),
            (PixelFormat::Rgb444Be, "rgb444be", 4, bpp(12)),
            (PixelFormat::Bgr444Le, "bgr444le", 4, bpp(12)),
            (PixelFormat::Bgr444Be, "bgr444be", 4, bpp(12)),
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
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_pixel);
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
        assert_eq!(gbrp.bits_per_pixel, bpp(24));
        assert_eq!(gbrp.plane_count, 3);
        assert!(gbrp.is_planar);
        assert!(!gbrp.has_alpha);
        assert!(!gbrp.is_float);
        assert_eq!(gbrp.packed_bytes_per_pixel, None);
        assert_eq!(PixelFormat::Gbrp.log2_chroma(), (0, 0));
        assert!(!PixelFormat::Gbrp.has_chroma_subsampling());

        for (format, name, expected_bits_per_component, expected_bits_per_pixel, is_float) in [
            (PixelFormat::Gbrp9Le, "gbrp9le", 9, 27, false),
            (PixelFormat::Gbrp9Be, "gbrp9be", 9, 27, false),
            (PixelFormat::Gbrp10Le, "gbrp10le", 10, 30, false),
            (PixelFormat::Gbrp10Be, "gbrp10be", 10, 30, false),
            (PixelFormat::Gbrp10MsbLe, "gbrp10msble", 10, 30, false),
            (PixelFormat::Gbrp10MsbBe, "gbrp10msbbe", 10, 30, false),
            (PixelFormat::Gbrp12Le, "gbrp12le", 12, 36, false),
            (PixelFormat::Gbrp12Be, "gbrp12be", 12, 36, false),
            (PixelFormat::Gbrp12MsbLe, "gbrp12msble", 12, 36, false),
            (PixelFormat::Gbrp12MsbBe, "gbrp12msbbe", 12, 36, false),
            (PixelFormat::Gbrp14Le, "gbrp14le", 14, 42, false),
            (PixelFormat::Gbrp14Be, "gbrp14be", 14, 42, false),
            (PixelFormat::Gbrp16Le, "gbrp16le", 16, 48, false),
            (PixelFormat::Gbrp16Be, "gbrp16be", 16, 48, false),
            (PixelFormat::GbrpF16Le, "gbrpf16le", 16, 48, true),
            (PixelFormat::GbrpF16Be, "gbrpf16be", 16, 48, true),
            (PixelFormat::GbrpF32Le, "gbrpf32le", 32, 96, true),
            (PixelFormat::GbrpF32Be, "gbrpf32be", 32, 96, true),
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
            assert_eq!(descriptor.bits_per_pixel, bpp(expected_bits_per_pixel));
            assert_eq!(descriptor.plane_count, 3);
            assert!(descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert_eq!(descriptor.is_float, is_float);
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
        assert_eq!(gbrap.bits_per_pixel, bpp(32));
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
            assert_eq!(descriptor.bits_per_pixel, bpp(expected_bits_per_pixel));
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
            assert_eq!(descriptor.bits_per_pixel, bpp(16));
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(2));
            assert_eq!(format.log2_chroma(), (1, 0));
            assert!(format.has_chroma_subsampling());
        }

        let uyyvyy411 = PixelFormat::Uyyvyy411.descriptor();
        assert_eq!(uyyvyy411.format, PixelFormat::Uyyvyy411);
        assert_eq!(uyyvyy411.name, "uyyvyy411");
        assert_eq!(
            PixelFormat::from_name(uyyvyy411.name),
            Some(PixelFormat::Uyyvyy411)
        );
        assert_eq!(uyyvyy411.class, PixelFormatClass::Yuv);
        assert!(PixelFormat::Uyyvyy411.is_yuv());
        assert!(!PixelFormat::Uyyvyy411.is_rgb());
        assert!(!PixelFormat::Uyyvyy411.is_gray());
        assert_eq!(uyyvyy411.component_count, 3);
        assert_eq!(uyyvyy411.bits_per_component, 8);
        assert_eq!(uyyvyy411.bits_per_pixel, bpp(12));
        assert_eq!(uyyvyy411.plane_count, 1);
        assert!(!uyyvyy411.is_planar);
        assert!(!uyyvyy411.has_alpha);
        assert!(!uyyvyy411.is_float);
        assert_eq!(uyyvyy411.packed_bytes_per_pixel, None);
        assert_eq!(PixelFormat::Uyyvyy411.log2_chroma(), (2, 0));
        assert!(PixelFormat::Uyyvyy411.has_chroma_subsampling());

        for (format, expected_name, expected_bits_per_component, expected_bits_per_pixel) in [
            (PixelFormat::Y210Le, "y210le", 10, bpp(20)),
            (PixelFormat::Y210Be, "y210be", 10, bpp(20)),
            (PixelFormat::Y212Le, "y212le", 12, bpp(24)),
            (PixelFormat::Y212Be, "y212be", 12, bpp(24)),
            (PixelFormat::Y216Le, "y216le", 16, bpp(32)),
            (PixelFormat::Y216Be, "y216be", 16, bpp(32)),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(4));
            assert_eq!(format.log2_chroma(), (1, 0));
            assert!(format.has_chroma_subsampling());
        }

        for (format, expected_name) in [
            (PixelFormat::Ayuv64Le, "ayuv64le"),
            (PixelFormat::Ayuv64Be, "ayuv64be"),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 4);
            assert_eq!(descriptor.bits_per_component, 16);
            assert_eq!(descriptor.bits_per_pixel, bpp(64));
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(8));
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }

        for (format, expected_name) in [
            (PixelFormat::Xyz12Le, "xyz12le"),
            (PixelFormat::Xyz12Be, "xyz12be"),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Xyz);
            assert!(format.is_xyz());
            assert!(!format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, 12);
            assert_eq!(descriptor.bits_per_pixel, bpp(36));
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(6));
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
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
            assert_eq!(descriptor.bits_per_pixel, bpp(12));
            assert_eq!(descriptor.plane_count, 2);
            assert!(descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), (1, 1));
            assert!(format.has_chroma_subsampling());
        }

        for (
            format,
            expected_name,
            expected_bits_per_component,
            expected_bits_per_pixel,
            expected_log2_chroma,
        ) in [
            (PixelFormat::Nv16, "nv16", 8, bpp(16), (1, 0)),
            (PixelFormat::Nv20Le, "nv20le", 10, bpp(20), (1, 0)),
            (PixelFormat::Nv20Be, "nv20be", 10, bpp(20), (1, 0)),
            (PixelFormat::Nv24, "nv24", 8, bpp(24), (0, 0)),
            (PixelFormat::Nv42, "nv42", 8, bpp(24), (0, 0)),
            (PixelFormat::P010Le, "p010le", 10, bpp(15), (1, 1)),
            (PixelFormat::P010Be, "p010be", 10, bpp(15), (1, 1)),
            (PixelFormat::P012Le, "p012le", 12, bpp(18), (1, 1)),
            (PixelFormat::P012Be, "p012be", 12, bpp(18), (1, 1)),
            (PixelFormat::P016Le, "p016le", 16, bpp(24), (1, 1)),
            (PixelFormat::P016Be, "p016be", 16, bpp(24), (1, 1)),
            (PixelFormat::P210Le, "p210le", 10, bpp(20), (1, 0)),
            (PixelFormat::P210Be, "p210be", 10, bpp(20), (1, 0)),
            (PixelFormat::P212Le, "p212le", 12, bpp(24), (1, 0)),
            (PixelFormat::P212Be, "p212be", 12, bpp(24), (1, 0)),
            (PixelFormat::P216Le, "p216le", 16, bpp(32), (1, 0)),
            (PixelFormat::P216Be, "p216be", 16, bpp(32), (1, 0)),
            (PixelFormat::P410Le, "p410le", 10, bpp(30), (0, 0)),
            (PixelFormat::P410Be, "p410be", 10, bpp(30), (0, 0)),
            (PixelFormat::P412Le, "p412le", 12, bpp(36), (0, 0)),
            (PixelFormat::P412Be, "p412be", 12, bpp(36), (0, 0)),
            (PixelFormat::P416Le, "p416le", 16, bpp(48), (0, 0)),
            (PixelFormat::P416Be, "p416be", 16, bpp(48), (0, 0)),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(descriptor.name, expected_name);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, expected_bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(descriptor.plane_count, 2);
            assert!(descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), expected_log2_chroma);
            assert_eq!(
                format.has_chroma_subsampling(),
                expected_log2_chroma != (0, 0)
            );
        }

        assert_eq!(PixelFormat::Rgb24.component_count(), 3);
        assert_eq!(PixelFormat::Pal8.component_count(), 1);
        assert_eq!(PixelFormat::Pal8.bits_per_component(), 8);
        assert_eq!(PixelFormat::Pal8.bits_per_pixel(), bpp(8));
        assert_eq!(PixelFormat::Rgb24.bits_per_pixel(), bpp(24));
        assert_eq!(PixelFormat::Rgb8.component_count(), 3);
        assert_eq!(PixelFormat::Rgb8.bits_per_component(), 3);
        assert_eq!(PixelFormat::Rgb8.component_bit_depths(), vec![3, 3, 2]);
        assert_eq!(PixelFormat::Bgr8.component_bit_depths(), vec![3, 3, 2]);
        assert_eq!(PixelFormat::Bgr8.bits_per_pixel(), bpp(8));
        assert_eq!(PixelFormat::Rgb4.bits_per_component(), 2);
        assert_eq!(PixelFormat::Rgb4.component_bit_depths(), vec![1, 2, 1]);
        assert_eq!(PixelFormat::Bgr4.component_bit_depths(), vec![1, 2, 1]);
        assert_eq!(PixelFormat::Bgr4.bits_per_pixel(), bpp(4));
        assert_eq!(PixelFormat::Rgb4Byte.bits_per_component(), 2);
        assert_eq!(PixelFormat::Rgb4Byte.component_bit_depths(), vec![1, 2, 1]);
        assert_eq!(PixelFormat::Bgr4Byte.component_bit_depths(), vec![1, 2, 1]);
        assert_eq!(PixelFormat::Bgr4Byte.bits_per_pixel(), bpp(4));
        assert_eq!(PixelFormat::BayerBggr8.component_count(), 3);
        assert_eq!(PixelFormat::BayerBggr8.bits_per_component(), 8);
        assert_eq!(PixelFormat::BayerBggr8.bits_per_pixel(), bpp(8));
        assert_eq!(
            PixelFormat::BayerBggr8.component_bit_depths(),
            vec![2, 4, 2]
        );
        assert_eq!(PixelFormat::BayerBggr8.packed_bytes_per_pixel(), Some(1));
        assert!(PixelFormat::BayerBggr8.is_bayer());
        assert_eq!(PixelFormat::BayerGrbg16Be.component_count(), 3);
        assert_eq!(PixelFormat::BayerGrbg16Be.bits_per_component(), 16);
        assert_eq!(PixelFormat::BayerGrbg16Be.bits_per_pixel(), bpp(16));
        assert_eq!(
            PixelFormat::BayerGrbg16Be.component_bit_depths(),
            vec![4, 8, 4]
        );
        assert_eq!(PixelFormat::BayerGrbg16Be.packed_bytes_per_pixel(), Some(2));
        assert!(PixelFormat::BayerGrbg16Be.is_bayer());
        assert_eq!(PixelFormat::RgbF16Le.component_count(), 3);
        assert_eq!(PixelFormat::RgbF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::RgbF16Le.bits_per_pixel(), bpp(48));
        assert_eq!(PixelFormat::RgbF16Le.packed_bytes_per_pixel(), Some(6));
        assert!(PixelFormat::RgbF16Le.is_float());
        assert_eq!(PixelFormat::RgbF32Le.component_count(), 3);
        assert_eq!(PixelFormat::RgbF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::RgbF32Le.bits_per_pixel(), bpp(96));
        assert_eq!(PixelFormat::RgbF32Le.packed_bytes_per_pixel(), Some(12));
        assert!(PixelFormat::RgbF32Le.is_float());
        assert_eq!(PixelFormat::RgbaF16Le.component_count(), 4);
        assert_eq!(PixelFormat::RgbaF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::RgbaF16Le.bits_per_pixel(), bpp(64));
        assert_eq!(PixelFormat::RgbaF16Le.packed_bytes_per_pixel(), Some(8));
        assert!(PixelFormat::RgbaF16Le.has_alpha());
        assert!(PixelFormat::RgbaF16Le.is_float());
        assert_eq!(PixelFormat::Rgb96Le.component_count(), 3);
        assert_eq!(PixelFormat::Rgb96Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::Rgb96Le.bits_per_pixel(), bpp(96));
        assert_eq!(PixelFormat::Rgb96Le.packed_bytes_per_pixel(), Some(12));
        assert!(!PixelFormat::Rgb96Le.has_alpha());
        assert!(!PixelFormat::Rgb96Le.is_float());
        assert_eq!(PixelFormat::RgbaF32Le.component_count(), 4);
        assert_eq!(PixelFormat::RgbaF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::RgbaF32Le.bits_per_pixel(), bpp(128));
        assert_eq!(PixelFormat::RgbaF32Le.packed_bytes_per_pixel(), Some(16));
        assert!(PixelFormat::RgbaF32Le.has_alpha());
        assert!(PixelFormat::RgbaF32Le.is_float());
        assert_eq!(PixelFormat::Rgba128Be.component_count(), 4);
        assert_eq!(PixelFormat::Rgba128Be.bits_per_component(), 32);
        assert_eq!(PixelFormat::Rgba128Be.bits_per_pixel(), bpp(128));
        assert_eq!(PixelFormat::Rgba128Be.packed_bytes_per_pixel(), Some(16));
        assert!(PixelFormat::Rgba128Be.has_alpha());
        assert!(!PixelFormat::Rgba128Be.is_float());
        assert_eq!(PixelFormat::Gbrp.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp.bits_per_component(), 8);
        assert_eq!(PixelFormat::Gbrp.bits_per_pixel(), bpp(24));
        assert_eq!(PixelFormat::Gbrp9Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp9Le.bits_per_component(), 9);
        assert_eq!(PixelFormat::Gbrp9Le.bits_per_pixel(), bpp(27));
        assert_eq!(PixelFormat::Gbrp10Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp10Le.bits_per_component(), 10);
        assert_eq!(PixelFormat::Gbrp10Le.bits_per_pixel(), bpp(30));
        assert_eq!(PixelFormat::Gbrp12Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp12Le.bits_per_component(), 12);
        assert_eq!(PixelFormat::Gbrp12Le.bits_per_pixel(), bpp(36));
        assert_eq!(PixelFormat::Gbrp14Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp14Le.bits_per_component(), 14);
        assert_eq!(PixelFormat::Gbrp14Le.bits_per_pixel(), bpp(42));
        assert_eq!(PixelFormat::Gbrp16Le.component_count(), 3);
        assert_eq!(PixelFormat::Gbrp16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Gbrp16Le.bits_per_pixel(), bpp(48));
        assert_eq!(PixelFormat::GbrpF16Le.component_count(), 3);
        assert_eq!(PixelFormat::GbrpF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::GbrpF16Le.bits_per_pixel(), bpp(48));
        assert!(PixelFormat::GbrpF16Le.is_float());
        assert_eq!(PixelFormat::GbrpF32Le.component_count(), 3);
        assert_eq!(PixelFormat::GbrpF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::GbrpF32Le.bits_per_pixel(), bpp(96));
        assert!(PixelFormat::GbrpF32Le.is_float());
        assert_eq!(PixelFormat::Gbrap.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap.bits_per_component(), 8);
        assert_eq!(PixelFormat::Gbrap.bits_per_pixel(), bpp(32));
        assert_eq!(PixelFormat::Gbrap10Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap10Le.bits_per_component(), 10);
        assert_eq!(PixelFormat::Gbrap10Le.bits_per_pixel(), bpp(40));
        assert_eq!(PixelFormat::Gbrap12Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap12Le.bits_per_component(), 12);
        assert_eq!(PixelFormat::Gbrap12Le.bits_per_pixel(), bpp(48));
        assert_eq!(PixelFormat::Gbrap14Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap14Le.bits_per_component(), 14);
        assert_eq!(PixelFormat::Gbrap14Le.bits_per_pixel(), bpp(56));
        assert_eq!(PixelFormat::Gbrap16Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Gbrap16Le.bits_per_pixel(), bpp(64));
        assert_eq!(PixelFormat::Gbrap32Le.component_count(), 4);
        assert_eq!(PixelFormat::Gbrap32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::Gbrap32Le.bits_per_pixel(), bpp(128));
        assert_eq!(PixelFormat::GbrapF16Le.component_count(), 4);
        assert_eq!(PixelFormat::GbrapF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::GbrapF16Le.bits_per_pixel(), bpp(64));
        assert_eq!(PixelFormat::GbrapF32Le.component_count(), 4);
        assert_eq!(PixelFormat::GbrapF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::GbrapF32Le.bits_per_pixel(), bpp(128));
        assert_eq!(PixelFormat::Ya16Le.component_count(), 2);
        assert_eq!(PixelFormat::Ya16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Ya16Le.bits_per_pixel(), bpp(32));
        assert_eq!(PixelFormat::Yaf16Le.component_count(), 2);
        assert_eq!(PixelFormat::Yaf16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Yaf16Le.bits_per_pixel(), bpp(32));
        assert_eq!(PixelFormat::Yaf32Le.component_count(), 2);
        assert_eq!(PixelFormat::Yaf32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::Yaf32Le.bits_per_pixel(), bpp(64));
        assert_eq!(PixelFormat::Gray32Le.component_count(), 1);
        assert_eq!(PixelFormat::Gray32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::Gray32Le.bits_per_pixel(), bpp(32));
        assert_eq!(PixelFormat::GrayF16Le.component_count(), 1);
        assert_eq!(PixelFormat::GrayF16Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::GrayF16Le.bits_per_pixel(), bpp(16));
        assert_eq!(PixelFormat::GrayF32Le.component_count(), 1);
        assert_eq!(PixelFormat::GrayF32Le.bits_per_component(), 32);
        assert_eq!(PixelFormat::GrayF32Le.bits_per_pixel(), bpp(32));
        assert_eq!(PixelFormat::Rgb48Le.component_count(), 3);
        assert_eq!(PixelFormat::Rgb48Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Rgb48Le.bits_per_pixel(), bpp(48));
        assert_eq!(PixelFormat::Bgr48Be.bits_per_component(), 16);
        assert_eq!(PixelFormat::Bgr48Be.bits_per_pixel(), bpp(48));
        assert_eq!(PixelFormat::Rgb565Le.component_count(), 3);
        assert_eq!(PixelFormat::Rgb565Le.bits_per_component(), 6);
        assert_eq!(PixelFormat::Rgb565Le.component_bit_depths(), vec![5, 6, 5]);
        assert_eq!(PixelFormat::Bgr565Be.component_bit_depths(), vec![5, 6, 5]);
        assert_eq!(PixelFormat::Rgb565Le.bits_per_pixel(), bpp(16));
        assert_eq!(PixelFormat::Bgr555Be.bits_per_component(), 5);
        assert_eq!(PixelFormat::Rgb24.component_bit_depths(), vec![8, 8, 8]);
        assert_eq!(PixelFormat::Bgr555Be.component_bit_depths(), vec![5, 5, 5]);
        assert_eq!(
            PixelFormat::Rgba64Le.component_bit_depths(),
            vec![16, 16, 16, 16]
        );
        assert_eq!(PixelFormat::Bgr555Be.bits_per_pixel(), bpp(15));
        assert_eq!(PixelFormat::Rgb444Le.bits_per_component(), 4);
        assert_eq!(PixelFormat::Bgr444Be.bits_per_component(), 4);
        assert_eq!(PixelFormat::Rgba64Le.component_count(), 4);
        assert_eq!(PixelFormat::Rgba64Le.bits_per_component(), 16);
        assert_eq!(PixelFormat::Rgba64Le.bits_per_pixel(), bpp(64));
        assert_eq!(PixelFormat::Bgra64Be.component_count(), 4);
        assert_eq!(PixelFormat::Bgra64Be.bits_per_component(), 16);
        assert_eq!(PixelFormat::Bgra64Be.bits_per_pixel(), bpp(64));
        assert_eq!(PixelFormat::Xyz12Le.component_count(), 3);
        assert_eq!(PixelFormat::Xyz12Le.bits_per_component(), 12);
        assert_eq!(PixelFormat::Xyz12Le.bits_per_pixel(), bpp(36));
        assert_eq!(PixelFormat::Xyz12Le.packed_bytes_per_pixel(), Some(6));
        assert!(PixelFormat::Xyz12Le.is_xyz());
        for (format, name) in [
            (PixelFormat::X2Rgb10Le, "x2rgb10le"),
            (PixelFormat::X2Rgb10Be, "x2rgb10be"),
            (PixelFormat::X2Bgr10Le, "x2bgr10le"),
            (PixelFormat::X2Bgr10Be, "x2bgr10be"),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.name, name);
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, 10);
            assert_eq!(descriptor.bits_per_pixel, bpp(30));
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(4));
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }
        for (format, name, bits_per_component, bits_per_pixel, bytes_per_pixel) in [
            (PixelFormat::BayerBggr8, "bayer_bggr8", 8, bpp(8), 1),
            (PixelFormat::BayerRggb8, "bayer_rggb8", 8, bpp(8), 1),
            (PixelFormat::BayerGbrg8, "bayer_gbrg8", 8, bpp(8), 1),
            (PixelFormat::BayerGrbg8, "bayer_grbg8", 8, bpp(8), 1),
            (PixelFormat::BayerBggr16Le, "bayer_bggr16le", 16, bpp(16), 2),
            (PixelFormat::BayerBggr16Be, "bayer_bggr16be", 16, bpp(16), 2),
            (PixelFormat::BayerRggb16Le, "bayer_rggb16le", 16, bpp(16), 2),
            (PixelFormat::BayerRggb16Be, "bayer_rggb16be", 16, bpp(16), 2),
            (PixelFormat::BayerGbrg16Le, "bayer_gbrg16le", 16, bpp(16), 2),
            (PixelFormat::BayerGbrg16Be, "bayer_gbrg16be", 16, bpp(16), 2),
            (PixelFormat::BayerGrbg16Le, "bayer_grbg16le", 16, bpp(16), 2),
            (PixelFormat::BayerGrbg16Be, "bayer_grbg16be", 16, bpp(16), 2),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.name, name);
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, bits_per_pixel);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(bytes_per_pixel));
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
            assert!(format.is_bayer());
        }
        for (format, name, components, bpp, has_alpha, bytes_per_pixel) in [
            (PixelFormat::Vuya, "vuya", 4, bpp(32), true, 4),
            (PixelFormat::Vuyx, "vuyx", 3, bpp(24), false, 4),
            (PixelFormat::Ayuv, "ayuv", 4, bpp(32), true, 4),
            (PixelFormat::Uyva, "uyva", 4, bpp(32), true, 4),
            (PixelFormat::Vyu444, "vyu444", 3, bpp(24), false, 3),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.name, name);
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert_eq!(descriptor.component_count, components);
            assert_eq!(descriptor.bits_per_component, 8);
            assert_eq!(descriptor.bits_per_pixel, bpp);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert_eq!(descriptor.has_alpha, has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(bytes_per_pixel));
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }
        for (format, name, bits_per_component, bits_per_pixel, bytes_per_pixel) in [
            (PixelFormat::Xv30Le, "xv30le", 10, bpp(30), 4),
            (PixelFormat::Xv30Be, "xv30be", 10, bpp(30), 4),
            (PixelFormat::Xv36Le, "xv36le", 12, bpp(36), 8),
            (PixelFormat::Xv36Be, "xv36be", 12, bpp(36), 8),
            (PixelFormat::Xv48Le, "xv48le", 16, bpp(48), 8),
            (PixelFormat::Xv48Be, "xv48be", 16, bpp(48), 8),
            (PixelFormat::V30xLe, "v30xle", 10, bpp(30), 4),
            (PixelFormat::V30xBe, "v30xbe", 10, bpp(30), 4),
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.name, name);
            assert_eq!(descriptor.class, PixelFormatClass::Yuv);
            assert_eq!(descriptor.component_count, 3);
            assert_eq!(descriptor.bits_per_component, bits_per_component);
            assert_eq!(descriptor.bits_per_pixel, bits_per_pixel);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert!(!descriptor.has_alpha);
            assert!(!descriptor.is_float);
            assert_eq!(descriptor.packed_bytes_per_pixel, Some(bytes_per_pixel));
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }
        assert_eq!(PixelFormat::Rgba.component_count(), 4);
        assert_eq!(PixelFormat::Rgba.bits_per_pixel(), bpp(32));
        assert_eq!(PixelFormat::ZeroRgb.component_count(), 3);
        assert_eq!(PixelFormat::ZeroRgb.bits_per_pixel(), bpp(24));
        assert_eq!(PixelFormat::Yuv420p9Le.bits_per_pixel_integer(), None);
        assert_eq!(PixelFormat::Yuva420p9Le.bits_per_pixel_integer(), None);
        assert_eq!(PixelFormat::Yuv422p9Le.bits_per_pixel_integer(), Some(18));
        assert_eq!(PixelFormat::Yuva422p9Le.bits_per_pixel_integer(), Some(27));
        assert_eq!(PixelFormat::Yuv440p10Le.bits_per_pixel_integer(), Some(20));
        assert_eq!(PixelFormat::Yuv440p12Be.bits_per_pixel_integer(), Some(24));
        assert_eq!(PixelFormat::Yuv420p14Le.bits_per_pixel_integer(), Some(21));
        assert_eq!(PixelFormat::Yuv420p16Le.bits_per_pixel_integer(), Some(24));

        for (
            format,
            expected_name,
            expected_bits_per_component,
            expected_bits_per_pixel,
            expected_log2_chroma,
        ) in [
            (PixelFormat::Yuv420p, "yuv420p", 8, bpp(12), (1, 1)),
            (PixelFormat::YuvJ420p, "yuvj420p", 8, bpp(12), (1, 1)),
            (PixelFormat::Yuv422p, "yuv422p", 8, bpp(16), (1, 0)),
            (PixelFormat::YuvJ422p, "yuvj422p", 8, bpp(16), (1, 0)),
            (PixelFormat::Yuv410p, "yuv410p", 8, bpp(9), (2, 2)),
            (PixelFormat::Yuv411p, "yuv411p", 8, bpp(12), (2, 0)),
            (PixelFormat::YuvJ411p, "yuvj411p", 8, bpp(12), (2, 0)),
            (PixelFormat::Yuv440p, "yuv440p", 8, bpp(16), (0, 1)),
            (PixelFormat::YuvJ440p, "yuvj440p", 8, bpp(16), (0, 1)),
            (PixelFormat::Yuv444p, "yuv444p", 8, bpp(24), (0, 0)),
            (PixelFormat::YuvJ444p, "yuvj444p", 8, bpp(24), (0, 0)),
            (PixelFormat::Yuv440p10Le, "yuv440p10le", 10, bpp(20), (0, 1)),
            (PixelFormat::Yuv440p10Be, "yuv440p10be", 10, bpp(20), (0, 1)),
            (PixelFormat::Yuv440p12Le, "yuv440p12le", 12, bpp(24), (0, 1)),
            (PixelFormat::Yuv440p12Be, "yuv440p12be", 12, bpp(24), (0, 1)),
            (
                PixelFormat::Yuv420p9Le,
                "yuv420p9le",
                9,
                Rational::from_raw(27, 2),
                (1, 1),
            ),
            (
                PixelFormat::Yuv420p9Be,
                "yuv420p9be",
                9,
                Rational::from_raw(27, 2),
                (1, 1),
            ),
            (PixelFormat::Yuv422p9Le, "yuv422p9le", 9, bpp(18), (1, 0)),
            (PixelFormat::Yuv422p9Be, "yuv422p9be", 9, bpp(18), (1, 0)),
            (PixelFormat::Yuv444p9Le, "yuv444p9le", 9, bpp(27), (0, 0)),
            (PixelFormat::Yuv444p9Be, "yuv444p9be", 9, bpp(27), (0, 0)),
            (PixelFormat::Yuv420p10Le, "yuv420p10le", 10, bpp(15), (1, 1)),
            (PixelFormat::Yuv420p10Be, "yuv420p10be", 10, bpp(15), (1, 1)),
            (PixelFormat::Yuv422p10Le, "yuv422p10le", 10, bpp(20), (1, 0)),
            (PixelFormat::Yuv422p10Be, "yuv422p10be", 10, bpp(20), (1, 0)),
            (PixelFormat::Yuv444p10Le, "yuv444p10le", 10, bpp(30), (0, 0)),
            (PixelFormat::Yuv444p10Be, "yuv444p10be", 10, bpp(30), (0, 0)),
            (
                PixelFormat::Yuv444p10MsbLe,
                "yuv444p10msble",
                10,
                bpp(30),
                (0, 0),
            ),
            (
                PixelFormat::Yuv444p10MsbBe,
                "yuv444p10msbbe",
                10,
                bpp(30),
                (0, 0),
            ),
            (PixelFormat::Yuv420p12Le, "yuv420p12le", 12, bpp(18), (1, 1)),
            (PixelFormat::Yuv420p12Be, "yuv420p12be", 12, bpp(18), (1, 1)),
            (PixelFormat::Yuv422p12Le, "yuv422p12le", 12, bpp(24), (1, 0)),
            (PixelFormat::Yuv422p12Be, "yuv422p12be", 12, bpp(24), (1, 0)),
            (PixelFormat::Yuv444p12Le, "yuv444p12le", 12, bpp(36), (0, 0)),
            (PixelFormat::Yuv444p12Be, "yuv444p12be", 12, bpp(36), (0, 0)),
            (
                PixelFormat::Yuv444p12MsbLe,
                "yuv444p12msble",
                12,
                bpp(36),
                (0, 0),
            ),
            (
                PixelFormat::Yuv444p12MsbBe,
                "yuv444p12msbbe",
                12,
                bpp(36),
                (0, 0),
            ),
            (PixelFormat::Yuv420p14Le, "yuv420p14le", 14, bpp(21), (1, 1)),
            (PixelFormat::Yuv420p14Be, "yuv420p14be", 14, bpp(21), (1, 1)),
            (PixelFormat::Yuv422p14Le, "yuv422p14le", 14, bpp(28), (1, 0)),
            (PixelFormat::Yuv422p14Be, "yuv422p14be", 14, bpp(28), (1, 0)),
            (PixelFormat::Yuv444p14Le, "yuv444p14le", 14, bpp(42), (0, 0)),
            (PixelFormat::Yuv444p14Be, "yuv444p14be", 14, bpp(42), (0, 0)),
            (PixelFormat::Yuv420p16Le, "yuv420p16le", 16, bpp(24), (1, 1)),
            (PixelFormat::Yuv420p16Be, "yuv420p16be", 16, bpp(24), (1, 1)),
            (PixelFormat::Yuv422p16Le, "yuv422p16le", 16, bpp(32), (1, 0)),
            (PixelFormat::Yuv422p16Be, "yuv422p16be", 16, bpp(32), (1, 0)),
            (PixelFormat::Yuv444p16Le, "yuv444p16le", 16, bpp(48), (0, 0)),
            (PixelFormat::Yuv444p16Be, "yuv444p16be", 16, bpp(48), (0, 0)),
        ] {
            let yuv = format.descriptor();
            assert_eq!(yuv.format, format);
            assert_eq!(yuv.name, expected_name);
            assert_eq!(yuv.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert_eq!(yuv.component_count, 3);
            assert_eq!(yuv.bits_per_component, expected_bits_per_component);
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
        for (format, expected_name, expected_bits_per_pixel, expected_log2_chroma) in [
            (PixelFormat::Yuva420p, "yuva420p", bpp(20), (1, 1)),
            (PixelFormat::Yuva422p, "yuva422p", bpp(24), (1, 0)),
            (PixelFormat::Yuva444p, "yuva444p", bpp(32), (0, 0)),
        ] {
            let yuva = format.descriptor();
            assert_eq!(yuva.format, format);
            assert_eq!(yuva.name, expected_name);
            assert_eq!(yuva.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(yuva.component_count, 4);
            assert_eq!(yuva.bits_per_component, 8);
            assert_eq!(yuva.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(yuva.plane_count, 4);
            assert!(yuva.is_planar);
            assert!(yuva.has_alpha);
            assert!(!yuva.is_float);
            assert_eq!(yuva.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), expected_log2_chroma);
            assert_eq!(
                format.has_chroma_subsampling(),
                expected_log2_chroma != (0, 0)
            );
        }
        for (
            format,
            expected_name,
            expected_bits_per_component,
            expected_bits_per_pixel,
            expected_log2_chroma,
        ) in [
            (
                PixelFormat::Yuva420p9Le,
                "yuva420p9le",
                9,
                Rational::from_raw(45, 2),
                (1, 1),
            ),
            (
                PixelFormat::Yuva420p9Be,
                "yuva420p9be",
                9,
                Rational::from_raw(45, 2),
                (1, 1),
            ),
            (PixelFormat::Yuva422p9Le, "yuva422p9le", 9, bpp(27), (1, 0)),
            (PixelFormat::Yuva422p9Be, "yuva422p9be", 9, bpp(27), (1, 0)),
            (PixelFormat::Yuva444p9Le, "yuva444p9le", 9, bpp(36), (0, 0)),
            (PixelFormat::Yuva444p9Be, "yuva444p9be", 9, bpp(36), (0, 0)),
            (
                PixelFormat::Yuva420p10Le,
                "yuva420p10le",
                10,
                bpp(25),
                (1, 1),
            ),
            (
                PixelFormat::Yuva420p10Be,
                "yuva420p10be",
                10,
                bpp(25),
                (1, 1),
            ),
            (
                PixelFormat::Yuva422p10Le,
                "yuva422p10le",
                10,
                bpp(30),
                (1, 0),
            ),
            (
                PixelFormat::Yuva422p10Be,
                "yuva422p10be",
                10,
                bpp(30),
                (1, 0),
            ),
            (
                PixelFormat::Yuva444p10Le,
                "yuva444p10le",
                10,
                bpp(40),
                (0, 0),
            ),
            (
                PixelFormat::Yuva444p10Be,
                "yuva444p10be",
                10,
                bpp(40),
                (0, 0),
            ),
            (
                PixelFormat::Yuva422p12Le,
                "yuva422p12le",
                12,
                bpp(36),
                (1, 0),
            ),
            (
                PixelFormat::Yuva422p12Be,
                "yuva422p12be",
                12,
                bpp(36),
                (1, 0),
            ),
            (
                PixelFormat::Yuva444p12Le,
                "yuva444p12le",
                12,
                bpp(48),
                (0, 0),
            ),
            (
                PixelFormat::Yuva444p12Be,
                "yuva444p12be",
                12,
                bpp(48),
                (0, 0),
            ),
            (
                PixelFormat::Yuva420p16Le,
                "yuva420p16le",
                16,
                bpp(40),
                (1, 1),
            ),
            (
                PixelFormat::Yuva420p16Be,
                "yuva420p16be",
                16,
                bpp(40),
                (1, 1),
            ),
            (
                PixelFormat::Yuva422p16Le,
                "yuva422p16le",
                16,
                bpp(48),
                (1, 0),
            ),
            (
                PixelFormat::Yuva422p16Be,
                "yuva422p16be",
                16,
                bpp(48),
                (1, 0),
            ),
            (
                PixelFormat::Yuva444p16Le,
                "yuva444p16le",
                16,
                bpp(64),
                (0, 0),
            ),
            (
                PixelFormat::Yuva444p16Be,
                "yuva444p16be",
                16,
                bpp(64),
                (0, 0),
            ),
        ] {
            let yuva = format.descriptor();
            assert_eq!(yuva.format, format);
            assert_eq!(yuva.name, expected_name);
            assert_eq!(yuva.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert!(!format.is_gray());
            assert_eq!(yuva.component_count, 4);
            assert_eq!(yuva.bits_per_component, expected_bits_per_component);
            assert_eq!(yuva.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(yuva.plane_count, 4);
            assert!(yuva.is_planar);
            assert!(yuva.has_alpha);
            assert!(!yuva.is_float);
            assert_eq!(yuva.packed_bytes_per_pixel, None);
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
        assert_eq!(PixelFormat::Yaf16Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::Yaf16Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Yaf32Le.plane_sizes(2, 2).unwrap(), vec![32]);
        assert_eq!(PixelFormat::Yaf32Be.frame_size(1, 2).unwrap(), 16);
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
        assert_eq!(PixelFormat::Pal8.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Pal8.frame_size(2, 2).unwrap(), 4);
        assert_eq!(PixelFormat::Rgb24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Bgr24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Rgb8.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Bgr8.frame_size(2, 2).unwrap(), 4);
        assert_eq!(PixelFormat::Rgb4.plane_sizes(3, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Bgr4.frame_size(4, 1).unwrap(), 2);
        assert_eq!(PixelFormat::Rgb4Byte.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Bgr4Byte.frame_size(2, 2).unwrap(), 4);
        assert_eq!(PixelFormat::BayerBggr8.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::BayerGrbg8.frame_size(3, 2).unwrap(), 6);
        assert_eq!(
            PixelFormat::BayerBggr16Le.plane_sizes(2, 2).unwrap(),
            vec![8]
        );
        assert_eq!(PixelFormat::BayerGrbg16Be.frame_size(3, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Rgb565Le.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Rgb565Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Bgr555Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Rgb444Le.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Bgr444Be.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Rgb48Le.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Rgb48Be.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Bgr48Le.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Bgr48Be.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::RgbF16Le.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::RgbF16Be.frame_size(2, 2).unwrap(), 24);
        assert_eq!(PixelFormat::RgbF32Le.plane_sizes(2, 2).unwrap(), vec![48]);
        assert_eq!(PixelFormat::RgbF32Be.frame_size(2, 2).unwrap(), 48);
        assert_eq!(PixelFormat::Rgb96Le.plane_sizes(2, 2).unwrap(), vec![48]);
        assert_eq!(PixelFormat::Rgb96Be.frame_size(2, 2).unwrap(), 48);
        assert_eq!(PixelFormat::RgbaF16Le.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::RgbaF16Be.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::RgbaF32Le.plane_sizes(2, 2).unwrap(), vec![64]);
        assert_eq!(PixelFormat::RgbaF32Be.frame_size(2, 2).unwrap(), 64);
        assert_eq!(PixelFormat::Rgba128Le.plane_sizes(2, 2).unwrap(), vec![64]);
        assert_eq!(PixelFormat::Rgba128Be.frame_size(2, 2).unwrap(), 64);
        assert_eq!(PixelFormat::Rgba64Le.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Rgba64Be.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Bgra64Le.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Bgra64Be.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::Ayuv64Le.plane_sizes(2, 2).unwrap(), vec![32]);
        assert_eq!(PixelFormat::Ayuv64Be.frame_size(1, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Xyz12Le.plane_sizes(2, 2).unwrap(), vec![24]);
        assert_eq!(PixelFormat::Xyz12Be.frame_size(1, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Rgba.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Bgra.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Argb.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Abgr.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::ZeroRgb.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Rgb0.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::ZeroBgr.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Bgr0.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::X2Rgb10Le.plane_sizes(3, 2).unwrap(), vec![24]);
        assert_eq!(PixelFormat::X2Bgr10Be.frame_size(3, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Vuya.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::Vuyx.frame_size(3, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Xv30Le.plane_sizes(3, 2).unwrap(), vec![24]);
        assert_eq!(PixelFormat::Xv36Be.frame_size(3, 2).unwrap(), 48);
        assert_eq!(PixelFormat::Xv48Le.frame_size(2, 2).unwrap(), 32);
        assert_eq!(PixelFormat::V30xBe.frame_size(3, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Ayuv.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Uyva.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Vyu444.frame_size(3, 2).unwrap(), 18);
        assert_eq!(PixelFormat::Yuyv422.plane_sizes(2, 2).unwrap(), vec![8]);
        assert_eq!(PixelFormat::Uyvy422.frame_size(2, 2).unwrap(), 8);
        assert_eq!(PixelFormat::Yvyu422.frame_size(4, 1).unwrap(), 8);
        assert_eq!(PixelFormat::Uyyvyy411.plane_sizes(4, 2).unwrap(), vec![12]);
        assert_eq!(PixelFormat::Uyyvyy411.frame_size(8, 1).unwrap(), 12);
        assert_eq!(PixelFormat::Uyyvyy411.frame_size(6, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Y210Le.plane_sizes(2, 2).unwrap(), vec![16]);
        assert_eq!(PixelFormat::Y210Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Y212Le.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Y212Be.frame_size(4, 1).unwrap(), 16);
        assert_eq!(PixelFormat::Y216Le.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Y216Be.frame_size(4, 1).unwrap(), 16);
        assert_eq!(PixelFormat::Nv12.plane_sizes(4, 2).unwrap(), vec![8, 4]);
        assert_eq!(PixelFormat::Nv12.frame_size(4, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Nv21.plane_sizes(2, 4).unwrap(), vec![8, 4]);
        assert_eq!(PixelFormat::Nv21.frame_size(2, 4).unwrap(), 12);
        assert_eq!(PixelFormat::Nv16.plane_sizes(4, 3).unwrap(), vec![12, 12]);
        assert_eq!(PixelFormat::Nv16.frame_size(4, 3).unwrap(), 24);
        assert_eq!(PixelFormat::Nv20Le.plane_sizes(4, 3).unwrap(), vec![24, 24]);
        assert_eq!(PixelFormat::Nv20Le.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::Nv20Be.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::Nv24.plane_sizes(3, 2).unwrap(), vec![6, 12]);
        assert_eq!(PixelFormat::Nv24.frame_size(3, 2).unwrap(), 18);
        assert_eq!(PixelFormat::Nv42.frame_size(3, 2).unwrap(), 18);
        assert_eq!(PixelFormat::P010Le.plane_sizes(4, 2).unwrap(), vec![16, 8]);
        assert_eq!(PixelFormat::P010Le.frame_size(4, 2).unwrap(), 24);
        assert_eq!(PixelFormat::P010Be.frame_size(4, 2).unwrap(), 24);
        assert_eq!(PixelFormat::P012Le.plane_sizes(4, 2).unwrap(), vec![16, 8]);
        assert_eq!(PixelFormat::P012Le.frame_size(4, 2).unwrap(), 24);
        assert_eq!(PixelFormat::P012Be.frame_size(4, 2).unwrap(), 24);
        assert_eq!(PixelFormat::P016Le.plane_sizes(4, 2).unwrap(), vec![16, 8]);
        assert_eq!(PixelFormat::P016Le.frame_size(4, 2).unwrap(), 24);
        assert_eq!(PixelFormat::P016Be.frame_size(4, 2).unwrap(), 24);
        assert_eq!(PixelFormat::P210Le.plane_sizes(4, 3).unwrap(), vec![24, 24]);
        assert_eq!(PixelFormat::P210Le.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::P210Be.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::P212Le.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::P212Be.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::P216Le.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::P216Be.frame_size(4, 3).unwrap(), 48);
        assert_eq!(PixelFormat::P410Le.plane_sizes(3, 2).unwrap(), vec![12, 24]);
        assert_eq!(PixelFormat::P410Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(PixelFormat::P410Be.frame_size(3, 2).unwrap(), 36);
        assert_eq!(PixelFormat::P412Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(PixelFormat::P412Be.frame_size(3, 2).unwrap(), 36);
        assert_eq!(PixelFormat::P416Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(PixelFormat::P416Be.frame_size(3, 2).unwrap(), 36);
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
        assert_eq!(
            PixelFormat::Yuva420p.plane_sizes(4, 2).unwrap(),
            vec![8, 2, 2, 8]
        );
        assert_eq!(PixelFormat::Yuva420p.frame_size(4, 2).unwrap(), 20);
        assert_eq!(
            PixelFormat::Yuva422p.plane_sizes(4, 3).unwrap(),
            vec![12, 6, 6, 12]
        );
        assert_eq!(PixelFormat::Yuva422p.frame_size(4, 3).unwrap(), 36);
        assert_eq!(
            PixelFormat::Yuva444p.plane_sizes(3, 2).unwrap(),
            vec![6, 6, 6, 6]
        );
        assert_eq!(PixelFormat::Yuva444p.frame_size(3, 2).unwrap(), 24);
        for (format, width, height, expected) in [
            (PixelFormat::Yuva420p9Le, 4, 2, vec![16, 4, 4, 16]),
            (PixelFormat::Yuva420p9Be, 4, 2, vec![16, 4, 4, 16]),
            (PixelFormat::Yuva422p9Le, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva422p9Be, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva444p9Le, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva444p9Be, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva420p10Le, 4, 2, vec![16, 4, 4, 16]),
            (PixelFormat::Yuva420p10Be, 4, 2, vec![16, 4, 4, 16]),
            (PixelFormat::Yuva422p10Le, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva422p10Be, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva444p10Le, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva444p10Be, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva422p12Le, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva422p12Be, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva444p12Le, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva444p12Be, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva420p16Le, 4, 2, vec![16, 4, 4, 16]),
            (PixelFormat::Yuva420p16Be, 4, 2, vec![16, 4, 4, 16]),
            (PixelFormat::Yuva422p16Le, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva422p16Be, 4, 3, vec![24, 12, 12, 24]),
            (PixelFormat::Yuva444p16Le, 3, 2, vec![12, 12, 12, 12]),
            (PixelFormat::Yuva444p16Be, 3, 2, vec![12, 12, 12, 12]),
        ] {
            assert_eq!(format.plane_sizes(width, height).unwrap(), expected);
            assert_eq!(
                format.frame_size(width, height).unwrap(),
                expected.iter().sum()
            );
        }
        for (format, width, height, expected) in [
            (PixelFormat::Yuv420p9Le, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv420p9Be, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv422p9Le, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv422p9Be, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv444p9Le, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p9Be, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv420p10Le, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv420p10Be, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv422p10Le, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv422p10Be, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv440p10Le, 3, 2, vec![12, 6, 6]),
            (PixelFormat::Yuv440p10Be, 3, 2, vec![12, 6, 6]),
            (PixelFormat::Yuv444p10Le, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p10Be, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p10MsbLe, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p10MsbBe, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv420p12Le, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv420p12Be, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv422p12Le, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv422p12Be, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv440p12Le, 3, 2, vec![12, 6, 6]),
            (PixelFormat::Yuv440p12Be, 3, 2, vec![12, 6, 6]),
            (PixelFormat::Yuv444p12Le, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p12Be, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p12MsbLe, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p12MsbBe, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv420p14Le, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv420p14Be, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv422p14Le, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv422p14Be, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv444p14Le, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p14Be, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv420p16Le, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv420p16Be, 4, 2, vec![16, 4, 4]),
            (PixelFormat::Yuv422p16Le, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv422p16Be, 4, 3, vec![24, 12, 12]),
            (PixelFormat::Yuv444p16Le, 3, 2, vec![12, 12, 12]),
            (PixelFormat::Yuv444p16Be, 3, 2, vec![12, 12, 12]),
        ] {
            assert_eq!(format.plane_sizes(width, height).unwrap(), expected);
            assert_eq!(
                format.frame_size(width, height).unwrap(),
                expected.iter().sum()
            );
        }
        for (format, width, height, expected) in [
            (PixelFormat::YuvJ420p, 4, 2, vec![8, 2, 2]),
            (PixelFormat::YuvJ422p, 4, 3, vec![12, 6, 6]),
            (PixelFormat::YuvJ411p, 4, 3, vec![12, 3, 3]),
            (PixelFormat::YuvJ440p, 3, 2, vec![6, 3, 3]),
            (PixelFormat::YuvJ444p, 3, 2, vec![6, 6, 6]),
        ] {
            assert_eq!(format.plane_sizes(width, height).unwrap(), expected);
            assert_eq!(
                format.frame_size(width, height).unwrap(),
                expected.iter().sum()
            );
        }
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
            PixelFormat::Gbrp10MsbLe.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp10MsbLe.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrp12Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp12Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Gbrp12MsbLe.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::Gbrp12MsbLe.frame_size(3, 2).unwrap(), 36);
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
            PixelFormat::GbrpF16Le.plane_sizes(3, 2).unwrap(),
            vec![12, 12, 12]
        );
        assert_eq!(PixelFormat::GbrpF16Le.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::GbrpF32Le.plane_sizes(3, 2).unwrap(),
            vec![24, 24, 24]
        );
        assert_eq!(PixelFormat::GbrpF32Le.frame_size(3, 2).unwrap(), 72);
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

        let planes = PixelFormat::YuvJ420p
            .split_planes(&(0..12).collect::<Vec<_>>(), 4, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..10).collect::<Vec<_>>(),
                (10..12).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv422p
            .split_planes(&[0, 1, 2, 3, 4, 5, 6, 7], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4, 5], vec![6, 7]]);

        let planes = PixelFormat::Yuv420p10Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 2, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..10).collect::<Vec<_>>(),
                (10..12).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv420p9Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 2, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..10).collect::<Vec<_>>(),
                (10..12).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv444p12Be
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..4).collect::<Vec<_>>(),
                (4..8).collect::<Vec<_>>(),
                (8..12).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv420p14Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 2, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..10).collect::<Vec<_>>(),
                (10..12).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv444p16Be
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..4).collect::<Vec<_>>(),
                (4..8).collect::<Vec<_>>(),
                (8..12).collect::<Vec<_>>()
            ]
        );

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

        let planes = PixelFormat::Yuv440p10Le
            .split_planes(&(0..24).collect::<Vec<_>>(), 3, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..12).collect::<Vec<_>>(),
                (12..18).collect::<Vec<_>>(),
                (18..24).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuv444p
            .split_planes(&[0, 1, 2, 3, 4, 5], 1, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1], vec![2, 3], vec![4, 5]]);

        let planes = PixelFormat::Yuva420p
            .split_planes(&(0..20).collect::<Vec<_>>(), 4, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..10).collect::<Vec<_>>(),
                (10..12).collect::<Vec<_>>(),
                (12..20).collect::<Vec<_>>()
            ]
        );

        let planes = PixelFormat::Yuva420p9Le
            .split_planes(&(0..40).collect::<Vec<_>>(), 4, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..16).collect::<Vec<_>>(),
                (16..20).collect::<Vec<_>>(),
                (20..24).collect::<Vec<_>>(),
                (24..40).collect::<Vec<_>>()
            ]
        );

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

        let planes = PixelFormat::Gbrp10MsbLe
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

        let planes = PixelFormat::Gbrp12MsbLe
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

        let planes = PixelFormat::GbrpF16Le
            .split_planes(&(0..12).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
        );

        let planes = PixelFormat::GbrpF32Be
            .split_planes(&(0..24).collect::<Vec<_>>(), 1, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![
                (0..8).collect::<Vec<_>>(),
                (8..16).collect::<Vec<_>>(),
                (16..24).collect::<Vec<_>>()
            ]
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

        let planes = PixelFormat::RgbF16Le
            .split_planes(&[6, 7, 8, 9, 10, 11], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![6, 7, 8, 9, 10, 11]]);

        let planes = PixelFormat::RgbF32Be
            .split_planes(&(12..24).collect::<Vec<_>>(), 1, 1)
            .unwrap();

        assert_eq!(planes, vec![(12..24).collect::<Vec<_>>()]);

        let planes = PixelFormat::RgbaF16Le
            .split_planes(&(24..32).collect::<Vec<_>>(), 1, 1)
            .unwrap();

        assert_eq!(planes, vec![(24..32).collect::<Vec<_>>()]);

        let planes = PixelFormat::RgbaF32Be
            .split_planes(&(32..48).collect::<Vec<_>>(), 1, 1)
            .unwrap();

        assert_eq!(planes, vec![(32..48).collect::<Vec<_>>()]);

        let planes = PixelFormat::Rgb96Le
            .split_planes(&(48..60).collect::<Vec<_>>(), 1, 1)
            .unwrap();

        assert_eq!(planes, vec![(48..60).collect::<Vec<_>>()]);

        let planes = PixelFormat::Rgba128Be
            .split_planes(&(60..76).collect::<Vec<_>>(), 1, 1)
            .unwrap();

        assert_eq!(planes, vec![(60..76).collect::<Vec<_>>()]);

        let planes = PixelFormat::X2Rgb10Le
            .split_planes(&(52..60).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(52..60).collect::<Vec<_>>()]);

        let planes = PixelFormat::Vuya
            .split_planes(&(60..68).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(60..68).collect::<Vec<_>>()]);

        let planes = PixelFormat::Xv36Le
            .split_planes(&(68..84).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(68..84).collect::<Vec<_>>()]);

        let planes = PixelFormat::V30xBe
            .split_planes(&(80..88).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(80..88).collect::<Vec<_>>()]);

        let planes = PixelFormat::Vyu444
            .split_planes(&(88..94).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(88..94).collect::<Vec<_>>()]);

        let planes = PixelFormat::Rgb565Le
            .split_planes(&[0, 1, 2, 3], 2, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Rgb8.split_planes(&[0, 1, 2, 3], 2, 2).unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Pal8.split_planes(&[0, 1, 2, 3], 2, 2).unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::BayerBggr8
            .split_planes(&[0, 1, 2, 3], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::BayerRggb16Be
            .split_planes(&(0..8).collect::<Vec<_>>(), 2, 2)
            .unwrap();

        assert_eq!(planes, vec![(0..8).collect::<Vec<_>>()]);

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

        let planes = PixelFormat::Uyyvyy411
            .split_planes(&[8, 9, 10, 11, 12, 13], 4, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![8, 9, 10, 11, 12, 13]]);

        let planes = PixelFormat::Y210Le
            .split_planes(&(0..8).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(0..8).collect::<Vec<_>>()]);

        let planes = PixelFormat::Y212Be
            .split_planes(&(8..16).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(8..16).collect::<Vec<_>>()]);

        let planes = PixelFormat::Y216Le
            .split_planes(&(16..24).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(16..24).collect::<Vec<_>>()]);

        let planes = PixelFormat::Ayuv64Le
            .split_planes(&(24..40).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(24..40).collect::<Vec<_>>()]);

        let planes = PixelFormat::Xyz12Le
            .split_planes(&(40..52).collect::<Vec<_>>(), 2, 1)
            .unwrap();

        assert_eq!(planes, vec![(40..52).collect::<Vec<_>>()]);

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

        let planes = PixelFormat::Nv16
            .split_planes(&(0..24).collect::<Vec<_>>(), 4, 3)
            .unwrap();

        assert_eq!(
            planes,
            vec![(0..12).collect::<Vec<_>>(), (12..24).collect()]
        );

        let planes = PixelFormat::Nv20Le
            .split_planes(&(0..48).collect::<Vec<_>>(), 4, 3)
            .unwrap();

        assert_eq!(
            planes,
            vec![(0..24).collect::<Vec<_>>(), (24..48).collect()]
        );

        let planes = PixelFormat::Nv42
            .split_planes(&(0..18).collect::<Vec<_>>(), 3, 2)
            .unwrap();

        assert_eq!(planes, vec![(0..6).collect::<Vec<_>>(), (6..18).collect()]);

        let planes = PixelFormat::P010Le
            .split_planes(&(0..24).collect::<Vec<_>>(), 4, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![(0..16).collect::<Vec<_>>(), (16..24).collect()]
        );

        let planes = PixelFormat::P212Be
            .split_planes(&(0..48).collect::<Vec<_>>(), 4, 3)
            .unwrap();

        assert_eq!(
            planes,
            vec![(0..24).collect::<Vec<_>>(), (24..48).collect()]
        );

        let planes = PixelFormat::P416Le
            .split_planes(&(0..36).collect::<Vec<_>>(), 3, 2)
            .unwrap();

        assert_eq!(
            planes,
            vec![(0..12).collect::<Vec<_>>(), (12..36).collect()]
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

        let planes = PixelFormat::Yaf16Be
            .split_planes(&[0, 1, 2, 3], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3]]);

        let planes = PixelFormat::Yaf32Le
            .split_planes(&[0, 1, 2, 3, 4, 5, 6, 7], 1, 1)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3, 4, 5, 6, 7]]);
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
            PixelFormat::Yuv420p10Le
                .frame_size(3, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv420p9Le.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv420p9Be.frame_size(4, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv422p9Le.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv444p9Be.frame_size(3, 2).unwrap(), 36);
        assert_eq!(
            PixelFormat::Yuv420p12Be
                .frame_size(4, 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv422p12Le
                .frame_size(3, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv420p14Le
                .frame_size(3, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv420p16Be
                .frame_size(4, 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv422p14Le
                .frame_size(3, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv444p16Be.frame_size(3, 2).unwrap(), 36);
        assert_eq!(PixelFormat::Yuv444p10Be.frame_size(3, 2).unwrap(), 36);
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
        assert_eq!(
            PixelFormat::Yuv440p10Le
                .frame_size(3, 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv440p10Le.frame_size(3, 2).unwrap(), 24);
        assert_eq!(
            PixelFormat::Yuv440p12Be
                .frame_size(3, 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv440p12Be.frame_size(3, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Yuv444p.frame_size(3, 2).unwrap(), 18);
        assert_eq!(
            PixelFormat::Yuva420p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuva420p.frame_size(4, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuva422p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuva422p.frame_size(4, 3).unwrap(), 36);
        assert_eq!(PixelFormat::Yuva444p.frame_size(3, 2).unwrap(), 24);
        assert_eq!(
            PixelFormat::Yuva420p9Le
                .frame_size(3, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuva420p10Be
                .frame_size(4, 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuva422p16Le
                .frame_size(3, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuva444p12Be.frame_size(3, 2).unwrap(), 48);
        assert_eq!(
            PixelFormat::YuvJ420p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::YuvJ422p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::YuvJ411p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::YuvJ440p.frame_size(3, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::YuvJ444p.frame_size(3, 2).unwrap(), 18);
        assert_eq!(
            PixelFormat::Yuyv422.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Uyyvyy411.frame_size(6, 2).unwrap(), 24);
        assert_eq!(PixelFormat::Uyyvyy411.frame_size(4, 3).unwrap(), 18);
        assert_eq!(PixelFormat::Uyvy422.frame_size(2, 3).unwrap(), 12);
        assert_eq!(
            PixelFormat::Yvyu422
                .split_planes(&[0; 3], 2, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Uyyvyy411
                .split_planes(&[0; 5], 4, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Y210Le.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Y212Be.frame_size(2, 2).unwrap(), 16);
        assert_eq!(
            PixelFormat::Y216Le
                .split_planes(&[0; 15], 2, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Ayuv64Le
                .split_planes(&[0; 7], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Xyz12Le
                .split_planes(&[0; 5], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::X2Bgr10Le
                .split_planes(&[0; 3], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Rgb96Le
                .split_planes(&[0; 11], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Rgba128Le
                .split_planes(&[0; 15], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Xv48Le
                .split_planes(&[0; 7], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Vuya
                .split_planes(&[0; 3], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Yaf32Le
                .split_planes(&[0; 7], 1, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            PixelFormat::Vyu444
                .split_planes(&[0; 2], 1, 1)
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
            PixelFormat::Nv16.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Nv20Be.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Nv24.frame_size(3, 2).unwrap(), 18);
        assert_eq!(
            PixelFormat::P010Le.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::P010Be.frame_size(4, 3).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::P210Be.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::P410Le.frame_size(3, 2).unwrap(), 36);
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
