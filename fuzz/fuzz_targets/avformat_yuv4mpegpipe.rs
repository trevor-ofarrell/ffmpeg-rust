#![no_main]

use avformat::{Yuv4MpegDemuxer, Yuv4MpegMuxer};
use avutil::{FrameColorRange, PixelFormat, Rational};
use libfuzzer_sys::fuzz_target;

const VALID_Y4M: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAME\nabcdef";
const VALID_Y4M_X_FIELD: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg X\nFRAME\nabcdef";
const VALID_Y4M_XCOLORRANGE_FULL: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg XCOLORRANGE=FULL\nFRAME\nabcdef";
const VALID_Y4M_XCOLORRANGE_LIMITED: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg XCOLORRANGE=LIMITED\nFRAME\nabcdef";
const VALID_Y4M_XCOLORRANGE_BOGUS: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg XCOLORRANGE=BOGUS\nFRAME\nabcdef";
const VALID_Y4M_A_MALFORMED_DUP: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg Afoo A4:3\nFRAME\nabcdef";
const VALID_Y4M_A_TRAILING_MALFORMED: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A4:3 Afoo\nFRAME\nabcdef";
const VALID_Y4M_A_RESET_TO_ZERO: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A4:3 A0:0\nFRAME\nabcdef";
const VALID_Y4M_A_MISSING_DENOM: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A1\nFRAME\nabcdef";
const VALID_Y4M_A_MALFORMED_DENOM: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A1:foo\nFRAME\nabcdef";
const VALID_Y4M_A_ZERO_NUMERATOR: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A4:3 A0:1\nFRAME\nabcdef";
const VALID_Y4M_A_NEGATIVE: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A-1:-2\nFRAME\nabcdef";
const VALID_Y4M_A_INVALID_ONLY: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg Afoo\nFRAME\nabcdef";
const VALID_Y4M_A_EMPTY_FIELD: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg A\nFRAME\nabcdef";
const VALID_Y4M_X_UNKNOWN_EXTENSION: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg XFOO=bar XCOLORRANGE=FULL XBAR=1\nFRAME\nabcdef";
const VALID_Y4M_X_UNKNOWN_AND_DUPLICATE_RANGE: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg XFOO=bar XCOLORRANGE=LIMITED XBAZ=2 XCOLORRANGE=FULL\nFRAME\nabcdef";
const VALID_Y4M_LEADING_WHITESPACE_FRAME_HEADER: &[u8] =
    b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg\n \nFRAME\nabcdef";
const BASE_Y4M_HEADER: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\n";
const VALID_FRAME_VARIANTS: &[&[u8]] = &[
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAME\nabcdef",
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAMEI\nabcdef",
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAME Iu\nabcdef",
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAME XYZ\nabcdef",
    b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAME foo bar\nabcdef",
];

fuzz_target!(|data: &[u8]| {
    exercise_y4m(data);
    exercise_xcolorrange_y4m();
    exercise_xcolorrange_roundtrip();
    exercise_truncated_tail_frame_header();
    exercise_frame_variants();
    exercise_sample_aspect_fields();
    exercise_unknown_x_extension_fields();
    exercise_leading_whitespace_frame_header();
    exercise_y4m(VALID_Y4M);
});

fn exercise_y4m(input: &[u8]) {
    let Ok(mut demuxer) = Yuv4MpegDemuxer::open(input) else {
        return;
    };

    let info = demuxer.info().clone();
    assert_eq!(info.pixel_format(), PixelFormat::Yuv420p);
    assert_eq!(info.width() % 2, 0);
    assert_eq!(info.height() % 2, 0);
    assert!(info.frame_size() > 0);

    for expected_pts in 0..16 {
        match demuxer.read_packet() {
            Ok(Some(packet)) => {
                assert_eq!(packet.stream_index(), 0);
                assert_eq!(packet.pts(), Some(expected_pts));
                assert_eq!(packet.dts(), Some(expected_pts));
                assert_eq!(packet.duration(), 1);
                assert_eq!(packet.data().len(), info.frame_size());
            }
            Ok(None) | Err(_) => break,
        }
    }
}

fn exercise_truncated_tail_frame_header() {
    let mut truncated = Vec::from(BASE_Y4M_HEADER);
    truncated.extend_from_slice(b"FRAME\nabcdefFRAME\nabc");

    let Ok(mut demuxer) = Yuv4MpegDemuxer::open(&truncated) else {
        return;
    };

    let packet = demuxer.read_packet();
    assert!(matches!(packet, Ok(Some(packet)) if packet.data() == b"abcdef"));
    assert!(matches!(demuxer.read_packet(), Ok(None)));
    assert!(matches!(demuxer.read_packet(), Ok(None)));
}

fn exercise_xcolorrange_y4m() {
    for (input, expected_color_range) in [
        (VALID_Y4M_X_FIELD, FrameColorRange::Unspecified),
        (VALID_Y4M_XCOLORRANGE_FULL, FrameColorRange::Jpeg),
        (VALID_Y4M_XCOLORRANGE_LIMITED, FrameColorRange::Mpeg),
        (VALID_Y4M_XCOLORRANGE_BOGUS, FrameColorRange::Unspecified),
    ] {
        let mut demuxer =
            Yuv4MpegDemuxer::open(input).expect("valid yuv4mpegpipe seed should parse");
        let info = demuxer.info().clone();
        assert_eq!(info.pixel_format(), PixelFormat::Yuv420p);
        assert_eq!(info.color_range(), expected_color_range);

        let packet = demuxer
            .read_packet()
            .expect("valid yuv4mpegpipe seed should yield a packet")
            .expect("valid yuv4mpegpipe seed should contain a frame");
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.pts(), Some(0));
        assert_eq!(packet.dts(), Some(0));
        assert_eq!(packet.duration(), 1);
        assert_eq!(packet.data(), b"abcdef");
        assert!(demuxer.read_packet().unwrap().is_none());
    }
}

