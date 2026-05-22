use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    packet_pack_dictionary, packet_unpack_dictionary, BufferRef, Dictionary, Frame, FrameSideData,
    FrameSideDataFlags, FrameSideDataKind, Packet, PacketActiveFormatDescription,
    PacketAudioServiceType, PacketCpbProperties, PacketDolbyVisionConf, PacketDoviCompression,
    PacketFallbackTrack, PacketFlags, PacketFrameCropping, PacketJpDualMono,
    PacketJpDualMonoSelection, PacketMatroskaBlockAdditional, PacketMpegTsStreamId, PacketOpaque,
    PacketParamChange, PacketPictureType, PacketProducerReferenceTime, PacketQualityStats,
    PacketReplayGain, PacketRtcpSenderReport, PacketS12mTimecode, PacketSideDataKind,
    PacketSideDataList, PacketSkipSamples, PacketSkipSamplesReason, PacketSubtitlePosition,
    PacketWebVttIdentifier, PacketWebVttSettings, Rational, SideData, AV_INPUT_BUFFER_PADDING_SIZE,
    AV_NOPTS_VALUE, AV_PACKET_POS_UNKNOWN,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavcodec/libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavcodec_packet_core_lifecycle_matches_packet_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavcodec = oracle_root.join("wsl/lib/libavcodec.a");
    let libswresample = oracle_root.join("wsl/lib/libswresample.a");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavcodec/packet.h").is_file(),
        "missing pinned FFmpeg libavcodec packet headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavcodec.is_file(),
        "missing pinned FFmpeg libavcodec static library `{}`",
        libavcodec.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );
    assert!(
        libswresample.is_file(),
        "missing pinned FFmpeg libswresample static library `{}`",
        libswresample.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-packet");
    fs::create_dir_all(&work_dir).expect("create avutil-packet oracle work dir");
    let source = work_dir.join("packet_oracle.c");
    let executable = work_dir.join("packet_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-packet oracle C source");

    let stdout = compile_and_run_oracle(
        &include_dir,
        &libavcodec,
        &libswresample,
        &libavutil,
        &source,
        &executable,
    );
    let oracle = parse_oracle_output(&stdout);
    let expected = expected_rows();

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "oracle row set diverged"
    );

    for (name, expected_fields) in expected {
        assert_eq!(
            row_fields(&oracle, &name),
            expected_fields.as_slice(),
            "{name} diverged"
        );
    }
}

