#![no_main]

use avformat::{
    mov::{parse_timed_text_sample, parse_webvtt_sample},
    MovDemuxer, MovInfo, MovSampleEntryDetails,
};
use avutil::{AvErrorKind, Packet};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_mov(data);
    exercise_seeded_mov(&valid_mov(), "raw ");
    exercise_seeded_mov(&valid_timed_text_mov(), "tx3g");
    exercise_seeded_mov(&valid_xml_subtitle_mov(), "stpp");
    exercise_seeded_mov(&valid_text_subtitle_mov(), "sbtt");
    exercise_seeded_mov(&valid_simple_text_mov(), "stxt");
    exercise_seeded_mov(&valid_webvtt_mov(), "wvtt");
    exercise_seeded_mov(&valid_xml_metadata_mov(), "metx");
    exercise_seeded_mov(&valid_text_metadata_mov(), "mett");
    exercise_seeded_mov(&valid_uri_metadata_mov(), "urim");
    exercise_timed_text_sample(data);
    exercise_timed_text_sample(&valid_timed_text_sample());
    exercise_webvtt_sample(data);
    exercise_webvtt_sample(&valid_webvtt_sample());
});

fn exercise_mov(input: &[u8]) {
    let Ok(mut demuxer) = MovDemuxer::open(input) else {
        return;
    };

    let info = demuxer.info().clone();
    exercise_mov_info(&info, None);
    exercise_mov_packets(&mut demuxer, &info);
}

fn exercise_seeded_mov(input: &[u8], expected_codec_tag: &str) {
    let mut demuxer =
        MovDemuxer::open(input).expect("seeded MOV fixture should parse successfully");
    let info = demuxer.info().clone();
    exercise_mov_info(&info, Some(expected_codec_tag));
    exercise_mov_packets(&mut demuxer, &info);
}

fn exercise_mov_info(info: &MovInfo, expected_codec_tag: Option<&str>) {
    assert!(!info.major_brand().is_empty());
    assert!(info.timescale() > 0);
    assert!(!info.tracks().is_empty());
    if let Some(expected_codec_tag) = expected_codec_tag {
        assert!(info.tracks().iter().any(|track| {
            track
                .codec_parameters()
                .is_some_and(|codec_parameters| codec_parameters.codec_tag() == expected_codec_tag)
        }));
        assert_seeded_sample_entry_details(info, expected_codec_tag);
    }

    for track in info.tracks() {
        assert!(track.id() > 0);
        assert!(track.media_timescale() > 0);
        if track.sample_count() > 0 {
            assert!(info.has_media_data());
            assert!(track.codec_parameters().is_some());
        }
        if let Some(codec_parameters) = track.codec_parameters() {
            assert!(!codec_parameters.codec_tag().is_empty());
            assert_sample_entry_details(codec_parameters.details());
        }
    }
}

