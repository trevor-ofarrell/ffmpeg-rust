use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    packet_pack_dictionary, packet_unpack_dictionary, AvErrorCode, BufferRef, Dictionary, Frame,
    FrameSideData, FrameSideDataFlags, FrameSideDataKind, MatchMode, Packet,
    PacketActiveFormatDescription, PacketAmbientViewingEnvironment, PacketAudioServiceType,
    PacketContentLightMetadata, PacketCpbProperties, PacketDisplayMatrix, PacketDolbyVisionConf,
    PacketDoviCompression, PacketDynamicHdr10Plus, PacketEncryptionSubsample, PacketFallbackTrack,
    PacketFifo, PacketFlags, PacketFrameCropping, PacketHdrPlusColorTransformParams,
    PacketIamfAnimationType, PacketIamfDemixingInfoSubblock, PacketIamfMixGainSubblock,
    PacketIamfParamDefinition, PacketIamfParamDefinitionType, PacketIamfReconGainSubblock,
    PacketJpDualMono, PacketJpDualMonoSelection, PacketMasteringDisplayMetadata,
    PacketMatroskaBlockAdditional, PacketMpegTsStreamId, PacketOpaque, PacketParamChange,
    PacketPictureType, PacketProducerReferenceTime, PacketQualityStats, PacketReplayGain,
    PacketRtcpSenderReport, PacketS12mTimecode, PacketSideDataKind, PacketSideDataList,
    PacketSkipSamples, PacketSkipSamplesReason, PacketSphericalMapping, PacketSphericalProjection,
    PacketStereo3d, PacketStereo3dFlags, PacketStereo3dPrimaryEye, PacketStereo3dType,
    PacketStereo3dView, PacketSubtitlePosition, PacketThreeDReferenceDisplay,
    PacketThreeDReferenceDisplays, PacketWebVttIdentifier, PacketWebVttSettings, Rational, SetMode,
    SideData, AVPALETTE_SIZE, AV_INPUT_BUFFER_PADDING_SIZE, AV_NOPTS_VALUE, AV_PACKET_ABI_LAYOUT,
    AV_PACKET_LIST_ABI_LAYOUT, AV_PACKET_MAX_PAYLOAD_SIZE, AV_PACKET_POS_UNKNOWN,
    AV_PACKET_SIDE_DATA_ABI_LAYOUT,
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

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 source/build cache; set FFMPEG_FATE_BUILD_DIR or run scripts/bootstrap_ffmpeg_oracle_wsl.sh"]
fn upstream_fate_avpacket_passes() {
    let output = if cfg!(windows) {
        let script = match env::var("FFMPEG_FATE_BUILD_DIR") {
            Ok(build_dir) => {
                let build_dir = if build_dir.starts_with('/') || build_dir.starts_with('~') {
                    build_dir
                } else {
                    to_wsl_path(Path::new(&build_dir))
                };
                format!(
                    "test -d {0} || {{ echo 'missing FFmpeg FATE build dir: {0}' >&2; exit 66; }}; make -C {0} fate-avpacket",
                    shell_quote(&build_dir)
                )
            }
            Err(_) => concat!(
                "build_dir=\"${FFMPEGRUST_ORACLE_WORK:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1}/build\"; ",
                "test -d \"$build_dir\" || { echo \"missing FFmpeg FATE build dir: $build_dir\" >&2; exit 66; }; ",
                "make -C \"$build_dir\" fate-avpacket"
            )
            .to_string(),
        };
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run upstream FFmpeg fate-avpacket through WSL")
    } else {
        let build_dir = env::var_os("FFMPEG_FATE_BUILD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env::var("HOME").expect("HOME must be set"))
                    .join(".cache/ffmpegrust/ffmpeg-oracle-n8.1.1/build")
            });
        Command::new("make")
            .arg("-C")
            .arg(&build_dir)
            .arg("fate-avpacket")
            .output()
            .unwrap_or_else(|err| {
                panic!(
                    "run upstream FFmpeg fate-avpacket in `{}`: {err}",
                    build_dir.display()
                )
            })
    };

    assert!(
        output.status.success(),
        "upstream FFmpeg fate-avpacket failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
    insert_side_data_name_boundary_row(&mut rows);
    insert_flag_inventory_row(&mut rows);
    insert_picture_type_inventory_row(&mut rows);
    insert_packet_abi_layout_rows(&mut rows);
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

    let mut rescaled_mixed = Packet::new(vec![0xaa, 0xbb], 2);
    rescaled_mixed.set_dts(Some(90_000));
    rescaled_mixed.set_duration(45_000).unwrap();
    rescaled_mixed.set_pos(Some(123)).unwrap();
    rescaled_mixed.set_key(true);
    rescaled_mixed
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    rescaled_mixed
        .rescale_ts(
            Rational::new(1, 90_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:rescale-mixed".to_string(),
        packet_fields(&rescaled_mixed),
    );

    let mut rescaled_mixed_dts = Packet::new(vec![0xcc, 0xdd], 4);
    rescaled_mixed_dts.set_pts(Some(180_000));
    rescaled_mixed_dts.set_duration(90_000).unwrap();
    rescaled_mixed_dts.set_pos(Some(456)).unwrap();
    rescaled_mixed_dts.set_flag(PacketFlags::DISCARD, true);
    rescaled_mixed_dts
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    rescaled_mixed_dts
        .rescale_ts(
            Rational::new(1, 90_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:rescale-mixed-dts".to_string(),
        packet_fields(&rescaled_mixed_dts),
    );

    let mut rescaled_zero_duration = Packet::new(vec![0xee], 5);
    rescaled_zero_duration.set_pts(Some(180_000));
    rescaled_zero_duration.set_dts(Some(90_000));
    rescaled_zero_duration.set_pos(Some(789)).unwrap();
    rescaled_zero_duration.set_flag(PacketFlags::TRUSTED, true);
    rescaled_zero_duration
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    rescaled_zero_duration
        .rescale_ts(
            Rational::new(1, 90_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:rescale-zero-duration".to_string(),
        packet_fields(&rescaled_zero_duration),
    );

    let mut rescaled_negative_ts = Packet::new(vec![0xde, 0xad], 6);
    rescaled_negative_ts.set_pts(Some(-180_000));
    rescaled_negative_ts.set_dts(Some(-90_000));
    rescaled_negative_ts.set_duration(45_000).unwrap();
    rescaled_negative_ts.set_pos(Some(321)).unwrap();
    rescaled_negative_ts.set_flag(PacketFlags::CORRUPT, true);
    rescaled_negative_ts
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    rescaled_negative_ts
        .rescale_ts(
            Rational::new(1, 90_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:rescale-negative-ts".to_string(),
        packet_fields(&rescaled_negative_ts),
    );

    let mut rescaled_near_inf_rounding = Packet::new(vec![0x51, 0x52, 0x53], 7);
    rescaled_near_inf_rounding.set_pts(Some(24));
    rescaled_near_inf_rounding.set_dts(Some(23));
    rescaled_near_inf_rounding.set_duration(24).unwrap();
    rescaled_near_inf_rounding.set_pos(Some(654)).unwrap();
    rescaled_near_inf_rounding.set_flag(PacketFlags::DISPOSABLE, true);
    rescaled_near_inf_rounding
        .set_time_base(Rational::new(1, 48_000).unwrap())
        .unwrap();
    rescaled_near_inf_rounding
        .rescale_ts(
            Rational::new(1, 48_000).unwrap(),
            Rational::new(1, 1_000).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:rescale-near-inf-rounding".to_string(),
        packet_fields(&rescaled_near_inf_rounding),
    );

    let src = packet_with_common_props();
    let mut copied = Packet::new(vec![0x99, 0x88], 1);
    copied.copy_props_from(&src);
    rows.insert("packet:copy-props".to_string(), packet_fields(&copied));

    let empty_src = Packet::default();
    let mut copy_empty_dst = Packet::from_data(vec![0x12, 0x34]).unwrap();
    copy_empty_dst.set_pts(Some(99));
    copy_empty_dst.set_duration(9).unwrap();
    copy_empty_dst
        .set_time_base(Rational::new(1, 1_000).unwrap())
        .unwrap();
    copy_empty_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    copy_empty_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    copy_empty_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x90])));
    copy_empty_dst.copy_props_from(&empty_src);
    rows.insert(
        "packet:copy-props-empty".to_string(),
        packet_fields(&copy_empty_dst),
    );
    rows.insert(
        "packet:copy-props-empty-side".to_string(),
        side_data_summary_fields(&copy_empty_dst),
    );
    rows.insert(
        "packet:copy-props-empty-payload".to_string(),
        payload_visible_fields(&copy_empty_dst),
    );

    let mut copy_replace_src = packet_with_common_props();
    copy_replace_src.push_side_data(
        SideData::new_with_kind(
            PacketSideDataKind::SkipSamples,
            vec![0x01, 0x02, 0x03, 0x04],
        )
        .unwrap(),
    );
    let mut copy_replace_dst = Packet::new(vec![0x77, 0x66], 11);
    copy_replace_dst.set_pts(Some(11));
    copy_replace_dst.set_duration(1).unwrap();
    copy_replace_dst
        .set_time_base(Rational::new(1, 1_000).unwrap())
        .unwrap();
    copy_replace_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    copy_replace_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    copy_replace_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x99, 0x00])));
    copy_replace_dst.copy_props_from(&copy_replace_src);
    rows.insert(
        "packet:copy-props-replace".to_string(),
        packet_fields(&copy_replace_dst),
    );
    rows.insert(
        "packet:copy-props-replace-side".to_string(),
        side_data_summary_fields(&copy_replace_dst),
    );
    rows.insert(
        "packet:copy-props-replace-payload".to_string(),
        payload_visible_fields(&copy_replace_dst),
    );

    let duplicate_src = packet_with_duplicate_side_data();
    let mut copy_duplicate_dst = Packet::new(vec![0x77], 4);
    copy_duplicate_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    copy_duplicate_dst.copy_props_from(&duplicate_src);
    rows.insert(
        "packet:copy-props-duplicate-side".to_string(),
        side_data_summary_fields(&copy_duplicate_dst),
    );

    let mut ref_duplicate_dst = Packet::default();
    ref_duplicate_dst.ref_from(&duplicate_src);
    rows.insert(
        "packet:ref-duplicate-side".to_string(),
        side_data_summary_fields(&ref_duplicate_dst),
    );

    let cloned_duplicate = duplicate_src.clone();
    rows.insert(
        "packet:clone-duplicate-side".to_string(),
        side_data_summary_fields(&cloned_duplicate),
    );

    let mut move_duplicate_src = packet_with_duplicate_side_data();
    let mut move_duplicate_dst = Packet::new(vec![0x55], 1);
    move_duplicate_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    move_duplicate_dst.move_ref_from(&mut move_duplicate_src);
    rows.insert(
        "packet:move-duplicate-dst-side".to_string(),
        side_data_summary_fields(&move_duplicate_dst),
    );
    rows.insert(
        "packet:move-duplicate-src-side".to_string(),
        side_data_summary_fields(&move_duplicate_src),
    );

    let mut referenced = Packet::default();
    referenced.ref_from(&src);
    rows.insert("packet:ref".to_string(), packet_fields(&referenced));

    let mut ref_empty_dst = Packet::from_data(vec![0x55, 0x44]).unwrap();
    ref_empty_dst.set_pts(Some(22));
    ref_empty_dst.set_duration(2).unwrap();
    ref_empty_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    ref_empty_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    ref_empty_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x99])));
    ref_empty_dst.ref_from(&empty_src);
    rows.insert(
        "packet:ref-empty".to_string(),
        packet_fields(&ref_empty_dst),
    );
    rows.insert(
        "packet:payload-ref-empty".to_string(),
        payload_fields(&ref_empty_dst),
    );

    let mut ref_replace_src = packet_with_common_props();
    ref_replace_src.push_side_data(
        SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x05, 0x06]).unwrap(),
    );
    let mut ref_replace_dst = Packet::from_data(vec![0x55, 0x44]).unwrap();
    ref_replace_dst.set_pts(Some(22));
    ref_replace_dst.set_duration(2).unwrap();
    ref_replace_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    ref_replace_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    ref_replace_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x99])));
    ref_replace_dst.ref_from(&ref_replace_src);
    rows.insert(
        "packet:ref-replace".to_string(),
        packet_fields(&ref_replace_dst),
    );
    rows.insert(
        "packet:ref-replace-side".to_string(),
        side_data_summary_fields(&ref_replace_dst),
    );
    rows.insert(
        "packet:ref-replace-payload".to_string(),
        payload_visible_fields(&ref_replace_dst),
    );

    let cloned = src.clone();
    rows.insert("packet:clone".to_string(), packet_fields(&cloned));

    let cloned_empty = empty_src.clone();
    rows.insert(
        "packet:clone-empty".to_string(),
        packet_fields(&cloned_empty),
    );
    rows.insert(
        "packet:payload-clone-empty".to_string(),
        payload_fields(&cloned_empty),
    );

    let mut move_src = packet_with_common_props();
    let mut move_dst = Packet::new(vec![0x44], 9);
    move_dst.move_ref_from(&mut move_src);
    rows.insert("packet:move-dst".to_string(), packet_fields(&move_dst));
    rows.insert("packet:move-src".to_string(), packet_fields(&move_src));

    let mut move_empty_src = Packet::default();
    let mut move_empty_dst = Packet::from_data(vec![0x33, 0x22]).unwrap();
    move_empty_dst.set_pts(Some(77));
    move_empty_dst.set_duration(7).unwrap();
    move_empty_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    move_empty_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    move_empty_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x99])));
    move_empty_dst.move_ref_from(&mut move_empty_src);
    rows.insert(
        "packet:move-empty-dst".to_string(),
        packet_fields(&move_empty_dst),
    );
    rows.insert(
        "packet:move-empty-src".to_string(),
        packet_fields(&move_empty_src),
    );

    let mut move_replace_src = packet_with_common_props();
    move_replace_src.push_side_data(
        SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x07, 0x08]).unwrap(),
    );
    let mut move_replace_dst = Packet::from_data(vec![0x33, 0x22]).unwrap();
    move_replace_dst.set_pts(Some(77));
    move_replace_dst.set_duration(7).unwrap();
    move_replace_dst
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xee]).unwrap());
    move_replace_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    move_replace_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x99])));
    move_replace_dst.move_ref_from(&mut move_replace_src);
    rows.insert(
        "packet:move-replace-dst".to_string(),
        packet_fields(&move_replace_dst),
    );
    rows.insert(
        "packet:move-replace-dst-side".to_string(),
        side_data_summary_fields(&move_replace_dst),
    );
    rows.insert(
        "packet:move-replace-dst-payload".to_string(),
        payload_visible_fields(&move_replace_dst),
    );
    rows.insert(
        "packet:move-replace-src".to_string(),
        packet_fields(&move_replace_src),
    );

    let mut unref = packet_with_common_props();
    unref.unref();
    rows.insert("packet:unref".to_string(), packet_fields(&unref));

    let mut unref_empty = Packet::default();
    unref_empty.unref();
    rows.insert(
        "packet:unref-empty".to_string(),
        packet_fields(&unref_empty),
    );

    let mut unref_repeat = packet_with_common_props();
    unref_repeat.unref();
    unref_repeat.unref();
    rows.insert(
        "packet:unref-repeat".to_string(),
        packet_fields(&unref_repeat),
    );

    insert_side_data_api_rows(&mut rows);
    insert_side_data_capacity_rows(&mut rows);
    insert_side_data_array_api_rows(&mut rows);
    insert_frame_packet_side_data_bridge_rows(&mut rows);
    insert_payload_api_rows(&mut rows);
    insert_dictionary_api_rows(&mut rows);
    insert_packet_fifo_rows(&mut rows);

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

fn insert_side_data_name_boundary_row(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut fields = Vec::new();
    for value in [
        i32::MIN,
        -1,
        PacketSideDataKind::KNOWN.len() as i32,
        PacketSideDataKind::KNOWN.len() as i32 + 1,
        i32::MAX,
    ] {
        fields.push(value.to_string());
        fields.push(
            PacketSideDataKind::ffmpeg_side_data_name_for_value(value)
                .unwrap_or("<null>")
                .to_string(),
        );
    }
    rows.insert("packet:side-kind-name-boundaries".to_string(), fields);
}

fn insert_flag_inventory_row(rows: &mut BTreeMap<String, Vec<String>>) {
    rows.insert(
        "packet:flag-inventory".to_string(),
        vec![
            "AV_PKT_FLAG_KEY".to_string(),
            PacketFlags::KEY.bits().to_string(),
            "AV_PKT_FLAG_CORRUPT".to_string(),
            PacketFlags::CORRUPT.bits().to_string(),
            "AV_PKT_FLAG_DISCARD".to_string(),
            PacketFlags::DISCARD.bits().to_string(),
            "AV_PKT_FLAG_TRUSTED".to_string(),
            PacketFlags::TRUSTED.bits().to_string(),
            "AV_PKT_FLAG_DISPOSABLE".to_string(),
            PacketFlags::DISPOSABLE.bits().to_string(),
            "all".to_string(),
            PacketFlags::all().bits().to_string(),
        ],
    );
}

fn insert_picture_type_inventory_row(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut fields = Vec::new();
    for picture_type in [
        PacketPictureType::Unknown,
        PacketPictureType::I,
        PacketPictureType::P,
        PacketPictureType::B,
        PacketPictureType::S,
        PacketPictureType::Si,
        PacketPictureType::Sp,
        PacketPictureType::Bi,
    ] {
        fields.push(picture_type.ffmpeg_constant().to_string());
        fields.push(picture_type.as_byte().to_string());
        fields.push(picture_type.ffmpeg_char().to_string());
    }
    rows.insert("packet:picture-type-inventory".to_string(), fields);
}

fn insert_packet_abi_layout_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    rows.insert(
        "packet:abi-side-data-layout".to_string(),
        packet_abi_layout_fields(&AV_PACKET_SIDE_DATA_ABI_LAYOUT),
    );
    rows.insert(
        "packet:abi-avpacket-layout".to_string(),
        packet_abi_layout_fields(&AV_PACKET_ABI_LAYOUT),
    );
    rows.insert(
        "packet:abi-avpacket-list-layout".to_string(),
        packet_abi_layout_fields(&AV_PACKET_LIST_ABI_LAYOUT),
    );
}

fn packet_abi_layout_fields(layout: &avutil::PacketAbiLayout) -> Vec<String> {
    let mut fields = vec![
        layout.name.to_string(),
        layout.size.to_string(),
        layout.align.to_string(),
        layout.fields.len().to_string(),
    ];
    for field in layout.fields {
        fields.push(field.name.to_string());
        fields.push(field.offset.to_string());
        fields.push(field.size.to_string());
    }
    fields
}