fn expected_rows() -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    rows.insert(
        "packet:default".to_string(),
        packet_fields(&Packet::default()),
    );
    let mut init = packet_with_common_props();
    init.init_legacy();
    rows.insert("packet:init".to_string(), packet_fields(&init));
    insert_side_data_kind_inventory_row(&mut rows);
    insert_side_data_payload_layout_rows(&mut rows);

    let mut rescaled = packet_with_common_props();
    rescaled
        .rescale_ts(
            Rational::new(1, 90_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert("packet:rescale".to_string(), packet_fields(&rescaled));

    let mut rescaled_unknown = Packet::new(vec![0xaa, 0xbb], 3);
    rescaled_unknown
        .rescale_ts(
            Rational::new(1, 90_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:rescale-unknown".to_string(),
        packet_fields(&rescaled_unknown),
    );

    let src = packet_with_common_props();
    let mut copied = Packet::new(vec![0x99, 0x88], 1);
    copied.copy_props_from(&src);
    rows.insert("packet:copy-props".to_string(), packet_fields(&copied));

    let mut referenced = Packet::default();
    referenced.ref_from(&src);
    rows.insert("packet:ref".to_string(), packet_fields(&referenced));

    let cloned = src.clone();
    rows.insert("packet:clone".to_string(), packet_fields(&cloned));

    let mut move_src = packet_with_common_props();
    let mut move_dst = Packet::new(vec![0x44], 9);
    move_dst.move_ref_from(&mut move_src);
    rows.insert("packet:move-dst".to_string(), packet_fields(&move_dst));
    rows.insert("packet:move-src".to_string(), packet_fields(&move_src));

    let mut unref = packet_with_common_props();
    unref.unref();
    rows.insert("packet:unref".to_string(), packet_fields(&unref));

    insert_side_data_api_rows(&mut rows);
    insert_side_data_array_api_rows(&mut rows);
    insert_frame_packet_side_data_bridge_rows(&mut rows);
    insert_payload_api_rows(&mut rows);
    insert_dictionary_api_rows(&mut rows);

    rows
}

fn insert_side_data_kind_inventory_row(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut fields = vec![PacketSideDataKind::KNOWN.len().to_string()];
    for kind in PacketSideDataKind::KNOWN {
        fields.push(kind.ffmpeg_constant().unwrap().to_string());
        fields.push(kind.ffmpeg_value().unwrap().to_string());
        fields.push(kind.ffmpeg_side_data_name().unwrap().to_string());
    }
    rows.insert("packet:side-kind-inventory".to_string(), fields);
}

fn insert_side_data_payload_layout_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    rows.insert(
        "packet:payload-layout-replaygain".to_string(),
        payload_layout_fields(
            &PacketReplayGain::new(-123_456, 100_000, i32::MIN, 0x0102_0304).to_bytes(),
            &[0, 4, 8, 12],
        ),
    );
    rows.insert(
        "packet:payload-layout-cpb-properties".to_string(),
        payload_layout_fields(
            &PacketCpbProperties::new(5_000_000, 1_000_000, 3_500_000, 750_000, u64::MAX - 123)
                .unwrap()
                .to_bytes(),
            &[0, 8, 16, 24, 32],
        ),
    );
    rows.insert(
        "packet:payload-layout-prft".to_string(),
        payload_layout_fields(
            &PacketProducerReferenceTime::new(1_700_000_000_123_456, 0x0102_0304).to_bytes(),
            &[0, 8],
        ),
    );
    rows.insert(
        "packet:payload-layout-rtcp-sr".to_string(),
        payload_layout_fields(
            &PacketRtcpSenderReport::new(
                0x0102_0304,
                0x0506_0708_090a_0b0c,
                0x0d0e_0f10,
                0x1112_1314,
                0x1516_1718,
            )
            .to_bytes(),
            &[0, 8, 16, 20, 24],
        ),
    );
    rows.insert(
        "packet:payload-layout-dovi-conf".to_string(),
        payload_layout_fields(
            &PacketDolbyVisionConf::new(
                1,
                0,
                8,
                6,
                true,
                false,
                true,
                4,
                PacketDoviCompression::Limited,
            )
            .to_bytes(),
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
        ),
    );

    let mut audio_fields =
        payload_layout_fields(&PacketAudioServiceType::Commentary.to_bytes(), &[]);
    for service_type in PacketAudioServiceType::KNOWN {
        audio_fields.push(service_type.as_raw().to_string());
    }
    audio_fields.push("9".to_string());
    rows.insert(
        "packet:payload-layout-audio-service-type".to_string(),
        audio_fields,
    );

    rows.insert(
        "packet:payload-layout-h263-mb-info".to_string(),
        payload_layout_fields(
            &[
                0x04, 0x03, 0x02, 0x01, 0x11, 0x22, 0x44, 0x33, 0x55, 0x66, 0x77, 0x88,
            ],
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-quality-stats".to_string(),
        payload_layout_fields(
            &PacketQualityStats::new(
                1234,
                PacketPictureType::B,
                vec![0x0102_0304_0506_0708, 0x1112_1314_1516_1718],
            )
            .unwrap()
            .to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-fallback-track".to_string(),
        payload_layout_fields(&PacketFallbackTrack::new(7).unwrap().to_bytes(), &[]),
    );
    rows.insert(
        "packet:payload-layout-skip-samples".to_string(),
        payload_layout_fields(
            &PacketSkipSamples::new(
                0x0102_0304,
                0x1112_1314,
                PacketSkipSamplesReason::PaddingSilence,
                PacketSkipSamplesReason::Convergence,
            )
            .to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-param-change".to_string(),
        payload_layout_fields(
            &PacketParamChange::new(Some(-48_000), Some((1920, 1080))).to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-jp-dualmono".to_string(),
        payload_layout_fields(
            &PacketJpDualMono::new(PacketJpDualMonoSelection::Both).to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-strings-metadata".to_string(),
        payload_layout_fields(b"title\0clip\0lang\0en\0", &[]),
    );
    rows.insert(
        "packet:payload-layout-subtitle-position".to_string(),
        payload_layout_fields(
            &PacketSubtitlePosition::new(10, 20, 640, 480).to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-matroska-blockadditional".to_string(),
        payload_layout_fields(
            &PacketMatroskaBlockAdditional::new(0x0102_0304_0506_0708, vec![0xaa, 0xbb, 0xcc])
                .to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-webvtt-identifier".to_string(),
        payload_layout_fields(
            &PacketWebVttIdentifier::new(b"cue-id".to_vec())
                .unwrap()
                .to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-webvtt-settings".to_string(),
        payload_layout_fields(
            &PacketWebVttSettings::new(b"line:10% align:start".to_vec())
                .unwrap()
                .to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-metadata-update".to_string(),
        payload_layout_fields(b"artist\0example\0", &[]),
    );
    rows.insert(
        "packet:payload-layout-mpegts-stream-id".to_string(),
        payload_layout_fields(&PacketMpegTsStreamId::new(0xe0).to_bytes(), &[]),
    );
    rows.insert(
        "packet:payload-layout-a53-cc".to_string(),
        payload_layout_fields(&[0x04, 0xff, 0x00, 0x05, 0xee, 0x01], &[]),
    );
    rows.insert(
        "packet:payload-layout-afd".to_string(),
        payload_layout_fields(&PacketActiveFormatDescription::SixteenNine.to_bytes(), &[]),
    );
    rows.insert(
        "packet:payload-layout-icc-profile-opaque".to_string(),
        payload_layout_fields(b"opaque-icc-profile-bytes", &[]),
    );
    rows.insert(
        "packet:payload-layout-s12m-timecode".to_string(),
        payload_layout_fields(
            &PacketS12mTimecode::new(&[0x0102_0304, 0x1112_1314, 0x2122_2324])
                .unwrap()
                .to_bytes(),
            &[],
        ),
    );
    rows.insert(
        "packet:payload-layout-frame-cropping".to_string(),
        payload_layout_fields(&PacketFrameCropping::new(1, 2, 3, 4).to_bytes(), &[]),
    );
    rows.insert(
        "packet:payload-layout-lcevc".to_string(),
        payload_layout_fields(&[0x00, 0x00, 0x01, 0xe0, 0x90], &[]),
    );
}

fn insert_payload_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let new_packet = Packet::new_zeroed(3, 0).unwrap();
    rows.insert(
        "packet:payload-new-packet-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-new-packet".to_string(),
        payload_allocation_fields(&new_packet),
    );

    let from_data = Packet::from_data(vec![0xaa, 0xbb, 0xcc]).unwrap();
    rows.insert(
        "packet:payload-from-data-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-from-data".to_string(),
        payload_fields(&from_data),
    );

    let mut grow = Packet::new_zeroed(2, 0).unwrap();
    grow.make_data_writable().copy_from_slice(&[0xaa, 0xbb]);
    grow.grow_data(3).unwrap();
    rows.insert("packet:payload-grow-ret".to_string(), vec!["0".to_string()]);
    rows.insert("packet:payload-grow".to_string(), payload_fields(&grow));

    grow.shrink_data(2).unwrap();
    rows.insert("packet:payload-shrink".to_string(), payload_fields(&grow));

    let src = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
    let mut writable = Packet::default();
    writable.ref_from(&src);
    writable.make_writable().unwrap();
    writable.make_data_writable()[0] = 0xcc;
    rows.insert(
        "packet:payload-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-writable-src".to_string(),
        payload_fields(&src),
    );
    rows.insert(
        "packet:payload-make-writable-dst".to_string(),
        payload_fields(&writable),
    );

    let mut refcounted = Packet::new(vec![0xaa, 0xbb], 0);
    refcounted.make_refcounted().unwrap();
    rows.insert(
        "packet:payload-make-refcounted-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted".to_string(),
        payload_fields(&refcounted),
    );
}

fn insert_dictionary_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let empty = Dictionary::new();
    rows.insert(
        "packet:dict-pack-empty".to_string(),
        dictionary_payload_fields(&packet_pack_dictionary(&empty)),
    );

    let mut dict = Dictionary::new();
    dict.set("title", "Clip").unwrap();
    dict.set("language", "eng").unwrap();
    dict.set("empty", "").unwrap();
    let packed = packet_pack_dictionary(&dict);
    rows.insert(
        "packet:dict-pack".to_string(),
        dictionary_payload_fields(&packed),
    );

    let unpacked = packet_unpack_dictionary(&packed).unwrap();
    rows.insert("packet:dict-unpack-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "packet:dict-unpack".to_string(),
        dictionary_fields(&unpacked),
    );

    let duplicate = b"title\0first\0TITLE\0second\0";
    let duplicate = packet_unpack_dictionary(duplicate).unwrap();
    rows.insert(
        "packet:dict-unpack-duplicate-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:dict-unpack-duplicate".to_string(),
        dictionary_fields(&duplicate),
    );
}

fn insert_side_data_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut packet = Packet::default();
    packet.push_side_data(SideData::new_extradata(vec![0x11, 0x22, 0x33, 0x44]).unwrap());
    rows.insert(
        "packet:side-new".to_string(),
        side_data_summary_fields(&packet),
    );
    rows.insert(
        "packet:side-get".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind("new_extradata")),
    );

    let shrunk = packet.shrink_side_data("new_extradata", 2).unwrap();
    assert!(shrunk, "expected new_extradata side data to shrink");
    rows.insert("packet:side-shrink-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "packet:side-shrink".to_string(),
        side_data_summary_fields(&packet),
    );
    rows.insert(
        "packet:side-get-shrunk".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind("new_extradata")),
    );
    rows.insert(
        "packet:side-get-missing".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind("palette")),
    );

    packet.clear_side_data();
    rows.insert(
        "packet:side-free".to_string(),
        side_data_summary_fields(&packet),
    );

    let mut packet = Packet::default();
    packet.add_side_data(SideData::new_extradata(vec![0x11, 0x22]).unwrap());
    packet.add_side_data(SideData::new_extradata(vec![0xaa, 0xbb, 0xcc]).unwrap());
    rows.insert(
        "packet:side-new-replace".to_string(),
        side_data_summary_fields(&packet),
    );

    let replaced = packet
        .add_side_data(SideData::new_extradata(vec![0x55, 0x66]).unwrap())
        .expect("new_extradata should be replaced");
    assert_eq!(replaced.data(), &[0xaa, 0xbb, 0xcc]);
    rows.insert(
        "packet:side-add-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:side-add-replace".to_string(),
        side_data_summary_fields(&packet),
    );

    let appended = packet
        .add_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x77]).unwrap());
    assert!(appended.is_none(), "palette side data should append");
    rows.insert(
        "packet:side-add-append-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:side-add-append".to_string(),
        side_data_summary_fields(&packet),
    );
}

fn insert_side_data_array_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut list = PacketSideDataList::new();
    list.new_side_data(PacketSideDataKind::NewExtradata, 4)
        .unwrap()
        .data_mut()
        .copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    rows.insert(
        "packet:array-new".to_string(),
        side_data_list_summary_fields(&list),
    );
    rows.insert(
        "packet:array-get".to_string(),
        side_data_lookup_fields(list.get(&PacketSideDataKind::NewExtradata)),
    );

    list.new_side_data(PacketSideDataKind::NewExtradata, 2)
        .unwrap()
        .data_mut()
        .copy_from_slice(&[0xaa, 0xbb]);
    rows.insert(
        "packet:array-new-replace".to_string(),
        side_data_list_summary_fields(&list),
    );

    let replaced = list
        .add_side_data(SideData::new_extradata(vec![0x55, 0x66, 0x77]).unwrap())
        .expect("new_extradata should be replaced");
    assert_eq!(replaced.data(), &[0xaa, 0xbb]);
    rows.insert(
        "packet:array-add-replace-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-add-replace".to_string(),
        side_data_list_summary_fields(&list),
    );

    let appended = list
        .add_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x99]).unwrap());
    assert!(appended.is_none(), "palette side data should append");
    rows.insert(
        "packet:array-add-append-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-add-append".to_string(),
        side_data_list_summary_fields(&list),
    );
    rows.insert(
        "packet:array-get-palette".to_string(),
        side_data_lookup_fields(list.get(&PacketSideDataKind::Palette)),
    );

    let removed = list
        .remove_kind(&PacketSideDataKind::NewExtradata)
        .expect("new_extradata should be removed");
    assert_eq!(removed.data(), &[0x55, 0x66, 0x77]);
    rows.insert(
        "packet:array-remove-new".to_string(),
        side_data_list_summary_fields(&list),
    );
    assert!(list
        .remove_kind(&PacketSideDataKind::NewExtradata)
        .is_none());
    rows.insert(
        "packet:array-remove-missing".to_string(),
        side_data_list_summary_fields(&list),
    );

    list.clear();
    rows.insert(
        "packet:array-free".to_string(),
        side_data_list_summary_fields(&list),
    );
}

