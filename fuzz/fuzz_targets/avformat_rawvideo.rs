#![no_main]

use avformat::{RawVideoDemuxer, RawVideoPixelFormat};
use avutil::Rational;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let width = data.first().copied().map(dimension_from).unwrap_or(2);
    let height = data.get(1).copied().map(dimension_from).unwrap_or(2);
    let pixel_format = data
        .get(2)
        .copied()
        .map(pixel_format_from)
        .unwrap_or(RawVideoPixelFormat::Gray8);
    let frame_rate = data
        .get(3)
        .copied()
        .map(frame_rate_from)
        .unwrap_or(Rational::ONE);
    let payload = data.get(4..).unwrap_or(&[]);

    exercise_rawvideo(payload, width, height, pixel_format, frame_rate);
    exercise_rawvideo(b"abcdefgh", 2, 2, RawVideoPixelFormat::Gray8, Rational::ONE);
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Gray16Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Gray9Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Gray10Be,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Gray12Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Gray14Be,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        1,
        RawVideoPixelFormat::Gray32Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcd",
        2,
        1,
        RawVideoPixelFormat::GrayF16Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        1,
        RawVideoPixelFormat::GrayF32Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        1,
        RawVideoPixelFormat::Yaf16Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefghijklmnop",
        2,
        1,
        RawVideoPixelFormat::Yaf32Be,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdef",
        2,
        1,
        RawVideoPixelFormat::Gbrp,
        Rational::ONE,
    );
    let gbrp9 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrp9,
        2,
        1,
        RawVideoPixelFormat::Gbrp9Le,
        Rational::ONE,
    );
    let gbrp10 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrp10,
        2,
        1,
        RawVideoPixelFormat::Gbrp10Le,
        Rational::ONE,
    );
    let gbrp12 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrp12,
        2,
        1,
        RawVideoPixelFormat::Gbrp12Le,
        Rational::ONE,
    );
    let gbrp14 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrp14,
        2,
        1,
        RawVideoPixelFormat::Gbrp14Le,
        Rational::ONE,
    );
    let gbrp16 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrp16,
        2,
        1,
        RawVideoPixelFormat::Gbrp16Le,
        Rational::ONE,
    );
    let gbrpf16 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrpf16,
        2,
        1,
        RawVideoPixelFormat::GbrpF16Le,
        Rational::ONE,
    );
    let gbrpf32 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrpf32,
        1,
        1,
        RawVideoPixelFormat::GbrpF32Be,
        Rational::ONE,
    );
    let rgbf16 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &rgbf16,
        2,
        1,
        RawVideoPixelFormat::RgbF16Le,
        Rational::ONE,
    );
    let rgbf32 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &rgbf32,
        1,
        1,
        RawVideoPixelFormat::RgbF32Be,
        Rational::ONE,
    );
    let rgb96 = (0..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &rgb96,
        1,
        1,
        RawVideoPixelFormat::Rgb96Le,
        Rational::ONE,
    );
    let rgbaf16 = (0..16).collect::<Vec<_>>();
    exercise_rawvideo(
        &rgbaf16,
        2,
        1,
        RawVideoPixelFormat::RgbaF16Le,
        Rational::ONE,
    );
    let rgbaf32 = (0..16).collect::<Vec<_>>();
    exercise_rawvideo(
        &rgbaf32,
        1,
        1,
        RawVideoPixelFormat::RgbaF32Be,
        Rational::ONE,
    );
    let rgba128 = (0..16).collect::<Vec<_>>();
    exercise_rawvideo(
        &rgba128,
        1,
        1,
        RawVideoPixelFormat::Rgba128Be,
        Rational::ONE,
    );
    let gbrap = (0..8).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrap,
        2,
        1,
        RawVideoPixelFormat::Gbrap,
        Rational::ONE,
    );
    let gbrap16 = (0..16).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrap16,
        2,
        1,
        RawVideoPixelFormat::Gbrap16Le,
        Rational::ONE,
    );
    let gbrapf32 = (0..16).collect::<Vec<_>>();
    exercise_rawvideo(
        &gbrapf32,
        1,
        1,
        RawVideoPixelFormat::GbrapF32Le,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Ya8,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        1,
        RawVideoPixelFormat::Ya16Be,
        Rational::ONE,
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        1,
        RawVideoPixelFormat::Rgb0,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Rgb565Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        2,
        2,
        RawVideoPixelFormat::Pal8,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        2,
        2,
        RawVideoPixelFormat::Rgb8,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        2,
        2,
        RawVideoPixelFormat::Bgr4Byte,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        2,
        2,
        RawVideoPixelFormat::BayerBggr8,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::BayerRggb16Be,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        3,
        2,
        RawVideoPixelFormat::Rgb4,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        3,
        2,
        RawVideoPixelFormat::Bgr4,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcd",
        9,
        2,
        RawVideoPixelFormat::MonoWhite,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"ab",
        16,
        1,
        RawVideoPixelFormat::MonoBlack,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Yuyv422,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Uyvy422,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Yvyu422,
        Rational::new(25, 1).unwrap(),
    );
    let uyyvyy411 = (0_u8..12).collect::<Vec<_>>();
    exercise_rawvideo(
        &uyyvyy411,
        4,
        2,
        RawVideoPixelFormat::Uyyvyy411,
        Rational::new(25, 1).unwrap(),
    );
    let y210le = (0..16).collect::<Vec<_>>();
    exercise_rawvideo(
        &y210le,
        2,
        2,
        RawVideoPixelFormat::Y210Le,
        Rational::new(25, 1).unwrap(),
    );
    let y212be = (16..32).collect::<Vec<_>>();
    exercise_rawvideo(
        &y212be,
        2,
        2,
        RawVideoPixelFormat::Y212Be,
        Rational::new(25, 1).unwrap(),
    );
    let y216le = (32..48).collect::<Vec<_>>();
    exercise_rawvideo(
        &y216le,
        2,
        2,
        RawVideoPixelFormat::Y216Le,
        Rational::new(25, 1).unwrap(),
    );
    let ayuv64le = (48..64).collect::<Vec<_>>();
    exercise_rawvideo(
        &ayuv64le,
        1,
        2,
        RawVideoPixelFormat::Ayuv64Le,
        Rational::new(25, 1).unwrap(),
    );
    let xyz12le = (64..76).collect::<Vec<_>>();
    exercise_rawvideo(
        &xyz12le,
        1,
        2,
        RawVideoPixelFormat::Xyz12Le,
        Rational::new(25, 1).unwrap(),
    );
    let x2rgb10le = (76..84).collect::<Vec<_>>();
    exercise_rawvideo(
        &x2rgb10le,
        2,
        1,
        RawVideoPixelFormat::X2Rgb10Le,
        Rational::new(25, 1).unwrap(),
    );
    let xv30le = (84..92).collect::<Vec<_>>();
    exercise_rawvideo(
        &xv30le,
        2,
        1,
        RawVideoPixelFormat::Xv30Le,
        Rational::new(25, 1).unwrap(),
    );
    let xv36be = (92..104).collect::<Vec<_>>();
    exercise_rawvideo(
        &xv36be,
        2,
        1,
        RawVideoPixelFormat::Xv36Be,
        Rational::new(25, 1).unwrap(),
    );
    let xv48le = (104..120).collect::<Vec<_>>();
    exercise_rawvideo(
        &xv48le,
        1,
        2,
        RawVideoPixelFormat::Xv48Le,
        Rational::new(25, 1).unwrap(),
    );
    let v30xbe = (120..128).collect::<Vec<_>>();
    exercise_rawvideo(
        &v30xbe,
        2,
        1,
        RawVideoPixelFormat::V30xBe,
        Rational::new(25, 1).unwrap(),
    );
    let vuya = (128..136).collect::<Vec<_>>();
    exercise_rawvideo(
        &vuya,
        2,
        1,
        RawVideoPixelFormat::Vuya,
        Rational::new(25, 1).unwrap(),
    );
    let vyu444 = (136..142).collect::<Vec<_>>();
    exercise_rawvideo(
        &vyu444,
        2,
        1,
        RawVideoPixelFormat::Vyu444,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        4,
        2,
        RawVideoPixelFormat::Nv12,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        4,
        2,
        RawVideoPixelFormat::Nv21,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        4,
        3,
        RawVideoPixelFormat::Nv16,
        Rational::new(25, 1).unwrap(),
    );
    let nv20le = (0..48).collect::<Vec<_>>();
    exercise_rawvideo(
        &nv20le,
        4,
        3,
        RawVideoPixelFormat::Nv20Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqr",
        3,
        2,
        RawVideoPixelFormat::Nv42,
        Rational::new(25, 1).unwrap(),
    );
    let p010le = (0..24).collect::<Vec<_>>();
    exercise_rawvideo(
        &p010le,
        4,
        2,
        RawVideoPixelFormat::P010Le,
        Rational::new(25, 1).unwrap(),
    );
    let p212be = (0..48).collect::<Vec<_>>();
    exercise_rawvideo(
        &p212be,
        4,
        3,
        RawVideoPixelFormat::P212Be,
        Rational::new(25, 1).unwrap(),
    );
    let p416le = (0..36).collect::<Vec<_>>();
    exercise_rawvideo(
        &p416le,
        3,
        2,
        RawVideoPixelFormat::P416Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefgh",
        2,
        2,
        RawVideoPixelFormat::Bgr444Be,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        2,
        1,
        RawVideoPixelFormat::Rgb48Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnop",
        1,
        1,
        RawVideoPixelFormat::Rgba64Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        2,
        2,
        RawVideoPixelFormat::Yuv420p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        4,
        3,
        RawVideoPixelFormat::Yuv422p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        4,
        2,
        RawVideoPixelFormat::Yuv420p9Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuv",
        4,
        3,
        RawVideoPixelFormat::Yuv422p9Be,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        4,
        2,
        RawVideoPixelFormat::Yuv420p10Le,
        Rational::new(25, 1).unwrap(),
    );
    let yuv420p14 = (0..24).collect::<Vec<_>>();
    exercise_rawvideo(
        &yuv420p14,
        4,
        2,
        RawVideoPixelFormat::Yuv420p14Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuv",
        4,
        3,
        RawVideoPixelFormat::Yuv422p12Be,
        Rational::new(25, 1).unwrap(),
    );
    let yuv422p16 = (0..48).collect::<Vec<_>>();
    exercise_rawvideo(
        &yuv422p16,
        4,
        3,
        RawVideoPixelFormat::Yuv422p16Be,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqr",
        4,
        3,
        RawVideoPixelFormat::Yuv411p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqr",
        4,
        4,
        RawVideoPixelFormat::Yuv410p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        3,
        2,
        RawVideoPixelFormat::Yuv440p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        3,
        2,
        RawVideoPixelFormat::Yuv440p10Le,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        3,
        2,
        RawVideoPixelFormat::Yuv440p12Be,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqr",
        3,
        2,
        RawVideoPixelFormat::Yuv444p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrst",
        4,
        2,
        RawVideoPixelFormat::Yuva420p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwxyzabcdefghij",
        4,
        3,
        RawVideoPixelFormat::Yuva422p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        3,
        2,
        RawVideoPixelFormat::Yuva444p,
        Rational::new(25, 1).unwrap(),
    );
    let yuva420p9 = (0..40).collect::<Vec<_>>();
    exercise_rawvideo(
        &yuva420p9,
        4,
        2,
        RawVideoPixelFormat::Yuva420p9Le,
        Rational::new(25, 1).unwrap(),
    );
    let yuva422p12 = (0..72).collect::<Vec<_>>();
    exercise_rawvideo(
        &yuva422p12,
        4,
        3,
        RawVideoPixelFormat::Yuva422p12Le,
        Rational::new(25, 1).unwrap(),
    );
    let yuva444p16 = (0..48).collect::<Vec<_>>();
    exercise_rawvideo(
        &yuva444p16,
        3,
        2,
        RawVideoPixelFormat::Yuva444p16Be,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        4,
        2,
        RawVideoPixelFormat::YuvJ420p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqrstuvwx",
        4,
        3,
        RawVideoPixelFormat::YuvJ422p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqr",
        4,
        3,
        RawVideoPixelFormat::YuvJ411p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijkl",
        3,
        2,
        RawVideoPixelFormat::YuvJ440p,
        Rational::new(25, 1).unwrap(),
    );
    exercise_rawvideo(
        b"abcdefghijklmnopqr",
        3,
        2,
        RawVideoPixelFormat::YuvJ444p,
        Rational::new(25, 1).unwrap(),
    );
});