fn insert_side_data_payload_layout_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let palette = (0..AVPALETTE_SIZE)
        .map(|index| (index & 0xff) as u8)
        .collect::<Vec<_>>();
    rows.insert(
        "packet:payload-layout-palette".to_string(),
        payload_layout_fields(&palette, &[]),
    );

    rows.insert(
        "packet:payload-layout-replaygain".to_string(),
        payload_layout_fields(
            &PacketReplayGain::new(-123_456, 100_000, i32::MIN, 0x0102_0304).to_bytes(),
            &[0, 4, 8, 12],
        ),
    );
    rows.insert(
        "packet:payload-layout-content-light".to_string(),
        payload_layout_fields(
            &PacketContentLightMetadata::new(1000, 400).to_bytes(),
            &[0, 4],
        ),
    );
    rows.insert(
        "packet:payload-layout-mastering-display".to_string(),
        payload_layout_fields(
            &PacketMasteringDisplayMetadata::new(
                [
                    [
                        Rational::from_raw(34_000, 50_000),
                        Rational::from_raw(16_000, 50_000),
                    ],
                    [
                        Rational::from_raw(13_250, 50_000),
                        Rational::from_raw(34_500, 50_000),
                    ],
                    [
                        Rational::from_raw(7_500, 50_000),
                        Rational::from_raw(3_000, 50_000),
                    ],
                ],
                [
                    Rational::from_raw(15_635, 50_000),
                    Rational::from_raw(16_450, 50_000),
                ],
                Rational::from_raw(50, 10_000),
                Rational::from_raw(1000, 1),
                1,
                2,
            )
            .to_bytes(),
            &[0, 48, 64, 72, 80, 84],
        ),
    );
    rows.insert(
        "packet:payload-layout-ambient-viewing-environment".to_string(),
        payload_layout_fields(
            &PacketAmbientViewingEnvironment::new(
                Rational::new(1000, 1).unwrap(),
                Rational::new(3127, 10000).unwrap(),
                Rational::new(3291, 10000).unwrap(),
            )
            .unwrap()
            .to_bytes(),
            &[0, 8, 16],
        ),
    );
    let first_tdrdi = PacketThreeDReferenceDisplay::new(0, 1, (12, 34), (5, 67), true, -11);
    let second_tdrdi = PacketThreeDReferenceDisplay::new(2, 3, (10, 20), (4, 40), false, 0);
    rows.insert(
        "packet:payload-layout-3d-reference-displays".to_string(),
        payload_layout_fields(
            &PacketThreeDReferenceDisplays::new(31, true, 7, vec![first_tdrdi, second_tdrdi])
                .unwrap()
                .to_bytes(),
            &[
                0,
                1,
                2,
                3,
                PacketThreeDReferenceDisplays::ENTRIES_OFFSET_OFFSET,
                PacketThreeDReferenceDisplays::ENTRY_SIZE_OFFSET,
                PacketThreeDReferenceDisplays::ENTRIES_OFFSET,
                PacketThreeDReferenceDisplays::ENTRY_DATA_LEN,
                0,
                2,
                4,
                5,
                6,
                7,
                8,
                10,
            ],
        ),
    );
    rows.insert(
        "packet:payload-layout-spherical".to_string(),
        payload_layout_fields(
            &PacketSphericalMapping::new(
                PacketSphericalProjection::ParametricImmersive,
                -11,
                22,
                -33,
                [0x0102_0304, 0x0506_0708, 0x090a_0b0c, 0x0d0e_0f10],
                0x1112_1314,
            )
            .to_bytes(),
            &[0, 4, 8, 12, 16, 20, 24, 28, 32],
        ),
    );
    rows.insert(
        "packet:payload-layout-displaymatrix".to_string(),
        payload_layout_fields(
            &PacketDisplayMatrix::new([
                i32::MIN,
                -1,
                0,
                1,
                1 << 16,
                -(1 << 16),
                1 << 30,
                -(1 << 30),
                i32::MAX,
            ])
            .to_bytes(),
            &[0, 4, 8, 12, 16, 20, 24, 28, 32],
        ),
    );
    rows.insert(
        "packet:display-rotation-set-get".to_string(),
        display_rotation_fields(&[0.0, 90.0, -90.0, 180.0, 45.0, -45.0]),
    );
    rows.insert(
        "packet:display-rotation-singular".to_string(),
        vec![u8::from(
            PacketDisplayMatrix::new([0; PacketDisplayMatrix::ELEMENTS])
                .counterclockwise_rotation_degrees()
                .is_none(),
        )
        .to_string()],
    );
    rows.insert(
        "packet:display-rotation-get-affine".to_string(),
        display_rotation_get_affine_fields(),
    );
    rows.insert("packet:display-flip".to_string(), display_flip_fields());
    rows.insert(
        "packet:payload-layout-stereo3d".to_string(),
        payload_layout_fields(
            &PacketStereo3d::new(
                PacketStereo3dType::SideBySide,
                PacketStereo3dFlags::INVERT,
                PacketStereo3dView::Right,
                PacketStereo3dPrimaryEye::Left,
                0x0102_0304,
                Rational::from_raw(1, 2),
                Rational::from_raw(75, 1),
            )
            .unwrap()
            .to_bytes(),
            &[0, 4, 8, 12, 16, 20, 28],
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
    rows.insert(
        "packet:payload-layout-dynamic-hdr10-plus".to_string(),
        payload_layout_fields(
            &dynamic_hdr10_plus_payload_layout_bytes(),
            &dynamic_hdr10_plus_payload_layout_offsets(),
        ),
    );
    rows.insert(
        "packet:payload-layout-encryption-info".to_string(),
        payload_layout_fields(&encryption_info_payload_layout_bytes(), &[]),
    );
    rows.insert(
        "packet:payload-layout-encryption-init-info".to_string(),
        payload_layout_fields(&encryption_init_info_payload_layout_bytes(), &[]),
    );
    rows.insert(
        "packet:payload-layout-iamf-mix-gain-param".to_string(),
        payload_layout_fields(
            &iamf_mix_gain_param_payload_layout_bytes(),
            &iamf_mix_gain_param_payload_layout_offsets(),
        ),
    );
    rows.insert(
        "packet:payload-layout-iamf-demixing-info-param".to_string(),
        payload_layout_fields(
            &iamf_demixing_info_param_payload_layout_bytes(),
            &iamf_demixing_info_param_payload_layout_offsets(),
        ),
    );
    rows.insert(
        "packet:payload-layout-iamf-recon-gain-info-param".to_string(),
        payload_layout_fields(
            &iamf_recon_gain_info_param_payload_layout_bytes(),
            &iamf_recon_gain_info_param_payload_layout_offsets(),
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

fn dynamic_hdr10_plus_payload_layout_bytes() -> Vec<u8> {
    let mut data = vec![0; PacketDynamicHdr10Plus::DATA_LEN];
    data[0] = PacketDynamicHdr10Plus::ITU_T_T35_COUNTRY_CODE;
    data[1] = PacketDynamicHdr10Plus::APPLICATION_VERSION;
    data[2] = 1;

    let targeted_max_offset = dynamic_hdr10_plus_targeted_max_offset();
    write_ne_i32(&mut data, targeted_max_offset, 1000);
    write_ne_i32(&mut data, targeted_max_offset + 4, 1);
    data[targeted_max_offset + 8] = 1;
    data[targeted_max_offset + 9] = 2;
    data[targeted_max_offset + 10] = 2;

    let targeted_table_offset = targeted_max_offset + 12;
    write_ne_i32(&mut data, targeted_table_offset, 1);
    write_ne_i32(&mut data, targeted_table_offset + 4, 15);

    let mastering_flag_offset = targeted_table_offset + dynamic_hdr10_plus_peak_table_len();
    data[mastering_flag_offset] = 1;
    data[mastering_flag_offset + 1] = 2;
    data[mastering_flag_offset + 2] = 2;

    let mastering_table_offset = mastering_flag_offset + 4;
    let second_mastering_entry =
        mastering_table_offset + (PacketDynamicHdr10Plus::MAX_PEAK_LUMINANCE_COLS + 1) * 8;
    write_ne_i32(&mut data, second_mastering_entry, 2);
    write_ne_i32(&mut data, second_mastering_entry + 4, 15);

    data
}

fn dynamic_hdr10_plus_payload_layout_offsets() -> Vec<usize> {
    let params_offset = dynamic_hdr10_plus_params_offset();
    let targeted_max_offset = dynamic_hdr10_plus_targeted_max_offset();
    let targeted_table_offset = targeted_max_offset + 12;
    let mastering_flag_offset = targeted_table_offset + dynamic_hdr10_plus_peak_table_len();
    let mastering_table_offset = mastering_flag_offset + 4;

    vec![
        0,
        1,
        2,
        params_offset,
        params_offset + PacketHdrPlusColorTransformParams::DATA_LEN,
        targeted_max_offset,
        targeted_max_offset + 8,
        targeted_max_offset + 9,
        targeted_max_offset + 10,
        targeted_table_offset,
        mastering_flag_offset,
        mastering_flag_offset + 1,
        mastering_flag_offset + 2,
        mastering_table_offset,
    ]
}

fn dynamic_hdr10_plus_params_offset() -> usize {
    4
}

fn dynamic_hdr10_plus_targeted_max_offset() -> usize {
    dynamic_hdr10_plus_params_offset()
        + PacketDynamicHdr10Plus::MAX_WINDOWS * PacketHdrPlusColorTransformParams::DATA_LEN
}

fn dynamic_hdr10_plus_peak_table_len() -> usize {
    PacketDynamicHdr10Plus::MAX_PEAK_LUMINANCE_ROWS
        * PacketDynamicHdr10Plus::MAX_PEAK_LUMINANCE_COLS
        * 8
}

fn write_ne_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_usize(data: &mut [u8], offset: usize, value: usize) {
    data[offset..offset + core::mem::size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_rational(data: &mut [u8], offset: usize, value: Rational) {
    write_ne_i32(data, offset, value.num());
    write_ne_i32(data, offset + 4, value.den());
}

fn encryption_info_payload_layout_bytes() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&u32::from_be_bytes(*b"cenc").to_be_bytes());
    data.extend_from_slice(&1_u32.to_be_bytes());
    data.extend_from_slice(&9_u32.to_be_bytes());
    data.extend_from_slice(&16_u32.to_be_bytes());
    data.extend_from_slice(&8_u32.to_be_bytes());
    data.extend_from_slice(&2_u32.to_be_bytes());
    data.extend_from_slice(&[
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f,
    ]);
    data.extend_from_slice(&[0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7]);
    data.extend_from_slice(&PacketEncryptionSubsample::new(3, 100).to_bytes());
    data.extend_from_slice(&PacketEncryptionSubsample::new(0, 55).to_bytes());
    data
}

fn encryption_init_info_payload_layout_bytes() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&2_u32.to_be_bytes());

    for value in [4_u32, 2, 3, 5] {
        data.extend_from_slice(&value.to_be_bytes());
    }
    data.extend_from_slice(b"sys1");
    data.extend_from_slice(b"abc");
    data.extend_from_slice(b"def");
    data.extend_from_slice(b"hello");

    for value in [0_u32, 0, 16, 3] {
        data.extend_from_slice(&value.to_be_bytes());
    }
    data.extend_from_slice(b"pss");
    data
}

fn iamf_param_definition_payload_layout_bytes(
    definition_type: PacketIamfParamDefinitionType,
    subblock_size: usize,
    subblocks: &[Vec<u8>],
) -> Vec<u8> {
    let subblocks_offset = PacketIamfParamDefinition::HEADER_LEN;
    let mut data = vec![0; subblocks_offset + subblock_size * subblocks.len()];
    write_ne_usize(
        &mut data,
        PacketIamfParamDefinition::SUBBLOCKS_OFFSET_OFFSET,
        subblocks_offset,
    );
    write_ne_usize(
        &mut data,
        PacketIamfParamDefinition::SUBBLOCK_SIZE_OFFSET,
        subblock_size,
    );
    write_ne_u32(
        &mut data,
        PacketIamfParamDefinition::SUBBLOCK_COUNT_OFFSET,
        subblocks.len() as u32,
    );
    write_ne_u32(
        &mut data,
        PacketIamfParamDefinition::TYPE_OFFSET,
        definition_type.as_raw(),
    );
    write_ne_u32(&mut data, PacketIamfParamDefinition::PARAMETER_ID_OFFSET, 7);
    write_ne_u32(
        &mut data,
        PacketIamfParamDefinition::PARAMETER_RATE_OFFSET,
        48_000,
    );
    write_ne_u32(&mut data, PacketIamfParamDefinition::DURATION_OFFSET, 960);
    write_ne_u32(
        &mut data,
        PacketIamfParamDefinition::CONSTANT_SUBBLOCK_DURATION_OFFSET,
        480,
    );

    for (index, subblock) in subblocks.iter().enumerate() {
        assert_eq!(subblock.len(), subblock_size);
        let offset = subblocks_offset + index * subblock_size;
        data[offset..offset + subblock_size].copy_from_slice(subblock);
    }

    data
}

fn iamf_mix_gain_subblock_payload_layout_bytes(
    duration: u32,
    animation_type: PacketIamfAnimationType,
) -> Vec<u8> {
    let mut data = vec![0; PacketIamfMixGainSubblock::MIN_DATA_LEN];
    write_ne_u32(
        &mut data,
        PacketIamfMixGainSubblock::SUBBLOCK_DURATION_OFFSET,
        duration,
    );
    write_ne_u32(
        &mut data,
        PacketIamfMixGainSubblock::ANIMATION_TYPE_OFFSET,
        animation_type.as_raw(),
    );
    write_ne_rational(
        &mut data,
        PacketIamfMixGainSubblock::START_POINT_VALUE_OFFSET,
        Rational::from_raw(-1, 2),
    );
    write_ne_rational(
        &mut data,
        PacketIamfMixGainSubblock::END_POINT_VALUE_OFFSET,
        Rational::from_raw(3, 4),
    );
    write_ne_rational(
        &mut data,
        PacketIamfMixGainSubblock::CONTROL_POINT_VALUE_OFFSET,
        Rational::from_raw(1, 3),
    );
    write_ne_rational(
        &mut data,
        PacketIamfMixGainSubblock::CONTROL_POINT_RELATIVE_TIME_OFFSET,
        Rational::from_raw(1, 2),
    );
    data
}

fn iamf_mix_gain_param_payload_layout_bytes() -> Vec<u8> {
    let subblocks = [
        iamf_mix_gain_subblock_payload_layout_bytes(480, PacketIamfAnimationType::Linear),
        iamf_mix_gain_subblock_payload_layout_bytes(480, PacketIamfAnimationType::Bezier),
    ];
    iamf_param_definition_payload_layout_bytes(
        PacketIamfParamDefinitionType::MixGain,
        PacketIamfMixGainSubblock::MIN_DATA_LEN,
        &subblocks,
    )
}

fn iamf_mix_gain_param_payload_layout_offsets() -> Vec<usize> {
    vec![
        PacketIamfParamDefinition::AV_CLASS_OFFSET,
        PacketIamfParamDefinition::SUBBLOCKS_OFFSET_OFFSET,
        PacketIamfParamDefinition::SUBBLOCK_SIZE_OFFSET,
        PacketIamfParamDefinition::SUBBLOCK_COUNT_OFFSET,
        PacketIamfParamDefinition::TYPE_OFFSET,
        PacketIamfParamDefinition::PARAMETER_ID_OFFSET,
        PacketIamfParamDefinition::PARAMETER_RATE_OFFSET,
        PacketIamfParamDefinition::DURATION_OFFSET,
        PacketIamfParamDefinition::CONSTANT_SUBBLOCK_DURATION_OFFSET,
        PacketIamfParamDefinition::HEADER_LEN,
        PacketIamfMixGainSubblock::MIN_DATA_LEN,
        PacketIamfMixGainSubblock::AV_CLASS_OFFSET,
        PacketIamfMixGainSubblock::SUBBLOCK_DURATION_OFFSET,
        PacketIamfMixGainSubblock::ANIMATION_TYPE_OFFSET,
        PacketIamfMixGainSubblock::START_POINT_VALUE_OFFSET,
        PacketIamfMixGainSubblock::END_POINT_VALUE_OFFSET,
        PacketIamfMixGainSubblock::CONTROL_POINT_VALUE_OFFSET,
        PacketIamfMixGainSubblock::CONTROL_POINT_RELATIVE_TIME_OFFSET,
    ]
}

fn iamf_demixing_info_subblock_payload_layout_bytes(duration: u32, dmixp_mode: u32) -> Vec<u8> {
    let mut data = vec![0; PacketIamfDemixingInfoSubblock::MIN_DATA_LEN];
    write_ne_u32(
        &mut data,
        PacketIamfDemixingInfoSubblock::SUBBLOCK_DURATION_OFFSET,
        duration,
    );
    write_ne_u32(
        &mut data,
        PacketIamfDemixingInfoSubblock::DMIXP_MODE_OFFSET,
        dmixp_mode,
    );
    data
}

fn iamf_demixing_info_param_payload_layout_bytes() -> Vec<u8> {
    iamf_param_definition_payload_layout_bytes(
        PacketIamfParamDefinitionType::Demixing,
        PacketIamfDemixingInfoSubblock::MIN_DATA_LEN,
        &[iamf_demixing_info_subblock_payload_layout_bytes(960, 7)],
    )
}

fn iamf_demixing_info_param_payload_layout_offsets() -> Vec<usize> {
    vec![
        PacketIamfParamDefinition::AV_CLASS_OFFSET,
        PacketIamfParamDefinition::SUBBLOCKS_OFFSET_OFFSET,
        PacketIamfParamDefinition::SUBBLOCK_SIZE_OFFSET,
        PacketIamfParamDefinition::SUBBLOCK_COUNT_OFFSET,
        PacketIamfParamDefinition::TYPE_OFFSET,
        PacketIamfParamDefinition::PARAMETER_ID_OFFSET,
        PacketIamfParamDefinition::PARAMETER_RATE_OFFSET,
        PacketIamfParamDefinition::DURATION_OFFSET,
        PacketIamfParamDefinition::CONSTANT_SUBBLOCK_DURATION_OFFSET,
        PacketIamfParamDefinition::HEADER_LEN,
        PacketIamfDemixingInfoSubblock::MIN_DATA_LEN,
        PacketIamfDemixingInfoSubblock::AV_CLASS_OFFSET,
        PacketIamfDemixingInfoSubblock::SUBBLOCK_DURATION_OFFSET,
        PacketIamfDemixingInfoSubblock::DMIXP_MODE_OFFSET,
    ]
}

fn iamf_recon_gain_subblock_payload_layout_bytes(duration: u32) -> Vec<u8> {
    let mut data = vec![0; PacketIamfReconGainSubblock::MIN_DATA_LEN];
    write_ne_u32(
        &mut data,
        PacketIamfReconGainSubblock::SUBBLOCK_DURATION_OFFSET,
        duration,
    );
    for layer in 0..PacketIamfReconGainSubblock::LAYERS {
        for channel in 0..PacketIamfReconGainSubblock::CHANNELS {
            data[PacketIamfReconGainSubblock::RECON_GAIN_OFFSET
                + layer * PacketIamfReconGainSubblock::CHANNELS
                + channel] = (layer * 16 + channel) as u8;
        }
    }
    data
}

fn iamf_recon_gain_info_param_payload_layout_bytes() -> Vec<u8> {
    iamf_param_definition_payload_layout_bytes(
        PacketIamfParamDefinitionType::ReconGain,
        PacketIamfReconGainSubblock::MIN_DATA_LEN,
        &[iamf_recon_gain_subblock_payload_layout_bytes(960)],
    )
}

fn iamf_recon_gain_info_param_payload_layout_offsets() -> Vec<usize> {
    vec![
        PacketIamfParamDefinition::AV_CLASS_OFFSET,
        PacketIamfParamDefinition::SUBBLOCKS_OFFSET_OFFSET,
        PacketIamfParamDefinition::SUBBLOCK_SIZE_OFFSET,
        PacketIamfParamDefinition::SUBBLOCK_COUNT_OFFSET,
        PacketIamfParamDefinition::TYPE_OFFSET,
        PacketIamfParamDefinition::PARAMETER_ID_OFFSET,
        PacketIamfParamDefinition::PARAMETER_RATE_OFFSET,
        PacketIamfParamDefinition::DURATION_OFFSET,
        PacketIamfParamDefinition::CONSTANT_SUBBLOCK_DURATION_OFFSET,
        PacketIamfParamDefinition::HEADER_LEN,
        PacketIamfReconGainSubblock::MIN_DATA_LEN,
        PacketIamfReconGainSubblock::AV_CLASS_OFFSET,
        PacketIamfReconGainSubblock::SUBBLOCK_DURATION_OFFSET,
        PacketIamfReconGainSubblock::RECON_GAIN_OFFSET,
    ]
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

    let new_zero = Packet::new_zeroed(0, 0).unwrap();
    rows.insert(
        "packet:payload-new-zero-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-new-zero".to_string(),
        payload_allocation_fields(&new_zero),
    );

    let mut new_packet_reset = packet_with_common_props();
    new_packet_reset.alloc_new_packet_payload(3).unwrap();
    new_packet_reset
        .make_data_writable()
        .copy_from_slice(&[0x10, 0x20, 0x30]);
    rows.insert(
        "packet:payload-new-packet-reset-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-new-packet-reset".to_string(),
        packet_fields(&new_packet_reset),
    );
    rows.insert(
        "packet:payload-new-packet-reset-payload".to_string(),
        payload_fields(&new_packet_reset),
    );

    let mut new_packet_invalid = packet_with_common_props();
    let invalid_ret = new_packet_invalid
        .alloc_new_packet_payload(AV_PACKET_MAX_PAYLOAD_SIZE + 1)
        .unwrap_err();
    rows.insert(
        "packet:payload-new-packet-invalid-ret".to_string(),
        vec![invalid_ret.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:payload-new-packet-invalid-preserve".to_string(),
        packet_fields(&new_packet_invalid),
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

    let from_zero_data = Packet::from_data(Vec::new()).unwrap();
    rows.insert(
        "packet:payload-from-data-zero-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-from-data-zero".to_string(),
        payload_fields(&from_zero_data),
    );

    let mut from_data_preserve = packet_with_common_props_no_payload();
    from_data_preserve
        .replace_data_from_vec(vec![0x10, 0x20, 0x30])
        .unwrap();
    rows.insert(
        "packet:payload-from-data-preserve-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-from-data-preserve".to_string(),
        packet_fields(&from_data_preserve),
    );
    rows.insert(
        "packet:payload-from-data-preserve-payload".to_string(),
        payload_fields(&from_data_preserve),
    );

    let from_data_invalid_ret =
        Packet::validate_payload_len(AV_PACKET_MAX_PAYLOAD_SIZE + 1).unwrap_err();
    let from_data_invalid = packet_with_common_props_no_payload();
    rows.insert(
        "packet:payload-from-data-invalid-ret".to_string(),
        vec![from_data_invalid_ret.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:payload-from-data-invalid-preserve".to_string(),
        packet_fields(&from_data_invalid),
    );

    let raw_ref_src = Packet::new(vec![0xaa, 0xbb], 0);
    let mut raw_ref_dst = Packet::default();
    raw_ref_dst.ref_from(&raw_ref_src);
    rows.insert(
        "packet:payload-ref-unrefcounted-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-ref-unrefcounted-src".to_string(),
        payload_visible_fields(&raw_ref_src),
    );
    rows.insert(
        "packet:payload-ref-unrefcounted-dst".to_string(),
        payload_fields(&raw_ref_dst),
    );
    rows.insert(
        "packet:payload-clone-unrefcounted".to_string(),
        payload_fields(&raw_ref_src.clone()),
    );

    let mut grow = Packet::new_zeroed(2, 0).unwrap();
    grow.make_data_writable().copy_from_slice(&[0xaa, 0xbb]);
    grow.grow_data(3).unwrap();
    rows.insert("packet:payload-grow-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "packet:payload-grow".to_string(),
        payload_prefix_fields(&grow, 2),
    );

    let mut grow_invalid = packet_with_common_props_no_payload();
    grow_invalid
        .replace_data_from_vec(vec![0x44, 0x55])
        .unwrap();
    let invalid_grow_by =
        i32::MAX as usize - (grow_invalid.len() + AV_INPUT_BUFFER_PADDING_SIZE) + 1;
    let grow_invalid_ret = grow_invalid.grow_data(invalid_grow_by).unwrap_err();
    rows.insert(
        "packet:payload-grow-invalid-ret".to_string(),
        vec![grow_invalid_ret.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:payload-grow-invalid-preserve".to_string(),
        packet_fields(&grow_invalid),
    );
    rows.insert(
        "packet:payload-grow-invalid-payload".to_string(),
        payload_fields(&grow_invalid),
    );

    grow.shrink_data(2).unwrap();
    rows.insert("packet:payload-shrink".to_string(), payload_fields(&grow));

    let mut grow_empty = Packet::default();
    grow_empty.grow_data(3).unwrap();
    rows.insert(
        "packet:payload-grow-empty-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-grow-empty".to_string(),
        payload_prefix_fields(&grow_empty, 0),
    );

    let mut shrink_edges = Packet::from_data(vec![0xaa, 0xbb, 0xcc]).unwrap();
    shrink_edges.shrink_data(9).unwrap();
    rows.insert(
        "packet:payload-shrink-oversize".to_string(),
        payload_fields(&shrink_edges),
    );
    shrink_edges.shrink_data(0).unwrap();
    rows.insert(
        "packet:payload-shrink-zero".to_string(),
        payload_fields(&shrink_edges),
    );

    let shared_grow_src = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
    let mut shared_grow_dst = Packet::default();
    shared_grow_dst.ref_from(&shared_grow_src);
    let shared_grow_dst_ptr = shared_grow_dst.data_buffer().as_padded_ptr();
    shared_grow_dst.grow_data(2).unwrap();
    rows.insert(
        "packet:payload-grow-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-grow-shared-same-ptr".to_string(),
        vec![
            u8::from(shared_grow_dst.data_buffer().as_padded_ptr() == shared_grow_dst_ptr)
                .to_string(),
        ],
    );
    rows.insert(
        "packet:payload-grow-shared-src".to_string(),
        payload_fields(&shared_grow_src),
    );
    rows.insert(
        "packet:payload-grow-shared-dst".to_string(),
        payload_prefix_fields(&shared_grow_dst, 2),
    );

    let mut grow_unrefcounted = Packet::new(vec![0xaa, 0xbb], 0);
    grow_unrefcounted.grow_data(2).unwrap();
    rows.insert(
        "packet:payload-grow-unrefcounted-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-grow-unrefcounted".to_string(),
        payload_prefix_fields(&grow_unrefcounted, 2),
    );

    let mut shrink_unrefcounted = Packet::new(vec![0xaa, 0xbb, 0xcc, 0xdd], 0);
    shrink_unrefcounted.shrink_data(2).unwrap();
    rows.insert(
        "packet:payload-shrink-unrefcounted".to_string(),
        payload_unowned_fields(&shrink_unrefcounted),
    );

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

    let mut unrefcounted_writable = Packet::new(vec![0xaa, 0xbb], 0);
    unrefcounted_writable.make_writable().unwrap();
    unrefcounted_writable.make_data_writable()[0] = 0xcc;
    rows.insert(
        "packet:payload-make-writable-unrefcounted-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-writable-unrefcounted".to_string(),
        payload_fields(&unrefcounted_writable),
    );

    let mut unique_writable = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
    let unique_writable_ptr = unique_writable.data_buffer().as_padded_ptr();
    unique_writable.make_writable().unwrap();
    rows.insert(
        "packet:payload-make-writable-unique-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-writable-unique-same-ptr".to_string(),
        vec![
            u8::from(unique_writable.data_buffer().as_padded_ptr() == unique_writable_ptr)
                .to_string(),
        ],
    );
    rows.insert(
        "packet:payload-make-writable-unique".to_string(),
        payload_fields(&unique_writable),
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

    let mut unique_refcounted = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
    let unique_refcounted_ptr = unique_refcounted.data_buffer().as_padded_ptr();
    unique_refcounted.make_refcounted().unwrap();
    rows.insert(
        "packet:payload-make-refcounted-unique-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted-unique-same-ptr".to_string(),
        vec![
            u8::from(unique_refcounted.data_buffer().as_padded_ptr() == unique_refcounted_ptr)
                .to_string(),
        ],
    );
    rows.insert(
        "packet:payload-make-refcounted-unique".to_string(),
        payload_fields(&unique_refcounted),
    );

    let mut readonly_bytes = vec![0xaa, 0xbb];
    readonly_bytes.resize(2 + AV_INPUT_BUFFER_PADDING_SIZE, 0);
    let readonly_payload = BufferRef::from_vec_with_len_readonly(readonly_bytes, 2).unwrap();
    let mut readonly_refcounted = Packet::with_buffer(readonly_payload, 0);
    let readonly_refcounted_ptr = readonly_refcounted.data_buffer().as_padded_ptr();
    readonly_refcounted.make_refcounted().unwrap();
    rows.insert(
        "packet:payload-make-refcounted-readonly-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted-readonly-same-ptr".to_string(),
        vec![u8::from(
            readonly_refcounted.data_buffer().as_padded_ptr() == readonly_refcounted_ptr,
        )
        .to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted-readonly".to_string(),
        payload_fields(&readonly_refcounted),
    );
    readonly_refcounted.make_writable().unwrap();
    rows.insert(
        "packet:payload-make-writable-readonly-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-writable-readonly-same-ptr".to_string(),
        vec![u8::from(
            readonly_refcounted.data_buffer().as_padded_ptr() == readonly_refcounted_ptr,
        )
        .to_string()],
    );
    rows.insert(
        "packet:payload-make-writable-readonly".to_string(),
        payload_fields(&readonly_refcounted),
    );

    let shared_refcounted_src = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
    let mut shared_refcounted_dst = Packet::default();
    shared_refcounted_dst.ref_from(&shared_refcounted_src);
    let shared_refcounted_dst_ptr = shared_refcounted_dst.data_buffer().as_padded_ptr();
    shared_refcounted_dst.make_refcounted().unwrap();
    rows.insert(
        "packet:payload-make-refcounted-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted-shared-same-ptr".to_string(),
        vec![u8::from(
            shared_refcounted_dst.data_buffer().as_padded_ptr() == shared_refcounted_dst_ptr,
        )
        .to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted-shared-src".to_string(),
        payload_fields(&shared_refcounted_src),
    );
    rows.insert(
        "packet:payload-make-refcounted-shared-dst".to_string(),
        payload_fields(&shared_refcounted_dst),
    );

    let mut empty_refcounted = Packet::default();
    empty_refcounted.make_refcounted().unwrap();
    rows.insert(
        "packet:payload-make-refcounted-empty-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-refcounted-empty".to_string(),
        payload_fields(&empty_refcounted),
    );

    let mut empty_writable = Packet::default();
    empty_writable.make_writable().unwrap();
    rows.insert(
        "packet:payload-make-writable-empty-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:payload-make-writable-empty".to_string(),
        payload_fields(&empty_writable),
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

    let mut duplicate_dict = Dictionary::new();
    duplicate_dict
        .set_with_mode(
            "title",
            "first",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    duplicate_dict
        .set_with_mode(
            "TITLE",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    duplicate_dict
        .set_with_mode(
            "artist",
            "Name",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    let duplicate_packed = packet_pack_dictionary(&duplicate_dict);
    rows.insert(
        "packet:dict-pack-multikey".to_string(),
        dictionary_payload_fields(&duplicate_packed),
    );

    let duplicate_unpacked = packet_unpack_dictionary(&duplicate_packed).unwrap();
    rows.insert(
        "packet:dict-unpack-multikey-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:dict-unpack-multikey".to_string(),
        dictionary_fields(&duplicate_unpacked),
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

    for (name, data) in [
        ("packet:dict-unpack-empty-ret", b"".as_slice()),
        (
            "packet:dict-unpack-missing-final-nul-ret",
            b"title\0Clip".as_slice(),
        ),
        (
            "packet:dict-unpack-key-without-value-ret",
            b"title\0".as_slice(),
        ),
        ("packet:dict-unpack-empty-key-ret", b"\0Clip\0".as_slice()),
        (
            "packet:dict-unpack-trailing-empty-key-ret",
            b"title\0Clip\0\0".as_slice(),
        ),
    ] {
        rows.insert(name.to_string(), dictionary_unpack_ret_fields(data));
    }
}

fn insert_packet_fifo_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut fifo = PacketFifo::new();
    rows.insert(
        "packet:fifo-new-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let mut move_src = packet_with_common_props();
    fifo.write_move(&mut move_src).unwrap();
    rows.insert(
        "packet:fifo-write-move-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-write-move-src".to_string(),
        packet_fields(&move_src),
    );
    rows.insert(
        "packet:fifo-after-write-move-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    rows.insert("packet:fifo-peek0-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "packet:fifo-peek0".to_string(),
        packet_fields(fifo.peek(0).unwrap()),
    );
    let err = fifo.peek(1).unwrap_err();
    rows.insert(
        "packet:fifo-peek1-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );

    let mut move_dst = Packet::new(vec![0x44], 9);
    fifo.read_move(&mut move_dst).unwrap();
    rows.insert(
        "packet:fifo-read-move-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-read-move-dst".to_string(),
        packet_fields(&move_dst),
    );
    rows.insert(
        "packet:fifo-after-read-move-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let ref_src = packet_with_common_props();
    fifo.write_ref(&ref_src).unwrap();
    rows.insert(
        "packet:fifo-write-ref-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-write-ref-src".to_string(),
        packet_fields(&ref_src),
    );
    rows.insert(
        "packet:fifo-after-write-ref-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let mut ref_dst = Packet::default();
    fifo.read_ref(&mut ref_dst).unwrap();
    rows.insert(
        "packet:fifo-read-ref-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-read-ref-dst".to_string(),
        packet_fields(&ref_dst),
    );
    rows.insert(
        "packet:fifo-after-read-ref-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let ref_replace_src = packet_with_common_props();
    fifo.write_ref(&ref_replace_src).unwrap();
    rows.insert(
        "packet:fifo-write-ref-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-write-ref-replace-src".to_string(),
        packet_fields(&ref_replace_src),
    );
    rows.insert(
        "packet:fifo-after-write-ref-replace-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let mut ref_replace_dst = Packet::new(vec![0x55, 0x44], 77);
    ref_replace_dst.set_pts(Some(22));
    ref_replace_dst.set_duration(2).unwrap();
    ref_replace_dst.push_side_data(SideData::new("palette", vec![0xee]).unwrap());
    ref_replace_dst.set_opaque(Some(PacketOpaque::new(0x5678).unwrap()));
    ref_replace_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0x99])));
    fifo.read_ref(&mut ref_replace_dst).unwrap();
    rows.insert(
        "packet:fifo-read-ref-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-read-ref-replace-dst".to_string(),
        packet_fields(&ref_replace_dst),
    );
    rows.insert(
        "packet:fifo-after-read-ref-replace-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let mut move_replace_src = packet_with_common_props();
    fifo.write_move(&mut move_replace_src).unwrap();
    rows.insert(
        "packet:fifo-write-move-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-write-move-replace-src".to_string(),
        packet_fields(&move_replace_src),
    );
    rows.insert(
        "packet:fifo-after-write-move-replace-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let mut move_replace_dst = Packet::new(vec![0x66, 0x77], 88);
    move_replace_dst.set_pts(Some(33));
    move_replace_dst.set_duration(3).unwrap();
    move_replace_dst.push_side_data(SideData::new("palette", vec![0xab]).unwrap());
    move_replace_dst.set_opaque(Some(PacketOpaque::new(0x6789).unwrap()));
    move_replace_dst.set_opaque_ref(Some(BufferRef::from_vec(vec![0xcd])));
    fifo.read_move(&mut move_replace_dst).unwrap();
    rows.insert(
        "packet:fifo-read-move-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:fifo-read-move-replace-dst".to_string(),
        packet_fields(&move_replace_dst),
    );
    rows.insert(
        "packet:fifo-after-read-move-replace-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let mut first = Packet::new(vec![1], 1);
    let mut second = Packet::new(vec![2], 2);
    fifo.write_move(&mut first).unwrap();
    fifo.write_move(&mut second).unwrap();
    rows.insert(
        "packet:fifo-before-drain-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    fifo.drain(0).unwrap();
    rows.insert(
        "packet:fifo-after-drain-zero-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    rows.insert(
        "packet:fifo-after-drain-zero-peek".to_string(),
        packet_fields(fifo.peek(0).unwrap()),
    );
    fifo.drain(1).unwrap();
    rows.insert(
        "packet:fifo-after-drain-one-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    rows.insert(
        "packet:fifo-after-drain-one-peek".to_string(),
        packet_fields(fifo.peek(0).unwrap()),
    );
    fifo.drain(1).unwrap();
    rows.insert(
        "packet:fifo-after-drain-all-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let err = fifo.peek(0).unwrap_err();
    rows.insert(
        "packet:fifo-peek-empty-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );
    let mut empty_dst = Packet::default();
    let err = fifo.read_move(&mut empty_dst).unwrap_err();
    rows.insert(
        "packet:fifo-read-empty-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:fifo-read-empty-dst".to_string(),
        packet_fields(&empty_dst),
    );
    let mut empty_move_preserve_dst = packet_with_common_props();
    let err = fifo.read_move(&mut empty_move_preserve_dst).unwrap_err();
    rows.insert(
        "packet:fifo-read-empty-move-preserve-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:fifo-read-empty-move-preserve-dst".to_string(),
        packet_fields(&empty_move_preserve_dst),
    );
    let mut empty_ref_preserve_dst = packet_with_common_props();
    let err = fifo.read_ref(&mut empty_ref_preserve_dst).unwrap_err();
    rows.insert(
        "packet:fifo-read-empty-ref-preserve-ret".to_string(),
        vec![err.code().unwrap().raw().to_string()],
    );
    rows.insert(
        "packet:fifo-read-empty-ref-preserve-dst".to_string(),
        packet_fields(&empty_ref_preserve_dst),
    );
}

fn insert_side_data_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut packet = Packet::default();
    packet
        .new_side_data(PacketSideDataKind::NewExtradata, 4)
        .unwrap()
        .data_mut()
        .copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
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
        "packet:side-get-shrunk-size-null".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind("new_extradata")),
    );
    rows.insert(
        "packet:side-get-missing".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind("palette")),
    );
    rows.insert(
        "packet:side-get-missing-size-null".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind("palette")),
    );
    let missing_shrink = packet
        .shrink_side_data_by_kind_id(&PacketSideDataKind::Palette, 1)
        .unwrap_err();
    rows.insert(
        "packet:side-shrink-missing-ret".to_string(),
        vec![missing_shrink
            .code()
            .expect("missing side data shrink should preserve an FFmpeg error code")
            .raw()
            .to_string()],
    );
    let oversize_shrink = packet
        .shrink_side_data_by_kind_id(&PacketSideDataKind::NewExtradata, 3)
        .unwrap_err();
    rows.insert(
        "packet:side-shrink-oversize-ret".to_string(),
        vec![oversize_shrink
            .code()
            .expect("oversize side data shrink should preserve an FFmpeg error code")
            .raw()
            .to_string()],
    );
    rows.insert(
        "packet:side-shrink-oversize".to_string(),
        side_data_summary_fields(&packet),
    );

    packet.clear_side_data();
    rows.insert(
        "packet:side-free".to_string(),
        side_data_summary_fields(&packet),
    );

    let mut packet = Packet::default();
    packet
        .new_side_data(PacketSideDataKind::NewExtradata, 2)
        .unwrap()
        .data_mut()
        .copy_from_slice(&[0x11, 0x22]);
    packet
        .new_side_data(PacketSideDataKind::NewExtradata, 3)
        .unwrap()
        .data_mut()
        .copy_from_slice(&[0xaa, 0xbb, 0xcc]);
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

    let mut duplicate_packet = Packet::default();
    duplicate_packet
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x11]).unwrap());
    duplicate_packet.push_side_data(
        SideData::new_with_kind(PacketSideDataKind::NewExtradata, vec![0x22]).unwrap(),
    );
    duplicate_packet
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x33]).unwrap());
    duplicate_packet.push_side_data(
        SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x44]).unwrap(),
    );
    rows.insert(
        "packet:side-duplicate-before".to_string(),
        side_data_summary_fields(&duplicate_packet),
    );
    rows.insert(
        "packet:side-get-duplicate-palette".to_string(),
        side_data_lookup_fields(
            duplicate_packet.side_data_by_kind_id(&PacketSideDataKind::Palette),
        ),
    );
    let new_duplicate = duplicate_packet
        .new_side_data(PacketSideDataKind::Palette, 2)
        .expect("first duplicate packet side data should be replaced by new_side_data");
    assert_eq!(new_duplicate.data(), &[0, 0]);
    new_duplicate.data_mut().copy_from_slice(&[0x66, 0x77]);
    rows.insert(
        "packet:side-new-duplicate-replace-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:side-new-duplicate-replace".to_string(),
        side_data_summary_fields(&duplicate_packet),
    );
    rows.insert(
        "packet:side-get-duplicate-palette-new".to_string(),
        side_data_lookup_fields(
            duplicate_packet.side_data_by_kind_id(&PacketSideDataKind::Palette),
        ),
    );
    let replaced = duplicate_packet
        .try_add_side_data(
            SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x55]).unwrap(),
        )
        .expect("duplicate packet side-data replacement should not hit capacity")
        .expect("first palette packet side data should be replaced");
    assert_eq!(replaced.data(), &[0x66, 0x77]);
    rows.insert(
        "packet:side-add-duplicate-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:side-add-duplicate-replace".to_string(),
        side_data_summary_fields(&duplicate_packet),
    );
    duplicate_packet
        .shrink_side_data_by_kind_id(&PacketSideDataKind::Palette, 0)
        .expect("first palette packet side data should shrink");
    rows.insert(
        "packet:side-shrink-duplicate-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:side-shrink-duplicate".to_string(),
        side_data_summary_fields(&duplicate_packet),
    );
    rows.insert(
        "packet:side-get-duplicate-palette-shrunk".to_string(),
        side_data_lookup_fields(
            duplicate_packet.side_data_by_kind_id(&PacketSideDataKind::Palette),
        ),
    );
    duplicate_packet.clear_side_data();
    rows.insert(
        "packet:side-free-duplicate".to_string(),
        side_data_summary_fields(&duplicate_packet),
    );
    rows.insert(
        "packet:side-get-duplicate-palette-free".to_string(),
        side_data_lookup_fields(
            duplicate_packet.side_data_by_kind_id(&PacketSideDataKind::Palette),
        ),
    );

    let mut packet = Packet::default();
    packet
        .new_side_data(PacketSideDataKind::NewExtradata, 0)
        .unwrap();
    rows.insert(
        "packet:side-new-zero".to_string(),
        side_data_summary_fields(&packet),
    );

    let mut packet = Packet::default();
    let appended = packet.add_side_data(SideData::new_extradata(Vec::new()).unwrap());
    assert!(
        appended.is_none(),
        "zero-size new_extradata should append to an empty packet"
    );
    rows.insert(
        "packet:side-add-zero-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:side-add-zero".to_string(),
        side_data_summary_fields(&packet),
    );
}

fn insert_side_data_capacity_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut packet = Packet::default();
    for (index, kind) in PacketSideDataKind::KNOWN.iter().enumerate() {
        packet
            .try_add_side_data(SideData::new_with_kind(kind.clone(), vec![index as u8]).unwrap())
            .unwrap();
    }
    rows.insert(
        "packet:side-add-capacity-count".to_string(),
        vec![packet.side_data().len().to_string()],
    );

    packet
        .try_add_side_data(
            SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xaa]).unwrap(),
        )
        .unwrap();
    rows.insert(
        "packet:side-add-capacity-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:side-add-capacity-replace-count".to_string(),
        vec![packet.side_data().len().to_string()],
    );
    rows.insert(
        "packet:side-add-capacity-replace-palette".to_string(),
        side_data_lookup_fields(packet.side_data_by_kind_id(&PacketSideDataKind::Palette)),
    );

    let mut extra_owned =
        Some(SideData::new("vendor.private.extra_packet_data", vec![0xee]).unwrap());
    let err = packet
        .try_add_side_data_owned(&mut extra_owned)
        .unwrap_err();
    rows.insert(
        "packet:side-add-capacity-overflow-ret".to_string(),
        vec![err
            .code()
            .expect("capacity error should preserve FFmpeg ERANGE code")
            .raw()
            .to_string()],
    );
    rows.insert(
        "packet:side-add-capacity-overflow-count".to_string(),
        vec![packet.side_data().len().to_string()],
    );
    rows.insert(
        "packet:side-add-capacity-overflow-owned".to_string(),
        side_data_lookup_fields(extra_owned.as_ref()),
    );

    let new_side_data_ok = packet
        .new_side_data(
            PacketSideDataKind::Unknown("vendor.private.new_packet_data".to_string()),
            1,
        )
        .is_ok();
    rows.insert(
        "packet:side-new-capacity-overflow".to_string(),
        vec![
            u8::from(new_side_data_ok).to_string(),
            packet.side_data().len().to_string(),
        ],
    );
}

fn insert_side_data_array_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mut empty_list = PacketSideDataList::new();
    rows.insert(
        "packet:array-empty-get".to_string(),
        side_data_lookup_fields(empty_list.get(&PacketSideDataKind::Palette)),
    );
    assert!(empty_list
        .remove_kind(&PacketSideDataKind::Palette)
        .is_none());
    rows.insert(
        "packet:array-empty-remove".to_string(),
        side_data_list_summary_fields(&empty_list),
    );
    empty_list.clear();
    rows.insert(
        "packet:array-empty-free".to_string(),
        side_data_list_summary_fields(&empty_list),
    );

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

    let new_flags = list
        .new_side_data_with_flags(PacketSideDataKind::SkipSamples, 2, 1)
        .unwrap();
    new_flags.data_mut().copy_from_slice(&[0xc2, 0x58]);
    rows.insert(
        "packet:array-new-flags-nonzero-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-new-flags-nonzero".to_string(),
        side_data_list_summary_fields(&list),
    );

    let mut caller_owned =
        Some(SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x5a]).unwrap());
    let add_flags_replaced = list
        .try_add_side_data_with_flags(&mut caller_owned, 1)
        .unwrap();
    assert_eq!(add_flags_replaced.unwrap().data(), &[0xc2, 0x58]);
    assert!(caller_owned.is_none());
    rows.insert(
        "packet:array-add-flags-nonzero-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-add-flags-nonzero".to_string(),
        side_data_list_summary_fields(&list),
    );
    rows.insert(
        "packet:array-add-flags-nonzero-owned".to_string(),
        side_data_lookup_fields(caller_owned.as_ref()),
    );

    let mut duplicate_list = PacketSideDataList::from_entries(vec![
        SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x11]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::NewExtradata, vec![0x22]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x33]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x44]).unwrap(),
    ]);
    rows.insert(
        "packet:array-remove-duplicate-before".to_string(),
        side_data_list_summary_fields(&duplicate_list),
    );
    rows.insert(
        "packet:array-get-duplicate-palette".to_string(),
        side_data_lookup_fields(duplicate_list.get(&PacketSideDataKind::Palette)),
    );
    let new_duplicate = duplicate_list
        .new_side_data(PacketSideDataKind::Palette, 2)
        .expect("first duplicate array side data should be replaced by new_side_data");
    assert_eq!(new_duplicate.data(), &[0, 0]);
    new_duplicate.data_mut().copy_from_slice(&[0x66, 0x77]);
    rows.insert(
        "packet:array-new-duplicate-replace-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-new-duplicate-replace".to_string(),
        side_data_list_summary_fields(&duplicate_list),
    );
    rows.insert(
        "packet:array-get-duplicate-palette-new".to_string(),
        side_data_lookup_fields(duplicate_list.get(&PacketSideDataKind::Palette)),
    );
    let replaced = duplicate_list
        .add_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x55]).unwrap())
        .expect("first palette side data should be replaced");
    assert_eq!(replaced.data(), &[0x66, 0x77]);
    rows.insert(
        "packet:array-add-duplicate-replace-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-add-duplicate-replace".to_string(),
        side_data_list_summary_fields(&duplicate_list),
    );
    let removed = duplicate_list
        .remove_kind(&PacketSideDataKind::Palette)
        .expect("last palette side data should be removed");
    assert_eq!(removed.data(), &[0x33]);
    rows.insert(
        "packet:array-remove-duplicate-last".to_string(),
        side_data_list_summary_fields(&duplicate_list),
    );

    let mut duplicate_free_list = PacketSideDataList::from_entries(vec![
        SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x11]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::NewExtradata, vec![0x22]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x33]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x44]).unwrap(),
    ]);
    rows.insert(
        "packet:array-free-duplicate-before".to_string(),
        side_data_list_summary_fields(&duplicate_free_list),
    );
    duplicate_free_list.clear();
    rows.insert(
        "packet:array-free-duplicate".to_string(),
        side_data_list_summary_fields(&duplicate_free_list),
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

    let mut list = PacketSideDataList::new();
    list.new_side_data(PacketSideDataKind::NewExtradata, 0)
        .unwrap();
    rows.insert(
        "packet:array-new-zero".to_string(),
        side_data_list_summary_fields(&list),
    );

    let mut list = PacketSideDataList::new();
    let appended = list.add_side_data(SideData::new_extradata(Vec::new()).unwrap());
    assert!(
        appended.is_none(),
        "zero-size new_extradata should append to an empty standalone list"
    );
    rows.insert(
        "packet:array-add-zero-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-add-zero".to_string(),
        side_data_list_summary_fields(&list),
    );

    let mut capacity_list = PacketSideDataList::new();
    for (index, kind) in PacketSideDataKind::KNOWN.iter().enumerate() {
        let added = capacity_list
            .try_add_side_data(SideData::new_with_kind(kind.clone(), vec![index as u8]).unwrap())
            .unwrap();
        assert!(added.is_none(), "capacity fill should append new kind");
    }
    rows.insert(
        "packet:array-add-capacity-count".to_string(),
        vec![capacity_list.len().to_string()],
    );

    let replaced = capacity_list
        .try_add_side_data(
            SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xaa]).unwrap(),
        )
        .unwrap()
        .expect("palette should be replaced at capacity");
    assert_eq!(replaced.data(), &[0]);
    rows.insert(
        "packet:array-add-capacity-replace-ret".to_string(),
        vec!["1".to_string()],
    );
    rows.insert(
        "packet:array-add-capacity-replace-count".to_string(),
        vec![capacity_list.len().to_string()],
    );
    rows.insert(
        "packet:array-add-capacity-replace-palette".to_string(),
        side_data_lookup_fields(capacity_list.get(&PacketSideDataKind::Palette)),
    );

    let extra_kind = PacketSideDataKind::Unknown("vendor.private.extra_array_data".to_string());
    let append_ok = capacity_list
        .try_add_side_data(SideData::new_with_kind(extra_kind.clone(), vec![0xee]).unwrap())
        .is_ok();
    rows.insert(
        "packet:array-add-capacity-overflow-ret".to_string(),
        vec![u8::from(append_ok).to_string()],
    );
    rows.insert(
        "packet:array-add-capacity-overflow-count".to_string(),
        vec![capacity_list.len().to_string()],
    );

    let new_ok = capacity_list.new_side_data(extra_kind, 1).is_ok();
    rows.insert(
        "packet:array-new-capacity-overflow".to_string(),
        vec![
            u8::from(new_ok).to_string(),
            capacity_list.len().to_string(),
        ],
    );
}