fn insert_frame_packet_side_data_bridge_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut packet_list = PacketSideDataList::new();
    let frame_side_data =
        FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x10, 0x20, 0x30])
            .unwrap();
    packet_list
        .add_from_frame_side_data(&frame_side_data)
        .unwrap();
    rows.insert(
        "packet:frame-to-packet-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:frame-to-packet".to_string(),
        side_data_list_summary_fields(&packet_list),
    );

    let replacement =
        FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0xaa]).unwrap();
    packet_list.add_from_frame_side_data(&replacement).unwrap();
    rows.insert(
        "packet:frame-to-packet-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:frame-to-packet-replace".to_string(),
        side_data_list_summary_fields(&packet_list),
    );

    let unmapped =
        FrameSideData::new_with_kind(FrameSideDataKind::A53ClosedCaptions, vec![0x55]).unwrap();
    let err = packet_list.add_from_frame_side_data(&unmapped).unwrap_err();
    rows.insert(
        "packet:frame-to-packet-unmapped-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:frame-to-packet-unmapped".to_string(),
        side_data_list_summary_fields(&packet_list),
    );

    let mut frame = Frame::empty();
    let packet_side_data =
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x01, 0x02, 0x03]).unwrap();
    packet_side_data
        .add_to_frame(&mut frame, FrameSideDataFlags::EMPTY)
        .unwrap();
    rows.insert(
        "packet:packet-to-frame-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:packet-to-frame".to_string(),
        frame_side_data_summary_fields(&frame),
    );

    let duplicate_err = packet_side_data
        .add_to_frame(&mut frame, FrameSideDataFlags::EMPTY)
        .unwrap_err();
    rows.insert(
        "packet:packet-to-frame-duplicate-ret".to_string(),
        vec![duplicate_err.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-duplicate".to_string(),
        frame_side_data_summary_fields(&frame),
    );

    SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x09, 0x08])
        .unwrap()
        .add_to_frame(&mut frame, FrameSideDataFlags::REPLACE)
        .unwrap();
    rows.insert(
        "packet:packet-to-frame-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-replace".to_string(),
        frame_side_data_summary_fields(&frame),
    );

    let unmapped_packet = SideData::new_extradata(vec![0x77]).unwrap();
    let err = unmapped_packet
        .add_to_frame(&mut frame, FrameSideDataFlags::EMPTY)
        .unwrap_err();
    rows.insert(
        "packet:packet-to-frame-unmapped-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-unmapped".to_string(),
        frame_side_data_summary_fields(&frame),
    );
}

fn packet_with_common_props() -> Packet {
    let mut packet = Packet::new(vec![0xaa, 0xbb, 0xcc], 7);
    packet.set_pts(Some(90_000));
    packet.set_dts(Some(45_000));
    packet.set_duration(180_000).unwrap();
    packet.set_pos(Some(1_234)).unwrap();
    packet.set_flag(PacketFlags::KEY, true);
    packet.set_flag(PacketFlags::CORRUPT, true);
    packet
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    packet.push_side_data(SideData::new_extradata(vec![0x11, 0x22, 0x33]).unwrap());
    packet.set_opaque(Some(PacketOpaque::new(0x1234).unwrap()));
    packet.set_opaque_ref(Some(BufferRef::from_vec(vec![0xde, 0xad, 0xbe])));
    packet
}

fn packet_fields(packet: &Packet) -> Vec<String> {
    let (side_type, side_size, side_hex) = first_side_data_fields(packet);
    vec![
        raw_ts(packet.pts()).to_string(),
        raw_ts(packet.dts()).to_string(),
        packet.duration().to_string(),
        raw_pos(packet.pos()).to_string(),
        packet.stream_index().to_string(),
        packet.flags().bits().to_string(),
        packet.len().to_string(),
        hex_or_dash(packet.data()),
        packet.side_data().len().to_string(),
        side_type,
        side_size,
        side_hex,
        packet.opaque_address().unwrap_or(0).to_string(),
        u8::from(packet.opaque_ref().is_some()).to_string(),
        packet.opaque_ref().map_or_else(
            || "0".to_string(),
            |opaque_ref| opaque_ref.len().to_string(),
        ),
        packet.opaque_ref().map_or_else(
            || "-".to_string(),
            |opaque_ref| hex_or_dash(opaque_ref.as_slice()),
        ),
        packet.opaque_ref().map_or_else(
            || "0".to_string(),
            |opaque_ref| u8::from(opaque_ref.is_writable()).to_string(),
        ),
        format!("{}/{}", packet.time_base().num(), packet.time_base().den()),
    ]
}

fn first_side_data_fields(packet: &Packet) -> (String, String, String) {
    let Some(side_data) = packet.side_data().first() else {
        return ("-".to_string(), "0".to_string(), "-".to_string());
    };

    (
        packet_side_data_type(side_data.kind_id()),
        side_data.len().to_string(),
        hex_or_dash(side_data.data()),
    )
}

fn side_data_summary_fields(packet: &Packet) -> Vec<String> {
    let mut fields = vec![packet.side_data().len().to_string()];
    for side_data in packet.side_data() {
        fields.push(packet_side_data_type(side_data.kind_id()));
        fields.push(side_data.len().to_string());
        fields.push(hex_or_dash(side_data.data()));
    }
    fields
}

fn side_data_list_summary_fields(list: &PacketSideDataList) -> Vec<String> {
    let mut fields = vec![list.len().to_string()];
    for side_data in list.entries() {
        fields.push(packet_side_data_type(side_data.kind_id()));
        fields.push(side_data.len().to_string());
        fields.push(hex_or_dash(side_data.data()));
    }
    fields
}

fn frame_side_data_summary_fields(frame: &Frame) -> Vec<String> {
    let mut fields = vec![frame.side_data().len().to_string()];
    for side_data in frame.side_data() {
        fields.push(frame_side_data_type(side_data.kind_id()));
        fields.push(side_data.data().len().to_string());
        fields.push(hex_or_dash(side_data.data()));
    }
    fields
}

fn side_data_lookup_fields(side_data: Option<&SideData>) -> Vec<String> {
    match side_data {
        Some(side_data) => vec![
            "1".to_string(),
            side_data.len().to_string(),
            hex_or_dash(side_data.data()),
        ],
        None => vec!["0".to_string(), "0".to_string(), "-".to_string()],
    }
}

fn payload_fields(packet: &Packet) -> Vec<String> {
    let padding = packet.data_buffer().padding_slice();
    assert!(
        padding.len() >= AV_INPUT_BUFFER_PADDING_SIZE,
        "packet payload should have FFmpeg input padding for oracle comparison"
    );
    vec![
        packet.len().to_string(),
        hex_or_dash(packet.data()),
        hex_or_dash(&padding[..AV_INPUT_BUFFER_PADDING_SIZE]),
        u8::from(packet.is_data_writable()).to_string(),
    ]
}

fn payload_allocation_fields(packet: &Packet) -> Vec<String> {
    let padding = packet.data_buffer().padding_slice();
    assert!(
        padding.len() >= AV_INPUT_BUFFER_PADDING_SIZE,
        "packet payload should have FFmpeg input padding for oracle comparison"
    );
    vec![
        packet.len().to_string(),
        hex_or_dash(&padding[..AV_INPUT_BUFFER_PADDING_SIZE]),
        u8::from(packet.is_data_writable()).to_string(),
    ]
}