fn exercise_xcolorrange_roundtrip() {
    let cases = [
        (VALID_Y4M_X_FIELD, None),
        (VALID_Y4M_XCOLORRANGE_FULL, Some(FrameColorRange::Jpeg)),
        (VALID_Y4M_XCOLORRANGE_LIMITED, Some(FrameColorRange::Mpeg)),
        (VALID_Y4M_XCOLORRANGE_BOGUS, None),
    ];

    for (input, expected_color_range) in cases {
        let mut demuxer = Yuv4MpegDemuxer::open(input).expect("xcolorrange seed should parse");
        let info = demuxer.info().clone();
        let mut muxer = Yuv4MpegMuxer::new(
            info.width(),
            info.height(),
            info.frame_rate(),
            info.sample_aspect_ratio(),
        )
        .expect("valid yuv4mpegpipe muxer should be constructible");
        muxer.set_color_range(info.color_range());
        let packet = demuxer
            .read_packet()
            .expect("xcolorrange seed should yield a packet")
            .expect("xcolorrange seed should contain a frame");
        muxer.write_packet(&packet).unwrap();

        let bytes = muxer.finish();
        let mut roundtripped =
            Yuv4MpegDemuxer::open(&bytes).expect("yuv4mpegpipe roundtrip should parse after remux");
        let expected_color_range = expected_color_range.unwrap_or(FrameColorRange::Unspecified);
        assert_eq!(roundtripped.info().color_range(), expected_color_range);
        assert_eq!(
            roundtripped.read_packet().unwrap().unwrap().data(),
            b"abcdef",
            "roundtrip packet payload should preserve data"
        );
        assert!(roundtripped.read_packet().unwrap().is_none());
    }
}

fn exercise_frame_variants() {
    for input in VALID_FRAME_VARIANTS {
        let mut demuxer = Yuv4MpegDemuxer::open(input)
            .expect("valid yuv4mpegpipe frame-variant seed should parse");
        let packet = demuxer
            .read_packet()
            .expect("valid yuv4mpegpipe frame-variant seed should yield a packet")
            .expect("valid yuv4mpegpipe frame-variant seed should contain a frame");
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.pts(), Some(0));
        assert_eq!(packet.dts(), Some(0));
        assert_eq!(packet.duration(), 1);
        assert_eq!(packet.data(), b"abcdef");
        assert!(demuxer.read_packet().unwrap().is_none());
    }
}

fn exercise_sample_aspect_fields() {
    let valid_sample_aspect = Rational::new(4, 3).unwrap();

    let cases = [
        (VALID_Y4M_A_MALFORMED_DUP, Some(valid_sample_aspect)),
        (VALID_Y4M_A_TRAILING_MALFORMED, Some(valid_sample_aspect)),
        (VALID_Y4M_A_RESET_TO_ZERO, None),
        (VALID_Y4M_A_MISSING_DENOM, Some(Rational::from_raw(1, 0))),
        (VALID_Y4M_A_MALFORMED_DENOM, Some(Rational::from_raw(1, 0))),
        (VALID_Y4M_A_ZERO_NUMERATOR, None),
        (VALID_Y4M_A_NEGATIVE, Some(Rational::from_raw(-1, -2))),
        (VALID_Y4M_A_INVALID_ONLY, None),
        (VALID_Y4M_A_EMPTY_FIELD, None),
    ];

    for (input, expected_sample_aspect) in cases {
        let Ok(demuxer) = Yuv4MpegDemuxer::open(input) else {
            continue;
        };

        let info = demuxer.info().clone();
        assert_eq!(info.sample_aspect_ratio(), expected_sample_aspect);

        let mut demuxer = demuxer;
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data(), b"abcdef");
        assert!(matches!(demuxer.read_packet(), Ok(None)));
    }
}

fn exercise_unknown_x_extension_fields() {
    for input in [
        VALID_Y4M_X_UNKNOWN_EXTENSION,
        VALID_Y4M_X_UNKNOWN_AND_DUPLICATE_RANGE,
    ] {
        let mut demuxer =
            Yuv4MpegDemuxer::open(input).expect("valid yuv4mpegpipe x extension seed should parse");
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data(), b"abcdef");
        assert!(matches!(demuxer.read_packet(), Ok(None)));
    }
}

fn exercise_leading_whitespace_frame_header() {
    let mut demuxer = Yuv4MpegDemuxer::open(VALID_Y4M_LEADING_WHITESPACE_FRAME_HEADER)
        .expect("leading whitespace frame-header seed should parse");
    let packet = demuxer
        .read_packet()
        .expect("leading whitespace frame-header seed should yield a packet")
        .expect("leading whitespace frame-header seed should contain a frame");
    assert_eq!(packet.stream_index(), 0);
    assert_eq!(packet.pts(), Some(0));
    assert_eq!(packet.dts(), Some(0));
    assert_eq!(packet.duration(), 1);
    assert_eq!(packet.data(), b"abcdef");
    assert!(matches!(demuxer.read_packet(), Ok(None)));
}
