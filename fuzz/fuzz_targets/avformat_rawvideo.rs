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
        b"abcdefghijklmnopqr",
        3,
        2,
        RawVideoPixelFormat::Yuv444p,
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