fn payload_layout_fields(bytes: &[u8], offsets: &[usize]) -> Vec<String> {
    let mut fields = vec![bytes.len().to_string(), hex_or_dash(bytes)];
    fields.extend(offsets.iter().map(ToString::to_string));
    fields
}

fn dictionary_payload_fields(bytes: &[u8]) -> Vec<String> {
    vec![
        u8::from(!bytes.is_empty()).to_string(),
        bytes.len().to_string(),
        hex_or_dash(bytes),
    ]
}

fn dictionary_fields(dict: &Dictionary) -> Vec<String> {
    let mut fields = vec![dict.len().to_string()];
    for entry in dict.entries() {
        fields.push(entry.key().to_string());
        fields.push(entry.value().to_string());
    }
    fields
}

fn packet_side_data_type(kind: &PacketSideDataKind) -> String {
    kind.ffmpeg_value()
        .unwrap_or_else(|| panic!("unexpected packet side data kind in oracle test: {kind:?}"))
        .to_string()
}

fn frame_side_data_type(kind: &FrameSideDataKind) -> String {
    kind.ffmpeg_value()
        .unwrap_or_else(|| panic!("unexpected frame side data kind in oracle test: {kind:?}"))
        .to_string()
}

fn raw_ts(value: Option<i64>) -> i64 {
    value.unwrap_or(AV_NOPTS_VALUE)
}

fn raw_pos(value: Option<i64>) -> i64 {
    value.unwrap_or(AV_PACKET_POS_UNKNOWN)
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.split('|');
        let name = parts
            .next()
            .unwrap_or_else(|| panic!("missing oracle row name in `{line}`"))
            .to_string();
        let fields = parts.map(ToOwned::to_owned).collect::<Vec<_>>();
        assert!(
            rows.insert(name.clone(), fields).is_none(),
            "duplicate oracle row `{name}`"
        );
    }
    rows
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
        .as_slice()
}

