#![no_main]

use avformat::{AviDemuxer, AviMediaType, AviMuxer};
use avutil::{Packet, Rational};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_avi(data);
    exercise_avi(&valid_avi());
});

fn exercise_avi(input: &[u8]) {
    let Ok(mut demuxer) = AviDemuxer::open(input) else {
        return;
    };

    let info = demuxer.info().clone();
    assert!(info.width() > 0);
    assert!(info.height() > 0);
    assert!(!info.streams().is_empty());

    for (expected_index, stream) in info.streams().iter().enumerate() {
        assert_eq!(stream.index(), expected_index);
        assert_eq!(stream.media_type(), AviMediaType::Video);
        assert!(stream.width() > 0);
        assert!(stream.height() > 0);
        assert!(stream.time_base().num() > 0);
        assert!(stream.time_base().den() > 0);
        assert!(stream.frame_rate().num() > 0);
        assert!(stream.frame_rate().den() > 0);
    }

    let mut packet_count = 0_usize;
    let mut next_pts = vec![0_i64; info.streams().len()];
    while let Some(packet) = demuxer.read_packet().unwrap() {
        assert_packet(&packet, &mut next_pts);
        packet_count += 1;
        assert!(packet_count <= info.packet_count());
    }
    assert_eq!(packet_count, info.packet_count());
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn assert_packet(packet: &Packet, next_pts: &mut [i64]) {
    let stream_index = packet.stream_index();
    assert!(stream_index < next_pts.len());
    assert_eq!(packet.pts(), Some(next_pts[stream_index]));
    assert_eq!(packet.dts(), Some(next_pts[stream_index]));
    assert_eq!(packet.duration(), 1);

    let side_data = packet.side_data();
    assert!(!side_data.is_empty());
    assert_eq!(side_data[0].kind(), "avi_chunk_id");
    let chunk_id = side_data[0].data();
    assert_eq!(chunk_id.len(), 4);
    assert!(chunk_id[0].is_ascii_digit());
    assert!(chunk_id[1].is_ascii_digit());
    assert!(matches!(&chunk_id[2..4], b"db" | b"dc"));

    next_pts[stream_index] += 1;
}

fn valid_avi() -> Vec<u8> {
    let mut muxer = AviMuxer::new_rgb24(2, 2, Rational::new(25, 1).unwrap()).unwrap();
    muxer
        .write_packet(&Packet::new((0_u8..12).collect::<Vec<_>>(), 0))
        .unwrap();
    muxer
        .write_packet(&Packet::new((12_u8..24).collect::<Vec<_>>(), 0))
        .unwrap();
    muxer.finish().unwrap()
}
