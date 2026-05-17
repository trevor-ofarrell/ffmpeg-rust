#![no_main]

use avformat::{AviDemuxer, AviMediaType, AviMuxer};
use avutil::{AvErrorKind, Packet, Rational};
use libfuzzer_sys::fuzz_target;

const MAX_PACKETS: usize = 6;
const MAX_PAYLOAD: usize = 256;

fuzz_target!(|data: &[u8]| {
    reject_invalid_constructors();

    let mut cursor = Cursor::new(data);
    exercise_avi_muxer(&mut cursor);
    exercise_fixtures();
});

fn exercise_avi_muxer(cursor: &mut Cursor<'_>) {
    let width = dimension_from(cursor.next());
    let height = dimension_from(cursor.next());
    let frame_rate = frame_rate_from(cursor.next());
    let Ok(mut muxer) = AviMuxer::new_rgb24(width, height, frame_rate) else {
        return;
    };

    let frame_size = muxer.frame_size();
    let packets = packets_from(cursor, frame_size);
    let mut expected = Vec::new();

    assert_eq!(muxer.width(), width);
    assert_eq!(muxer.height(), height);
    assert_eq!(muxer.frame_rate(), frame_rate);
    assert_eq!(muxer.packet_count(), 0);

    for packet in &packets {
        let before = avi_state(&muxer);
        let result = muxer.write_packet(packet);
        if packet.stream_index() == 0 && packet.data().len() == frame_size {
            result.unwrap();
            expected.push(packet.data().to_vec());
            assert_eq!(muxer.packet_count(), expected.len());
            assert_eq!(muxer.width(), width);
            assert_eq!(muxer.height(), height);
            assert_eq!(muxer.frame_rate(), frame_rate);
        } else {
            assert!(result.is_err());
            assert_eq!(avi_state(&muxer), before);
        }
    }

    let rendered = muxer.render().unwrap();
    assert_avi_roundtrip(&rendered, width, height, frame_rate, &expected);

    let finished = muxer.finish().unwrap();
    assert!(muxer.is_finished());
    assert_eq!(finished, rendered);

    let err = muxer
        .write_packet(&Packet::new(vec![0; frame_size], 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.packet_count(), expected.len());
    assert_eq!(muxer.render().unwrap(), rendered);
}

fn reject_invalid_constructors() {
    let rate = Rational::new(25, 1).unwrap();
    assert_eq!(
        AviMuxer::new_rgb24(0, 2, rate).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        AviMuxer::new_rgb24(2, 0, rate).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        AviMuxer::new_rgb24(2, 2, Rational::ZERO)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
}

fn exercise_fixtures() {
    let mut odd = AviMuxer::new_rgb24(1, 1, Rational::ONE).unwrap();
    odd.write_packet(&Packet::new(vec![1, 2, 3], 0)).unwrap();
    let odd_output = odd.finish().unwrap();
    assert_eq!(odd_output.len() % 2, 0);
    assert_avi_roundtrip(&odd_output, 1, 1, Rational::ONE, &[vec![1, 2, 3]]);

    let mut two_frames = AviMuxer::new_rgb24(2, 2, Rational::new(25, 1).unwrap()).unwrap();
    two_frames
        .write_packet(&Packet::new((0_u8..12).collect::<Vec<_>>(), 0))
        .unwrap();
    two_frames
        .write_packet(&Packet::new((12_u8..24).collect::<Vec<_>>(), 0))
        .unwrap();
    assert_avi_roundtrip(
        &two_frames.finish().unwrap(),
        2,
        2,
        Rational::new(25, 1).unwrap(),
        &[(0_u8..12).collect::<Vec<_>>(), (12_u8..24).collect()],
    );
}

fn assert_avi_roundtrip(
    bytes: &[u8],
    width: u32,
    height: u32,
    frame_rate: Rational,
    expected: &[Vec<u8>],
) {
    let mut demuxer = AviDemuxer::open(bytes).unwrap();
    let info = demuxer.info().clone();

    assert_eq!(info.width(), width);
    assert_eq!(info.height(), height);
    assert_eq!(info.total_frames(), expected.len() as u32);
    assert_eq!(info.packet_count(), expected.len());
    assert_eq!(info.streams().len(), 1);

    let stream = &info.streams()[0];
    assert_eq!(stream.index(), 0);
    assert_eq!(stream.media_type(), AviMediaType::Video);
    assert_eq!(stream.handler(), "DIB ");
    assert_eq!(stream.frame_rate(), frame_rate);
    assert_eq!(
        stream.time_base(),
        Rational::new(frame_rate.den(), frame_rate.num()).unwrap()
    );
    assert_eq!(stream.length(), expected.len() as u32);
    assert_eq!(stream.sample_size(), 0);
    assert_eq!(stream.width(), width);
    assert_eq!(stream.height(), height);
    assert_eq!(stream.bit_count(), 24);
    assert_eq!(stream.compression(), "BI_RGB");

    for (index, expected_packet) in expected.iter().enumerate() {
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.data(), expected_packet);
        assert_eq!(packet.pts(), Some(index as i64));
        assert_eq!(packet.dts(), Some(index as i64));
        assert_eq!(packet.duration(), 1);
        assert_eq!(packet.side_data()[0].kind(), "avi_chunk_id");
        assert_eq!(packet.side_data()[0].data(), b"00db");
    }
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn packets_from(cursor: &mut Cursor<'_>, frame_size: usize) -> Vec<Packet> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_PACKETS + 1);
    let mut packets = Vec::with_capacity(count);
    for _ in 0..count {
        let stream_index = stream_index_from(cursor.next());
        let len = payload_len_for_mode(cursor, frame_size);
        packets.push(Packet::new(payload_from(cursor, len), stream_index));
    }
    packets
}

fn payload_len_for_mode(cursor: &mut Cursor<'_>, frame_size: usize) -> usize {
    match cursor.next().unwrap_or_default() % 5 {
        0 => frame_size,
        1 => frame_size.saturating_sub(1),
        2 => frame_size.saturating_add(1).min(MAX_PAYLOAD),
        3 => usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1),
        _ => 0,
    }
}

fn payload_from(cursor: &mut Cursor<'_>, len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
}

fn dimension_from(byte: Option<u8>) -> u32 {
    u32::from(byte.unwrap_or_default() % 8) + 1
}

fn frame_rate_from(byte: Option<u8>) -> Rational {
    match byte.unwrap_or_default() % 4 {
        0 => Rational::ONE,
        1 => Rational::new(24, 1).unwrap(),
        2 => Rational::new(25, 1).unwrap(),
        _ => Rational::new(30000, 1001).unwrap(),
    }
}

fn stream_index_from(byte: Option<u8>) -> usize {
    usize::from(byte.unwrap_or_default().is_multiple_of(5))
}

fn avi_state(muxer: &AviMuxer) -> (usize, Vec<u8>) {
    (muxer.packet_count(), muxer.render().unwrap())
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.data.get(self.offset).copied();
        self.offset = self.offset.saturating_add(usize::from(byte.is_some()));
        byte
    }
}