fn compile_and_run_oracle(
    include_dir: &Path,
    libavcodec: &Path,
    libswresample: &Path,
    libavutil: &Path,
    source: &Path,
    executable: &Path,
) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "gcc -I {} {} {} {} {} -lz -lm -pthread -ldl -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavcodec)),
            shell_quote(&to_wsl_path(libswresample)),
            shell_quote(&to_wsl_path(libavutil)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavcodec packet oracle")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gcc -I {} {} {} {} {} -lz -lm -pthread -ldl -o {} && {}",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavcodec.display().to_string()),
                shell_quote(&libswresample.display().to_string()),
                shell_quote(&libavutil.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavcodec packet oracle")
    };

    assert!(
        output.status.success(),
        "libavcodec packet oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "libavcodec/defs.h"
#include "libavcodec/packet.h"
#include "libavutil/buffer.h"
#include "libavutil/dict.h"
#include "libavutil/dovi_meta.h"
#include "libavutil/frame.h"
#include "libavutil/intreadwrite.h"
#include "libavutil/mem.h"
#include "libavutil/replaygain.h"

static void fail_if(int condition, const char *message) {
    if (condition) {
        fprintf(stderr, "%s\n", message);
        exit(1);
    }
}

static void print_hex_or_dash(const uint8_t *data, int size) {
    if (!data || size <= 0) {
        printf("-");
        return;
    }
    for (int i = 0; i < size; i++)
        printf("%02x", data[i]);
}

static void print_side_data(const AVPacket *pkt) {
    if (pkt->side_data_elems <= 0 || !pkt->side_data) {
        printf("|0|-|0|-");
        return;
    }
    const AVPacketSideData *sd = &pkt->side_data[0];
    printf("|%d|%d|%zu|", pkt->side_data_elems, (int)sd->type, sd->size);
    print_hex_or_dash(sd->data, (int)sd->size);
}

static void print_packet(const char *name, const AVPacket *pkt) {
    printf("%s|%" PRId64 "|%" PRId64 "|%" PRId64 "|%" PRId64 "|%d|%d|%d|",
           name, pkt->pts, pkt->dts, pkt->duration, pkt->pos,
           pkt->stream_index, pkt->flags, pkt->size);
    print_hex_or_dash(pkt->data, pkt->size);
    print_side_data(pkt);
    printf("|%" PRIuPTR "|%d|%zu|", (uintptr_t)pkt->opaque,
           pkt->opaque_ref != NULL, pkt->opaque_ref ? pkt->opaque_ref->size : 0);
    print_hex_or_dash(pkt->opaque_ref ? pkt->opaque_ref->data : NULL,
                      pkt->opaque_ref ? (int)pkt->opaque_ref->size : 0);
    printf("|%d|%d/%d\n",
           pkt->opaque_ref ? av_buffer_is_writable(pkt->opaque_ref) : 0,
           pkt->time_base.num, pkt->time_base.den);
}

static void print_side_data_summary(const char *name, const AVPacket *pkt) {
    printf("%s|%d", name, pkt->side_data_elems);
    for (int i = 0; i < pkt->side_data_elems; i++) {
        const AVPacketSideData *sd = &pkt->side_data[i];
        printf("|%d|%zu|", (int)sd->type, sd->size);
        print_hex_or_dash(sd->data, (int)sd->size);
    }
    printf("\n");
}

static void print_side_data_lookup(const char *name, const AVPacket *pkt,
                                   enum AVPacketSideDataType type) {
    size_t size = 999;
    uint8_t *data = av_packet_get_side_data(pkt, type, &size);
    printf("%s|%d|%zu|", name, data != NULL, size);
    print_hex_or_dash(data, (int)size);
    printf("\n");
}

static void print_side_data_array_summary(const char *name,
                                          const AVPacketSideData *sd,
                                          int nb_sd) {
    printf("%s|%d", name, nb_sd);
    for (int i = 0; i < nb_sd; i++) {
        printf("|%d|%zu|", (int)sd[i].type, sd[i].size);
        print_hex_or_dash(sd[i].data, (int)sd[i].size);
    }
    printf("\n");
}

static void print_side_data_array_lookup(const char *name,
                                         const AVPacketSideData *sd,
                                         int nb_sd,
                                         enum AVPacketSideDataType type) {
    const AVPacketSideData *entry = av_packet_side_data_get(sd, nb_sd, type);
    printf("%s|%d|%zu|", name, entry != NULL, entry ? entry->size : 0);
    print_hex_or_dash(entry ? entry->data : NULL, entry ? (int)entry->size : 0);
    printf("\n");
}

static void print_frame_side_data_array_summary(const char *name,
                                                AVFrameSideData * const *sd,
                                                int nb_sd) {
    printf("%s|%d", name, nb_sd);
    for (int i = 0; i < nb_sd; i++) {
        printf("|%d|%zu|", (int)sd[i]->type, sd[i]->size);
        print_hex_or_dash(sd[i]->data, (int)sd[i]->size);
    }
    printf("\n");
}

static void print_side_data_kind_inventory(void) {
#define PRINT_SIDE_KIND(kind) do { \
    const char *name = av_packet_side_data_name(kind); \
    printf("|" #kind "|%d|%s", (int)(kind), name ? name : "<null>"); \
} while (0)
    printf("packet:side-kind-inventory|%d", (int)AV_PKT_DATA_NB);
    PRINT_SIDE_KIND(AV_PKT_DATA_PALETTE);
    PRINT_SIDE_KIND(AV_PKT_DATA_NEW_EXTRADATA);
    PRINT_SIDE_KIND(AV_PKT_DATA_PARAM_CHANGE);
    PRINT_SIDE_KIND(AV_PKT_DATA_H263_MB_INFO);
    PRINT_SIDE_KIND(AV_PKT_DATA_REPLAYGAIN);
    PRINT_SIDE_KIND(AV_PKT_DATA_DISPLAYMATRIX);
    PRINT_SIDE_KIND(AV_PKT_DATA_STEREO3D);
    PRINT_SIDE_KIND(AV_PKT_DATA_AUDIO_SERVICE_TYPE);
    PRINT_SIDE_KIND(AV_PKT_DATA_QUALITY_STATS);
    PRINT_SIDE_KIND(AV_PKT_DATA_FALLBACK_TRACK);
    PRINT_SIDE_KIND(AV_PKT_DATA_CPB_PROPERTIES);
    PRINT_SIDE_KIND(AV_PKT_DATA_SKIP_SAMPLES);
    PRINT_SIDE_KIND(AV_PKT_DATA_JP_DUALMONO);
    PRINT_SIDE_KIND(AV_PKT_DATA_STRINGS_METADATA);
    PRINT_SIDE_KIND(AV_PKT_DATA_SUBTITLE_POSITION);
    PRINT_SIDE_KIND(AV_PKT_DATA_MATROSKA_BLOCKADDITIONAL);
    PRINT_SIDE_KIND(AV_PKT_DATA_WEBVTT_IDENTIFIER);
    PRINT_SIDE_KIND(AV_PKT_DATA_WEBVTT_SETTINGS);
    PRINT_SIDE_KIND(AV_PKT_DATA_METADATA_UPDATE);
    PRINT_SIDE_KIND(AV_PKT_DATA_MPEGTS_STREAM_ID);
    PRINT_SIDE_KIND(AV_PKT_DATA_MASTERING_DISPLAY_METADATA);
    PRINT_SIDE_KIND(AV_PKT_DATA_SPHERICAL);
    PRINT_SIDE_KIND(AV_PKT_DATA_CONTENT_LIGHT_LEVEL);
    PRINT_SIDE_KIND(AV_PKT_DATA_A53_CC);
    PRINT_SIDE_KIND(AV_PKT_DATA_ENCRYPTION_INIT_INFO);
    PRINT_SIDE_KIND(AV_PKT_DATA_ENCRYPTION_INFO);
    PRINT_SIDE_KIND(AV_PKT_DATA_AFD);
    PRINT_SIDE_KIND(AV_PKT_DATA_PRFT);
    PRINT_SIDE_KIND(AV_PKT_DATA_ICC_PROFILE);
    PRINT_SIDE_KIND(AV_PKT_DATA_DOVI_CONF);
    PRINT_SIDE_KIND(AV_PKT_DATA_S12M_TIMECODE);
    PRINT_SIDE_KIND(AV_PKT_DATA_DYNAMIC_HDR10_PLUS);
    PRINT_SIDE_KIND(AV_PKT_DATA_IAMF_MIX_GAIN_PARAM);
    PRINT_SIDE_KIND(AV_PKT_DATA_IAMF_DEMIXING_INFO_PARAM);
    PRINT_SIDE_KIND(AV_PKT_DATA_IAMF_RECON_GAIN_INFO_PARAM);
    PRINT_SIDE_KIND(AV_PKT_DATA_AMBIENT_VIEWING_ENVIRONMENT);
    PRINT_SIDE_KIND(AV_PKT_DATA_FRAME_CROPPING);
    PRINT_SIDE_KIND(AV_PKT_DATA_LCEVC);
    PRINT_SIDE_KIND(AV_PKT_DATA_3D_REFERENCE_DISPLAYS);
    PRINT_SIDE_KIND(AV_PKT_DATA_RTCP_SR);
    PRINT_SIDE_KIND(AV_PKT_DATA_EXIF);
    printf("\n");
#undef PRINT_SIDE_KIND
}

static void print_payload_layout_header(const char *name, const void *payload, size_t size) {
    printf("%s|%zu|", name, size);
    print_hex_or_dash(payload, (int)size);
}

static void print_payload_layout_bytes(const char *name, const uint8_t *payload, size_t size) {
    print_payload_layout_header(name, payload, size);
    printf("\n");
}

static void print_side_data_payload_layouts(void) {
    AVReplayGain replaygain;
    memset(&replaygain, 0, sizeof(replaygain));
    replaygain.track_gain = -123456;
    replaygain.track_peak = 100000;
    replaygain.album_gain = INT32_MIN;
    replaygain.album_peak = 0x01020304;
    print_payload_layout_header("packet:payload-layout-replaygain",
                                &replaygain, sizeof(replaygain));
    printf("|%zu|%zu|%zu|%zu\n",
           offsetof(AVReplayGain, track_gain),
           offsetof(AVReplayGain, track_peak),
           offsetof(AVReplayGain, album_gain),
           offsetof(AVReplayGain, album_peak));

    AVCPBProperties cpb;
    memset(&cpb, 0, sizeof(cpb));
    cpb.max_bitrate = 5000000;
    cpb.min_bitrate = 1000000;
    cpb.avg_bitrate = 3500000;
    cpb.buffer_size = 750000;
    cpb.vbv_delay = UINT64_MAX - 123;
    print_payload_layout_header("packet:payload-layout-cpb-properties",
                                &cpb, sizeof(cpb));
    printf("|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVCPBProperties, max_bitrate),
           offsetof(AVCPBProperties, min_bitrate),
           offsetof(AVCPBProperties, avg_bitrate),
           offsetof(AVCPBProperties, buffer_size),
           offsetof(AVCPBProperties, vbv_delay));

    AVProducerReferenceTime prft;
    memset(&prft, 0, sizeof(prft));
    prft.wallclock = 1700000000123456LL;
    prft.flags = 0x01020304;
    print_payload_layout_header("packet:payload-layout-prft",
                                &prft, sizeof(prft));
    printf("|%zu|%zu\n",
           offsetof(AVProducerReferenceTime, wallclock),
           offsetof(AVProducerReferenceTime, flags));

    AVRTCPSenderReport rtcp;
    memset(&rtcp, 0, sizeof(rtcp));
    rtcp.ssrc = 0x01020304;
    rtcp.ntp_timestamp = 0x05060708090a0b0cULL;
    rtcp.rtp_timestamp = 0x0d0e0f10;
    rtcp.sender_nb_packets = 0x11121314;
    rtcp.sender_nb_bytes = 0x15161718;
    print_payload_layout_header("packet:payload-layout-rtcp-sr",
                                &rtcp, sizeof(rtcp));
    printf("|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVRTCPSenderReport, ssrc),
           offsetof(AVRTCPSenderReport, ntp_timestamp),
           offsetof(AVRTCPSenderReport, rtp_timestamp),
           offsetof(AVRTCPSenderReport, sender_nb_packets),
           offsetof(AVRTCPSenderReport, sender_nb_bytes));

    AVDOVIDecoderConfigurationRecord dovi;
    memset(&dovi, 0, sizeof(dovi));
    dovi.dv_version_major = 1;
    dovi.dv_version_minor = 0;
    dovi.dv_profile = 8;
    dovi.dv_level = 6;
    dovi.rpu_present_flag = 1;
    dovi.el_present_flag = 0;
    dovi.bl_present_flag = 1;
    dovi.dv_bl_signal_compatibility_id = 4;
    dovi.dv_md_compression = AV_DOVI_COMPRESSION_LIMITED;
    print_payload_layout_header("packet:payload-layout-dovi-conf",
                                &dovi, sizeof(dovi));
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVDOVIDecoderConfigurationRecord, dv_version_major),
           offsetof(AVDOVIDecoderConfigurationRecord, dv_version_minor),
           offsetof(AVDOVIDecoderConfigurationRecord, dv_profile),
           offsetof(AVDOVIDecoderConfigurationRecord, dv_level),
           offsetof(AVDOVIDecoderConfigurationRecord, rpu_present_flag),
           offsetof(AVDOVIDecoderConfigurationRecord, el_present_flag),
           offsetof(AVDOVIDecoderConfigurationRecord, bl_present_flag),
           offsetof(AVDOVIDecoderConfigurationRecord, dv_bl_signal_compatibility_id),
           offsetof(AVDOVIDecoderConfigurationRecord, dv_md_compression));

    enum AVAudioServiceType service_type = AV_AUDIO_SERVICE_TYPE_COMMENTARY;
    print_payload_layout_header("packet:payload-layout-audio-service-type",
                                &service_type, sizeof(service_type));
    printf("|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d\n",
           AV_AUDIO_SERVICE_TYPE_MAIN,
           AV_AUDIO_SERVICE_TYPE_EFFECTS,
           AV_AUDIO_SERVICE_TYPE_VISUALLY_IMPAIRED,
           AV_AUDIO_SERVICE_TYPE_HEARING_IMPAIRED,
           AV_AUDIO_SERVICE_TYPE_DIALOGUE,
           AV_AUDIO_SERVICE_TYPE_COMMENTARY,
           AV_AUDIO_SERVICE_TYPE_EMERGENCY,
           AV_AUDIO_SERVICE_TYPE_VOICE_OVER,
           AV_AUDIO_SERVICE_TYPE_KARAOKE,
           AV_AUDIO_SERVICE_TYPE_NB);

    uint8_t h263[12] = {0};
    AV_WL32(h263, 0x01020304U);
    h263[4] = 0x11;
    h263[5] = 0x22;
    AV_WL16(h263 + 6, 0x3344U);
    h263[8] = 0x55;
    h263[9] = 0x66;
    h263[10] = 0x77;
    h263[11] = 0x88;
    print_payload_layout_bytes("packet:payload-layout-h263-mb-info", h263, sizeof(h263));

    uint8_t quality[24] = {0};
    AV_WL32(quality, 1234U);
    quality[4] = AV_PICTURE_TYPE_B;
    quality[5] = 2;
    AV_WL64(quality + 8, 0x0102030405060708ULL);
    AV_WL64(quality + 16, 0x1112131415161718ULL);
    print_payload_layout_bytes("packet:payload-layout-quality-stats", quality, sizeof(quality));

    int32_t fallback = 7;
    print_payload_layout_bytes("packet:payload-layout-fallback-track",
                               (const uint8_t *)&fallback, sizeof(fallback));

    uint8_t skip[10] = {0};
    AV_WL32(skip, 0x01020304U);
    AV_WL32(skip + 4, 0x11121314U);
    skip[8] = 0;
    skip[9] = 1;
    print_payload_layout_bytes("packet:payload-layout-skip-samples", skip, sizeof(skip));

    uint8_t param[16] = {0};
    AV_WL32(param, AV_SIDE_DATA_PARAM_CHANGE_SAMPLE_RATE |
                   AV_SIDE_DATA_PARAM_CHANGE_DIMENSIONS);
    AV_WL32(param + 4, (uint32_t)(int32_t)-48000);
    AV_WL32(param + 8, 1920U);
    AV_WL32(param + 12, 1080U);
    print_payload_layout_bytes("packet:payload-layout-param-change", param, sizeof(param));

    const uint8_t jp_dualmono[] = { 2 };
    print_payload_layout_bytes("packet:payload-layout-jp-dualmono",
                               jp_dualmono, sizeof(jp_dualmono));

    static const uint8_t strings_metadata[] = "title\0clip\0lang\0en";
    print_payload_layout_bytes("packet:payload-layout-strings-metadata",
                               strings_metadata, sizeof(strings_metadata));

    uint8_t subtitle[16] = {0};
    AV_WL32(subtitle, 10U);
    AV_WL32(subtitle + 4, 20U);
    AV_WL32(subtitle + 8, 640U);
    AV_WL32(subtitle + 12, 480U);
    print_payload_layout_bytes("packet:payload-layout-subtitle-position",
                               subtitle, sizeof(subtitle));

    uint8_t matroska[11] = {0};
    AV_WB64(matroska, 0x0102030405060708ULL);
    matroska[8] = 0xaa;
    matroska[9] = 0xbb;
    matroska[10] = 0xcc;
    print_payload_layout_bytes("packet:payload-layout-matroska-blockadditional",
                               matroska, sizeof(matroska));

    static const uint8_t webvtt_id[] = "cue-id";
    print_payload_layout_bytes("packet:payload-layout-webvtt-identifier",
                               webvtt_id, sizeof(webvtt_id) - 1);
    static const uint8_t webvtt_settings[] = "line:10% align:start";
    print_payload_layout_bytes("packet:payload-layout-webvtt-settings",
                               webvtt_settings, sizeof(webvtt_settings) - 1);

    static const uint8_t metadata_update[] = "artist\0example";
    print_payload_layout_bytes("packet:payload-layout-metadata-update",
                               metadata_update, sizeof(metadata_update));

    const uint8_t stream_id[] = { 0xe0 };
    print_payload_layout_bytes("packet:payload-layout-mpegts-stream-id",
                               stream_id, sizeof(stream_id));

    const uint8_t a53_cc[] = { 0x04, 0xff, 0x00, 0x05, 0xee, 0x01 };
    print_payload_layout_bytes("packet:payload-layout-a53-cc", a53_cc, sizeof(a53_cc));

    const uint8_t afd[] = { AV_AFD_16_9 };
    print_payload_layout_bytes("packet:payload-layout-afd", afd, sizeof(afd));

    static const uint8_t icc_profile[] = "opaque-icc-profile-bytes";
    print_payload_layout_bytes("packet:payload-layout-icc-profile-opaque",
                               icc_profile, sizeof(icc_profile) - 1);

    uint32_t s12m[4] = { 3, 0x01020304U, 0x11121314U, 0x21222324U };
    print_payload_layout_bytes("packet:payload-layout-s12m-timecode",
                               (const uint8_t *)s12m, sizeof(s12m));

    uint8_t cropping[16] = {0};
    AV_WL32(cropping, 1U);
    AV_WL32(cropping + 4, 2U);
    AV_WL32(cropping + 8, 3U);
    AV_WL32(cropping + 12, 4U);
    print_payload_layout_bytes("packet:payload-layout-frame-cropping",
                               cropping, sizeof(cropping));

    const uint8_t lcevc[] = { 0x00, 0x00, 0x01, 0xe0, 0x90 };
    print_payload_layout_bytes("packet:payload-layout-lcevc", lcevc, sizeof(lcevc));
}

