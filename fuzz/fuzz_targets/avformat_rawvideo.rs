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
        b"abcdefghijkl",
        2,
        2,
        RawVideoPixelFormat::Yuv420p,
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
    match byte % 4 {
        0 => RawVideoPixelFormat::Gray8,
        1 => RawVideoPixelFormat::Rgb24,
        2 => RawVideoPixelFormat::Rgba,
        _ => RawVideoPixelFormat::Yuv420p,
    }
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
