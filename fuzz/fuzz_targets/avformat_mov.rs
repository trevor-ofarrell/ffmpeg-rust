#![no_main]

use avformat::{mov::parse_webvtt_sample, MovDemuxer, MovInfo};
use avutil::{AvErrorKind, Packet};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_mov(data);
    exercise_mov(&valid_mov());
    exercise_webvtt_sample(data);
    exercise_webvtt_sample(&valid_webvtt_sample());
});

fn exercise_mov(input: &[u8]) {
    let Ok(mut demuxer) = MovDemuxer::open(input) else {
        return;
    };

    let info = demuxer.info().clone();
    assert!(!info.major_brand().is_empty());
    assert!(info.timescale() > 0);
    assert!(!info.tracks().is_empty());

    let expected_packets = info
        .tracks()
        .iter()
        .map(|track| {
            assert!(track.id() > 0);
            assert!(track.media_timescale() > 0);
            if track.sample_count() > 0 {
                assert!(info.has_media_data());
                assert!(track.codec_parameters().is_some());
            }
            if let Some(codec_parameters) = track.codec_parameters() {
                assert!(!codec_parameters.codec_tag().is_empty());
            }
            track.sample_count()
        })
        .sum::<usize>();

    let mut packet_count = 0_usize;
    let mut next_dts = vec![0_i64; info.tracks().len()];
    loop {
        match demuxer.read_packet() {
            Ok(Some(packet)) => {
                assert_packet(&packet, &info, &mut next_dts);
                packet_count += 1;
                assert!(packet_count <= expected_packets);
            }
            Ok(None) => break,
            Err(err) => {
                assert_eq!(err.kind(), AvErrorKind::Unsupported);
                assert_eq!(expected_packets, 0);
                return;
            }
        }
    }
    assert_eq!(packet_count, expected_packets);
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn assert_packet(packet: &Packet, info: &MovInfo, next_dts: &mut [i64]) {
    let stream_index = packet.stream_index();
    assert!(stream_index < info.tracks().len());
    assert!(!packet.data().is_empty());
    assert_eq!(packet.dts(), Some(next_dts[stream_index]));
    assert!(packet.pts().is_some());
    assert!(packet.duration() >= 0);

    let side_data = packet.side_data();
    assert!(side_data.len() >= 2);
    assert_eq!(side_data[0].kind(), "mov_track_id");
    assert_eq!(side_data[0].data().len(), 4);
    assert_eq!(side_data[1].kind(), "mov_codec_tag");
    assert!(!side_data[1].data().is_empty());

    if let Some(codec_tag) = info.tracks()[stream_index].codec_tag() {
        assert_eq!(side_data[1].data(), codec_tag.as_bytes());
    }

    next_dts[stream_index] += packet.duration();
}

fn exercise_webvtt_sample(input: &[u8]) {
    let Ok(sample) = parse_webvtt_sample(input) else {
        return;
    };
    assert_eq!(
        sample.is_empty_cue(),
        sample.cue_count() == 0 && sample.additional_text_count() == 0
    );
    assert!(sample.is_empty_cue() || sample.cue_count() > 0);
}

fn valid_mov() -> Vec<u8> {
    let samples = [b"aa".as_slice(), b"bbb".as_slice()];
    let durations = [1_000_u32, 2_000_u32];
    mov_with_samples(&samples, &durations)
}

fn valid_webvtt_sample() -> Vec<u8> {
    box4(
        *b"vttc",
        &[
            box4(*b"iden", b"cue-1"),
            box4(*b"sttg", b"align:start"),
            box4(*b"payl", b"hello"),
        ]
        .concat(),
    )
}

fn mov_with_samples(samples: &[&[u8]], durations: &[u32]) -> Vec<u8> {
    assert_eq!(samples.len(), durations.len());

    let ftyp = ftyp_box();
    let mdat_payload = samples.concat();
    let sample_sizes = samples
        .iter()
        .map(|sample| u32::try_from(sample.len()).unwrap())
        .collect::<Vec<_>>();
    let placeholder_moov = box4(*b"moov", &moov_payload(0, &sample_sizes, durations));
    let chunk_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
    let moov = box4(
        *b"moov",
        &moov_payload(chunk_offset, &sample_sizes, durations),
    );

    [ftyp, moov, box4(*b"mdat", &mdat_payload)].concat()
}

fn ftyp_box() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"isom");
    payload.extend_from_slice(&512_u32.to_be_bytes());
    payload.extend_from_slice(b"isom");
    payload.extend_from_slice(b"iso2");
    payload.extend_from_slice(b"avc1");
    box4(*b"ftyp", &payload)
}