static void print_payload(const char *name, const AVPacket *pkt) {
    printf("%s|%d|", name, pkt->size);
    print_hex_or_dash(pkt->data, pkt->size);
    printf("|");
    print_hex_or_dash(pkt->data ? pkt->data + pkt->size : NULL,
                      AV_INPUT_BUFFER_PADDING_SIZE);
    printf("|%d\n", pkt->buf ? av_buffer_is_writable(pkt->buf) : 0);
}

static void print_payload_allocation(const char *name, const AVPacket *pkt) {
    printf("%s|%d|", name, pkt->size);
    print_hex_or_dash(pkt->data ? pkt->data + pkt->size : NULL,
                      AV_INPUT_BUFFER_PADDING_SIZE);
    printf("|%d\n", pkt->buf ? av_buffer_is_writable(pkt->buf) : 0);
}

static void print_dictionary_payload(const char *name, const uint8_t *data, size_t size) {
    printf("%s|%d|%zu|", name, data != NULL, size);
    print_hex_or_dash(data, (int)size);
    printf("\n");
}

static void print_dictionary(const char *name, const AVDictionary *dict) {
    const AVDictionaryEntry *entry = NULL;
    printf("%s|%d", name, av_dict_count(dict));
    while ((entry = av_dict_iterate(dict, entry))) {
        printf("|%s|%s", entry->key, entry->value);
    }
    printf("\n");
}

static AVPacket *new_packet(void) {
    AVPacket *pkt = av_packet_alloc();
    fail_if(!pkt, "av_packet_alloc failed");
    return pkt;
}

static AVPacket *packet_with_common_props(void) {
    AVPacket *pkt = new_packet();
    int ret = av_new_packet(pkt, 3);
    fail_if(ret < 0, "av_new_packet failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    pkt->data[2] = 0xcc;
    pkt->pts = 90000;
    pkt->dts = 45000;
    pkt->duration = 180000;
    pkt->pos = 1234;
    pkt->stream_index = 7;
    pkt->flags = AV_PKT_FLAG_KEY | AV_PKT_FLAG_CORRUPT;
    pkt->time_base = (AVRational){ 1, 90000 };
    pkt->opaque = (void *)(uintptr_t)0x1234;
    pkt->opaque_ref = av_buffer_alloc(3);
    fail_if(!pkt->opaque_ref, "av_buffer_alloc opaque_ref failed");
    pkt->opaque_ref->data[0] = 0xde;
    pkt->opaque_ref->data[1] = 0xad;
    pkt->opaque_ref->data[2] = 0xbe;

    uint8_t *sd = av_packet_new_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 3);
    fail_if(!sd, "av_packet_new_side_data failed");
    sd[0] = 0x11;
    sd[1] = 0x22;
    sd[2] = 0x33;
    return pkt;
}

