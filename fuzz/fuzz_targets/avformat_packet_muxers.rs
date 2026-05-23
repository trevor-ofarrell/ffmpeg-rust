#![no_main]

use avformat::{
    ffmpeg_framecrc_checksum, FrameCrcMuxer, FrameHashMuxer, HashAlgorithm, HashDigest, HashMuxer,
    NullMuxer, StreamHashMuxer, StreamHashStreamType,
};
use avutil::{
    adler32, crc32_ieee, md5, murmur3, ripemd128, ripemd160, ripemd256, ripemd320, sha1,
    sha224, sha256, sha384, sha512, sha512_224, sha512_256, AvErrorKind, Packet, SideData,
};
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
    exercise_framehash_muxers(&packets);
    exercise_streamhash_muxers(&packets);
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
        HashAlgorithm::Murmur3,
        HashAlgorithm::Md5,
        HashAlgorithm::Ripemd128,
        HashAlgorithm::Ripemd160,
        HashAlgorithm::Ripemd256,
        HashAlgorithm::Ripemd320,
        HashAlgorithm::Sha160,
        HashAlgorithm::Sha224,
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
        HashAlgorithm::Sha512Trunc224,
        HashAlgorithm::Sha512Trunc256,
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
        assert_eq!(record.checksum(), ffmpeg_framecrc_checksum(packet.data()));
        assert!(record.line().starts_with(&format!("{},", packet.stream_index())));
        assert!(record.line().contains(&format!(
            ", {:>8}, 0x",
            packet.data().len()
        )));
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