fn moov_payload(chunk_offset: u64, sample_sizes: &[u32], durations: &[u32]) -> Vec<u8> {
    let media_duration = durations.iter().copied().sum::<u32>();
    [
        mvhd_v0(1_000, media_duration),
        trak_v0(
            1,
            media_duration,
            90_000,
            chunk_offset,
            sample_sizes,
            durations,
        ),
    ]
    .concat()
}

fn trak_v0(
    track_id: u32,
    duration: u32,
    timescale: u32,
    chunk_offset: u64,
    sample_sizes: &[u32],
    sample_durations: &[u32],
) -> Vec<u8> {
    let stbl = stbl_box(chunk_offset, sample_sizes, sample_durations);
    let minf = box4(*b"minf", &stbl);
    let mdia = box4(*b"mdia", &[mdhd_v0(timescale, duration), minf].concat());
    box4(
        *b"trak",
        &[tkhd_v0(track_id, duration, 1_920, 1_080), mdia].concat(),
    )
}

fn stbl_box(chunk_offset: u64, sample_sizes: &[u32], sample_durations: &[u32]) -> Vec<u8> {
    let payload = [
        stsd_box(),
        stts_box(sample_durations),
        stsc_box(u32::try_from(sample_sizes.len()).unwrap()),
        stsz_box(sample_sizes),
        stco_box(u32::try_from(chunk_offset).unwrap()),
    ]
    .concat();
    box4(*b"stbl", &payload)
}

fn stsd_box() -> Vec<u8> {
    let mut sample_entry = Vec::new();
    sample_entry.extend_from_slice(&16_u32.to_be_bytes());
    sample_entry.extend_from_slice(b"raw ");
    sample_entry.extend_from_slice(&[0; 6]);
    sample_entry.extend_from_slice(&1_u16.to_be_bytes());

    let mut body = Vec::new();
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend_from_slice(&sample_entry);
    box4(*b"stsd", &full_box(0, &body))
}

fn stts_box(durations: &[u32]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&u32::try_from(durations.len()).unwrap().to_be_bytes());
    for duration in durations {
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
    }
    box4(*b"stts", &full_box(0, &body))
}

fn stsc_box(samples_per_chunk: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend_from_slice(&samples_per_chunk.to_be_bytes());
    body.extend_from_slice(&1_u32.to_be_bytes());
    box4(*b"stsc", &full_box(0, &body))
}

fn stsz_box(sample_sizes: &[u32]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&u32::try_from(sample_sizes.len()).unwrap().to_be_bytes());
    for sample_size in sample_sizes {
        body.extend_from_slice(&sample_size.to_be_bytes());
    }
    box4(*b"stsz", &full_box(0, &body))
}

fn stco_box(chunk_offset: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend_from_slice(&chunk_offset.to_be_bytes());
    box4(*b"stco", &full_box(0, &body))
}

fn mvhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    box4(*b"mvhd", &full_box(0, &body))
}

fn tkhd_v0(track_id: u32, duration: u32, width: u32, height: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&track_id.to_be_bytes());
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    write_tkhd_tail(&mut body, width, height);
    box4(*b"tkhd", &full_box(0, &body))
}

fn write_tkhd_tail(body: &mut Vec<u8>, width: u32, height: u32) {
    body.extend_from_slice(&0_u64.to_be_bytes());
    body.extend_from_slice(&0_i16.to_be_bytes());
    body.extend_from_slice(&0_i16.to_be_bytes());
    body.extend_from_slice(&0_i16.to_be_bytes());
    body.extend_from_slice(&0_u16.to_be_bytes());
    for _ in 0..9 {
        body.extend_from_slice(&0_u32.to_be_bytes());
    }
    body.extend_from_slice(&(width << 16).to_be_bytes());
    body.extend_from_slice(&(height << 16).to_be_bytes());
}

fn mdhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&0_u16.to_be_bytes());
    body.extend_from_slice(&0_u16.to_be_bytes());
    box4(*b"mdhd", &full_box(0, &body))
}

fn full_box(version: u8, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + body.len());
    out.push(version);
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(body);
    out
}

fn box4(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = u32::try_from(8 + payload.len()).unwrap();
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(payload);
    out
}