fn exercise_mov_packets(demuxer: &mut MovDemuxer<'_>, info: &MovInfo) {
    let expected_packets = info
        .tracks()
        .iter()
        .map(|track| track.sample_count())
        .sum::<usize>();

    let mut packet_count = 0_usize;
    let mut next_dts = vec![0_i64; info.tracks().len()];
    loop {
        match demuxer.read_packet() {
            Ok(Some(packet)) => {
                assert_packet(&packet, info, &mut next_dts);
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

fn assert_sample_entry_details(details: &MovSampleEntryDetails) {
    match details {
        MovSampleEntryDetails::Subtitle(subtitle) => {
            if let Some(timed_text) = subtitle.timed_text() {
                let _ = (
                    timed_text.display_flags(),
                    timed_text.default_text_box(),
                    timed_text.default_style(),
                    timed_text.child_boxes(),
                );
            }
            if let Some(xml) = subtitle.xml_subtitle() {
                if let Some(bit_rate) = xml.bit_rate() {
                    let _ = (
                        bit_rate.buffer_size_db(),
                        bit_rate.max_bitrate(),
                        bit_rate.avg_bitrate(),
                    );
                    assert!(has_child_box(xml.child_boxes(), "btrt"));
                }
            }
            if let Some(text) = subtitle.text_subtitle() {
                assert_text_entry_invariants(
                    text.bit_rate().map(|bit_rate| {
                        (
                            bit_rate.buffer_size_db(),
                            bit_rate.max_bitrate(),
                            bit_rate.avg_bitrate(),
                        )
                    }),
                    text.text_config()
                        .map(|config| (config.version(), config.flags(), config.text_config())),
                    text.child_boxes(),
                );
            }
            if let Some(text) = subtitle.simple_text() {
                assert_text_entry_invariants(
                    text.bit_rate().map(|bit_rate| {
                        (
                            bit_rate.buffer_size_db(),
                            bit_rate.max_bitrate(),
                            bit_rate.avg_bitrate(),
                        )
                    }),
                    text.text_config()
                        .map(|config| (config.version(), config.flags(), config.text_config())),
                    text.child_boxes(),
                );
            }
            if let Some(webvtt) = subtitle.webvtt() {
                assert!(webvtt.configuration().config().starts_with("WEBVTT"));
                if let Some(bit_rate) = webvtt.bit_rate() {
                    let _ = (
                        bit_rate.buffer_size_db(),
                        bit_rate.max_bitrate(),
                        bit_rate.avg_bitrate(),
                    );
                    assert!(has_child_box(webvtt.child_boxes(), "btrt"));
                }
            }
        }
        MovSampleEntryDetails::Data(data) => {
            if let Some(xml) = data.xml_metadata() {
                if let Some(bit_rate) = xml.bit_rate() {
                    let _ = (
                        bit_rate.buffer_size_db(),
                        bit_rate.max_bitrate(),
                        bit_rate.avg_bitrate(),
                    );
                    assert!(has_child_box(xml.child_boxes(), "btrt"));
                }
            }
            if let Some(text) = data.text_metadata() {
                assert_text_entry_invariants(
                    text.bit_rate().map(|bit_rate| {
                        (
                            bit_rate.buffer_size_db(),
                            bit_rate.max_bitrate(),
                            bit_rate.avg_bitrate(),
                        )
                    }),
                    text.text_config()
                        .map(|config| (config.version(), config.flags(), config.text_config())),
                    text.child_boxes(),
                );
            }
            if let Some(uri) = data.uri_metadata() {
                if uri.uri_initialization_data().is_some() {
                    assert!(has_child_box(uri.child_boxes(), "uriI"));
                }
                if let Some(bit_rate) = uri.bit_rate() {
                    let _ = (
                        bit_rate.buffer_size_db(),
                        bit_rate.max_bitrate(),
                        bit_rate.avg_bitrate(),
                    );
                    assert!(has_child_box(uri.child_boxes(), "btrt"));
                }
                assert!(has_child_box(uri.child_boxes(), "uri "));
            }
        }
        MovSampleEntryDetails::Generic
        | MovSampleEntryDetails::Audio(_)
        | MovSampleEntryDetails::Video(_) => {}
    }
}

fn assert_seeded_sample_entry_details(info: &MovInfo, expected_codec_tag: &str) {
    let details = info
        .tracks()
        .iter()
        .filter_map(|track| track.codec_parameters())
        .find(|codec_parameters| codec_parameters.codec_tag() == expected_codec_tag)
        .map(|codec_parameters| codec_parameters.details())
        .expect("seeded MOV fixture should expose expected sample-entry details");

    match (expected_codec_tag, details) {
        ("raw ", MovSampleEntryDetails::Generic) => {}
        ("tx3g", MovSampleEntryDetails::Subtitle(subtitle)) => {
            let timed_text = subtitle.timed_text().expect("tx3g details");
            assert_eq!(timed_text.display_flags(), 0);
            assert_eq!(timed_text.default_text_box().top(), 0);
            assert_eq!(timed_text.default_style().font_id(), 1);
            assert_eq!(timed_text.child_boxes().len(), 1);
            assert!(has_child_box(timed_text.child_boxes(), "ftab"));
        }
        ("stpp", MovSampleEntryDetails::Subtitle(subtitle)) => {
            let xml = subtitle.xml_subtitle().expect("stpp details");
            assert_eq!(xml.namespace(), "urn:seed:ttml");
            assert_eq!(xml.schema_location(), "seed.xsd");
            assert_eq!(xml.auxiliary_mime_types(), "application/ttml+xml");
            assert_seeded_bitrate(xml.bit_rate());
        }
        ("sbtt", MovSampleEntryDetails::Subtitle(subtitle)) => {
            let text = subtitle.text_subtitle().expect("sbtt details");
            assert_seeded_text_entry(text, "text/vtt", "seed-subtitle-config");
        }
        ("stxt", MovSampleEntryDetails::Subtitle(subtitle)) => {
            let text = subtitle.simple_text().expect("stxt details");
            assert_seeded_text_entry(text, "text/plain", "seed-simple-text-config");
        }
        ("wvtt", MovSampleEntryDetails::Subtitle(subtitle)) => {
            let webvtt = subtitle.webvtt().expect("wvtt details");
            assert_eq!(webvtt.configuration().config(), "WEBVTT\n\n");
            assert_eq!(
                webvtt.source_label().map(|label| label.source_label()),
                Some("seed-track")
            );
            assert_seeded_bitrate(webvtt.bit_rate());
        }
        ("metx", MovSampleEntryDetails::Data(data)) => {
            let xml = data.xml_metadata().expect("metx details");
            assert_eq!(xml.content_encoding(), "utf-8");
            assert_eq!(xml.namespace(), "urn:seed:metadata");
            assert_eq!(xml.schema_location(), "metadata.xsd");
            assert_seeded_bitrate(xml.bit_rate());
        }
        ("mett", MovSampleEntryDetails::Data(data)) => {
            let text = data.text_metadata().expect("mett details");
            assert_eq!(text.content_encoding(), "utf-8");
            assert_eq!(text.mime_format(), "text/plain");
            assert_seeded_bitrate(text.bit_rate());
            let text_config = text.text_config().expect("mett text config");
            assert_eq!(text_config.version(), 0);
            assert_eq!(text_config.flags(), [0, 0, 0]);
            assert_eq!(text_config.text_config(), "seed-config");
        }
        ("urim", MovSampleEntryDetails::Data(data)) => {
            let uri = data.uri_metadata().expect("urim details");
            assert_eq!(uri.uri(), "urn:ffmpegrust:seed");
            assert_eq!(uri.uri_initialization_data(), Some([1, 2, 3].as_slice()));
            assert_seeded_bitrate(uri.bit_rate());
        }
        _ => panic!("seeded MOV fixture exposed unexpected sample-entry details"),
    }
}

fn assert_seeded_text_entry(
    entry: &avformat::mov::MovTextSubtitleSampleEntry,
    expected_mime_format: &str,
    expected_text_config: &str,
) {
    assert_eq!(entry.content_encoding(), "utf-8");
    assert_eq!(entry.mime_format(), expected_mime_format);
    assert_seeded_bitrate(entry.bit_rate());
    let text_config = entry.text_config().expect("seeded text config");
    assert_eq!(text_config.version(), 0);
    assert_eq!(text_config.flags(), [0, 0, 0]);
    assert_eq!(text_config.text_config(), expected_text_config);
}

fn assert_text_entry_invariants(
    bit_rate: Option<(u32, u32, u32)>,
    text_config: Option<(u8, [u8; 3], &str)>,
    child_boxes: &[avformat::mov::MovSampleEntryChildBox],
) {
    if bit_rate.is_some() {
        assert!(has_child_box(child_boxes, "btrt"));
    }
    if let Some((version, flags, _text_config)) = text_config {
        assert_eq!(version, 0);
        assert_eq!(flags, [0, 0, 0]);
        assert!(has_child_box(child_boxes, "txtC"));
    }
}

fn assert_seeded_bitrate(bit_rate: Option<&avformat::mov::MovBitRateBox>) {
    let bit_rate = bit_rate.expect("seeded entry should parse btrt");
    assert_eq!(bit_rate.buffer_size_db(), 1_024);
    assert_eq!(bit_rate.max_bitrate(), 2_048);
    assert_eq!(bit_rate.avg_bitrate(), 1_024);
}

fn has_child_box(child_boxes: &[avformat::mov::MovSampleEntryChildBox], box_type: &str) -> bool {
    child_boxes
        .iter()
        .any(|child_box| child_box.box_type() == box_type)
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

fn exercise_timed_text_sample(input: &[u8]) {
    let Ok(sample) = parse_timed_text_sample(input) else {
        return;
    };
    let char_count = sample.text().chars().count();
    for style in sample.style_records() {
        assert!(style.end_char() >= style.start_char());
        assert!(usize::from(style.end_char()) <= char_count);
    }
    for range in sample.highlights() {
        assert!(range.end_char() >= range.start_char());
        assert!(usize::from(range.start_char()) <= char_count);
        assert!(usize::from(range.end_char()) <= char_count.saturating_add(1));
    }
    if let Some(karaoke) = sample.karaoke() {
        let mut previous_end_time = karaoke.highlight_start_time();
        for event in karaoke.events() {
            assert!(event.highlight_end_time() >= previous_end_time);
            previous_end_time = event.highlight_end_time();
            assert!(event.text_range().end_char() >= event.text_range().start_char());
            assert!(usize::from(event.text_range().start_char()) <= char_count);
            assert!(usize::from(event.text_range().end_char()) <= char_count.saturating_add(1));
        }
    }
    for hyperlink in sample.hyperlinks() {
        assert!(hyperlink.text_range().end_char() >= hyperlink.text_range().start_char());
        assert!(usize::from(hyperlink.text_range().start_char()) <= char_count);
        assert!(usize::from(hyperlink.text_range().end_char()) <= char_count.saturating_add(1));
    }
    for range in sample.blinks() {
        assert!(range.end_char() >= range.start_char());
        assert!(usize::from(range.start_char()) <= char_count);
        assert!(usize::from(range.end_char()) <= char_count.saturating_add(1));
    }
    assert!(match sample.wrap_flag() {
        Some(flag) => flag <= 1,
        None => true,
    });
    if sample.text_box().is_some() {
        assert!(sample
            .modifier_boxes()
            .iter()
            .any(|modifier| modifier.box_type() == "tbox"));
    }
}

fn valid_mov() -> Vec<u8> {
    let samples = [b"aa".as_slice(), b"bbb".as_slice()];
    let durations = [1_000_u32, 2_000_u32];
    mov_with_sample_entry(*b"raw ", 1, &[], &samples, &durations)
}

fn valid_timed_text_sample() -> Vec<u8> {
    let style = [
        1_u16.to_be_bytes().as_slice(),
        0_u16.to_be_bytes().as_slice(),
        5_u16.to_be_bytes().as_slice(),
        1_u16.to_be_bytes().as_slice(),
        &[1, 16, 255, 255, 255, 255],
    ]
    .concat();
    let highlight = box4(
        *b"hlit",
        &[1_u16.to_be_bytes(), 5_u16.to_be_bytes()].concat(),
    );
    let wrap = box4(*b"twrp", &[1]);
    let modifiers = [box4(*b"styl", &style), highlight, wrap].concat();
    let mut out = Vec::new();
    out.extend_from_slice(&5_u16.to_be_bytes());
    out.extend_from_slice(b"hello");
    out.extend_from_slice(&modifiers);
    out
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

fn valid_timed_text_mov() -> Vec<u8> {
    let sample = valid_timed_text_sample();
    let samples = [sample.as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(*b"tx3g", 1, &timed_text_entry_extra(), &samples, &durations)
}

fn valid_xml_subtitle_mov() -> Vec<u8> {
    let samples = [b"<tt/>".as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(
        *b"stpp",
        1,
        &xml_subtitle_entry_extra(),
        &samples,
        &durations,
    )
}

fn valid_text_subtitle_mov() -> Vec<u8> {
    let samples = [b"WEBVTT cue".as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(
        *b"sbtt",
        1,
        &text_entry_extra("text/vtt", "seed-subtitle-config"),
        &samples,
        &durations,
    )
}

fn valid_simple_text_mov() -> Vec<u8> {
    let samples = [b"simple text".as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(
        *b"stxt",
        1,
        &text_entry_extra("text/plain", "seed-simple-text-config"),
        &samples,
        &durations,
    )
}

fn valid_webvtt_mov() -> Vec<u8> {
    let sample = valid_webvtt_sample();
    let samples = [sample.as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(*b"wvtt", 1, &webvtt_entry_extra(), &samples, &durations)
}

fn valid_xml_metadata_mov() -> Vec<u8> {
    let samples = [b"<metadata/>".as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(
        *b"metx",
        1,
        &xml_metadata_entry_extra(),
        &samples,
        &durations,
    )
}

fn valid_text_metadata_mov() -> Vec<u8> {
    let samples = [b"metadata".as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(
        *b"mett",
        1,
        &text_metadata_entry_extra(),
        &samples,
        &durations,
    )
}

fn valid_uri_metadata_mov() -> Vec<u8> {
    let samples = [b"uri-metadata".as_slice()];
    let durations = [1_000_u32];
    mov_with_sample_entry(
        *b"urim",
        1,
        &uri_metadata_entry_extra(),
        &samples,
        &durations,
    )
}

fn timed_text_entry_extra() -> Vec<u8> {
    let mut extra = Vec::new();
    extra.extend_from_slice(&0_u32.to_be_bytes());
    extra.extend_from_slice(&[0, 0]);
    extra.extend_from_slice(&[0, 0, 0, 0]);
    for value in [0_i16, 0, 1, 1] {
        extra.extend_from_slice(&value.to_be_bytes());
    }
    extra.extend_from_slice(&0_u16.to_be_bytes());
    extra.extend_from_slice(&0_u16.to_be_bytes());
    extra.extend_from_slice(&1_u16.to_be_bytes());
    extra.extend_from_slice(&[0, 12, 255, 255, 255, 255]);
    extra.extend_from_slice(&box4(*b"ftab", b"seed"));
    extra
}

fn xml_subtitle_entry_extra() -> Vec<u8> {
    [
        null_terminated("urn:seed:ttml"),
        null_terminated("seed.xsd"),
        null_terminated("application/ttml+xml"),
        seeded_bit_rate_box(),
    ]
    .concat()
}

fn webvtt_entry_extra() -> Vec<u8> {
    [
        box4(*b"vttC", b"WEBVTT\n\n"),
        box4(*b"vlab", b"seed-track"),
        seeded_bit_rate_box(),
    ]
    .concat()
}

fn xml_metadata_entry_extra() -> Vec<u8> {
    [
        null_terminated("utf-8"),
        null_terminated("urn:seed:metadata"),
        null_terminated("metadata.xsd"),
        seeded_bit_rate_box(),
    ]
    .concat()
}

fn text_metadata_entry_extra() -> Vec<u8> {
    text_entry_extra("text/plain", "seed-config")
}

fn text_entry_extra(mime_format: &str, text_config: &str) -> Vec<u8> {
    [
        null_terminated("utf-8"),
        null_terminated(mime_format),
        seeded_bit_rate_box(),
        box4(*b"txtC", &full_box(0, text_config.as_bytes())),
    ]
    .concat()
}

fn uri_metadata_entry_extra() -> Vec<u8> {
    [
        box4(
            *b"uri ",
            &full_box(0, &null_terminated("urn:ffmpegrust:seed")),
        ),
        box4(*b"uriI", &full_box(0, &[1, 2, 3])),
        seeded_bit_rate_box(),
    ]
    .concat()
}

fn seeded_bit_rate_box() -> Vec<u8> {
    box4(
        *b"btrt",
        &[
            1_024_u32.to_be_bytes(),
            2_048_u32.to_be_bytes(),
            1_024_u32.to_be_bytes(),
        ]
        .concat(),
    )
}

fn null_terminated(value: &str) -> Vec<u8> {
    let mut out = value.as_bytes().to_vec();
    out.push(0);
    out
}

#[derive(Clone, Copy)]
struct SampleEntrySpec<'a> {
    codec_tag: [u8; 4],
    data_reference_index: u16,
    extra_data: &'a [u8],
}

fn mov_with_sample_entry(
    codec_tag: [u8; 4],
    data_reference_index: u16,
    sample_entry_extra_data: &[u8],
    samples: &[&[u8]],
    durations: &[u32],
) -> Vec<u8> {
    assert_eq!(samples.len(), durations.len());
    let sample_entry = SampleEntrySpec {
        codec_tag,
        data_reference_index,
        extra_data: sample_entry_extra_data,
    };

    let ftyp = ftyp_box();
    let mdat_payload = samples.concat();
    let sample_sizes = samples
        .iter()
        .map(|sample| u32::try_from(sample.len()).unwrap())
        .collect::<Vec<_>>();
    let placeholder_moov = box4(
        *b"moov",
        &moov_payload(0, sample_entry, &sample_sizes, durations),
    );
    let chunk_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
    let moov = box4(
        *b"moov",
        &moov_payload(chunk_offset, sample_entry, &sample_sizes, durations),
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

fn moov_payload(
    chunk_offset: u64,
    sample_entry: SampleEntrySpec<'_>,
    sample_sizes: &[u32],
    durations: &[u32],
) -> Vec<u8> {
    let media_duration = durations.iter().copied().sum::<u32>();
    [
        mvhd_v0(1_000, media_duration),
        trak_v0(
            1,
            media_duration,
            90_000,
            chunk_offset,
            sample_entry,
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
    sample_entry: SampleEntrySpec<'_>,
    sample_sizes: &[u32],
    sample_durations: &[u32],
) -> Vec<u8> {
    let stbl = stbl_box(chunk_offset, sample_entry, sample_sizes, sample_durations);
    let minf = box4(*b"minf", &stbl);
    let mdia = box4(*b"mdia", &[mdhd_v0(timescale, duration), minf].concat());
    box4(
        *b"trak",
        &[tkhd_v0(track_id, duration, 1_920, 1_080), mdia].concat(),
    )
}

fn stbl_box(
    chunk_offset: u64,
    sample_entry: SampleEntrySpec<'_>,
    sample_sizes: &[u32],
    sample_durations: &[u32],
) -> Vec<u8> {
    let payload = [
        stsd_box(sample_entry),
        stts_box(sample_durations),
        stsc_box(u32::try_from(sample_sizes.len()).unwrap()),
        stsz_box(sample_sizes),
        stco_box(u32::try_from(chunk_offset).unwrap()),
    ]
    .concat();
    box4(*b"stbl", &payload)
}

fn stsd_box(sample_entry_spec: SampleEntrySpec<'_>) -> Vec<u8> {
    let mut sample_entry = Vec::new();
    let sample_entry_size = u32::try_from(16 + sample_entry_spec.extra_data.len()).unwrap();
    sample_entry.extend_from_slice(&sample_entry_size.to_be_bytes());
    sample_entry.extend_from_slice(&sample_entry_spec.codec_tag);
    sample_entry.extend_from_slice(&[0; 6]);
    sample_entry.extend_from_slice(&sample_entry_spec.data_reference_index.to_be_bytes());
    sample_entry.extend_from_slice(sample_entry_spec.extra_data);

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