fn exercise_rawvideo(
    input: &[u8],
    width: usize,
    height: usize,
    pixel_format: RawVideoPixelFormat,
    frame_rate: Rational,
) {
    let Ok(mut demuxer) = RawVideoDemuxer::open(input, width, height, pixel_format, frame_rate)
    else {
        return;
    };

    let info = demuxer.info().clone();
    assert_eq!(info.width(), width);
    assert_eq!(info.height(), height);
    assert_eq!(info.pixel_format(), pixel_format);
    assert_eq!(info.frame_rate(), frame_rate);
    assert!(info.frame_size() > 0);
    assert_eq!(
        info.frame_count().checked_mul(info.frame_size()),
        Some(input.len())
    );

    for expected_pts in 0..32 {
        match demuxer.read_packet() {
            Ok(Some(packet)) => {
                assert_eq!(packet.stream_index(), 0);
                assert_eq!(packet.pts(), Some(expected_pts));
                assert_eq!(packet.dts(), Some(expected_pts));
                assert_eq!(packet.duration(), 1);
                assert_eq!(packet.data().len(), info.frame_size());
                assert_eq!(packet.side_data()[0].kind(), "rawvideo_pix_fmt");
                assert_eq!(
                    packet.side_data()[0].data(),
                    info.pixel_format().name().as_bytes()
                );
            }
            Ok(None) | Err(_) => break,
        }
    }
}

fn dimension_from(byte: u8) -> usize {
    match byte % 8 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 8,
        6 => 16,
        _ => usize::from(byte) + 1,
    }
}

fn pixel_format_from(byte: u8) -> RawVideoPixelFormat {
    let formats = RawVideoPixelFormat::ALL;
    formats[usize::from(byte) % formats.len()]
}

fn frame_rate_from(byte: u8) -> Rational {
    match byte % 5 {
        0 => Rational::ZERO,
        1 => Rational::ONE,
        2 => Rational::new(24, 1).unwrap(),
        3 => Rational::new(30000, 1001).unwrap(),
        _ => Rational::from_raw(1, 0),
    }
}
