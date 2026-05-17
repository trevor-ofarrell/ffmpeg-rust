#![no_main]

use avformat::{FrameCrcMuxer, HashAlgorithm, HashDigest, HashMuxer, NullMuxer};
use avutil::{adler32, crc32_ieee, md5, AvErrorKind, Packet, SideData};
use libfuzzer_sys::fuzz_target;

const MAX_PACKETS: usize = 16;
const MAX_PAYLOAD_LEN: usize = 32;
const MAX_STREAM_INDEX: usize = 3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExpectedStreamStats {
    packets: u64,
    bytes: u64,
    duration: i64,
    last_pts: Option<i64>,
    last_dts: Option<i64>,
}

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    let packets = packets_from(&mut cursor);

    exercise_null_muxer(&packets);
    exercise_hash_muxers(&packets);
    exercise_framecrc_muxer(&packets);
    exercise_fixtures();
});

fn exercise_null_muxer(packets: &[Packet]) {
    let mut muxer = NullMuxer::new();
    let mut streams = Vec::<ExpectedStreamStats>::new();
    let mut total_packets = 0_u64;
    let mut total_bytes = 0_u64;

    for packet in packets {
        muxer.write_packet(packet).unwrap();
        while streams.len() <= packet.stream_index() {
            streams.push(ExpectedStreamStats::default());
        }
        let expected_stream = &mut streams[packet.stream_index()];
        expected_stream.packets += 1;
        expected_stream.bytes += packet.data().len() as u64;
        expected_stream.duration += packet.duration();
        expected_stream.last_pts = packet.pts();
        expected_stream.last_dts = packet.dts();
        total_packets += 1;
        total_bytes += packet.data().len() as u64;

        let report = muxer.report();
        assert_eq!(report.total_packets(), total_packets);
        assert_eq!(report.total_bytes(), total_bytes);
        assert_eq!(report.streams().len(), streams.len());
        for (stream_index, expected) in streams.iter().enumerate() {
            let actual = &report.streams()[stream_index];
            assert_eq!(actual.stream_index(), stream_index);
            assert_eq!(actual.packets(), expected.packets);
            assert_eq!(actual.bytes(), expected.bytes);
            assert_eq!(actual.duration(), expected.duration);
            assert_eq!(actual.last_pts(), expected.last_pts);
            assert_eq!(actual.last_dts(), expected.last_dts);
        }
    }

    let before_finish = muxer.report();
    let finished = muxer.finish();
    assert!(muxer.is_finished());
    assert_eq!(finished, before_finish);
    let err = muxer
        .write_packet(&Packet::new(b"x".to_vec(), 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.report(), before_finish);
}

fn exercise_hash_muxers(packets: &[Packet]) {
    for algorithm in [
        HashAlgorithm::Adler32,
        HashAlgorithm::Crc32,
        HashAlgorithm::Md5,
    ] {
        let mut muxer = HashMuxer::new(algorithm);
        let mut payload = Vec::new();
        let mut packet_count = 0_u64;

        for packet in packets {
            muxer.write_packet(packet).unwrap();
            payload.extend_from_slice(packet.data());
            packet_count += 1;

            let report = muxer.report();
            assert_eq!(report.algorithm(), algorithm);
            assert_eq!(report.packets(), packet_count);
            assert_eq!(report.bytes(), payload.len() as u64);
            assert_eq!(report.digest(), &digest_for(algorithm, &payload));
        }

        let before_finish = muxer.report();
        let finished = muxer.finish();
        assert!(muxer.is_finished());
        assert_eq!(finished, before_finish);
        assert_eq!(
            finished.line(),
            format!(
                "{}={}\n",
                algorithm.name(),
                digest_for(algorithm, &payload).hex()
            )
        );
        let err = muxer
            .write_packet(&Packet::new(b"x".to_vec(), 0))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.report(), before_finish);
    }
}

fn exercise_framecrc_muxer(packets: &[Packet]) {
    let mut muxer = FrameCrcMuxer::new();

    for (index, packet) in packets.iter().enumerate() {
        muxer.write_packet(packet).unwrap();
        assert_eq!(muxer.records().len(), index + 1);

        let record = &muxer.records()[index];
        assert_eq!(record.stream_index(), packet.stream_index());
        assert_eq!(record.pts(), packet.pts());
        assert_eq!(record.dts(), packet.dts());
        assert_eq!(record.duration(), packet.duration());
        assert_eq!(record.size(), packet.data().len());
        assert_eq!(record.crc32(), crc32_ieee(packet.data()));
        assert!(record
            .line()
            .contains(&format!("stream={}", packet.stream_index())));
        assert!(record
            .line()
            .contains(&format!("size={}", packet.data().len())));
    }

    let before_finish = muxer.render();
    let record_count = muxer.records().len();
    let finished = muxer.finish();
    assert!(muxer.is_finished());
    assert_eq!(finished, before_finish);
    let err = muxer
        .write_packet(&Packet::new(b"x".to_vec(), 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.records().len(), record_count);
    assert_eq!(muxer.render(), before_finish);
}

fn exercise_fixtures() {
    let mut first = Packet::new(b"abc".to_vec(), 2);
    first.set_pts(Some(10));
    first.set_dts(Some(8));
    first.set_duration(2).unwrap();
    first.set_key(true);
    first.push_side_data(SideData::new("fixture", b"side".to_vec()).unwrap());

    let mut second = Packet::new(Vec::new(), 0);
    second.set_duration(0).unwrap();

    let packets = vec![first, second, Packet::new(b"tail".to_vec(), 1)];
    exercise_null_muxer(&packets);
    exercise_hash_muxers(&packets);
    exercise_framecrc_muxer(&packets);
}

fn digest_for(algorithm: HashAlgorithm, data: &[u8]) -> HashDigest {
    match algorithm {
        HashAlgorithm::Adler32 => HashDigest::U32(adler32(data)),
        HashAlgorithm::Crc32 => HashDigest::U32(crc32_ieee(data)),
        HashAlgorithm::Md5 => HashDigest::Bytes(md5(data).to_vec()),
    }
}

fn packets_from(cursor: &mut Cursor<'_>) -> Vec<Packet> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_PACKETS + 1);
    let mut packets = Vec::with_capacity(count);

    for _ in 0..count {
        let stream_index = usize::from(cursor.next().unwrap_or_default()) % (MAX_STREAM_INDEX + 1);
        let mut packet = Packet::new(payload_from(cursor), stream_index);
        packet.set_pts(timestamp_from(cursor.next(), cursor.next()));
        packet.set_dts(timestamp_from(cursor.next(), cursor.next()));
        packet
            .set_duration(i64::from(cursor.next().unwrap_or_default() % 32))
            .unwrap();
        packet.set_key(cursor.next().unwrap_or_default() & 1 == 1);

        if cursor.next().unwrap_or_default().is_multiple_of(4) {
            let side_payload = payload_from(cursor);
            if let Ok(side_data) = SideData::new("fuzz", side_payload) {
                packet.push_side_data(side_data);
            }
        }

        packets.push(packet);
    }

    packets
}

fn payload_from(cursor: &mut Cursor<'_>) -> Vec<u8> {
    let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD_LEN + 1);
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
}

fn timestamp_from(mode: Option<u8>, magnitude: Option<u8>) -> Option<i64> {
    let magnitude = i64::from(magnitude.unwrap_or_default());
    match mode.unwrap_or_default() % 6 {
        0 => None,
        1 => Some(0),
        2 => Some(magnitude),
        3 => Some(-magnitude),
        4 => Some(i64::from(i16::from(magnitude as i8))),
        _ => Some(i64::MAX - magnitude),
    }
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