static void exercise_side_data_api(void) {
    AVPacket *pkt = new_packet();
    uint8_t *sd = av_packet_new_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 4);
    fail_if(!sd, "av_packet_new_side_data side API failed");
    sd[0] = 0x11;
    sd[1] = 0x22;
    sd[2] = 0x33;
    sd[3] = 0x44;
    print_side_data_summary("packet:side-new", pkt);
    print_side_data_lookup("packet:side-get", pkt, AV_PKT_DATA_NEW_EXTRADATA);

    int ret = av_packet_shrink_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 2);
    printf("packet:side-shrink-ret|%d\n", ret);
    print_side_data_summary("packet:side-shrink", pkt);
    print_side_data_lookup("packet:side-get-shrunk", pkt, AV_PKT_DATA_NEW_EXTRADATA);
    print_side_data_lookup("packet:side-get-missing", pkt, AV_PKT_DATA_PALETTE);

    av_packet_free_side_data(pkt);
    print_side_data_summary("packet:side-free", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    sd = av_packet_new_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 2);
    fail_if(!sd, "av_packet_new_side_data replace seed failed");
    sd[0] = 0x11;
    sd[1] = 0x22;
    sd = av_packet_new_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 3);
    fail_if(!sd, "av_packet_new_side_data replace failed");
    sd[0] = 0xaa;
    sd[1] = 0xbb;
    sd[2] = 0xcc;
    print_side_data_summary("packet:side-new-replace", pkt);

    uint8_t *owned = av_mallocz(2 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz replace side data failed");
    owned[0] = 0x55;
    owned[1] = 0x66;
    ret = av_packet_add_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, owned, 2);
    printf("packet:side-add-replace-ret|%d\n", ret);
    print_side_data_summary("packet:side-add-replace", pkt);

    owned = av_mallocz(1 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz append side data failed");
    owned[0] = 0x77;
    ret = av_packet_add_side_data(pkt, AV_PKT_DATA_PALETTE, owned, 1);
    printf("packet:side-add-append-ret|%d\n", ret);
    print_side_data_summary("packet:side-add-append", pkt);
    av_packet_free(&pkt);
}

