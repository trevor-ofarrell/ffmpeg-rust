#![no_main]

use avformat::Yuv4MpegDemuxer;
use avutil::PixelFormat;
use libfuzzer_sys::fuzz_target;

const VALID_Y4M: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\nFRAME\nabcdef";
const BASE_Y4M_HEADER: &[u8] = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\n";

fuzz_target!(|data: &[u8]| {
    exercise_y4m(data);
    exercise_truncated_tail_frame_header();
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