fn insert_frame_packet_side_data_bridge_rows(rows: &mut BTreeMap<String, Vec<String>>) {
    let mapped_pairs = [
        (
            PacketSideDataKind::ReplayGain,
            FrameSideDataKind::ReplayGain,
        ),
        (
            PacketSideDataKind::DisplayMatrix,
            FrameSideDataKind::DisplayMatrix,
        ),
        (PacketSideDataKind::Spherical, FrameSideDataKind::Spherical),
        (PacketSideDataKind::Stereo3d, FrameSideDataKind::Stereo3d),
        (
            PacketSideDataKind::AudioServiceType,
            FrameSideDataKind::AudioServiceType,
        ),
        (
            PacketSideDataKind::MasteringDisplayMetadata,
            FrameSideDataKind::MasteringDisplayMetadata,
        ),
        (
            PacketSideDataKind::ContentLightLevel,
            FrameSideDataKind::ContentLightLevel,
        ),
        (
            PacketSideDataKind::IccProfile,
            FrameSideDataKind::IccProfile,
        ),
        (
            PacketSideDataKind::AmbientViewingEnvironment,
            FrameSideDataKind::AmbientViewingEnvironment,
        ),
        (
            PacketSideDataKind::ThreeDReferenceDisplays,
            FrameSideDataKind::ThreeDReferenceDisplays,
        ),
        (PacketSideDataKind::Exif, FrameSideDataKind::Exif),
    ];

    let mut mapped_packet_list = PacketSideDataList::new();
    for (index, (_, frame_kind)) in mapped_pairs.iter().enumerate() {
        mapped_packet_list
            .add_from_frame_side_data(
                &FrameSideData::new_with_kind(frame_kind.clone(), vec![0x80 + index as u8])
                    .unwrap(),
            )
            .unwrap();
    }
    rows.insert(
        "packet:frame-to-packet-map-inventory".to_string(),
        side_data_list_summary_fields(&mapped_packet_list),
    );

    let mut mapped_frame = Frame::empty();
    for (index, (packet_kind, _)) in mapped_pairs.iter().enumerate() {
        SideData::new_with_kind(packet_kind.clone(), vec![0xa0 + index as u8])
            .unwrap()
            .add_to_frame(&mut mapped_frame, FrameSideDataFlags::EMPTY)
            .unwrap();
    }
    rows.insert(
        "packet:packet-to-frame-map-inventory".to_string(),
        frame_side_data_summary_fields(&mapped_frame),
    );

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

    let mut replace_flag_packet_list = PacketSideDataList::from_entries(vec![
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x11]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::DisplayMatrix, vec![0x22]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x33]).unwrap(),
    ]);
    let replace_flag =
        FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x55]).unwrap();
    replace_flag_packet_list
        .add_from_frame_side_data_with_flags(&replace_flag, FrameSideDataFlags::REPLACE)
        .unwrap();
    rows.insert(
        "packet:frame-to-packet-replace-flag-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:frame-to-packet-replace-flag".to_string(),
        side_data_list_summary_fields(&replace_flag_packet_list),
    );

    let mut unique_packet_list = PacketSideDataList::from_entries(vec![
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x11]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::DisplayMatrix, vec![0x22]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x33]).unwrap(),
    ]);
    let unique = FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x44]).unwrap();
    unique_packet_list
        .add_from_frame_side_data_with_flags(&unique, FrameSideDataFlags::UNIQUE)
        .unwrap();
    rows.insert(
        "packet:frame-to-packet-unique-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:frame-to-packet-unique".to_string(),
        side_data_list_summary_fields(&unique_packet_list),
    );

    let mut combined_flag_packet_list = PacketSideDataList::from_entries(vec![
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x11]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::DisplayMatrix, vec![0x22]).unwrap(),
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x33]).unwrap(),
    ]);
    let combined = FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x99]).unwrap();
    combined_flag_packet_list
        .add_from_frame_side_data_with_flags(
            &combined,
            FrameSideDataFlags::UNIQUE.union(FrameSideDataFlags::REPLACE),
        )
        .unwrap();
    rows.insert(
        "packet:frame-to-packet-unique-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:frame-to-packet-unique-replace".to_string(),
        side_data_list_summary_fields(&combined_flag_packet_list),
    );

    let mut new_ref_packet_list = PacketSideDataList::new();
    let new_ref_frame =
        FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x66, 0x77]).unwrap();
    new_ref_packet_list
        .add_from_frame_side_data_with_flags(&new_ref_frame, FrameSideDataFlags::NEW_REF)
        .unwrap();
    rows.insert(
        "packet:frame-to-packet-new-ref-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:frame-to-packet-new-ref".to_string(),
        side_data_list_summary_fields(&new_ref_packet_list),
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

    let mut replace_order_frame = Frame::empty();
    replace_order_frame
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x11]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    replace_order_frame
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0x22]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x55])
        .unwrap()
        .add_to_frame(&mut replace_order_frame, FrameSideDataFlags::REPLACE)
        .unwrap();
    rows.insert(
        "packet:packet-to-frame-replace-order-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-replace-order".to_string(),
        frame_side_data_summary_fields(&replace_order_frame),
    );

    frame
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0x22]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x44])
        .unwrap()
        .add_to_frame(&mut frame, FrameSideDataFlags::UNIQUE)
        .unwrap();
    rows.insert(
        "packet:packet-to-frame-unique-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-unique".to_string(),
        frame_side_data_summary_fields(&frame),
    );

    let mut combined_frame = Frame::empty();
    combined_frame
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x11]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    combined_frame
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0x22]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x99])
        .unwrap()
        .add_to_frame(
            &mut combined_frame,
            FrameSideDataFlags::UNIQUE.union(FrameSideDataFlags::REPLACE),
        )
        .unwrap();
    rows.insert(
        "packet:packet-to-frame-unique-replace-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-unique-replace".to_string(),
        frame_side_data_summary_fields(&combined_frame),
    );

    let mut new_ref_frame = Frame::empty();
    SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![0x66, 0x77])
        .unwrap()
        .add_to_frame(&mut new_ref_frame, FrameSideDataFlags::NEW_REF)
        .unwrap();
    rows.insert(
        "packet:packet-to-frame-new-ref-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "packet:packet-to-frame-new-ref".to_string(),
        frame_side_data_summary_fields(&new_ref_frame),
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
    set_common_packet_props(&mut packet);
    packet
}