static void exercise_side_data_array_api(void) {
    AVPacketSideData *sd = NULL;
    int nb_sd = 0;
    AVPacketSideData *entry = av_packet_side_data_new(&sd, &nb_sd,
                                                      AV_PKT_DATA_NEW_EXTRADATA,
                                                      4, 0);
    fail_if(!entry, "av_packet_side_data_new failed");
    entry->data[0] = 0x11;
    entry->data[1] = 0x22;
    entry->data[2] = 0x33;
    entry->data[3] = 0x44;
    print_side_data_array_summary("packet:array-new", sd, nb_sd);
    print_side_data_array_lookup("packet:array-get", sd, nb_sd,
                                 AV_PKT_DATA_NEW_EXTRADATA);

    entry = av_packet_side_data_new(&sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA,
                                    2, 0);
    fail_if(!entry, "av_packet_side_data_new replace failed");
    entry->data[0] = 0xaa;
    entry->data[1] = 0xbb;
    print_side_data_array_summary("packet:array-new-replace", sd, nb_sd);

    uint8_t *owned = av_mallocz(3 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz array replace side data failed");
    owned[0] = 0x55;
    owned[1] = 0x66;
    owned[2] = 0x77;
    entry = av_packet_side_data_add(&sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA,
                                    owned, 3, 0);
    printf("packet:array-add-replace-ret|%d\n", entry != NULL);
    print_side_data_array_summary("packet:array-add-replace", sd, nb_sd);

    owned = av_mallocz(1 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz array append side data failed");
    owned[0] = 0x99;
    entry = av_packet_side_data_add(&sd, &nb_sd, AV_PKT_DATA_PALETTE,
                                    owned, 1, 0);
    printf("packet:array-add-append-ret|%d\n", entry != NULL);
    print_side_data_array_summary("packet:array-add-append", sd, nb_sd);
    print_side_data_array_lookup("packet:array-get-palette", sd, nb_sd,
                                 AV_PKT_DATA_PALETTE);

    av_packet_side_data_remove(sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA);
    print_side_data_array_summary("packet:array-remove-new", sd, nb_sd);
    av_packet_side_data_remove(sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA);
    print_side_data_array_summary("packet:array-remove-missing", sd, nb_sd);
    av_packet_side_data_free(&sd, &nb_sd);
    print_side_data_array_summary("packet:array-free", sd, nb_sd);
}

static void exercise_frame_packet_side_data_bridge_api(void) {
    AVPacketSideData *psd = NULL;
    int nb_psd = 0;
    AVFrameSideData **fsd = NULL;
    int nb_fsd = 0;
    int ret;

    AVFrameSideData *frame_entry = av_frame_side_data_new(&fsd, &nb_fsd,
                                                          AV_FRAME_DATA_REPLAYGAIN,
                                                          3, 0);
    fail_if(!frame_entry, "av_frame_side_data_new bridge seed failed");
    frame_entry->data[0] = 0x10;
    frame_entry->data[1] = 0x20;
    frame_entry->data[2] = 0x30;
    ret = av_packet_side_data_from_frame(&psd, &nb_psd, frame_entry, 0);
    printf("packet:frame-to-packet-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet", psd, nb_psd);

    frame_entry = av_frame_side_data_new(&fsd, &nb_fsd,
                                         AV_FRAME_DATA_REPLAYGAIN,
                                         1, AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    fail_if(!frame_entry, "av_frame_side_data_new bridge replace seed failed");
    frame_entry->data[0] = 0xaa;
    ret = av_packet_side_data_from_frame(&psd, &nb_psd, frame_entry, 0);
    printf("packet:frame-to-packet-replace-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet-replace", psd, nb_psd);

    AVFrameSideData *unmapped_frame = av_frame_side_data_new(&fsd, &nb_fsd,
                                                             AV_FRAME_DATA_A53_CC,
                                                             1, 0);
    fail_if(!unmapped_frame, "av_frame_side_data_new bridge unmapped seed failed");
    unmapped_frame->data[0] = 0x55;
    ret = av_packet_side_data_from_frame(&psd, &nb_psd, unmapped_frame, 0);
    printf("packet:frame-to-packet-unmapped-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet-unmapped", psd, nb_psd);
    av_frame_side_data_free(&fsd, &nb_fsd);
    av_packet_side_data_free(&psd, &nb_psd);

    AVPacketSideData *packet_entry = av_packet_side_data_new(&psd, &nb_psd,
                                                            AV_PKT_DATA_REPLAYGAIN,
                                                            3, 0);
    fail_if(!packet_entry, "av_packet_side_data_new bridge seed failed");
    packet_entry->data[0] = 0x01;
    packet_entry->data[1] = 0x02;
    packet_entry->data[2] = 0x03;
    ret = av_packet_side_data_to_frame(&fsd, &nb_fsd, packet_entry, 0);
    printf("packet:packet-to-frame-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame", fsd, nb_fsd);

    ret = av_packet_side_data_to_frame(&fsd, &nb_fsd, packet_entry, 0);
    printf("packet:packet-to-frame-duplicate-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-duplicate", fsd, nb_fsd);

    packet_entry = av_packet_side_data_new(&psd, &nb_psd,
                                           AV_PKT_DATA_REPLAYGAIN,
                                           2, 0);
    fail_if(!packet_entry, "av_packet_side_data_new bridge replace seed failed");
    packet_entry->data[0] = 0x09;
    packet_entry->data[1] = 0x08;
    ret = av_packet_side_data_to_frame(&fsd, &nb_fsd, packet_entry,
                                       AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    printf("packet:packet-to-frame-replace-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-replace", fsd, nb_fsd);

    packet_entry = av_packet_side_data_new(&psd, &nb_psd,
                                           AV_PKT_DATA_NEW_EXTRADATA,
                                           1, 0);
    fail_if(!packet_entry, "av_packet_side_data_new bridge unmapped seed failed");
    packet_entry->data[0] = 0x77;
    ret = av_packet_side_data_to_frame(&fsd, &nb_fsd, packet_entry, 0);
    printf("packet:packet-to-frame-unmapped-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-unmapped", fsd, nb_fsd);

    av_frame_side_data_free(&fsd, &nb_fsd);
    av_packet_side_data_free(&psd, &nb_psd);
}

static void exercise_payload_api(void) {
    AVPacket *pkt = new_packet();
    int ret = av_new_packet(pkt, 3);
    printf("packet:payload-new-packet-ret|%d\n", ret);
    fail_if(ret < 0, "av_new_packet payload allocation failed");
    print_payload_allocation("packet:payload-new-packet", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    uint8_t *owned = av_mallocz(3 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz payload from-data failed");
    owned[0] = 0xaa;
    owned[1] = 0xbb;
    owned[2] = 0xcc;
    ret = av_packet_from_data(pkt, owned, 3);
    printf("packet:payload-from-data-ret|%d\n", ret);
    print_payload("packet:payload-from-data", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet payload grow failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    ret = av_grow_packet(pkt, 3);
    printf("packet:payload-grow-ret|%d\n", ret);
    print_payload("packet:payload-grow", pkt);
    av_shrink_packet(pkt, 2);
    print_payload("packet:payload-shrink", pkt);
    av_packet_free(&pkt);

    AVPacket *src = new_packet();
    fail_if(av_new_packet(src, 2) < 0, "av_new_packet writable src failed");
    src->data[0] = 0xaa;
    src->data[1] = 0xbb;
    AVPacket *dst = new_packet();
    fail_if(av_packet_ref(dst, src) < 0, "av_packet_ref writable dst failed");
    ret = av_packet_make_writable(dst);
    printf("packet:payload-make-writable-ret|%d\n", ret);
    dst->data[0] = 0xcc;
    print_payload("packet:payload-make-writable-src", src);
    print_payload("packet:payload-make-writable-dst", dst);
    av_packet_free(&dst);
    av_packet_free(&src);

    pkt = new_packet();
    uint8_t stack_data[2] = { 0xaa, 0xbb };
    pkt->data = stack_data;
    pkt->size = 2;
    ret = av_packet_make_refcounted(pkt);
    printf("packet:payload-make-refcounted-ret|%d\n", ret);
    print_payload("packet:payload-make-refcounted", pkt);
    av_packet_free(&pkt);
}

static void exercise_dictionary_api(void) {
    size_t packed_size = 999;
    uint8_t *packed = av_packet_pack_dictionary(NULL, &packed_size);
    print_dictionary_payload("packet:dict-pack-empty", packed, packed_size);
    av_free(packed);

    AVDictionary *dict = NULL;
    fail_if(av_dict_set(&dict, "title", "Clip", 0) < 0, "dict set title failed");
    fail_if(av_dict_set(&dict, "language", "eng", 0) < 0, "dict set language failed");
    fail_if(av_dict_set(&dict, "empty", "", 0) < 0, "dict set empty failed");

    packed_size = 999;
    packed = av_packet_pack_dictionary(dict, &packed_size);
    fail_if(!packed, "av_packet_pack_dictionary failed");
    print_dictionary_payload("packet:dict-pack", packed, packed_size);

    AVDictionary *unpacked = NULL;
    int ret = av_packet_unpack_dictionary(packed, packed_size, &unpacked);
    printf("packet:dict-unpack-ret|%d\n", ret);
    print_dictionary("packet:dict-unpack", unpacked);
    av_dict_free(&unpacked);
    av_free(packed);
    av_dict_free(&dict);

    static const uint8_t duplicate[] = "title\0first\0TITLE\0second";
    ret = av_packet_unpack_dictionary(duplicate, sizeof(duplicate), &unpacked);
    printf("packet:dict-unpack-duplicate-ret|%d\n", ret);
    print_dictionary("packet:dict-unpack-duplicate", unpacked);
    av_dict_free(&unpacked);
}

int main(void) {
    AVPacket *pkt = new_packet();
    print_packet("packet:default", pkt);
    av_packet_free(&pkt);

    pkt = packet_with_common_props();
    av_init_packet(pkt);
    print_packet("packet:init", pkt);
    av_free(pkt);

    print_side_data_kind_inventory();
    print_side_data_payload_layouts();

    pkt = packet_with_common_props();
    av_packet_rescale_ts(pkt, (AVRational){ 1, 90000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet unknown failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    pkt->stream_index = 3;
    av_packet_rescale_ts(pkt, (AVRational){ 1, 90000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale-unknown", pkt);
    av_packet_free(&pkt);

    AVPacket *src = packet_with_common_props();
    AVPacket *dst = new_packet();
    fail_if(av_new_packet(dst, 2) < 0, "av_new_packet copy dst failed");
    dst->data[0] = 0x99;
    dst->data[1] = 0x88;
    dst->stream_index = 1;
    fail_if(av_packet_copy_props(dst, src) < 0, "av_packet_copy_props failed");
    print_packet("packet:copy-props", dst);
    av_packet_free(&dst);

    dst = new_packet();
    fail_if(av_packet_ref(dst, src) < 0, "av_packet_ref failed");
    print_packet("packet:ref", dst);
    av_packet_free(&dst);

    AVPacket *cloned = av_packet_clone(src);
    fail_if(!cloned, "av_packet_clone failed");
    print_packet("packet:clone", cloned);
    av_packet_free(&cloned);
    av_packet_free(&src);

    src = packet_with_common_props();
    dst = new_packet();
    fail_if(av_new_packet(dst, 1) < 0, "av_new_packet move dst failed");
    dst->data[0] = 0x44;
    dst->stream_index = 9;
    av_packet_move_ref(dst, src);
    print_packet("packet:move-dst", dst);
    print_packet("packet:move-src", src);
    av_packet_free(&dst);
    av_packet_free(&src);

    pkt = packet_with_common_props();
    av_packet_unref(pkt);
    print_packet("packet:unref", pkt);
    av_packet_free(&pkt);

    exercise_side_data_api();
    exercise_side_data_array_api();
    exercise_frame_packet_side_data_bridge_api();
    exercise_payload_api();
    exercise_dictionary_api();

    return 0;
}
"#
}

fn hex_or_dash(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".to_string();
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/avutil should have a repo root grandparent")
        .to_path_buf()
}

fn oracle_root(repo_root: &Path) -> PathBuf {
    let default_root = repo_root.join("third_party/ffmpeg-oracle");
    if let Ok(ffmpeg) = env::var("FFMPEG_ORACLE") {
        let ffmpeg = PathBuf::from(ffmpeg);
        let ffmpeg = if ffmpeg.is_absolute() {
            ffmpeg
        } else {
            repo_root.join(ffmpeg)
        };
        if let Some(root) = ffmpeg.ancestors().find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name == "ffmpeg-oracle")
        }) {
            return root.to_path_buf();
        }
    }
    default_root
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(windows)]
fn to_wsl_path(path: &Path) -> String {
    let absolute = absolute_path(path);
    let mut text = absolute.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = text.strip_prefix("//?/") {
        text = stripped.to_string();
    }
    let bytes = text.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        text.replace_range(0..3, &format!("/mnt/{drive}/"));
    }
    text
}

#[cfg(windows)]
fn absolute_path(path: &Path) -> PathBuf {
    if path.exists() {
        return path.canonicalize().unwrap_or_else(|err| {
            panic!(
                "failed to canonicalize existing path `{}`: {err}",
                path.display()
            )
        });
    }
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("path `{}` has no parent", path.display()))
        .canonicalize()
        .unwrap_or_else(|err| {
            panic!(
                "failed to canonicalize parent of `{}`: {err}",
                path.display()
            )
        });
    parent.join(
        path.file_name()
            .unwrap_or_else(|| panic!("path `{}` has no file name", path.display())),
    )
}

#[cfg(not(windows))]
fn to_wsl_path(path: &Path) -> String {
    path.display().to_string()
}