fn exercise_framehash_muxers(packets: &[Packet]) {
    for algorithm in [
        HashAlgorithm::Adler32,
        HashAlgorithm::Crc32,
        HashAlgorithm::Murmur3,
        HashAlgorithm::Md5,
        HashAlgorithm::Ripemd128,
        HashAlgorithm::Ripemd160,
        HashAlgorithm::Ripemd256,
        HashAlgorithm::Ripemd320,
        HashAlgorithm::Sha160,
        HashAlgorithm::Sha224,
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
        HashAlgorithm::Sha512Trunc224,
        HashAlgorithm::Sha512Trunc256,
    ] {
        let mut muxer = FrameHashMuxer::new(algorithm);

        for (index, packet) in packets.iter().enumerate() {
            muxer.write_packet(packet).unwrap();
            assert_eq!(muxer.records().len(), index + 1);

            let record = &muxer.records()[index];
            assert_eq!(record.algorithm(), algorithm);
            assert_eq!(record.stream_index(), packet.stream_index());
            assert_eq!(record.pts(), packet.pts());
            assert_eq!(record.dts(), packet.dts());
            assert_eq!(record.duration(), packet.duration());
            assert_eq!(record.size(), packet.data().len());
            assert_eq!(record.digest(), &digest_for(algorithm, packet.data()));
            assert!(record
                .line()
                .contains(&format!("stream={}", packet.stream_index())));
            assert!(record
                .line()
                .contains(&format!("size={}", packet.data().len())));
            assert!(record
                .line()
                .contains(&format!("{}=", algorithm.name().to_ascii_lowercase())));
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
}

fn exercise_streamhash_muxers(packets: &[Packet]) {
    for algorithm in [
        HashAlgorithm::Adler32,
        HashAlgorithm::Crc32,
        HashAlgorithm::Murmur3,
        HashAlgorithm::Md5,
        HashAlgorithm::Ripemd128,
        HashAlgorithm::Ripemd160,
        HashAlgorithm::Ripemd256,
        HashAlgorithm::Ripemd320,
        HashAlgorithm::Sha160,
        HashAlgorithm::Sha224,
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
        HashAlgorithm::Sha512Trunc224,
        HashAlgorithm::Sha512Trunc256,
    ] {
        let mut muxer = StreamHashMuxer::new(algorithm);
        let mut streams = Vec::<ExpectedStreamHash>::new();

        for packet in packets {
            let stream_type = stream_type_for(packet.stream_index());
            muxer.write_packet(packet, stream_type).unwrap();
            while streams.len() <= packet.stream_index() {
                let stream_index = streams.len();
                streams.push(ExpectedStreamHash {
                    stream_type: stream_type_for(stream_index),
                    payload: Vec::new(),
                    packets: 0,
                    bytes: 0,
                });
            }
            let expected = &mut streams[packet.stream_index()];
            expected.payload.extend_from_slice(packet.data());
            expected.packets += 1;
            expected.bytes += packet.data().len() as u64;

            let records = muxer.records();
            let record = records
                .iter()
                .find(|record| record.stream_index() == packet.stream_index())
                .unwrap();
            assert_eq!(record.stream_type(), stream_type);
            assert_eq!(record.algorithm(), algorithm);
            assert_eq!(record.digest(), &digest_for(algorithm, &expected.payload));
            assert_eq!(record.packets(), expected.packets);
            assert_eq!(record.bytes(), expected.bytes);
            assert_eq!(
                record.line(),
                format!(
                    "{},{},{}={}\n",
                    packet.stream_index(),
                    stream_type.code(),
                    algorithm.name(),
                    digest_for(algorithm, &expected.payload).hex()
                )
            );
        }

        let records = muxer.records();
        assert_eq!(
            records.len(),
            streams
                .iter()
                .enumerate()
                .filter(|(_, stream)| !stream.payload.is_empty() || stream.packets != 0)
                .count()
        );
        for record in &records {
            let expected = &streams[record.stream_index()];
            assert_eq!(record.stream_type(), expected.stream_type);
            assert_eq!(record.digest(), &digest_for(algorithm, &expected.payload));
        }

        let before_finish = muxer.render();
        let finished = muxer.finish();
        assert!(muxer.is_finished());
        assert_eq!(finished, before_finish);
        let err = muxer
            .write_packet(&Packet::new(b"x".to_vec(), 0), stream_type_for(0))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.render(), before_finish);
    }

    let mut conflicting = StreamHashMuxer::new(HashAlgorithm::Sha256);
    let packet = Packet::new(b"abc".to_vec(), 0);
    conflicting
        .write_packet(&packet, StreamHashStreamType::Audio)
        .unwrap();
    let before = conflicting.records();
    let err = conflicting
        .write_packet(&packet, StreamHashStreamType::Video)
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(conflicting.records(), before);
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
    exercise_framehash_muxers(&packets);
    exercise_streamhash_muxers(&packets);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedStreamHash {
    stream_type: StreamHashStreamType,
    payload: Vec<u8>,
    packets: u64,
    bytes: u64,
}

fn stream_type_for(stream_index: usize) -> StreamHashStreamType {
    if stream_index.is_multiple_of(2) {
        StreamHashStreamType::Video
    } else {
        StreamHashStreamType::Audio
    }
}

fn digest_for(algorithm: HashAlgorithm, data: &[u8]) -> HashDigest {
    match algorithm {
        HashAlgorithm::Adler32 => HashDigest::U32(adler32(data)),
        HashAlgorithm::Crc32 => HashDigest::U32(crc32_ieee(data)),
        HashAlgorithm::Murmur3 => HashDigest::Bytes(murmur3(data).to_vec()),
        HashAlgorithm::Md5 => HashDigest::Bytes(md5(data).to_vec()),
        HashAlgorithm::Ripemd128 => HashDigest::Bytes(ripemd128(data).to_vec()),
        HashAlgorithm::Ripemd160 => HashDigest::Bytes(ripemd160(data).to_vec()),
        HashAlgorithm::Ripemd256 => HashDigest::Bytes(ripemd256(data).to_vec()),
        HashAlgorithm::Ripemd320 => HashDigest::Bytes(ripemd320(data).to_vec()),
        HashAlgorithm::Sha160 => HashDigest::Bytes(sha1(data).to_vec()),
        HashAlgorithm::Sha224 => HashDigest::Bytes(sha224(data).to_vec()),
        HashAlgorithm::Sha256 => HashDigest::Bytes(sha256(data).to_vec()),
        HashAlgorithm::Sha384 => HashDigest::Bytes(sha384(data).to_vec()),
        HashAlgorithm::Sha512 => HashDigest::Bytes(sha512(data).to_vec()),
        HashAlgorithm::Sha512Trunc224 => HashDigest::Bytes(sha512_224(data).to_vec()),
        HashAlgorithm::Sha512Trunc256 => HashDigest::Bytes(sha512_256(data).to_vec()),
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