fn packet_with_common_props_no_payload() -> Packet {
    let mut packet = Packet::new(Vec::new(), 7);
    set_common_packet_props(&mut packet);
    packet
}

fn set_common_packet_props(packet: &mut Packet) {
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
}

fn packet_with_duplicate_side_data() -> Packet {
    let mut packet = packet_with_common_props();
    packet.clear_side_data();
    packet
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x11]).unwrap());
    packet.push_side_data(
        SideData::new_with_kind(PacketSideDataKind::NewExtradata, vec![0x22]).unwrap(),
    );
    packet
        .push_side_data(SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x33]).unwrap());
    packet.push_side_data(
        SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x44]).unwrap(),
    );
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

fn payload_prefix_fields(packet: &Packet, prefix_len: usize) -> Vec<String> {
    let padding = packet.data_buffer().padding_slice();
    assert!(
        prefix_len <= packet.len(),
        "packet payload prefix exceeds visible payload length"
    );
    assert!(
        padding.len() >= AV_INPUT_BUFFER_PADDING_SIZE,
        "packet payload should have FFmpeg input padding for oracle comparison"
    );
    vec![
        packet.len().to_string(),
        hex_or_dash(&packet.data()[..prefix_len]),
        hex_or_dash(&padding[..AV_INPUT_BUFFER_PADDING_SIZE]),
        u8::from(packet.is_data_writable()).to_string(),
    ]
}

fn payload_visible_fields(packet: &Packet) -> Vec<String> {
    vec![packet.len().to_string(), hex_or_dash(packet.data())]
}

fn payload_unowned_fields(packet: &Packet) -> Vec<String> {
    let padding = packet.data_buffer().padding_slice();
    assert!(
        padding.len() >= AV_INPUT_BUFFER_PADDING_SIZE,
        "packet payload should have FFmpeg input padding for oracle comparison"
    );
    vec![
        packet.len().to_string(),
        hex_or_dash(packet.data()),
        hex_or_dash(&padding[..AV_INPUT_BUFFER_PADDING_SIZE]),
    ]
}

fn payload_layout_fields(bytes: &[u8], offsets: &[usize]) -> Vec<String> {
    let mut fields = vec![bytes.len().to_string(), hex_or_dash(bytes)];
    fields.extend(offsets.iter().map(ToString::to_string));
    fields
}

fn display_rotation_fields(angles: &[f64]) -> Vec<String> {
    let mut fields = Vec::new();
    for &angle in angles {
        let matrix = PacketDisplayMatrix::from_clockwise_rotation_degrees(angle)
            .unwrap_or_else(|err| panic!("display rotation matrix for {angle}: {err}"));
        fields.push(format!("{angle:.0}"));
        fields.extend(matrix.elements().iter().map(ToString::to_string));
        fields.push(rounded_rotation_field(
            matrix.counterclockwise_rotation_degrees(),
        ));
    }
    fields
}

fn rounded_rotation_field(rotation: Option<f64>) -> String {
    match rotation {
        Some(value) => (value.round() as i64).to_string(),
        None => "nan".to_string(),
    }
}

fn display_rotation_get_affine_fields() -> Vec<String> {
    let raw_affine = PacketDisplayMatrix::new([
        65_536,
        12_345,
        7,
        -23_456,
        32_768,
        -9,
        11,
        -13,
        PacketDisplayMatrix::FIXED_2_30_ONE,
    ]);
    let x_axis_singular = PacketDisplayMatrix::new([
        0,
        65_536,
        0,
        0,
        65_536,
        0,
        0,
        0,
        PacketDisplayMatrix::FIXED_2_30_ONE,
    ]);
    let y_axis_singular = PacketDisplayMatrix::new([
        65_536,
        0,
        0,
        65_536,
        0,
        0,
        0,
        0,
        PacketDisplayMatrix::FIXED_2_30_ONE,
    ]);

    let mut fields = Vec::new();
    for (name, matrix) in [
        ("raw-affine", raw_affine),
        ("x-axis-singular", x_axis_singular),
        ("y-axis-singular", y_axis_singular),
    ] {
        fields.push(name.to_string());
        fields.push(rounded_rotation_field(
            matrix.counterclockwise_rotation_degrees(),
        ));
    }
    fields
}

fn display_flip_fields() -> Vec<String> {
    let mut fields = Vec::new();
    let identity = PacketDisplayMatrix::identity();
    let rot90 = PacketDisplayMatrix::from_clockwise_rotation_degrees(90.0)
        .expect("display rotation matrix for 90 degrees");
    let raw = PacketDisplayMatrix::new([
        65_536,
        12_345,
        7,
        -23_456,
        32_768,
        -9,
        11,
        -13,
        PacketDisplayMatrix::FIXED_2_30_ONE,
    ]);
    for (name, matrix) in [("identity", identity), ("rot90", rot90), ("raw", raw)] {
        for (horizontal, vertical) in [(false, false), (true, false), (false, true), (true, true)] {
            fields.push(name.to_string());
            fields.push(u8::from(horizontal).to_string());
            fields.push(u8::from(vertical).to_string());
            fields.extend(
                matrix
                    .flipped(horizontal, vertical)
                    .elements()
                    .iter()
                    .map(ToString::to_string),
            );
        }
    }
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

fn dictionary_unpack_ret_fields(data: &[u8]) -> Vec<String> {
    let ret = match packet_unpack_dictionary(data) {
        Ok(_) => 0,
        Err(err) => {
            assert_eq!(err.code(), Some(AvErrorCode::INVALIDDATA));
            err.code()
                .unwrap_or_else(|| panic!("dictionary unpack error without AVERROR code: {err}"))
                .raw()
        }
    };
    vec![ret.to_string()]
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
#include <limits.h>
#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "libavcodec/defs.h"
#include "libavcodec/packet.h"
#include "libavutil/ambient_viewing_environment.h"
#include "libavutil/avutil.h"
#include "libavutil/buffer.h"
#include "libavutil/container_fifo.h"
#include "libavutil/dict.h"
#include "libavutil/display.h"
#include "libavutil/dovi_meta.h"
#include "libavutil/encryption_info.h"
#include "libavutil/frame.h"
#include "libavutil/hdr_dynamic_metadata.h"
#include "libavutil/iamf.h"
#include "libavutil/intreadwrite.h"
#include "libavutil/mastering_display_metadata.h"
#include "libavutil/mem.h"
#include "libavutil/pixfmt.h"
#include "libavutil/replaygain.h"
#include "libavutil/spherical.h"
#include "libavutil/stereo3d.h"
#include "libavutil/tdrdi.h"

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

static void print_side_data_lookup_size_null(const char *name,
                                             const AVPacket *pkt,
                                             enum AVPacketSideDataType type,
                                             size_t expected_size) {
    uint8_t *data = av_packet_get_side_data(pkt, type, NULL);
    printf("%s|%d|%zu|", name, data != NULL, data ? expected_size : 0);
    print_hex_or_dash(data, data ? (int)expected_size : 0);
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

static void print_owned_side_data_byte(const char *name, const uint8_t *data) {
    printf("%s|%d|%d|", name, data != NULL, data ? 1 : 0);
    print_hex_or_dash(data, data ? 1 : 0);
    printf("\n");
}

static AVPacketSideData make_stack_side_data(enum AVPacketSideDataType type,
                                             uint8_t value) {
    AVPacketSideData sd = { 0 };
    sd.type = type;
    sd.size = 1;
    sd.data = av_mallocz(sd.size + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!sd.data, "stack side-data allocation failed");
    sd.data[0] = value;
    return sd;
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

#define PRINT_ABI_FIELD(type, field) \
    printf("|%s|%zu|%zu", #field, offsetof(type, field), sizeof(((type *)0)->field))

static void print_packet_abi_layout(void) {
    printf("packet:abi-side-data-layout|AVPacketSideData|%zu|%zu|3",
           sizeof(AVPacketSideData), (size_t)_Alignof(AVPacketSideData));
    PRINT_ABI_FIELD(AVPacketSideData, data);
    PRINT_ABI_FIELD(AVPacketSideData, size);
    PRINT_ABI_FIELD(AVPacketSideData, type);
    printf("\n");

    printf("packet:abi-avpacket-layout|AVPacket|%zu|%zu|14",
           sizeof(AVPacket), (size_t)_Alignof(AVPacket));
    PRINT_ABI_FIELD(AVPacket, buf);
    PRINT_ABI_FIELD(AVPacket, pts);
    PRINT_ABI_FIELD(AVPacket, dts);
    PRINT_ABI_FIELD(AVPacket, data);
    PRINT_ABI_FIELD(AVPacket, size);
    PRINT_ABI_FIELD(AVPacket, stream_index);
    PRINT_ABI_FIELD(AVPacket, flags);
    PRINT_ABI_FIELD(AVPacket, side_data);
    PRINT_ABI_FIELD(AVPacket, side_data_elems);
    PRINT_ABI_FIELD(AVPacket, duration);
    PRINT_ABI_FIELD(AVPacket, pos);
    PRINT_ABI_FIELD(AVPacket, opaque);
    PRINT_ABI_FIELD(AVPacket, opaque_ref);
    PRINT_ABI_FIELD(AVPacket, time_base);
    printf("\n");

    printf("packet:abi-avpacket-list-layout|AVPacketList|%zu|%zu|2",
           sizeof(AVPacketList), (size_t)_Alignof(AVPacketList));
    PRINT_ABI_FIELD(AVPacketList, pkt);
    PRINT_ABI_FIELD(AVPacketList, next);
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

static void print_side_data_name_boundaries(void) {
#define PRINT_SIDE_NAME(value) do { \
    int v = (value); \
    const char *name = av_packet_side_data_name((enum AVPacketSideDataType)v); \
    printf("|%d|%s", v, name ? name : "<null>"); \
} while (0)
    printf("packet:side-kind-name-boundaries");
    PRINT_SIDE_NAME(INT_MIN);
    PRINT_SIDE_NAME(-1);
    PRINT_SIDE_NAME(AV_PKT_DATA_NB);
    PRINT_SIDE_NAME(AV_PKT_DATA_NB + 1);
    PRINT_SIDE_NAME(INT_MAX);
    printf("\n");
#undef PRINT_SIDE_NAME
}

static void print_flag_inventory(void) {
    const int all = AV_PKT_FLAG_KEY |
                    AV_PKT_FLAG_CORRUPT |
                    AV_PKT_FLAG_DISCARD |
                    AV_PKT_FLAG_TRUSTED |
                    AV_PKT_FLAG_DISPOSABLE;
    printf("packet:flag-inventory"
           "|AV_PKT_FLAG_KEY|%d"
           "|AV_PKT_FLAG_CORRUPT|%d"
           "|AV_PKT_FLAG_DISCARD|%d"
           "|AV_PKT_FLAG_TRUSTED|%d"
           "|AV_PKT_FLAG_DISPOSABLE|%d"
           "|all|%d\n",
           AV_PKT_FLAG_KEY,
           AV_PKT_FLAG_CORRUPT,
           AV_PKT_FLAG_DISCARD,
           AV_PKT_FLAG_TRUSTED,
           AV_PKT_FLAG_DISPOSABLE,
           all);
}

static void print_picture_type_inventory(void) {
#define PRINT_PICTURE_TYPE(kind) do { \
    printf("|" #kind "|%d|%c", (int)(kind), av_get_picture_type_char(kind)); \
} while (0)
    printf("packet:picture-type-inventory");
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_NONE);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_I);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_P);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_B);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_S);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_SI);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_SP);
    PRINT_PICTURE_TYPE(AV_PICTURE_TYPE_BI);
    printf("\n");
#undef PRINT_PICTURE_TYPE
}

static void print_payload_layout_header(const char *name, const void *payload, size_t size) {
    printf("%s|%zu|", name, size);
    print_hex_or_dash(payload, (int)size);
}

static void print_payload_layout_bytes(const char *name, const uint8_t *payload, size_t size) {
    print_payload_layout_header(name, payload, size);
    printf("\n");
}

static uint8_t *copy_payload_zero_pointer(const void *payload, size_t size, size_t offset) {
    fail_if(offset > size || size - offset < sizeof(void *),
            "payload pointer field offset is out of bounds");
    uint8_t *copy = av_malloc(size);
    fail_if(!copy, "av_malloc for payload copy returned NULL");
    memcpy(copy, payload, size);
    memset(copy + offset, 0, sizeof(void *));
    return copy;
}

static void zero_payload_pointer(uint8_t *payload, size_t size, size_t offset) {
    fail_if(offset > size || size - offset < sizeof(void *),
            "payload pointer field offset is out of bounds");
    memset(payload + offset, 0, sizeof(void *));
}

static void print_side_data_payload_layouts(void) {
    uint8_t palette[AVPALETTE_SIZE];
    for (size_t i = 0; i < sizeof(palette); i++)
        palette[i] = (uint8_t)(i & 0xff);
    print_payload_layout_bytes("packet:payload-layout-palette",
                               palette, sizeof(palette));

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

    AVContentLightMetadata content_light;
    memset(&content_light, 0, sizeof(content_light));
    content_light.MaxCLL = 1000;
    content_light.MaxFALL = 400;
    print_payload_layout_header("packet:payload-layout-content-light",
                                &content_light, sizeof(content_light));
    printf("|%zu|%zu\n",
           offsetof(AVContentLightMetadata, MaxCLL),
           offsetof(AVContentLightMetadata, MaxFALL));

    AVMasteringDisplayMetadata mastering;
    memset(&mastering, 0, sizeof(mastering));
    mastering.display_primaries[0][0] = (AVRational){ 34000, 50000 };
    mastering.display_primaries[0][1] = (AVRational){ 16000, 50000 };
    mastering.display_primaries[1][0] = (AVRational){ 13250, 50000 };
    mastering.display_primaries[1][1] = (AVRational){ 34500, 50000 };
    mastering.display_primaries[2][0] = (AVRational){ 7500, 50000 };
    mastering.display_primaries[2][1] = (AVRational){ 3000, 50000 };
    mastering.white_point[0] = (AVRational){ 15635, 50000 };
    mastering.white_point[1] = (AVRational){ 16450, 50000 };
    mastering.min_luminance = (AVRational){ 50, 10000 };
    mastering.max_luminance = (AVRational){ 1000, 1 };
    mastering.has_primaries = 1;
    mastering.has_luminance = 2;
    print_payload_layout_header("packet:payload-layout-mastering-display",
                                &mastering, sizeof(mastering));
    printf("|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVMasteringDisplayMetadata, display_primaries),
           offsetof(AVMasteringDisplayMetadata, white_point),
           offsetof(AVMasteringDisplayMetadata, min_luminance),
           offsetof(AVMasteringDisplayMetadata, max_luminance),
           offsetof(AVMasteringDisplayMetadata, has_primaries),
           offsetof(AVMasteringDisplayMetadata, has_luminance));

    AVAmbientViewingEnvironment ambient;
    memset(&ambient, 0, sizeof(ambient));
    ambient.ambient_illuminance = (AVRational){ 1000, 1 };
    ambient.ambient_light_x = (AVRational){ 3127, 10000 };
    ambient.ambient_light_y = (AVRational){ 3291, 10000 };
    print_payload_layout_header("packet:payload-layout-ambient-viewing-environment",
                                &ambient, sizeof(ambient));
    printf("|%zu|%zu|%zu\n",
           offsetof(AVAmbientViewingEnvironment, ambient_illuminance),
           offsetof(AVAmbientViewingEnvironment, ambient_light_x),
           offsetof(AVAmbientViewingEnvironment, ambient_light_y));

    size_t tdrdi_size = 0;
    AV3DReferenceDisplaysInfo *tdrdi = av_tdrdi_alloc(2, &tdrdi_size);
    fail_if(!tdrdi, "av_tdrdi_alloc returned NULL");
    tdrdi->prec_ref_display_width = 31;
    tdrdi->ref_viewing_distance_flag = 1;
    tdrdi->prec_ref_viewing_dist = 7;
    AV3DReferenceDisplay *tdrdi_first = av_tdrdi_get_display(tdrdi, 0);
    tdrdi_first->left_view_id = 0;
    tdrdi_first->right_view_id = 1;
    tdrdi_first->exponent_ref_display_width = 12;
    tdrdi_first->mantissa_ref_display_width = 34;
    tdrdi_first->exponent_ref_viewing_distance = 5;
    tdrdi_first->mantissa_ref_viewing_distance = 67;
    tdrdi_first->additional_shift_present_flag = 1;
    tdrdi_first->num_sample_shift = -11;
    AV3DReferenceDisplay *tdrdi_second = av_tdrdi_get_display(tdrdi, 1);
    tdrdi_second->left_view_id = 2;
    tdrdi_second->right_view_id = 3;
    tdrdi_second->exponent_ref_display_width = 10;
    tdrdi_second->mantissa_ref_display_width = 20;
    tdrdi_second->exponent_ref_viewing_distance = 4;
    tdrdi_second->mantissa_ref_viewing_distance = 40;
    tdrdi_second->additional_shift_present_flag = 0;
    tdrdi_second->num_sample_shift = 0;
    print_payload_layout_header("packet:payload-layout-3d-reference-displays",
                                tdrdi, tdrdi_size);
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AV3DReferenceDisplaysInfo, prec_ref_display_width),
           offsetof(AV3DReferenceDisplaysInfo, ref_viewing_distance_flag),
           offsetof(AV3DReferenceDisplaysInfo, prec_ref_viewing_dist),
           offsetof(AV3DReferenceDisplaysInfo, num_ref_displays),
           offsetof(AV3DReferenceDisplaysInfo, entries_offset),
           offsetof(AV3DReferenceDisplaysInfo, entry_size),
           tdrdi->entries_offset,
           tdrdi->entry_size,
           offsetof(AV3DReferenceDisplay, left_view_id),
           offsetof(AV3DReferenceDisplay, right_view_id),
           offsetof(AV3DReferenceDisplay, exponent_ref_display_width),
           offsetof(AV3DReferenceDisplay, mantissa_ref_display_width),
           offsetof(AV3DReferenceDisplay, exponent_ref_viewing_distance),
           offsetof(AV3DReferenceDisplay, mantissa_ref_viewing_distance),
           offsetof(AV3DReferenceDisplay, additional_shift_present_flag),
           offsetof(AV3DReferenceDisplay, num_sample_shift));
    av_free(tdrdi);

    AVSphericalMapping spherical;
    memset(&spherical, 0, sizeof(spherical));
    spherical.projection = AV_SPHERICAL_PARAMETRIC_IMMERSIVE;
    spherical.yaw = -11;
    spherical.pitch = 22;
    spherical.roll = -33;
    spherical.bound_left = 0x01020304;
    spherical.bound_top = 0x05060708;
    spherical.bound_right = 0x090a0b0c;
    spherical.bound_bottom = 0x0d0e0f10;
    spherical.padding = 0x11121314;
    print_payload_layout_header("packet:payload-layout-spherical",
                                &spherical, sizeof(spherical));
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVSphericalMapping, projection),
           offsetof(AVSphericalMapping, yaw),
           offsetof(AVSphericalMapping, pitch),
           offsetof(AVSphericalMapping, roll),
           offsetof(AVSphericalMapping, bound_left),
           offsetof(AVSphericalMapping, bound_top),
           offsetof(AVSphericalMapping, bound_right),
           offsetof(AVSphericalMapping, bound_bottom),
           offsetof(AVSphericalMapping, padding));

    int32_t displaymatrix[9] = {
        INT32_MIN, -1, 0, 1, 1 << 16, -(1 << 16),
        1 << 30, -(1 << 30), INT32_MAX
    };
    print_payload_layout_header("packet:payload-layout-displaymatrix",
                                displaymatrix, sizeof(displaymatrix));
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           (size_t)((const uint8_t *)&displaymatrix[0] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[1] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[2] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[3] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[4] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[5] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[6] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[7] - (const uint8_t *)displaymatrix),
           (size_t)((const uint8_t *)&displaymatrix[8] - (const uint8_t *)displaymatrix));

    AVStereo3D stereo3d;
    memset(&stereo3d, 0, sizeof(stereo3d));
    stereo3d.type = AV_STEREO3D_SIDEBYSIDE;
    stereo3d.flags = AV_STEREO3D_FLAG_INVERT;
    stereo3d.view = AV_STEREO3D_VIEW_RIGHT;
    stereo3d.primary_eye = AV_PRIMARY_EYE_LEFT;
    stereo3d.baseline = 0x01020304;
    stereo3d.horizontal_disparity_adjustment = (AVRational){ 1, 2 };
    stereo3d.horizontal_field_of_view = (AVRational){ 75, 1 };
    print_payload_layout_header("packet:payload-layout-stereo3d",
                                &stereo3d, sizeof(stereo3d));
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVStereo3D, type),
           offsetof(AVStereo3D, flags),
           offsetof(AVStereo3D, view),
           offsetof(AVStereo3D, primary_eye),
           offsetof(AVStereo3D, baseline),
           offsetof(AVStereo3D, horizontal_disparity_adjustment),
           offsetof(AVStereo3D, horizontal_field_of_view));

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

    AVDynamicHDRPlus hdr10_plus;
    memset(&hdr10_plus, 0, sizeof(hdr10_plus));
    hdr10_plus.itu_t_t35_country_code = 0xB5;
    hdr10_plus.application_version = 0;
    hdr10_plus.num_windows = 1;
    hdr10_plus.targeted_system_display_maximum_luminance = (AVRational){ 1000, 1 };
    hdr10_plus.targeted_system_display_actual_peak_luminance_flag = 1;
    hdr10_plus.num_rows_targeted_system_display_actual_peak_luminance = 2;
    hdr10_plus.num_cols_targeted_system_display_actual_peak_luminance = 2;
    hdr10_plus.targeted_system_display_actual_peak_luminance[0][0] = (AVRational){ 1, 15 };
    hdr10_plus.mastering_display_actual_peak_luminance_flag = 1;
    hdr10_plus.num_rows_mastering_display_actual_peak_luminance = 2;
    hdr10_plus.num_cols_mastering_display_actual_peak_luminance = 2;
    hdr10_plus.mastering_display_actual_peak_luminance[1][1] = (AVRational){ 2, 15 };
    print_payload_layout_header("packet:payload-layout-dynamic-hdr10-plus",
                                &hdr10_plus, sizeof(hdr10_plus));
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVDynamicHDRPlus, itu_t_t35_country_code),
           offsetof(AVDynamicHDRPlus, application_version),
           offsetof(AVDynamicHDRPlus, num_windows),
           offsetof(AVDynamicHDRPlus, params),
           (size_t)((const uint8_t *)&hdr10_plus.params[1] - (const uint8_t *)&hdr10_plus),
           offsetof(AVDynamicHDRPlus, targeted_system_display_maximum_luminance),
           offsetof(AVDynamicHDRPlus, targeted_system_display_actual_peak_luminance_flag),
           offsetof(AVDynamicHDRPlus, num_rows_targeted_system_display_actual_peak_luminance),
           offsetof(AVDynamicHDRPlus, num_cols_targeted_system_display_actual_peak_luminance),
           offsetof(AVDynamicHDRPlus, targeted_system_display_actual_peak_luminance),
           offsetof(AVDynamicHDRPlus, mastering_display_actual_peak_luminance_flag),
           offsetof(AVDynamicHDRPlus, num_rows_mastering_display_actual_peak_luminance),
           offsetof(AVDynamicHDRPlus, num_cols_mastering_display_actual_peak_luminance),
           offsetof(AVDynamicHDRPlus, mastering_display_actual_peak_luminance));

    AVEncryptionInfo *enc = av_encryption_info_alloc(2, 16, 8);
    fail_if(!enc, "av_encryption_info_alloc returned NULL");
    enc->scheme = ((uint32_t)'c' << 24) |
                  ((uint32_t)'e' << 16) |
                  ((uint32_t)'n' << 8) |
                  (uint32_t)'c';
    enc->crypt_byte_block = 1;
    enc->skip_byte_block = 9;
    for (uint32_t i = 0; i < enc->key_id_size; i++)
        enc->key_id[i] = (uint8_t)(0x10 + i);
    for (uint32_t i = 0; i < enc->iv_size; i++)
        enc->iv[i] = (uint8_t)(0xa0 + i);
    enc->subsamples[0].bytes_of_clear_data = 3;
    enc->subsamples[0].bytes_of_protected_data = 100;
    enc->subsamples[1].bytes_of_clear_data = 0;
    enc->subsamples[1].bytes_of_protected_data = 55;
    size_t enc_side_size = 0;
    uint8_t *enc_side = av_encryption_info_add_side_data(enc, &enc_side_size);
    fail_if(!enc_side, "av_encryption_info_add_side_data returned NULL");
    print_payload_layout_bytes("packet:payload-layout-encryption-info",
                               enc_side, enc_side_size);
    av_free(enc_side);
    av_encryption_info_free(enc);

    AVEncryptionInitInfo *init_a = av_encryption_init_info_alloc(4, 2, 3, 5);
    fail_if(!init_a, "av_encryption_init_info_alloc first returned NULL");
    AVEncryptionInitInfo *init_b = av_encryption_init_info_alloc(0, 0, 16, 3);
    fail_if(!init_b, "av_encryption_init_info_alloc second returned NULL");
    memcpy(init_a->system_id, "sys1", 4);
    memcpy(init_a->key_ids[0], "abc", 3);
    memcpy(init_a->key_ids[1], "def", 3);
    memcpy(init_a->data, "hello", 5);
    memcpy(init_b->data, "pss", 3);
    init_a->next = init_b;
    size_t init_side_size = 0;
    uint8_t *init_side = av_encryption_init_info_add_side_data(init_a,
                                                               &init_side_size);
    fail_if(!init_side, "av_encryption_init_info_add_side_data returned NULL");
    print_payload_layout_bytes("packet:payload-layout-encryption-init-info",
                               init_side, init_side_size);
    av_free(init_side);
    av_encryption_init_info_free(init_a);

    size_t iamf_mix_size = 0;
    AVIAMFParamDefinition *iamf_mix =
        av_iamf_param_definition_alloc(AV_IAMF_PARAMETER_DEFINITION_MIX_GAIN,
                                       2, &iamf_mix_size);
    fail_if(!iamf_mix, "av_iamf_param_definition_alloc mix returned NULL");
    iamf_mix->parameter_id = 7;
    iamf_mix->parameter_rate = 48000;
    iamf_mix->duration = 960;
    iamf_mix->constant_subblock_duration = 480;
    AVIAMFMixGain *iamf_mix_first =
        av_iamf_param_definition_get_subblock(iamf_mix, 0);
    iamf_mix_first->subblock_duration = 480;
    iamf_mix_first->animation_type = AV_IAMF_ANIMATION_TYPE_LINEAR;
    iamf_mix_first->start_point_value = (AVRational){ -1, 2 };
    iamf_mix_first->end_point_value = (AVRational){ 3, 4 };
    iamf_mix_first->control_point_value = (AVRational){ 1, 3 };
    iamf_mix_first->control_point_relative_time = (AVRational){ 1, 2 };
    AVIAMFMixGain *iamf_mix_second =
        av_iamf_param_definition_get_subblock(iamf_mix, 1);
    iamf_mix_second->subblock_duration = 480;
    iamf_mix_second->animation_type = AV_IAMF_ANIMATION_TYPE_BEZIER;
    iamf_mix_second->start_point_value = (AVRational){ -1, 2 };
    iamf_mix_second->end_point_value = (AVRational){ 3, 4 };
    iamf_mix_second->control_point_value = (AVRational){ 1, 3 };
    iamf_mix_second->control_point_relative_time = (AVRational){ 1, 2 };
    uint8_t *iamf_mix_bytes = copy_payload_zero_pointer(
        iamf_mix, iamf_mix_size, offsetof(AVIAMFParamDefinition, av_class));
    for (unsigned int i = 0; i < iamf_mix->nb_subblocks; i++) {
        zero_payload_pointer(iamf_mix_bytes, iamf_mix_size,
                             iamf_mix->subblocks_offset +
                             i * iamf_mix->subblock_size +
                             offsetof(AVIAMFMixGain, av_class));
    }
    print_payload_layout_header("packet:payload-layout-iamf-mix-gain-param",
                                iamf_mix_bytes, iamf_mix_size);
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVIAMFParamDefinition, av_class),
           offsetof(AVIAMFParamDefinition, subblocks_offset),
           offsetof(AVIAMFParamDefinition, subblock_size),
           offsetof(AVIAMFParamDefinition, nb_subblocks),
           offsetof(AVIAMFParamDefinition, type),
           offsetof(AVIAMFParamDefinition, parameter_id),
           offsetof(AVIAMFParamDefinition, parameter_rate),
           offsetof(AVIAMFParamDefinition, duration),
           offsetof(AVIAMFParamDefinition, constant_subblock_duration),
           iamf_mix->subblocks_offset,
           iamf_mix->subblock_size,
           offsetof(AVIAMFMixGain, av_class),
           offsetof(AVIAMFMixGain, subblock_duration),
           offsetof(AVIAMFMixGain, animation_type),
           offsetof(AVIAMFMixGain, start_point_value),
           offsetof(AVIAMFMixGain, end_point_value),
           offsetof(AVIAMFMixGain, control_point_value),
           offsetof(AVIAMFMixGain, control_point_relative_time));
    av_free(iamf_mix_bytes);
    av_free(iamf_mix);

    size_t iamf_demix_size = 0;
    AVIAMFParamDefinition *iamf_demix =
        av_iamf_param_definition_alloc(AV_IAMF_PARAMETER_DEFINITION_DEMIXING,
                                       1, &iamf_demix_size);
    fail_if(!iamf_demix, "av_iamf_param_definition_alloc demixing returned NULL");
    iamf_demix->parameter_id = 7;
    iamf_demix->parameter_rate = 48000;
    iamf_demix->duration = 960;
    iamf_demix->constant_subblock_duration = 480;
    AVIAMFDemixingInfo *iamf_demix_sub =
        av_iamf_param_definition_get_subblock(iamf_demix, 0);
    iamf_demix_sub->subblock_duration = 960;
    iamf_demix_sub->dmixp_mode = 7;
    uint8_t *iamf_demix_bytes = copy_payload_zero_pointer(
        iamf_demix, iamf_demix_size, offsetof(AVIAMFParamDefinition, av_class));
    zero_payload_pointer(iamf_demix_bytes, iamf_demix_size,
                         iamf_demix->subblocks_offset +
                         offsetof(AVIAMFDemixingInfo, av_class));
    print_payload_layout_header("packet:payload-layout-iamf-demixing-info-param",
                                iamf_demix_bytes, iamf_demix_size);
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVIAMFParamDefinition, av_class),
           offsetof(AVIAMFParamDefinition, subblocks_offset),
           offsetof(AVIAMFParamDefinition, subblock_size),
           offsetof(AVIAMFParamDefinition, nb_subblocks),
           offsetof(AVIAMFParamDefinition, type),
           offsetof(AVIAMFParamDefinition, parameter_id),
           offsetof(AVIAMFParamDefinition, parameter_rate),
           offsetof(AVIAMFParamDefinition, duration),
           offsetof(AVIAMFParamDefinition, constant_subblock_duration),
           iamf_demix->subblocks_offset,
           iamf_demix->subblock_size,
           offsetof(AVIAMFDemixingInfo, av_class),
           offsetof(AVIAMFDemixingInfo, subblock_duration),
           offsetof(AVIAMFDemixingInfo, dmixp_mode));
    av_free(iamf_demix_bytes);
    av_free(iamf_demix);

    size_t iamf_recon_size = 0;
    AVIAMFParamDefinition *iamf_recon =
        av_iamf_param_definition_alloc(AV_IAMF_PARAMETER_DEFINITION_RECON_GAIN,
                                       1, &iamf_recon_size);
    fail_if(!iamf_recon, "av_iamf_param_definition_alloc recon returned NULL");
    iamf_recon->parameter_id = 7;
    iamf_recon->parameter_rate = 48000;
    iamf_recon->duration = 960;
    iamf_recon->constant_subblock_duration = 480;
    AVIAMFReconGain *iamf_recon_sub =
        av_iamf_param_definition_get_subblock(iamf_recon, 0);
    iamf_recon_sub->subblock_duration = 960;
    for (size_t layer = 0; layer < 6; layer++) {
        for (size_t channel = 0; channel < 12; channel++)
            iamf_recon_sub->recon_gain[layer][channel] =
                (uint8_t)(layer * 16 + channel);
    }
    uint8_t *iamf_recon_bytes = copy_payload_zero_pointer(
        iamf_recon, iamf_recon_size, offsetof(AVIAMFParamDefinition, av_class));
    zero_payload_pointer(iamf_recon_bytes, iamf_recon_size,
                         iamf_recon->subblocks_offset +
                         offsetof(AVIAMFReconGain, av_class));
    print_payload_layout_header("packet:payload-layout-iamf-recon-gain-info-param",
                                iamf_recon_bytes, iamf_recon_size);
    printf("|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu|%zu\n",
           offsetof(AVIAMFParamDefinition, av_class),
           offsetof(AVIAMFParamDefinition, subblocks_offset),
           offsetof(AVIAMFParamDefinition, subblock_size),
           offsetof(AVIAMFParamDefinition, nb_subblocks),
           offsetof(AVIAMFParamDefinition, type),
           offsetof(AVIAMFParamDefinition, parameter_id),
           offsetof(AVIAMFParamDefinition, parameter_rate),
           offsetof(AVIAMFParamDefinition, duration),
           offsetof(AVIAMFParamDefinition, constant_subblock_duration),
           iamf_recon->subblocks_offset,
           iamf_recon->subblock_size,
           offsetof(AVIAMFReconGain, av_class),
           offsetof(AVIAMFReconGain, subblock_duration),
           offsetof(AVIAMFReconGain, recon_gain));
    av_free(iamf_recon_bytes);
    av_free(iamf_recon);

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

static void print_payload_prefix(const char *name, const AVPacket *pkt, int prefix_size) {
    fail_if(prefix_size < 0 || prefix_size > pkt->size,
            "payload prefix size is out of bounds");
    printf("%s|%d|", name, pkt->size);
    print_hex_or_dash(pkt->data, prefix_size);
    printf("|");
    print_hex_or_dash(pkt->data ? pkt->data + pkt->size : NULL,
                      AV_INPUT_BUFFER_PADDING_SIZE);
    printf("|%d\n", pkt->buf ? av_buffer_is_writable(pkt->buf) : 0);
}

static void print_payload_visible(const char *name, const AVPacket *pkt) {
    printf("%s|%d|", name, pkt->size);
    print_hex_or_dash(pkt->data, pkt->size);
    printf("\n");
}

static void print_payload_unowned(const char *name, const AVPacket *pkt) {
    printf("%s|%d|", name, pkt->size);
    print_hex_or_dash(pkt->data, pkt->size);
    printf("|");
    print_hex_or_dash(pkt->data ? pkt->data + pkt->size : NULL,
                      AV_INPUT_BUFFER_PADDING_SIZE);
    printf("\n");
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

static void print_dictionary_unpack_ret(const char *name, const uint8_t *data, size_t size) {
    AVDictionary *dict = NULL;
    int ret = av_packet_unpack_dictionary(data, size, &dict);
    printf("%s|%d\n", name, ret);
    av_dict_free(&dict);
}

static void print_display_rotation_case(double angle) {
    int32_t matrix[9];
    av_display_rotation_set(matrix, angle);
    double rotation = av_display_rotation_get(matrix);
    printf("|%.0f", angle);
    for (int i = 0; i < 9; i++)
        printf("|%d", matrix[i]);
    printf("|");
    if (isnan(rotation))
        printf("nan");
    else
        printf("%lld", (long long)llround(rotation));
}

static void print_display_rotation_helpers(void) {
    printf("packet:display-rotation-set-get");
    print_display_rotation_case(0.0);
    print_display_rotation_case(90.0);
    print_display_rotation_case(-90.0);
    print_display_rotation_case(180.0);
    print_display_rotation_case(45.0);
    print_display_rotation_case(-45.0);
    printf("\n");

    int32_t singular[9] = { 0 };
    printf("packet:display-rotation-singular|%d\n",
           isnan(av_display_rotation_get(singular)) ? 1 : 0);
}

static void print_display_rotation_get_case(const char *name, const int32_t matrix[9]) {
    double rotation = av_display_rotation_get(matrix);
    printf("|%s|", name);
    if (isnan(rotation))
        printf("nan");
    else
        printf("%lld", (long long)llround(rotation));
}

static void print_display_rotation_get_affine_helpers(void) {
    const int32_t raw_affine[9] = {
        65536, 12345, 7, -23456, 32768, -9, 11, -13, 1073741824
    };
    const int32_t x_axis_singular[9] = {
        0, 65536, 0, 0, 65536, 0, 0, 0, 1073741824
    };
    const int32_t y_axis_singular[9] = {
        65536, 0, 0, 65536, 0, 0, 0, 0, 1073741824
    };

    printf("packet:display-rotation-get-affine");
    print_display_rotation_get_case("raw-affine", raw_affine);
    print_display_rotation_get_case("x-axis-singular", x_axis_singular);
    print_display_rotation_get_case("y-axis-singular", y_axis_singular);
    printf("\n");
}

static void print_display_flip_case(const char *name, const int32_t input[9], int hflip, int vflip) {
    int32_t matrix[9];
    for (int i = 0; i < 9; i++)
        matrix[i] = input[i];
    av_display_matrix_flip(matrix, hflip, vflip);
    printf("|%s|%d|%d", name, hflip ? 1 : 0, vflip ? 1 : 0);
    for (int i = 0; i < 9; i++)
        printf("|%d", matrix[i]);
}

static void print_display_flip_helpers(void) {
    const int32_t identity[9] = { 65536, 0, 0, 0, 65536, 0, 0, 0, 1073741824 };
    int32_t rot90[9];
    const int32_t raw[9] = {
        65536, 12345, 7, -23456, 32768, -9, 11, -13, 1073741824
    };
    av_display_rotation_set(rot90, 90.0);

    printf("packet:display-flip");
    print_display_flip_case("identity", identity, 0, 0);
    print_display_flip_case("identity", identity, 1, 0);
    print_display_flip_case("identity", identity, 0, 1);
    print_display_flip_case("identity", identity, 1, 1);
    print_display_flip_case("rot90", rot90, 0, 0);
    print_display_flip_case("rot90", rot90, 1, 0);
    print_display_flip_case("rot90", rot90, 0, 1);
    print_display_flip_case("rot90", rot90, 1, 1);
    print_display_flip_case("raw", raw, 0, 0);
    print_display_flip_case("raw", raw, 1, 0);
    print_display_flip_case("raw", raw, 0, 1);
    print_display_flip_case("raw", raw, 1, 1);
    printf("\n");
}

static AVPacket *new_packet(void) {
    AVPacket *pkt = av_packet_alloc();
    fail_if(!pkt, "av_packet_alloc failed");
    return pkt;
}

static void set_common_packet_props(AVPacket *pkt) {
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
}

static AVPacket *packet_with_common_props_no_payload(void) {
    AVPacket *pkt = new_packet();
    set_common_packet_props(pkt);
    return pkt;
}

static AVPacket *packet_with_common_props(void) {
    AVPacket *pkt = new_packet();
    int ret = av_new_packet(pkt, 3);
    fail_if(ret < 0, "av_new_packet failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    pkt->data[2] = 0xcc;
    set_common_packet_props(pkt);
    return pkt;
}

static AVPacket *packet_with_duplicate_side_data(void) {
    AVPacket *pkt = packet_with_common_props();
    av_packet_free_side_data(pkt);
    pkt->side_data = av_mallocz(4 * sizeof(*pkt->side_data));
    fail_if(!pkt->side_data, "av_mallocz lifecycle duplicate side data failed");
    pkt->side_data_elems = 4;
    pkt->side_data[0] = make_stack_side_data(AV_PKT_DATA_PALETTE, 0x11);
    pkt->side_data[1] = make_stack_side_data(AV_PKT_DATA_NEW_EXTRADATA, 0x22);
    pkt->side_data[2] = make_stack_side_data(AV_PKT_DATA_PALETTE, 0x33);
    pkt->side_data[3] = make_stack_side_data(AV_PKT_DATA_SKIP_SAMPLES, 0x44);
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
    print_side_data_lookup_size_null("packet:side-get-shrunk-size-null", pkt,
                                     AV_PKT_DATA_NEW_EXTRADATA, 2);
    print_side_data_lookup("packet:side-get-missing", pkt, AV_PKT_DATA_PALETTE);
    print_side_data_lookup_size_null("packet:side-get-missing-size-null", pkt,
                                     AV_PKT_DATA_PALETTE, 0);
    ret = av_packet_shrink_side_data(pkt, AV_PKT_DATA_PALETTE, 1);
    printf("packet:side-shrink-missing-ret|%d\n", ret);
    ret = av_packet_shrink_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 3);
    printf("packet:side-shrink-oversize-ret|%d\n", ret);
    print_side_data_summary("packet:side-shrink-oversize", pkt);

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

    pkt = new_packet();
    pkt->side_data = av_mallocz(4 * sizeof(*pkt->side_data));
    fail_if(!pkt->side_data, "av_mallocz duplicate packet side data failed");
    pkt->side_data_elems = 4;
    pkt->side_data[0] = make_stack_side_data(AV_PKT_DATA_PALETTE, 0x11);
    pkt->side_data[1] = make_stack_side_data(AV_PKT_DATA_NEW_EXTRADATA, 0x22);
    pkt->side_data[2] = make_stack_side_data(AV_PKT_DATA_PALETTE, 0x33);
    pkt->side_data[3] = make_stack_side_data(AV_PKT_DATA_SKIP_SAMPLES, 0x44);
    print_side_data_summary("packet:side-duplicate-before", pkt);
    print_side_data_lookup("packet:side-get-duplicate-palette", pkt,
                           AV_PKT_DATA_PALETTE);
    sd = av_packet_new_side_data(pkt, AV_PKT_DATA_PALETTE, 2);
    printf("packet:side-new-duplicate-replace-ret|%d\n", sd != NULL);
    fail_if(!sd, "av_packet_new_side_data duplicate replace failed");
    sd[0] = 0x66;
    sd[1] = 0x77;
    print_side_data_summary("packet:side-new-duplicate-replace", pkt);
    print_side_data_lookup("packet:side-get-duplicate-palette-new", pkt,
                           AV_PKT_DATA_PALETTE);
    owned = av_mallocz(1 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz duplicate packet side data replacement failed");
    owned[0] = 0x55;
    ret = av_packet_add_side_data(pkt, AV_PKT_DATA_PALETTE, owned, 1);
    if (ret < 0)
        av_free(owned);
    printf("packet:side-add-duplicate-replace-ret|%d\n", ret);
    fail_if(ret < 0, "av_packet_add_side_data duplicate replace failed");
    print_side_data_summary("packet:side-add-duplicate-replace", pkt);
    ret = av_packet_shrink_side_data(pkt, AV_PKT_DATA_PALETTE, 0);
    printf("packet:side-shrink-duplicate-ret|%d\n", ret);
    fail_if(ret < 0, "av_packet_shrink_side_data duplicate shrink failed");
    print_side_data_summary("packet:side-shrink-duplicate", pkt);
    print_side_data_lookup("packet:side-get-duplicate-palette-shrunk", pkt,
                           AV_PKT_DATA_PALETTE);
    av_packet_free_side_data(pkt);
    print_side_data_summary("packet:side-free-duplicate", pkt);
    print_side_data_lookup("packet:side-get-duplicate-palette-free", pkt,
                           AV_PKT_DATA_PALETTE);
    av_packet_free(&pkt);

    pkt = new_packet();
    sd = av_packet_new_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, 0);
    fail_if(!sd, "av_packet_new_side_data zero failed");
    print_side_data_summary("packet:side-new-zero", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    owned = av_mallocz(1);
    fail_if(!owned, "av_mallocz zero packet side data failed");
    ret = av_packet_add_side_data(pkt, AV_PKT_DATA_NEW_EXTRADATA, owned, 0);
    if (ret < 0)
        av_free(owned);
    printf("packet:side-add-zero-ret|%d\n", ret);
    fail_if(ret < 0, "av_packet_add_side_data zero failed");
    print_side_data_summary("packet:side-add-zero", pkt);
    av_packet_free(&pkt);
}

static uint8_t *alloc_owned_side_data_byte(uint8_t value) {
    uint8_t *data = av_mallocz(1 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!data, "side data byte allocation failed");
    data[0] = value;
    return data;
}

static void exercise_side_data_capacity_api(void) {
    AVPacket *pkt = av_packet_alloc();
    fail_if(!pkt, "av_packet_alloc side-data capacity failed");

    for (int type = 0; type < AV_PKT_DATA_NB; type++) {
        uint8_t *owned = alloc_owned_side_data_byte((uint8_t)type);
        int ret = av_packet_add_side_data(pkt,
                                          (enum AVPacketSideDataType)type,
                                          owned, 1);
        if (ret < 0)
            av_free(owned);
        fail_if(ret < 0, "av_packet_add_side_data capacity fill failed");
    }
    printf("packet:side-add-capacity-count|%d\n", pkt->side_data_elems);

    uint8_t *replacement = alloc_owned_side_data_byte(0xaa);
    int ret = av_packet_add_side_data(pkt, AV_PKT_DATA_PALETTE, replacement, 1);
    if (ret < 0)
        av_free(replacement);
    printf("packet:side-add-capacity-replace-ret|%d\n", ret);
    fail_if(ret < 0, "av_packet_add_side_data capacity replace failed");
    printf("packet:side-add-capacity-replace-count|%d\n",
           pkt->side_data_elems);
    print_side_data_lookup("packet:side-add-capacity-replace-palette", pkt,
                           AV_PKT_DATA_PALETTE);

    uint8_t *extra = alloc_owned_side_data_byte(0xee);
    ret = av_packet_add_side_data(pkt, (enum AVPacketSideDataType)AV_PKT_DATA_NB,
                                  extra, 1);
    printf("packet:side-add-capacity-overflow-ret|%d\n", ret);
    print_owned_side_data_byte("packet:side-add-capacity-overflow-owned",
                               ret < 0 ? extra : NULL);
    if (ret < 0)
        av_free(extra);
    printf("packet:side-add-capacity-overflow-count|%d\n",
           pkt->side_data_elems);

    uint8_t *created = av_packet_new_side_data(pkt,
                                               (enum AVPacketSideDataType)AV_PKT_DATA_NB,
                                               1);
    printf("packet:side-new-capacity-overflow|%d|%d\n",
           created ? 1 : 0, pkt->side_data_elems);

    av_packet_free(&pkt);
}

static void exercise_side_data_array_api(void) {
    AVPacketSideData *sd = NULL;
    int nb_sd = 0;
    AVPacketSideData *entry = NULL;
    print_side_data_array_lookup("packet:array-empty-get", NULL, 0,
                                 AV_PKT_DATA_PALETTE);
    av_packet_side_data_remove(sd, &nb_sd, AV_PKT_DATA_PALETTE);
    print_side_data_array_summary("packet:array-empty-remove", sd, nb_sd);
    av_packet_side_data_free(&sd, &nb_sd);
    print_side_data_array_summary("packet:array-empty-free", sd, nb_sd);

    entry = av_packet_side_data_new(&sd, &nb_sd,
                                    AV_PKT_DATA_NEW_EXTRADATA, 4, 0);
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

    entry = av_packet_side_data_new(&sd, &nb_sd, AV_PKT_DATA_SKIP_SAMPLES,
                                    2, 1);
    printf("packet:array-new-flags-nonzero-ret|%d\n", entry != NULL);
    fail_if(!entry, "av_packet_side_data_new flags failed");
    entry->data[0] = 0xc2;
    entry->data[1] = 0x58;
    print_side_data_array_summary("packet:array-new-flags-nonzero",
                                  sd, nb_sd);

    uint8_t *flags_owned = alloc_owned_side_data_byte(0x5a);
    entry = av_packet_side_data_add(&sd, &nb_sd, AV_PKT_DATA_SKIP_SAMPLES,
                                    flags_owned, 1, 1);
    printf("packet:array-add-flags-nonzero-ret|%d\n", entry != NULL);
    print_side_data_array_summary("packet:array-add-flags-nonzero",
                                  sd, nb_sd);
    print_owned_side_data_byte("packet:array-add-flags-nonzero-owned",
                               entry ? NULL : flags_owned);
    if (!entry)
        av_free(flags_owned);

    AVPacketSideData duplicate_sd[4] = {
        make_stack_side_data(AV_PKT_DATA_PALETTE, 0x11),
        make_stack_side_data(AV_PKT_DATA_NEW_EXTRADATA, 0x22),
        make_stack_side_data(AV_PKT_DATA_PALETTE, 0x33),
        make_stack_side_data(AV_PKT_DATA_SKIP_SAMPLES, 0x44),
    };
    int duplicate_nb_sd = 4;
    print_side_data_array_summary("packet:array-remove-duplicate-before",
                                  duplicate_sd, duplicate_nb_sd);
    print_side_data_array_lookup("packet:array-get-duplicate-palette",
                                 duplicate_sd, duplicate_nb_sd,
                                 AV_PKT_DATA_PALETTE);
    AVPacketSideData *duplicate_ptr = duplicate_sd;
    AVPacketSideData *duplicate_entry =
        av_packet_side_data_new(&duplicate_ptr, &duplicate_nb_sd,
                                AV_PKT_DATA_PALETTE, 2, 0);
    printf("packet:array-new-duplicate-replace-ret|%d\n",
           duplicate_entry != NULL);
    fail_if(!duplicate_entry, "av_packet_side_data_new duplicate replace failed");
    duplicate_entry->data[0] = 0x66;
    duplicate_entry->data[1] = 0x77;
    print_side_data_array_summary("packet:array-new-duplicate-replace",
                                  duplicate_sd, duplicate_nb_sd);
    print_side_data_array_lookup("packet:array-get-duplicate-palette-new",
                                 duplicate_sd, duplicate_nb_sd,
                                 AV_PKT_DATA_PALETTE);
    uint8_t *duplicate_replacement = alloc_owned_side_data_byte(0x55);
    duplicate_entry =
        av_packet_side_data_add(&duplicate_ptr, &duplicate_nb_sd,
                                AV_PKT_DATA_PALETTE,
                                duplicate_replacement, 1, 0);
    printf("packet:array-add-duplicate-replace-ret|%d\n",
           duplicate_entry != NULL);
    print_side_data_array_summary("packet:array-add-duplicate-replace",
                                  duplicate_sd, duplicate_nb_sd);
    av_packet_side_data_remove(duplicate_sd, &duplicate_nb_sd,
                               AV_PKT_DATA_PALETTE);
    print_side_data_array_summary("packet:array-remove-duplicate-last",
                                  duplicate_sd, duplicate_nb_sd);
    for (int i = 0; i < duplicate_nb_sd; i++)
        av_free(duplicate_sd[i].data);

    AVPacketSideData *duplicate_free_sd =
        av_mallocz(4 * sizeof(*duplicate_free_sd));
    fail_if(!duplicate_free_sd, "av_mallocz duplicate free array failed");
    int duplicate_free_nb_sd = 4;
    duplicate_free_sd[0] = make_stack_side_data(AV_PKT_DATA_PALETTE, 0x11);
    duplicate_free_sd[1] = make_stack_side_data(AV_PKT_DATA_NEW_EXTRADATA, 0x22);
    duplicate_free_sd[2] = make_stack_side_data(AV_PKT_DATA_PALETTE, 0x33);
    duplicate_free_sd[3] = make_stack_side_data(AV_PKT_DATA_SKIP_SAMPLES, 0x44);
    print_side_data_array_summary("packet:array-free-duplicate-before",
                                  duplicate_free_sd, duplicate_free_nb_sd);
    av_packet_side_data_free(&duplicate_free_sd, &duplicate_free_nb_sd);
    print_side_data_array_summary("packet:array-free-duplicate",
                                  duplicate_free_sd, duplicate_free_nb_sd);

    av_packet_side_data_remove(sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA);
    print_side_data_array_summary("packet:array-remove-new", sd, nb_sd);
    av_packet_side_data_remove(sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA);
    print_side_data_array_summary("packet:array-remove-missing", sd, nb_sd);
    av_packet_side_data_free(&sd, &nb_sd);
    print_side_data_array_summary("packet:array-free", sd, nb_sd);

    entry = av_packet_side_data_new(&sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA,
                                    0, 0);
    fail_if(!entry, "av_packet_side_data_new zero failed");
    print_side_data_array_summary("packet:array-new-zero", sd, nb_sd);
    av_packet_side_data_free(&sd, &nb_sd);

    owned = av_mallocz(1);
    fail_if(!owned, "av_mallocz zero array side data failed");
    entry = av_packet_side_data_add(&sd, &nb_sd, AV_PKT_DATA_NEW_EXTRADATA,
                                    owned, 0, 0);
    printf("packet:array-add-zero-ret|%d\n", entry != NULL);
    if (!entry)
        av_free(owned);
    fail_if(!entry, "av_packet_side_data_add zero failed");
    print_side_data_array_summary("packet:array-add-zero", sd, nb_sd);
    av_packet_side_data_free(&sd, &nb_sd);

    for (int type = 0; type < AV_PKT_DATA_NB; type++) {
        entry = av_packet_side_data_new(&sd, &nb_sd,
                                        (enum AVPacketSideDataType)type,
                                        1, 0);
        fail_if(!entry, "av_packet_side_data_new capacity fill failed");
        entry->data[0] = (uint8_t)type;
    }
    printf("packet:array-add-capacity-count|%d\n", nb_sd);

    owned = alloc_owned_side_data_byte(0xaa);
    entry = av_packet_side_data_add(&sd, &nb_sd, AV_PKT_DATA_PALETTE,
                                    owned, 1, 0);
    printf("packet:array-add-capacity-replace-ret|%d\n", entry != NULL);
    if (!entry)
        av_free(owned);
    fail_if(!entry, "av_packet_side_data_add capacity replace failed");
    printf("packet:array-add-capacity-replace-count|%d\n", nb_sd);
    print_side_data_array_lookup("packet:array-add-capacity-replace-palette",
                                 sd, nb_sd, AV_PKT_DATA_PALETTE);

    owned = alloc_owned_side_data_byte(0xee);
    entry = av_packet_side_data_add(&sd, &nb_sd,
                                    (enum AVPacketSideDataType)AV_PKT_DATA_NB,
                                    owned, 1, 0);
    printf("packet:array-add-capacity-overflow-ret|%d\n", entry != NULL);
    if (!entry)
        av_free(owned);
    printf("packet:array-add-capacity-overflow-count|%d\n", nb_sd);

    entry = av_packet_side_data_new(&sd, &nb_sd,
                                    (enum AVPacketSideDataType)AV_PKT_DATA_NB,
                                    1, 0);
    printf("packet:array-new-capacity-overflow|%d|%d\n",
           entry != NULL, nb_sd);
    av_packet_side_data_free(&sd, &nb_sd);
}

static void exercise_frame_packet_side_data_bridge_api(void) {
    AVPacketSideData *psd = NULL;
    int nb_psd = 0;
    AVFrameSideData **fsd = NULL;
    int nb_fsd = 0;
    int ret;

    const enum AVPacketSideDataType mapped_packet_types[] = {
        AV_PKT_DATA_REPLAYGAIN,
        AV_PKT_DATA_DISPLAYMATRIX,
        AV_PKT_DATA_SPHERICAL,
        AV_PKT_DATA_STEREO3D,
        AV_PKT_DATA_AUDIO_SERVICE_TYPE,
        AV_PKT_DATA_MASTERING_DISPLAY_METADATA,
        AV_PKT_DATA_CONTENT_LIGHT_LEVEL,
        AV_PKT_DATA_ICC_PROFILE,
        AV_PKT_DATA_AMBIENT_VIEWING_ENVIRONMENT,
        AV_PKT_DATA_3D_REFERENCE_DISPLAYS,
        AV_PKT_DATA_EXIF,
    };
    const enum AVFrameSideDataType mapped_frame_types[] = {
        AV_FRAME_DATA_REPLAYGAIN,
        AV_FRAME_DATA_DISPLAYMATRIX,
        AV_FRAME_DATA_SPHERICAL,
        AV_FRAME_DATA_STEREO3D,
        AV_FRAME_DATA_AUDIO_SERVICE_TYPE,
        AV_FRAME_DATA_MASTERING_DISPLAY_METADATA,
        AV_FRAME_DATA_CONTENT_LIGHT_LEVEL,
        AV_FRAME_DATA_ICC_PROFILE,
        AV_FRAME_DATA_AMBIENT_VIEWING_ENVIRONMENT,
        AV_FRAME_DATA_3D_REFERENCE_DISPLAYS,
        AV_FRAME_DATA_EXIF,
    };
    const int mapped_count = (int)(sizeof(mapped_packet_types) / sizeof(mapped_packet_types[0]));
    const int mapped_frame_count = (int)(sizeof(mapped_frame_types) / sizeof(mapped_frame_types[0]));
    fail_if(mapped_count != mapped_frame_count,
            "packet/frame side-data bridge map table mismatch");

    AVPacketSideData *mapped_psd = NULL;
    int mapped_nb_psd = 0;
    AVFrameSideData **mapped_from_fsd = NULL;
    int mapped_from_nb_fsd = 0;
    for (int i = 0; i < mapped_count; i++) {
        AVFrameSideData *mapped_frame = av_frame_side_data_new(
            &mapped_from_fsd, &mapped_from_nb_fsd, mapped_frame_types[i], 1, 0);
        fail_if(!mapped_frame, "av_frame_side_data_new mapped bridge seed failed");
        mapped_frame->data[0] = (uint8_t)(0x80 + i);
        ret = av_packet_side_data_from_frame(&mapped_psd, &mapped_nb_psd,
                                             mapped_frame, 0);
        fail_if(ret < 0, "av_packet_side_data_from_frame mapped inventory failed");
    }
    print_side_data_array_summary("packet:frame-to-packet-map-inventory",
                                  mapped_psd, mapped_nb_psd);
    av_frame_side_data_free(&mapped_from_fsd, &mapped_from_nb_fsd);
    av_packet_side_data_free(&mapped_psd, &mapped_nb_psd);

    AVPacketSideData *mapped_to_psd = NULL;
    int mapped_to_nb_psd = 0;
    AVFrameSideData **mapped_to_fsd = NULL;
    int mapped_to_nb_fsd = 0;
    for (int i = 0; i < mapped_count; i++) {
        AVPacketSideData *mapped_packet = av_packet_side_data_new(
            &mapped_to_psd, &mapped_to_nb_psd, mapped_packet_types[i], 1, 0);
        fail_if(!mapped_packet, "av_packet_side_data_new mapped bridge seed failed");
        mapped_packet->data[0] = (uint8_t)(0xa0 + i);
        ret = av_packet_side_data_to_frame(&mapped_to_fsd, &mapped_to_nb_fsd,
                                           mapped_packet, 0);
        fail_if(ret < 0, "av_packet_side_data_to_frame mapped inventory failed");
    }
    print_frame_side_data_array_summary("packet:packet-to-frame-map-inventory",
                                        mapped_to_fsd, mapped_to_nb_fsd);
    av_frame_side_data_free(&mapped_to_fsd, &mapped_to_nb_fsd);
    av_packet_side_data_free(&mapped_to_psd, &mapped_to_nb_psd);

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

    AVPacketSideData *replace_flag_psd = av_mallocz(3 * sizeof(*replace_flag_psd));
    fail_if(!replace_flag_psd, "bridge replace-flag packet side-data seed failed");
    int replace_flag_nb_psd = 3;
    replace_flag_psd[0] = make_stack_side_data(AV_PKT_DATA_REPLAYGAIN, 0x11);
    replace_flag_psd[1] = make_stack_side_data(AV_PKT_DATA_DISPLAYMATRIX, 0x22);
    replace_flag_psd[2] = make_stack_side_data(AV_PKT_DATA_REPLAYGAIN, 0x33);
    AVFrameSideData **replace_flag_fsd = NULL;
    int replace_flag_nb_fsd = 0;
    AVFrameSideData *replace_flag_frame = av_frame_side_data_new(
        &replace_flag_fsd, &replace_flag_nb_fsd, AV_FRAME_DATA_REPLAYGAIN, 1, 0);
    fail_if(!replace_flag_frame, "av_frame_side_data_new bridge replace-flag seed failed");
    replace_flag_frame->data[0] = 0x55;
    ret = av_packet_side_data_from_frame(&replace_flag_psd, &replace_flag_nb_psd,
                                         replace_flag_frame,
                                         AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    printf("packet:frame-to-packet-replace-flag-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet-replace-flag",
                                  replace_flag_psd, replace_flag_nb_psd);
    av_frame_side_data_free(&replace_flag_fsd, &replace_flag_nb_fsd);
    av_packet_side_data_free(&replace_flag_psd, &replace_flag_nb_psd);

    AVPacketSideData *unique_psd = av_mallocz(3 * sizeof(*unique_psd));
    fail_if(!unique_psd, "bridge unique packet side-data seed failed");
    int unique_nb_psd = 3;
    unique_psd[0] = make_stack_side_data(AV_PKT_DATA_REPLAYGAIN, 0x11);
    unique_psd[1] = make_stack_side_data(AV_PKT_DATA_DISPLAYMATRIX, 0x22);
    unique_psd[2] = make_stack_side_data(AV_PKT_DATA_REPLAYGAIN, 0x33);
    AVFrameSideData *unique_frame = av_frame_side_data_new(
        &fsd, &nb_fsd, AV_FRAME_DATA_REPLAYGAIN, 1,
        AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    fail_if(!unique_frame, "av_frame_side_data_new bridge unique seed failed");
    unique_frame->data[0] = 0x44;
    ret = av_packet_side_data_from_frame(&unique_psd, &unique_nb_psd,
                                         unique_frame,
                                         AV_FRAME_SIDE_DATA_FLAG_UNIQUE);
    printf("packet:frame-to-packet-unique-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet-unique",
                                  unique_psd, unique_nb_psd);
    av_packet_side_data_free(&unique_psd, &unique_nb_psd);

    AVPacketSideData *combined_flag_psd = av_mallocz(3 * sizeof(*combined_flag_psd));
    fail_if(!combined_flag_psd, "bridge unique-replace packet side-data seed failed");
    int combined_flag_nb_psd = 3;
    combined_flag_psd[0] = make_stack_side_data(AV_PKT_DATA_REPLAYGAIN, 0x11);
    combined_flag_psd[1] = make_stack_side_data(AV_PKT_DATA_DISPLAYMATRIX, 0x22);
    combined_flag_psd[2] = make_stack_side_data(AV_PKT_DATA_REPLAYGAIN, 0x33);
    AVFrameSideData **combined_flag_fsd = NULL;
    int combined_flag_nb_fsd = 0;
    AVFrameSideData *combined_flag_frame = av_frame_side_data_new(
        &combined_flag_fsd, &combined_flag_nb_fsd,
        AV_FRAME_DATA_REPLAYGAIN, 1, 0);
    fail_if(!combined_flag_frame, "av_frame_side_data_new bridge unique-replace seed failed");
    combined_flag_frame->data[0] = 0x99;
    ret = av_packet_side_data_from_frame(&combined_flag_psd, &combined_flag_nb_psd,
                                         combined_flag_frame,
                                         AV_FRAME_SIDE_DATA_FLAG_UNIQUE |
                                             AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    printf("packet:frame-to-packet-unique-replace-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet-unique-replace",
                                  combined_flag_psd, combined_flag_nb_psd);
    av_frame_side_data_free(&combined_flag_fsd, &combined_flag_nb_fsd);
    av_packet_side_data_free(&combined_flag_psd, &combined_flag_nb_psd);

    AVPacketSideData *new_ref_psd = NULL;
    int new_ref_nb_psd = 0;
    AVFrameSideData **new_ref_fsd = NULL;
    int new_ref_nb_fsd = 0;
    AVFrameSideData *new_ref_frame = av_frame_side_data_new(
        &new_ref_fsd, &new_ref_nb_fsd, AV_FRAME_DATA_REPLAYGAIN, 2, 0);
    fail_if(!new_ref_frame, "av_frame_side_data_new bridge new-ref seed failed");
    new_ref_frame->data[0] = 0x66;
    new_ref_frame->data[1] = 0x77;
    ret = av_packet_side_data_from_frame(&new_ref_psd, &new_ref_nb_psd,
                                         new_ref_frame,
                                         AV_FRAME_SIDE_DATA_FLAG_NEW_REF);
    printf("packet:frame-to-packet-new-ref-ret|%d\n", ret);
    print_side_data_array_summary("packet:frame-to-packet-new-ref",
                                  new_ref_psd, new_ref_nb_psd);
    av_frame_side_data_free(&new_ref_fsd, &new_ref_nb_fsd);
    av_packet_side_data_free(&new_ref_psd, &new_ref_nb_psd);

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

    AVFrameSideData **replace_order_fsd = NULL;
    int replace_order_nb_fsd = 0;
    AVFrameSideData *replace_order_frame = av_frame_side_data_new(
        &replace_order_fsd, &replace_order_nb_fsd,
        AV_FRAME_DATA_REPLAYGAIN, 1, 0);
    fail_if(!replace_order_frame, "av_frame_side_data_new bridge replace-order seed failed");
    replace_order_frame->data[0] = 0x11;
    AVFrameSideData *replace_order_display = av_frame_side_data_new(
        &replace_order_fsd, &replace_order_nb_fsd,
        AV_FRAME_DATA_DISPLAYMATRIX, 1, 0);
    fail_if(!replace_order_display, "av_frame_side_data_new bridge replace-order display failed");
    replace_order_display->data[0] = 0x22;
    AVPacketSideData *replace_order_psd = NULL;
    int replace_order_nb_psd = 0;
    AVPacketSideData *replace_order_packet = av_packet_side_data_new(
        &replace_order_psd, &replace_order_nb_psd,
        AV_PKT_DATA_REPLAYGAIN, 1, 0);
    fail_if(!replace_order_packet, "av_packet_side_data_new bridge replace-order seed failed");
    replace_order_packet->data[0] = 0x55;
    ret = av_packet_side_data_to_frame(&replace_order_fsd, &replace_order_nb_fsd,
                                       replace_order_packet,
                                       AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    printf("packet:packet-to-frame-replace-order-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-replace-order",
                                        replace_order_fsd, replace_order_nb_fsd);
    av_frame_side_data_free(&replace_order_fsd, &replace_order_nb_fsd);
    av_packet_side_data_free(&replace_order_psd, &replace_order_nb_psd);

    AVFrameSideData *display_entry = av_frame_side_data_new(
        &fsd, &nb_fsd, AV_FRAME_DATA_DISPLAYMATRIX, 1, 0);
    fail_if(!display_entry, "av_frame_side_data_new bridge unique display seed failed");
    display_entry->data[0] = 0x22;
    packet_entry = av_packet_side_data_new(&psd, &nb_psd,
                                           AV_PKT_DATA_REPLAYGAIN,
                                           1, 0);
    fail_if(!packet_entry, "av_packet_side_data_new bridge unique seed failed");
    packet_entry->data[0] = 0x44;
    ret = av_packet_side_data_to_frame(&fsd, &nb_fsd, packet_entry,
                                       AV_FRAME_SIDE_DATA_FLAG_UNIQUE);
    printf("packet:packet-to-frame-unique-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-unique", fsd, nb_fsd);

    AVFrameSideData **unique_replace_fsd = NULL;
    int unique_replace_nb_fsd = 0;
    AVFrameSideData *unique_replace_replay = av_frame_side_data_new(
        &unique_replace_fsd, &unique_replace_nb_fsd,
        AV_FRAME_DATA_REPLAYGAIN, 1, 0);
    fail_if(!unique_replace_replay, "av_frame_side_data_new bridge to-frame unique-replace replay failed");
    unique_replace_replay->data[0] = 0x11;
    AVFrameSideData *unique_replace_display = av_frame_side_data_new(
        &unique_replace_fsd, &unique_replace_nb_fsd,
        AV_FRAME_DATA_DISPLAYMATRIX, 1, 0);
    fail_if(!unique_replace_display, "av_frame_side_data_new bridge to-frame unique-replace display failed");
    unique_replace_display->data[0] = 0x22;
    AVPacketSideData *unique_replace_psd = NULL;
    int unique_replace_nb_psd = 0;
    AVPacketSideData *unique_replace_packet = av_packet_side_data_new(
        &unique_replace_psd, &unique_replace_nb_psd,
        AV_PKT_DATA_REPLAYGAIN, 1, 0);
    fail_if(!unique_replace_packet, "av_packet_side_data_new bridge to-frame unique-replace seed failed");
    unique_replace_packet->data[0] = 0x99;
    ret = av_packet_side_data_to_frame(&unique_replace_fsd, &unique_replace_nb_fsd,
                                       unique_replace_packet,
                                       AV_FRAME_SIDE_DATA_FLAG_UNIQUE |
                                           AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    printf("packet:packet-to-frame-unique-replace-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-unique-replace",
                                        unique_replace_fsd, unique_replace_nb_fsd);
    av_frame_side_data_free(&unique_replace_fsd, &unique_replace_nb_fsd);
    av_packet_side_data_free(&unique_replace_psd, &unique_replace_nb_psd);

    AVPacketSideData *new_ref_to_psd = NULL;
    int new_ref_to_nb_psd = 0;
    AVFrameSideData **new_ref_to_fsd = NULL;
    int new_ref_to_nb_fsd = 0;
    AVPacketSideData *new_ref_packet = av_packet_side_data_new(
        &new_ref_to_psd, &new_ref_to_nb_psd, AV_PKT_DATA_REPLAYGAIN, 2, 0);
    fail_if(!new_ref_packet, "av_packet_side_data_new bridge to-frame new-ref seed failed");
    new_ref_packet->data[0] = 0x66;
    new_ref_packet->data[1] = 0x77;
    ret = av_packet_side_data_to_frame(&new_ref_to_fsd, &new_ref_to_nb_fsd,
                                       new_ref_packet,
                                       AV_FRAME_SIDE_DATA_FLAG_NEW_REF);
    printf("packet:packet-to-frame-new-ref-ret|%d\n", ret);
    print_frame_side_data_array_summary("packet:packet-to-frame-new-ref",
                                        new_ref_to_fsd, new_ref_to_nb_fsd);
    av_frame_side_data_free(&new_ref_to_fsd, &new_ref_to_nb_fsd);
    av_packet_side_data_free(&new_ref_to_psd, &new_ref_to_nb_psd);

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
    ret = av_new_packet(pkt, 0);
    printf("packet:payload-new-zero-ret|%d\n", ret);
    fail_if(ret < 0, "av_new_packet zero-size payload allocation failed");
    print_payload_allocation("packet:payload-new-zero", pkt);
    av_packet_free(&pkt);

    pkt = packet_with_common_props();
    ret = av_new_packet(pkt, 3);
    printf("packet:payload-new-packet-reset-ret|%d\n", ret);
    fail_if(ret < 0, "av_new_packet reset payload failed");
    pkt->data[0] = 0x10;
    pkt->data[1] = 0x20;
    pkt->data[2] = 0x30;
    print_packet("packet:payload-new-packet-reset", pkt);
    print_payload("packet:payload-new-packet-reset-payload", pkt);
    av_packet_free(&pkt);

    pkt = packet_with_common_props();
    ret = av_new_packet(pkt, INT_MAX - AV_INPUT_BUFFER_PADDING_SIZE);
    printf("packet:payload-new-packet-invalid-ret|%d\n", ret);
    print_packet("packet:payload-new-packet-invalid-preserve", pkt);
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
    uint8_t *zero_owned = av_mallocz(AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!zero_owned, "av_mallocz zero-size payload from-data failed");
    ret = av_packet_from_data(pkt, zero_owned, 0);
    printf("packet:payload-from-data-zero-ret|%d\n", ret);
    if (ret < 0) {
        av_free(zero_owned);
        fail_if(1, "av_packet_from_data zero-size payload failed");
    }
    print_payload("packet:payload-from-data-zero", pkt);
    av_packet_free(&pkt);

    pkt = packet_with_common_props_no_payload();
    uint8_t *preserve_owned = av_mallocz(3 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!preserve_owned, "av_mallocz preserve payload from-data failed");
    preserve_owned[0] = 0x10;
    preserve_owned[1] = 0x20;
    preserve_owned[2] = 0x30;
    ret = av_packet_from_data(pkt, preserve_owned, 3);
    printf("packet:payload-from-data-preserve-ret|%d\n", ret);
    if (ret < 0) {
        av_free(preserve_owned);
        fail_if(1, "av_packet_from_data preserve payload failed");
    }
    print_packet("packet:payload-from-data-preserve", pkt);
    print_payload("packet:payload-from-data-preserve-payload", pkt);
    av_packet_free(&pkt);

    pkt = packet_with_common_props_no_payload();
    ret = av_packet_from_data(pkt, NULL,
                              INT_MAX - AV_INPUT_BUFFER_PADDING_SIZE);
    printf("packet:payload-from-data-invalid-ret|%d\n", ret);
    print_packet("packet:payload-from-data-invalid-preserve", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    uint8_t *ref_raw = av_mallocz(2 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!ref_raw, "av_mallocz raw ref payload failed");
    ref_raw[0] = 0xaa;
    ref_raw[1] = 0xbb;
    pkt->data = ref_raw;
    pkt->size = 2;
    AVPacket *ref_dst = new_packet();
    ret = av_packet_ref(ref_dst, pkt);
    printf("packet:payload-ref-unrefcounted-ret|%d\n", ret);
    fail_if(ret < 0, "av_packet_ref raw payload failed");
    print_payload_visible("packet:payload-ref-unrefcounted-src", pkt);
    print_payload("packet:payload-ref-unrefcounted-dst", ref_dst);
    AVPacket *raw_cloned = av_packet_clone(pkt);
    fail_if(!raw_cloned, "av_packet_clone raw payload failed");
    print_payload("packet:payload-clone-unrefcounted", raw_cloned);
    av_packet_free(&raw_cloned);
    av_packet_free(&ref_dst);
    av_packet_free(&pkt);
    av_free(ref_raw);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet payload grow failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    ret = av_grow_packet(pkt, 3);
    printf("packet:payload-grow-ret|%d\n", ret);
    print_payload_prefix("packet:payload-grow", pkt, 2);
    av_packet_free(&pkt);

    pkt = packet_with_common_props_no_payload();
    uint8_t *grow_invalid_owned = av_mallocz(2 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!grow_invalid_owned, "av_mallocz grow invalid payload failed");
    grow_invalid_owned[0] = 0x44;
    grow_invalid_owned[1] = 0x55;
    ret = av_packet_from_data(pkt, grow_invalid_owned, 2);
    if (ret < 0) {
        av_free(grow_invalid_owned);
        fail_if(1, "av_packet_from_data grow invalid payload failed");
    }
    int invalid_grow_by = INT_MAX - (pkt->size + AV_INPUT_BUFFER_PADDING_SIZE) + 1;
    ret = av_grow_packet(pkt, invalid_grow_by);
    printf("packet:payload-grow-invalid-ret|%d\n", ret);
    print_packet("packet:payload-grow-invalid-preserve", pkt);
    print_payload("packet:payload-grow-invalid-payload", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet payload shrink failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    av_shrink_packet(pkt, 2);
    print_payload("packet:payload-shrink", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    ret = av_grow_packet(pkt, 3);
    printf("packet:payload-grow-empty-ret|%d\n", ret);
    fail_if(ret < 0, "av_grow_packet empty payload failed");
    print_payload_prefix("packet:payload-grow-empty", pkt, 0);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 3) < 0, "av_new_packet shrink edge payload failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    pkt->data[2] = 0xcc;
    av_shrink_packet(pkt, 9);
    print_payload("packet:payload-shrink-oversize", pkt);
    av_shrink_packet(pkt, 0);
    print_payload("packet:payload-shrink-zero", pkt);
    av_packet_free(&pkt);

    AVPacket *shared_grow_src = new_packet();
    fail_if(av_new_packet(shared_grow_src, 2) < 0, "av_new_packet shared grow src failed");
    shared_grow_src->data[0] = 0xaa;
    shared_grow_src->data[1] = 0xbb;
    AVPacket *shared_grow_dst = new_packet();
    fail_if(av_packet_ref(shared_grow_dst, shared_grow_src) < 0,
            "av_packet_ref shared grow dst failed");
    uint8_t *shared_grow_dst_ptr = shared_grow_dst->data;
    ret = av_grow_packet(shared_grow_dst, 2);
    printf("packet:payload-grow-shared-ret|%d\n", ret);
    fail_if(ret < 0, "av_grow_packet shared payload failed");
    printf("packet:payload-grow-shared-same-ptr|%d\n",
           shared_grow_dst->data == shared_grow_dst_ptr);
    print_payload("packet:payload-grow-shared-src", shared_grow_src);
    print_payload_prefix("packet:payload-grow-shared-dst", shared_grow_dst, 2);
    av_packet_free(&shared_grow_dst);
    av_packet_free(&shared_grow_src);

    pkt = new_packet();
    uint8_t grow_stack_data[2] = { 0xaa, 0xbb };
    pkt->data = grow_stack_data;
    pkt->size = 2;
    ret = av_grow_packet(pkt, 2);
    printf("packet:payload-grow-unrefcounted-ret|%d\n", ret);
    print_payload_prefix("packet:payload-grow-unrefcounted", pkt, 2);
    av_packet_free(&pkt);

    pkt = new_packet();
    uint8_t *shrink_raw = av_mallocz(4 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!shrink_raw, "av_mallocz raw shrink payload failed");
    shrink_raw[0] = 0xaa;
    shrink_raw[1] = 0xbb;
    shrink_raw[2] = 0xcc;
    shrink_raw[3] = 0xdd;
    pkt->data = shrink_raw;
    pkt->size = 4;
    av_shrink_packet(pkt, 2);
    print_payload_unowned("packet:payload-shrink-unrefcounted", pkt);
    av_packet_free(&pkt);
    av_free(shrink_raw);

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
    uint8_t writable_stack_data[2] = { 0xaa, 0xbb };
    pkt->data = writable_stack_data;
    pkt->size = 2;
    ret = av_packet_make_writable(pkt);
    printf("packet:payload-make-writable-unrefcounted-ret|%d\n", ret);
    pkt->data[0] = 0xcc;
    print_payload("packet:payload-make-writable-unrefcounted", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet unique writable no-op failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    uint8_t *unique_writable_ptr = pkt->data;
    ret = av_packet_make_writable(pkt);
    printf("packet:payload-make-writable-unique-ret|%d\n", ret);
    printf("packet:payload-make-writable-unique-same-ptr|%d\n",
           pkt->data == unique_writable_ptr);
    print_payload("packet:payload-make-writable-unique", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    uint8_t stack_data[2] = { 0xaa, 0xbb };
    pkt->data = stack_data;
    pkt->size = 2;
    ret = av_packet_make_refcounted(pkt);
    printf("packet:payload-make-refcounted-ret|%d\n", ret);
    print_payload("packet:payload-make-refcounted", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet unique refcounted no-op failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    uint8_t *unique_refcounted_ptr = pkt->data;
    ret = av_packet_make_refcounted(pkt);
    printf("packet:payload-make-refcounted-unique-ret|%d\n", ret);
    printf("packet:payload-make-refcounted-unique-same-ptr|%d\n",
           pkt->data == unique_refcounted_ptr);
    print_payload("packet:payload-make-refcounted-unique", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    uint8_t *readonly_owned = av_mallocz(2 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!readonly_owned, "av_mallocz readonly payload failed");
    readonly_owned[0] = 0xaa;
    readonly_owned[1] = 0xbb;
    pkt->buf = av_buffer_create(readonly_owned,
                                2 + AV_INPUT_BUFFER_PADDING_SIZE,
                                av_buffer_default_free, NULL,
                                AV_BUFFER_FLAG_READONLY);
    fail_if(!pkt->buf, "av_buffer_create readonly payload failed");
    pkt->data = readonly_owned;
    pkt->size = 2;
    uint8_t *readonly_ptr = pkt->data;
    ret = av_packet_make_refcounted(pkt);
    printf("packet:payload-make-refcounted-readonly-ret|%d\n", ret);
    printf("packet:payload-make-refcounted-readonly-same-ptr|%d\n",
           pkt->data == readonly_ptr);
    print_payload("packet:payload-make-refcounted-readonly", pkt);
    ret = av_packet_make_writable(pkt);
    printf("packet:payload-make-writable-readonly-ret|%d\n", ret);
    printf("packet:payload-make-writable-readonly-same-ptr|%d\n",
           pkt->data == readonly_ptr);
    print_payload("packet:payload-make-writable-readonly", pkt);
    av_packet_free(&pkt);

    AVPacket *shared_src = new_packet();
    fail_if(av_new_packet(shared_src, 2) < 0, "av_new_packet shared refcounted src failed");
    shared_src->data[0] = 0xaa;
    shared_src->data[1] = 0xbb;
    AVPacket *shared_dst = new_packet();
    fail_if(av_packet_ref(shared_dst, shared_src) < 0,
            "av_packet_ref shared refcounted dst failed");
    uint8_t *shared_dst_ptr = shared_dst->data;
    ret = av_packet_make_refcounted(shared_dst);
    printf("packet:payload-make-refcounted-shared-ret|%d\n", ret);
    printf("packet:payload-make-refcounted-shared-same-ptr|%d\n",
           shared_dst->data == shared_dst_ptr);
    print_payload("packet:payload-make-refcounted-shared-src", shared_src);
    print_payload("packet:payload-make-refcounted-shared-dst", shared_dst);
    av_packet_free(&shared_dst);
    av_packet_free(&shared_src);

    pkt = new_packet();
    ret = av_packet_make_refcounted(pkt);
    printf("packet:payload-make-refcounted-empty-ret|%d\n", ret);
    print_payload("packet:payload-make-refcounted-empty", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    ret = av_packet_make_writable(pkt);
    printf("packet:payload-make-writable-empty-ret|%d\n", ret);
    print_payload("packet:payload-make-writable-empty", pkt);
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

    dict = NULL;
    fail_if(av_dict_set(&dict, "title", "first", AV_DICT_MULTIKEY) < 0,
            "dict multikey set title failed");
    fail_if(av_dict_set(&dict, "TITLE", "second", AV_DICT_MULTIKEY) < 0,
            "dict multikey set TITLE failed");
    fail_if(av_dict_set(&dict, "artist", "Name", AV_DICT_MULTIKEY) < 0,
            "dict multikey set artist failed");
    packed_size = 999;
    packed = av_packet_pack_dictionary(dict, &packed_size);
    fail_if(!packed, "av_packet_pack_dictionary multikey failed");
    print_dictionary_payload("packet:dict-pack-multikey", packed, packed_size);

    unpacked = NULL;
    ret = av_packet_unpack_dictionary(packed, packed_size, &unpacked);
    printf("packet:dict-unpack-multikey-ret|%d\n", ret);
    print_dictionary("packet:dict-unpack-multikey", unpacked);
    av_dict_free(&unpacked);
    av_free(packed);
    av_dict_free(&dict);

    static const uint8_t duplicate[] = "title\0first\0TITLE\0second";
    ret = av_packet_unpack_dictionary(duplicate, sizeof(duplicate), &unpacked);
    printf("packet:dict-unpack-duplicate-ret|%d\n", ret);
    print_dictionary("packet:dict-unpack-duplicate", unpacked);
    av_dict_free(&unpacked);

    print_dictionary_unpack_ret("packet:dict-unpack-empty-ret", NULL, 0);
    static const uint8_t missing_final_nul[] = {
        't', 'i', 't', 'l', 'e', 0, 'C', 'l', 'i', 'p'
    };
    print_dictionary_unpack_ret("packet:dict-unpack-missing-final-nul-ret",
                                missing_final_nul, sizeof(missing_final_nul));
    static const uint8_t key_without_value[] = {
        't', 'i', 't', 'l', 'e', 0
    };
    print_dictionary_unpack_ret("packet:dict-unpack-key-without-value-ret",
                                key_without_value, sizeof(key_without_value));
    static const uint8_t empty_key[] = { 0, 'C', 'l', 'i', 'p', 0 };
    print_dictionary_unpack_ret("packet:dict-unpack-empty-key-ret",
                                empty_key, sizeof(empty_key));
    static const uint8_t trailing_empty_key[] = {
        't', 'i', 't', 'l', 'e', 0, 'C', 'l', 'i', 'p', 0, 0
    };
    print_dictionary_unpack_ret("packet:dict-unpack-trailing-empty-key-ret",
                                trailing_empty_key, sizeof(trailing_empty_key));
}

static void exercise_packet_fifo_api(void) {
    AVContainerFifo *fifo = av_container_fifo_alloc_avpacket(123);
    fail_if(!fifo, "av_container_fifo_alloc_avpacket failed");
    printf("packet:fifo-new-can-read|%zu\n", av_container_fifo_can_read(fifo));

    AVPacket *move_src = packet_with_common_props();
    int ret = av_container_fifo_write(fifo, move_src, 0);
    printf("packet:fifo-write-move-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_write move failed");
    print_packet("packet:fifo-write-move-src", move_src);
    printf("packet:fifo-after-write-move-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVPacket *peek = NULL;
    ret = av_container_fifo_peek(fifo, (void **)&peek, 0);
    printf("packet:fifo-peek0-ret|%d\n", ret);
    fail_if(ret < 0 || !peek, "av_container_fifo_peek 0 failed");
    print_packet("packet:fifo-peek0", peek);

    peek = NULL;
    ret = av_container_fifo_peek(fifo, (void **)&peek, 1);
    printf("packet:fifo-peek1-ret|%d\n", ret);

    AVPacket *move_dst = new_packet();
    fail_if(av_new_packet(move_dst, 1) < 0, "fifo move dst seed failed");
    move_dst->data[0] = 0x44;
    move_dst->stream_index = 9;
    ret = av_container_fifo_read(fifo, move_dst, 0);
    printf("packet:fifo-read-move-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_read move failed");
    print_packet("packet:fifo-read-move-dst", move_dst);
    printf("packet:fifo-after-read-move-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_packet_free(&move_dst);
    av_packet_free(&move_src);
    av_container_fifo_free(&fifo);

    fifo = av_container_fifo_alloc_avpacket(0);
    fail_if(!fifo, "av_container_fifo_alloc_avpacket ref failed");
    AVPacket *ref_src = packet_with_common_props();
    ret = av_container_fifo_write(fifo, ref_src, AV_CONTAINER_FIFO_FLAG_REF);
    printf("packet:fifo-write-ref-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_write ref failed");
    print_packet("packet:fifo-write-ref-src", ref_src);
    printf("packet:fifo-after-write-ref-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVPacket *ref_dst = new_packet();
    ret = av_container_fifo_read(fifo, ref_dst, AV_CONTAINER_FIFO_FLAG_REF);
    printf("packet:fifo-read-ref-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_read ref failed");
    print_packet("packet:fifo-read-ref-dst", ref_dst);
    printf("packet:fifo-after-read-ref-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_packet_free(&ref_dst);
    av_packet_free(&ref_src);

    AVPacket *ref_replace_src = packet_with_common_props();
    ret = av_container_fifo_write(
        fifo, ref_replace_src, AV_CONTAINER_FIFO_FLAG_REF);
    printf("packet:fifo-write-ref-replace-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_write ref replace failed");
    print_packet("packet:fifo-write-ref-replace-src", ref_replace_src);
    printf("packet:fifo-after-write-ref-replace-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVPacket *ref_replace_dst = new_packet();
    fail_if(av_new_packet(ref_replace_dst, 2) < 0,
            "fifo ref replace dst seed failed");
    ref_replace_dst->data[0] = 0x55;
    ref_replace_dst->data[1] = 0x44;
    ref_replace_dst->stream_index = 77;
    ref_replace_dst->pts = 22;
    ref_replace_dst->duration = 2;
    uint8_t *ref_replace_old_side = av_packet_new_side_data(
        ref_replace_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!ref_replace_old_side, "fifo ref replace old side data failed");
    ref_replace_old_side[0] = 0xee;
    ref_replace_dst->opaque = (void *)(uintptr_t)0x5678;
    ref_replace_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!ref_replace_dst->opaque_ref,
            "fifo ref replace old opaque_ref failed");
    ref_replace_dst->opaque_ref->data[0] = 0x99;
    ret = av_container_fifo_read(
        fifo, ref_replace_dst, AV_CONTAINER_FIFO_FLAG_REF);
    printf("packet:fifo-read-ref-replace-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_read ref replace failed");
    print_packet("packet:fifo-read-ref-replace-dst", ref_replace_dst);
    printf("packet:fifo-after-read-ref-replace-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_packet_free(&ref_replace_dst);
    av_packet_free(&ref_replace_src);

    AVPacket *move_replace_src = packet_with_common_props();
    ret = av_container_fifo_write(fifo, move_replace_src, 0);
    printf("packet:fifo-write-move-replace-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_write move replace failed");
    print_packet("packet:fifo-write-move-replace-src", move_replace_src);
    printf("packet:fifo-after-write-move-replace-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVPacket *move_replace_dst = new_packet();
    fail_if(av_new_packet(move_replace_dst, 2) < 0,
            "fifo move replace dst seed failed");
    move_replace_dst->data[0] = 0x66;
    move_replace_dst->data[1] = 0x77;
    move_replace_dst->stream_index = 88;
    move_replace_dst->pts = 33;
    move_replace_dst->duration = 3;
    uint8_t *move_replace_old_side = av_packet_new_side_data(
        move_replace_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!move_replace_old_side, "fifo move replace old side data failed");
    move_replace_old_side[0] = 0xab;
    move_replace_dst->opaque = (void *)(uintptr_t)0x6789;
    move_replace_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!move_replace_dst->opaque_ref,
            "fifo move replace old opaque_ref failed");
    move_replace_dst->opaque_ref->data[0] = 0xcd;
    ret = av_container_fifo_read(fifo, move_replace_dst, 0);
    printf("packet:fifo-read-move-replace-ret|%d\n", ret);
    fail_if(ret < 0, "av_container_fifo_read move replace failed");
    print_packet("packet:fifo-read-move-replace-dst", move_replace_dst);
    printf("packet:fifo-after-read-move-replace-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_packet_free(&move_replace_dst);
    av_packet_free(&move_replace_src);

    AVPacket *first = new_packet();
    fail_if(av_new_packet(first, 1) < 0, "fifo first seed failed");
    first->data[0] = 0x01;
    first->stream_index = 1;
    AVPacket *second = new_packet();
    fail_if(av_new_packet(second, 1) < 0, "fifo second seed failed");
    second->data[0] = 0x02;
    second->stream_index = 2;
    fail_if(av_container_fifo_write(fifo, first, 0) < 0,
            "fifo first write failed");
    fail_if(av_container_fifo_write(fifo, second, 0) < 0,
            "fifo second write failed");
    printf("packet:fifo-before-drain-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_container_fifo_drain(fifo, 0);
    printf("packet:fifo-after-drain-zero-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    peek = NULL;
    ret = av_container_fifo_peek(fifo, (void **)&peek, 0);
    fail_if(ret < 0 || !peek, "fifo peek after zero drain failed");
    print_packet("packet:fifo-after-drain-zero-peek", peek);
    av_container_fifo_drain(fifo, 1);
    printf("packet:fifo-after-drain-one-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    peek = NULL;
    ret = av_container_fifo_peek(fifo, (void **)&peek, 0);
    fail_if(ret < 0 || !peek, "fifo peek after drain failed");
    print_packet("packet:fifo-after-drain-one-peek", peek);
    av_container_fifo_drain(fifo, 1);
    printf("packet:fifo-after-drain-all-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    peek = NULL;
    ret = av_container_fifo_peek(fifo, (void **)&peek, 0);
    printf("packet:fifo-peek-empty-ret|%d\n", ret);

    AVPacket *empty_dst = new_packet();
    ret = av_container_fifo_read(fifo, empty_dst, 0);
    printf("packet:fifo-read-empty-ret|%d\n", ret);
    print_packet("packet:fifo-read-empty-dst", empty_dst);
    av_packet_free(&empty_dst);

    AVPacket *empty_move_preserve_dst = packet_with_common_props();
    ret = av_container_fifo_read(fifo, empty_move_preserve_dst, 0);
    printf("packet:fifo-read-empty-move-preserve-ret|%d\n", ret);
    print_packet("packet:fifo-read-empty-move-preserve-dst",
                 empty_move_preserve_dst);
    av_packet_free(&empty_move_preserve_dst);

    AVPacket *empty_ref_preserve_dst = packet_with_common_props();
    ret = av_container_fifo_read(
        fifo, empty_ref_preserve_dst, AV_CONTAINER_FIFO_FLAG_REF);
    printf("packet:fifo-read-empty-ref-preserve-ret|%d\n", ret);
    print_packet("packet:fifo-read-empty-ref-preserve-dst",
                 empty_ref_preserve_dst);
    av_packet_free(&empty_ref_preserve_dst);

    av_packet_free(&first);
    av_packet_free(&second);
    av_container_fifo_free(&fifo);
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
    print_side_data_name_boundaries();
    print_flag_inventory();
    print_picture_type_inventory();
    print_packet_abi_layout();
    print_side_data_payload_layouts();
    print_display_rotation_helpers();
    print_display_rotation_get_affine_helpers();
    print_display_flip_helpers();

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

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet mixed rescale failed");
    pkt->data[0] = 0xaa;
    pkt->data[1] = 0xbb;
    pkt->pts = AV_NOPTS_VALUE;
    pkt->dts = 90000;
    pkt->duration = 45000;
    pkt->pos = 123;
    pkt->stream_index = 2;
    pkt->flags = AV_PKT_FLAG_KEY;
    pkt->time_base = (AVRational){ 1, 90000 };
    av_packet_rescale_ts(pkt, (AVRational){ 1, 90000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale-mixed", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet mixed dts rescale failed");
    pkt->data[0] = 0xcc;
    pkt->data[1] = 0xdd;
    pkt->pts = 180000;
    pkt->dts = AV_NOPTS_VALUE;
    pkt->duration = 90000;
    pkt->pos = 456;
    pkt->stream_index = 4;
    pkt->flags = AV_PKT_FLAG_DISCARD;
    pkt->time_base = (AVRational){ 1, 90000 };
    av_packet_rescale_ts(pkt, (AVRational){ 1, 90000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale-mixed-dts", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 1) < 0, "av_new_packet zero duration rescale failed");
    pkt->data[0] = 0xee;
    pkt->pts = 180000;
    pkt->dts = 90000;
    pkt->duration = 0;
    pkt->pos = 789;
    pkt->stream_index = 5;
    pkt->flags = AV_PKT_FLAG_TRUSTED;
    pkt->time_base = (AVRational){ 1, 90000 };
    av_packet_rescale_ts(pkt, (AVRational){ 1, 90000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale-zero-duration", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 2) < 0, "av_new_packet negative timestamp rescale failed");
    pkt->data[0] = 0xde;
    pkt->data[1] = 0xad;
    pkt->pts = -180000;
    pkt->dts = -90000;
    pkt->duration = 45000;
    pkt->pos = 321;
    pkt->stream_index = 6;
    pkt->flags = AV_PKT_FLAG_CORRUPT;
    pkt->time_base = (AVRational){ 1, 90000 };
    av_packet_rescale_ts(pkt, (AVRational){ 1, 90000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale-negative-ts", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    fail_if(av_new_packet(pkt, 3) < 0, "av_new_packet near-inf rounding rescale failed");
    pkt->data[0] = 0x51;
    pkt->data[1] = 0x52;
    pkt->data[2] = 0x53;
    pkt->pts = 24;
    pkt->dts = 23;
    pkt->duration = 24;
    pkt->pos = 654;
    pkt->stream_index = 7;
    pkt->flags = AV_PKT_FLAG_DISPOSABLE;
    pkt->time_base = (AVRational){ 1, 48000 };
    av_packet_rescale_ts(pkt, (AVRational){ 1, 48000 }, (AVRational){ 1, 1000 });
    print_packet("packet:rescale-near-inf-rounding", pkt);
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

    AVPacket *empty_src = new_packet();
    AVPacket *copy_empty_dst = new_packet();
    fail_if(av_new_packet(copy_empty_dst, 2) < 0,
            "av_new_packet copy empty dst failed");
    copy_empty_dst->data[0] = 0x12;
    copy_empty_dst->data[1] = 0x34;
    copy_empty_dst->pts = 99;
    copy_empty_dst->duration = 9;
    copy_empty_dst->time_base = (AVRational){ 1, 1000 };
    uint8_t *copy_empty_old_side = av_packet_new_side_data(
        copy_empty_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!copy_empty_old_side, "copy props empty old side data failed");
    copy_empty_old_side[0] = 0xee;
    copy_empty_dst->opaque = (void *)(uintptr_t)0x5678;
    copy_empty_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!copy_empty_dst->opaque_ref,
            "copy props empty old opaque_ref failed");
    copy_empty_dst->opaque_ref->data[0] = 0x90;
    fail_if(av_packet_copy_props(copy_empty_dst, empty_src) < 0,
            "av_packet_copy_props empty source failed");
    print_packet("packet:copy-props-empty", copy_empty_dst);
    print_side_data_summary("packet:copy-props-empty-side", copy_empty_dst);
    print_payload_visible("packet:copy-props-empty-payload", copy_empty_dst);
    av_packet_free(&copy_empty_dst);

    AVPacket *copy_replace_src = packet_with_common_props();
    uint8_t *copy_replace_extra = av_packet_new_side_data(
        copy_replace_src, AV_PKT_DATA_SKIP_SAMPLES, 4);
    fail_if(!copy_replace_extra, "copy props replace source side data failed");
    copy_replace_extra[0] = 0x01;
    copy_replace_extra[1] = 0x02;
    copy_replace_extra[2] = 0x03;
    copy_replace_extra[3] = 0x04;

    AVPacket *copy_replace_dst = new_packet();
    fail_if(av_new_packet(copy_replace_dst, 2) < 0,
            "av_new_packet copy replace dst failed");
    copy_replace_dst->data[0] = 0x77;
    copy_replace_dst->data[1] = 0x66;
    copy_replace_dst->pts = 11;
    copy_replace_dst->duration = 1;
    copy_replace_dst->stream_index = 11;
    copy_replace_dst->time_base = (AVRational){ 1, 1000 };
    uint8_t *copy_replace_old_side = av_packet_new_side_data(
        copy_replace_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!copy_replace_old_side, "copy props replace old side data failed");
    copy_replace_old_side[0] = 0xee;
    copy_replace_dst->opaque = (void *)(uintptr_t)0x5678;
    copy_replace_dst->opaque_ref = av_buffer_alloc(2);
    fail_if(!copy_replace_dst->opaque_ref,
            "copy props replace old opaque_ref failed");
    copy_replace_dst->opaque_ref->data[0] = 0x99;
    copy_replace_dst->opaque_ref->data[1] = 0x00;

    fail_if(av_packet_copy_props(copy_replace_dst, copy_replace_src) < 0,
            "av_packet_copy_props replace failed");
    print_packet("packet:copy-props-replace", copy_replace_dst);
    print_side_data_summary("packet:copy-props-replace-side",
                            copy_replace_dst);
    print_payload_visible("packet:copy-props-replace-payload",
                          copy_replace_dst);
    av_packet_free(&copy_replace_dst);
    av_packet_free(&copy_replace_src);

    AVPacket *duplicate_src = packet_with_duplicate_side_data();
    AVPacket *duplicate_copy_dst = new_packet();
    uint8_t *duplicate_old_side = av_packet_new_side_data(
        duplicate_copy_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!duplicate_old_side, "copy props duplicate old side data failed");
    duplicate_old_side[0] = 0xee;
    fail_if(av_packet_copy_props(duplicate_copy_dst, duplicate_src) < 0,
            "av_packet_copy_props duplicate side data failed");
    print_side_data_summary("packet:copy-props-duplicate-side",
                            duplicate_copy_dst);

    AVPacket *duplicate_ref_dst = new_packet();
    fail_if(av_packet_ref(duplicate_ref_dst, duplicate_src) < 0,
            "av_packet_ref duplicate side data failed");
    print_side_data_summary("packet:ref-duplicate-side", duplicate_ref_dst);

    AVPacket *duplicate_cloned = av_packet_clone(duplicate_src);
    fail_if(!duplicate_cloned, "av_packet_clone duplicate side data failed");
    print_side_data_summary("packet:clone-duplicate-side", duplicate_cloned);

    av_packet_free(&duplicate_copy_dst);
    av_packet_free(&duplicate_ref_dst);
    av_packet_free(&duplicate_cloned);
    av_packet_free(&duplicate_src);

    AVPacket *duplicate_move_src = packet_with_duplicate_side_data();
    AVPacket *duplicate_move_dst = new_packet();
    duplicate_old_side = av_packet_new_side_data(
        duplicate_move_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!duplicate_old_side, "move duplicate old side data failed");
    duplicate_old_side[0] = 0xee;
    av_packet_move_ref(duplicate_move_dst, duplicate_move_src);
    print_side_data_summary("packet:move-duplicate-dst-side",
                            duplicate_move_dst);
    print_side_data_summary("packet:move-duplicate-src-side",
                            duplicate_move_src);
    av_packet_free(&duplicate_move_dst);
    av_packet_free(&duplicate_move_src);

    dst = new_packet();
    fail_if(av_packet_ref(dst, src) < 0, "av_packet_ref failed");
    print_packet("packet:ref", dst);
    av_packet_free(&dst);

    AVPacket *ref_empty_dst = new_packet();
    fail_if(av_new_packet(ref_empty_dst, 2) < 0,
            "av_new_packet ref empty dst failed");
    ref_empty_dst->data[0] = 0x55;
    ref_empty_dst->data[1] = 0x44;
    ref_empty_dst->pts = 22;
    ref_empty_dst->duration = 2;
    uint8_t *ref_empty_old_side = av_packet_new_side_data(
        ref_empty_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!ref_empty_old_side, "av_packet_ref empty old side data failed");
    ref_empty_old_side[0] = 0xee;
    ref_empty_dst->opaque = (void *)(uintptr_t)0x5678;
    ref_empty_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!ref_empty_dst->opaque_ref,
            "av_packet_ref empty old opaque_ref failed");
    ref_empty_dst->opaque_ref->data[0] = 0x99;
    fail_if(av_packet_ref(ref_empty_dst, empty_src) < 0,
            "av_packet_ref empty source failed");
    print_packet("packet:ref-empty", ref_empty_dst);
    print_payload("packet:payload-ref-empty", ref_empty_dst);
    av_packet_free(&ref_empty_dst);

    AVPacket *ref_replace_src = packet_with_common_props();
    uint8_t *ref_replace_extra = av_packet_new_side_data(
        ref_replace_src, AV_PKT_DATA_SKIP_SAMPLES, 2);
    fail_if(!ref_replace_extra, "av_packet_ref replace source side data failed");
    ref_replace_extra[0] = 0x05;
    ref_replace_extra[1] = 0x06;

    AVPacket *ref_replace_dst = new_packet();
    fail_if(av_new_packet(ref_replace_dst, 2) < 0,
            "av_new_packet ref replace dst failed");
    ref_replace_dst->data[0] = 0x55;
    ref_replace_dst->data[1] = 0x44;
    ref_replace_dst->pts = 22;
    ref_replace_dst->duration = 2;
    uint8_t *ref_replace_old_side = av_packet_new_side_data(
        ref_replace_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!ref_replace_old_side, "av_packet_ref replace old side data failed");
    ref_replace_old_side[0] = 0xee;
    ref_replace_dst->opaque = (void *)(uintptr_t)0x5678;
    ref_replace_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!ref_replace_dst->opaque_ref,
            "av_packet_ref replace old opaque_ref failed");
    ref_replace_dst->opaque_ref->data[0] = 0x99;

    fail_if(av_packet_ref(ref_replace_dst, ref_replace_src) < 0,
            "av_packet_ref replace failed");
    print_packet("packet:ref-replace", ref_replace_dst);
    print_side_data_summary("packet:ref-replace-side", ref_replace_dst);
    print_payload_visible("packet:ref-replace-payload", ref_replace_dst);
    av_packet_free(&ref_replace_dst);
    av_packet_free(&ref_replace_src);

    AVPacket *cloned = av_packet_clone(src);
    fail_if(!cloned, "av_packet_clone failed");
    print_packet("packet:clone", cloned);
    av_packet_free(&cloned);

    AVPacket *empty_cloned = av_packet_clone(empty_src);
    fail_if(!empty_cloned, "av_packet_clone empty source failed");
    print_packet("packet:clone-empty", empty_cloned);
    print_payload("packet:payload-clone-empty", empty_cloned);
    av_packet_free(&empty_cloned);
    av_packet_free(&empty_src);

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

    AVPacket *move_empty_src = new_packet();
    AVPacket *move_empty_dst = new_packet();
    fail_if(av_new_packet(move_empty_dst, 2) < 0,
            "av_new_packet move empty dst failed");
    move_empty_dst->data[0] = 0x33;
    move_empty_dst->data[1] = 0x22;
    move_empty_dst->pts = 77;
    move_empty_dst->duration = 7;
    uint8_t *move_empty_old_side = av_packet_new_side_data(
        move_empty_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!move_empty_old_side,
            "av_packet_move_ref empty old side data failed");
    move_empty_old_side[0] = 0xee;
    move_empty_dst->opaque = (void *)(uintptr_t)0x5678;
    move_empty_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!move_empty_dst->opaque_ref,
            "av_packet_move_ref empty old opaque_ref failed");
    move_empty_dst->opaque_ref->data[0] = 0x99;
    av_packet_move_ref(move_empty_dst, move_empty_src);
    print_packet("packet:move-empty-dst", move_empty_dst);
    print_packet("packet:move-empty-src", move_empty_src);
    av_packet_free(&move_empty_dst);
    av_packet_free(&move_empty_src);

    AVPacket *move_replace_src = packet_with_common_props();
    uint8_t *move_replace_extra = av_packet_new_side_data(
        move_replace_src, AV_PKT_DATA_SKIP_SAMPLES, 2);
    fail_if(!move_replace_extra,
            "av_packet_move_ref replace source side data failed");
    move_replace_extra[0] = 0x07;
    move_replace_extra[1] = 0x08;

    AVPacket *move_replace_dst = new_packet();
    fail_if(av_new_packet(move_replace_dst, 2) < 0,
            "av_new_packet move replace dst failed");
    move_replace_dst->data[0] = 0x33;
    move_replace_dst->data[1] = 0x22;
    move_replace_dst->pts = 77;
    move_replace_dst->duration = 7;
    uint8_t *move_replace_old_side = av_packet_new_side_data(
        move_replace_dst, AV_PKT_DATA_PALETTE, 1);
    fail_if(!move_replace_old_side,
            "av_packet_move_ref replace old side data failed");
    move_replace_old_side[0] = 0xee;
    move_replace_dst->opaque = (void *)(uintptr_t)0x5678;
    move_replace_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!move_replace_dst->opaque_ref,
            "av_packet_move_ref replace old opaque_ref failed");
    move_replace_dst->opaque_ref->data[0] = 0x99;

    av_packet_move_ref(move_replace_dst, move_replace_src);
    print_packet("packet:move-replace-dst", move_replace_dst);
    print_side_data_summary("packet:move-replace-dst-side",
                            move_replace_dst);
    print_payload_visible("packet:move-replace-dst-payload",
                          move_replace_dst);
    print_packet("packet:move-replace-src", move_replace_src);
    av_packet_free(&move_replace_dst);
    av_packet_free(&move_replace_src);

    pkt = packet_with_common_props();
    av_packet_unref(pkt);
    print_packet("packet:unref", pkt);
    av_packet_free(&pkt);

    pkt = new_packet();
    av_packet_unref(pkt);
    print_packet("packet:unref-empty", pkt);
    av_packet_free(&pkt);

    pkt = packet_with_common_props();
    av_packet_unref(pkt);
    av_packet_unref(pkt);
    print_packet("packet:unref-repeat", pkt);
    av_packet_free(&pkt);

    exercise_side_data_api();
    exercise_side_data_capacity_api();
    exercise_side_data_array_api();
    exercise_frame_packet_side_data_bridge_api();
    exercise_payload_api();
    exercise_dictionary_api();
    exercise_packet_fifo_api();

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
