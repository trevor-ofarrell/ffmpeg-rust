use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    AudioFrame, AvErrorCode, BufferRef, ChannelLayout, Frame, FrameAlphaMode, FrameChromaLocation,
    FrameColorPrimaries, FrameColorRange, FrameColorSpace, FrameColorTransferCharacteristic,
    FrameData, FrameDecodeErrorFlags, FrameFlags, FramePictureType, FrameSideData,
    FrameSideDataFlags, FrameSideDataKind, FrameSideDataProperties, PixelFormat, Rational,
    SampleFormat, VideoFrame, AV_NOPTS_VALUE,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_frame_core_lifecycle_matches_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/frame.h").is_file(),
        "missing pinned FFmpeg libavutil frame headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-frame");
    fs::create_dir_all(&work_dir).expect("create avutil-frame oracle work dir");
    let source = work_dir.join("frame_oracle.c");
    let executable = work_dir.join("frame_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-frame oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
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
        "frame:alloc-default".to_string(),
        frame_fields(&Frame::empty()),
    );
    rows.insert(
        "frame:side-kind-inventory".to_string(),
        frame_side_data_inventory_fields(),
    );
    rows.insert(
        "frame:side-flags".to_string(),
        vec![
            FrameSideDataFlags::UNIQUE.bits().to_string(),
            FrameSideDataFlags::REPLACE.bits().to_string(),
            FrameSideDataFlags::NEW_REF.bits().to_string(),
        ],
    );
    rows.insert(
        "frame:side-props".to_string(),
        vec![
            FrameSideDataProperties::GLOBAL.bits().to_string(),
            FrameSideDataProperties::MULTI.bits().to_string(),
            FrameSideDataProperties::SIZE_DEPENDENT.bits().to_string(),
            FrameSideDataProperties::COLOR_DEPENDENT.bits().to_string(),
            FrameSideDataProperties::CHANNEL_DEPENDENT
                .bits()
                .to_string(),
        ],
    );

    let mut video = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            2,
            3,
            PixelFormat::Gray8,
            vec![vec![1, 2, 3, 4, 5, 6]],
            1,
        )
        .unwrap(),
    );
    video.set_pts(Some(123));
    video.set_pkt_dts(Some(122));
    video.set_duration(121).unwrap();
    video
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    video
        .set_sample_aspect_ratio(Rational::new(16, 9).unwrap())
        .unwrap();
    video.set_crop_offsets(1, 2, 3, 4);
    video.set_picture_type(FramePictureType::P);
    video.set_quality(23);
    video.set_repeat_pict(2);
    video.set_flags(FrameFlags::KEY | FrameFlags::INTERLACED | FrameFlags::TOP_FIELD_FIRST);
    video.set_color_range(FrameColorRange::Jpeg);
    video.set_color_primaries(FrameColorPrimaries::Bt2020);
    video.set_color_transfer_characteristic(FrameColorTransferCharacteristic::Smpte2084);
    video.set_color_space(FrameColorSpace::Bt2020Ncl);
    video.set_chroma_location(FrameChromaLocation::TopLeft);
    video.set_best_effort_timestamp(Some(124));
    video.set_decode_error_flags(
        FrameDecodeErrorFlags::INVALID_BITSTREAM | FrameDecodeErrorFlags::CONCEALMENT_ACTIVE,
    );
    video.set_opaque_address(0x1234);
    video.set_opaque_ref(Some(BufferRef::copy_from_slice(&[0xde, 0xad])));
    video.set_alpha_mode(FrameAlphaMode::Premultiplied);
    video.metadata_mut().set("encoder", "oracle").unwrap();
    rows.insert("frame:video-buffer".to_string(), frame_fields(&video));

    let mut video_ref = Frame::empty();
    video_ref.ref_from(&video);
    rows.insert("frame:video-ref-src".to_string(), frame_fields(&video));
    rows.insert("frame:video-ref-dst".to_string(), frame_fields(&video_ref));
    rows.insert(
        "frame:video-ref-shares".to_string(),
        first_plane_share_fields(&video, &video_ref),
    );

    video_ref.make_writable();
    rows.insert(
        "frame:video-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "frame:video-after-make-writable-src".to_string(),
        frame_fields(&video),
    );
    rows.insert(
        "frame:video-after-make-writable-dst".to_string(),
        frame_fields(&video_ref),
    );
    rows.insert(
        "frame:video-after-make-writable-shares".to_string(),
        first_plane_share_fields(&video, &video_ref),
    );

    let mut move_dst = Frame::empty();
    move_dst.move_ref_from(&mut video_ref);
    rows.insert("frame:move-dst".to_string(), frame_fields(&move_dst));
    rows.insert("frame:move-src".to_string(), frame_fields(&video_ref));

    video.unref();
    rows.insert("frame:unref".to_string(), frame_fields(&video));

    let audio_payload = (1..=12).collect::<Vec<u8>>();
    let audio = Frame::audio(
        AudioFrame::new_with_channel_layout_and_aligned_line_sizes(
            48_000,
            ChannelLayout::stereo(),
            SampleFormat::S16,
            3,
            vec![audio_payload],
            1,
        )
        .unwrap(),
    );
    rows.insert("frame:audio-buffer".to_string(), frame_fields(&audio));

    let copy_source_side = (1..=36).collect::<Vec<u8>>();
    let copy_source_video =
        VideoFrame::new_with_aligned_line_sizes(2, 1, PixelFormat::Gray8, vec![vec![1, 2]], 1)
            .unwrap();
    let mut copy_source =
        Frame::video(copy_source_video).with_hw_frames_context(BufferRef::copy_from_slice(&[0xEE]));
    copy_source.set_pts(Some(321));
    copy_source.set_pkt_dts(Some(320));
    copy_source.set_duration(319).unwrap();
    copy_source
        .set_time_base(Rational::new(1, 48_000).unwrap())
        .unwrap();
    copy_source
        .set_sample_aspect_ratio(Rational::new(64, 45).unwrap())
        .unwrap();
    copy_source.set_crop_offsets(1, 2, 3, 4);
    copy_source.set_picture_type(FramePictureType::Bi);
    copy_source.set_quality(66);
    copy_source.set_repeat_pict(4);
    copy_source.set_flags(FrameFlags::KEY | FrameFlags::LOSSLESS);
    copy_source.set_color_range(FrameColorRange::Jpeg);
    copy_source.set_color_primaries(FrameColorPrimaries::Bt2020);
    copy_source.set_color_transfer_characteristic(FrameColorTransferCharacteristic::Smpte2084);
    copy_source.set_color_space(FrameColorSpace::Bt2020Ncl);
    copy_source.set_chroma_location(FrameChromaLocation::TopLeft);
    copy_source.set_best_effort_timestamp(Some(322));
    copy_source.set_decode_error_flags(
        FrameDecodeErrorFlags::MISSING_REFERENCE | FrameDecodeErrorFlags::DECODE_SLICES,
    );
    copy_source.set_opaque_address(0x2222);
    copy_source.set_opaque_ref(Some(BufferRef::copy_from_slice(&[0x22, 0x23, 0x24])));
    copy_source.set_alpha_mode(FrameAlphaMode::Straight);
    copy_source.metadata_mut().set("title", "source").unwrap();
    copy_source
        .metadata_mut()
        .set("artist", "libavutil")
        .unwrap();
    copy_source
        .set_side_data_kind_buffer(
            FrameSideDataKind::DisplayMatrix,
            BufferRef::copy_from_slice(&copy_source_side),
        )
        .unwrap();
    let copy_destination_video =
        VideoFrame::new_with_aligned_line_sizes(2, 1, PixelFormat::Gray8, vec![vec![9, 8]], 1)
            .unwrap();
    let mut copy_destination = Frame::video(copy_destination_video)
        .with_hw_frames_context(BufferRef::copy_from_slice(&[0xAA]));
    copy_destination.set_pts(Some(999));
    copy_destination.set_pkt_dts(Some(998));
    copy_destination.set_duration(997).unwrap();
    copy_destination
        .set_time_base(Rational::new(1, 1_000).unwrap())
        .unwrap();
    copy_destination
        .set_sample_aspect_ratio(Rational::new(1, 1).unwrap())
        .unwrap();
    copy_destination.set_crop_offsets(9, 8, 7, 6);
    copy_destination.set_picture_type(FramePictureType::I);
    copy_destination.set_quality(11);
    copy_destination.set_repeat_pict(1);
    copy_destination.set_flags(FrameFlags::CORRUPT);
    copy_destination.set_color_range(FrameColorRange::Mpeg);
    copy_destination.set_color_primaries(FrameColorPrimaries::Smpte170M);
    copy_destination.set_color_transfer_characteristic(FrameColorTransferCharacteristic::Bt709);
    copy_destination.set_color_space(FrameColorSpace::Smpte170M);
    copy_destination.set_chroma_location(FrameChromaLocation::Center);
    copy_destination.set_best_effort_timestamp(Some(1000));
    copy_destination.set_decode_error_flags(FrameDecodeErrorFlags::INVALID_BITSTREAM);
    copy_destination.set_opaque_address(0x3333);
    copy_destination.set_opaque_ref(Some(BufferRef::copy_from_slice(&[0x33])));
    copy_destination.set_alpha_mode(FrameAlphaMode::Premultiplied);
    copy_destination
        .metadata_mut()
        .set("title", "destination")
        .unwrap();
    copy_destination
        .metadata_mut()
        .set("keep", "destination")
        .unwrap();
    copy_destination
        .set_side_data_kind(FrameSideDataKind::DisplayMatrix, vec![0x99; 36])
        .unwrap();
    copy_destination.copy_props_from(&copy_source);
    rows.insert("frame:copy-props-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "frame:copy-props-src".to_string(),
        frame_fields(&copy_source),
    );
    rows.insert(
        "frame:copy-props-dst".to_string(),
        frame_fields(&copy_destination),
    );
    rows.insert(
        "frame:copy-props-plane-shares".to_string(),
        first_plane_share_fields(&copy_source, &copy_destination),
    );
    rows.insert(
        "frame:copy-props-side-shares".to_string(),
        first_side_data_share_fields(&copy_source, &copy_destination),
    );
    rows.insert(
        "frame:copy-props-hw-shares".to_string(),
        hw_frames_context_share_fields(&copy_source, &copy_destination),
    );

    let side_payload = (1..=36).collect::<Vec<u8>>();
    let mut side_frame = Frame::empty();
    side_frame
        .add_side_data_kind(FrameSideDataKind::DisplayMatrix, side_payload)
        .unwrap();
    rows.insert(
        "frame:side-data-new".to_string(),
        frame_side_data_fields(&side_frame),
    );
    let found = side_frame
        .side_data_by_kind(&FrameSideDataKind::DisplayMatrix)
        .is_some();
    side_frame.remove_side_data_kind(&FrameSideDataKind::DisplayMatrix);
    rows.insert(
        "frame:side-data-remove".to_string(),
        vec![bool_field(found), side_frame.side_data().len().to_string()],
    );

    let mut side_array = Frame::empty();
    let added = side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x11]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .is_ok();
    rows.insert(
        "frame:side-array-new".to_string(),
        side_array_status_fields(added, &side_array),
    );
    let duplicate = side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x22]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .is_ok();
    rows.insert(
        "frame:side-array-duplicate".to_string(),
        side_array_status_fields(duplicate, &side_array),
    );
    let replaced = side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x33, 0x44]).unwrap(),
            FrameSideDataFlags::REPLACE,
        )
        .is_ok();
    rows.insert(
        "frame:side-array-replace".to_string(),
        side_array_status_fields(replaced, &side_array),
    );
    side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0x55]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    let uniqued = side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0x66]).unwrap(),
            FrameSideDataFlags::UNIQUE,
        )
        .is_ok();
    rows.insert(
        "frame:side-array-unique".to_string(),
        side_array_status_fields(uniqued, &side_array),
    );
    side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::SeiUnregistered, vec![0x77; 16])
                .unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    let multi_replaced = side_array
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::SeiUnregistered, vec![0x88; 16])
                .unwrap(),
            FrameSideDataFlags::REPLACE,
        )
        .is_ok();
    rows.insert(
        "frame:side-array-multi-replace".to_string(),
        side_array_status_fields(multi_replaced, &side_array),
    );
    side_array.remove_side_data_by_properties(FrameSideDataProperties::MULTI);
    rows.insert(
        "frame:side-array-remove-multi".to_string(),
        side_array_fields(&side_array),
    );

    let mut side_add = Frame::empty();
    let mut take_source = Some(BufferRef::copy_from_slice(&[0xa1, 0xa2, 0xa3]));
    let take_success = side_add
        .add_side_data_kind_buffer_with_flags(
            FrameSideDataKind::ReplayGain,
            &mut take_source,
            FrameSideDataFlags::EMPTY,
        )
        .is_ok();
    rows.insert(
        "frame:side-add-take".to_string(),
        side_add_buffer_fields(
            take_success,
            &side_add,
            take_source.as_ref(),
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut duplicate_source = Some(BufferRef::copy_from_slice(&[0xb1]));
    let duplicate_success = side_add
        .add_side_data_kind_buffer_with_flags(
            FrameSideDataKind::ReplayGain,
            &mut duplicate_source,
            FrameSideDataFlags::EMPTY,
        )
        .is_ok();
    rows.insert(
        "frame:side-add-duplicate".to_string(),
        side_add_buffer_fields(
            duplicate_success,
            &side_add,
            duplicate_source.as_ref(),
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut replace_source = Some(BufferRef::copy_from_slice(&[0xc1, 0xc2]));
    let replace_success = side_add
        .add_side_data_kind_buffer_with_flags(
            FrameSideDataKind::ReplayGain,
            &mut replace_source,
            FrameSideDataFlags::REPLACE,
        )
        .is_ok();
    rows.insert(
        "frame:side-add-replace".to_string(),
        side_add_buffer_fields(
            replace_success,
            &side_add,
            replace_source.as_ref(),
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut ref_source = Some(BufferRef::copy_from_slice(&[0xd1, 0xd2]));
    let ref_success = side_add
        .add_side_data_kind_buffer_with_flags(
            FrameSideDataKind::DisplayMatrix,
            &mut ref_source,
            FrameSideDataFlags::NEW_REF,
        )
        .is_ok();
    rows.insert(
        "frame:side-add-new-ref".to_string(),
        side_add_buffer_fields(
            ref_success,
            &side_add,
            ref_source.as_ref(),
            &FrameSideDataKind::DisplayMatrix,
        ),
    );

    let mut clone_source = FrameSideData::new_with_kind_and_buffer_ref(
        FrameSideDataKind::ReplayGain,
        BufferRef::copy_from_slice(&[0xe1, 0xe2]),
    )
    .unwrap();
    clone_source.metadata_mut().set("gain", "source").unwrap();
    let mut side_clone = Frame::empty();
    let clone_new_ret = side_clone
        .clone_side_data_with_flags(&clone_source, FrameSideDataFlags::EMPTY)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:side-clone-new".to_string(),
        side_clone_fields(
            clone_new_ret,
            &side_clone,
            &clone_source,
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let clone_duplicate_ret = side_clone
        .clone_side_data_with_flags(&clone_source, FrameSideDataFlags::EMPTY)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:side-clone-duplicate".to_string(),
        side_clone_fields(
            clone_duplicate_ret,
            &side_clone,
            &clone_source,
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut clone_replacement = FrameSideData::new_with_kind_and_buffer_ref(
        FrameSideDataKind::ReplayGain,
        BufferRef::copy_from_slice(&[0xf1, 0xf2]),
    )
    .unwrap();
    clone_replacement
        .metadata_mut()
        .set("gain", "replacement")
        .unwrap();
    let clone_replace_ret = side_clone
        .clone_side_data_with_flags(&clone_replacement, FrameSideDataFlags::REPLACE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:side-clone-replace".to_string(),
        side_clone_fields(
            clone_replace_ret,
            &side_clone,
            &clone_replacement,
            &FrameSideDataKind::ReplayGain,
        ),
    );
    side_clone
        .add_side_data_with_flags(
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0xe5]).unwrap(),
            FrameSideDataFlags::EMPTY,
        )
        .unwrap();
    let clone_unique_ret = side_clone
        .clone_side_data_with_flags(&clone_source, FrameSideDataFlags::UNIQUE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:side-clone-unique".to_string(),
        side_clone_fields(
            clone_unique_ret,
            &side_clone,
            &clone_source,
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut clone_multi_source = FrameSideData::new_with_kind_and_buffer_ref(
        FrameSideDataKind::SeiUnregistered,
        BufferRef::copy_from_slice(&[0x77; 16]),
    )
    .unwrap();
    clone_multi_source
        .metadata_mut()
        .set("gain", "multi")
        .unwrap();
    let mut clone_multi = Frame::empty();
    let clone_multi_new_ret = clone_multi
        .clone_side_data_with_flags(&clone_multi_source, FrameSideDataFlags::EMPTY)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:side-clone-multi-new".to_string(),
        side_clone_fields(
            clone_multi_new_ret,
            &clone_multi,
            &clone_multi_source,
            &FrameSideDataKind::SeiUnregistered,
        ),
    );
    let clone_multi_replace_ret = clone_multi
        .clone_side_data_with_flags(&clone_multi_source, FrameSideDataFlags::REPLACE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:side-clone-multi-replace".to_string(),
        side_clone_fields(
            clone_multi_replace_ret,
            &clone_multi,
            &clone_multi_source,
            &FrameSideDataKind::SeiUnregistered,
        ),
    );

    rows
}

fn frame_fields(frame: &Frame) -> Vec<String> {
    let pts = frame.pts().unwrap_or(AV_NOPTS_VALUE);
    let pkt_dts = frame.pkt_dts().unwrap_or(AV_NOPTS_VALUE);
    let crop = frame.crop();
    let (kind, format, width, height, nb_samples, sample_rate, channels, line_sizes, planes) =
        match frame.data() {
            FrameData::Empty => (
                "empty",
                "none",
                0usize,
                0usize,
                0usize,
                0u32,
                0u16,
                Vec::new(),
                Vec::new(),
            ),
            FrameData::Video(video) => (
                "video",
                video.pixel_format_name(),
                video.width(),
                video.height(),
                0usize,
                0u32,
                0u16,
                video.line_sizes().to_vec(),
                video.planes().to_vec(),
            ),
            FrameData::Audio(audio) => (
                "audio",
                audio.sample_format_name(),
                0usize,
                0usize,
                audio.samples_per_channel(),
                audio.sample_rate(),
                audio.channels(),
                audio.line_sizes().to_vec(),
                audio.planes().to_vec(),
            ),
        };

    vec![
        pts.to_string(),
        pkt_dts.to_string(),
        frame.duration().to_string(),
        format!("{}/{}", frame.time_base().num(), frame.time_base().den()),
        format!(
            "{}/{}",
            frame.sample_aspect_ratio().num(),
            frame.sample_aspect_ratio().den()
        ),
        crop.top().to_string(),
        crop.bottom().to_string(),
        crop.left().to_string(),
        crop.right().to_string(),
        frame.picture_type().as_byte().to_string(),
        frame.picture_type().ffmpeg_char().to_string(),
        frame.quality().to_string(),
        frame.repeat_pict().to_string(),
        frame.flags().bits().to_string(),
        frame.color_range().as_raw().to_string(),
        frame.color_primaries().as_raw().to_string(),
        frame.color_transfer_characteristic().as_raw().to_string(),
        frame.color_space().as_raw().to_string(),
        frame.chroma_location().as_raw().to_string(),
        frame
            .best_effort_timestamp()
            .unwrap_or(AV_NOPTS_VALUE)
            .to_string(),
        frame.decode_error_flags().bits().to_string(),
        frame.opaque_address().unwrap_or(0).to_string(),
        opaque_ref_summary(frame.opaque_ref()),
        frame.alpha_mode().as_raw().to_string(),
        metadata_summary(frame.metadata()),
        kind.to_string(),
        format.to_string(),
        width.to_string(),
        height.to_string(),
        nb_samples.to_string(),
        sample_rate.to_string(),
        channels.to_string(),
        join_usizes(&line_sizes),
        join_hex_planes(&planes),
        bool_field(frame.is_writable()),
        frame.side_data().len().to_string(),
        side_summary(frame.side_data()),
        bool_field(frame.hw_frames_context().is_some()),
    ]
}

fn metadata_summary(metadata: &avutil::Dictionary) -> String {
    let mut values = Vec::new();
    for key in ["encoder", "title", "keep", "artist"] {
        if let Some(value) = metadata.get(key) {
            values.push(format!("{key}={value}"));
        }
    }

    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(";")
    }
}

fn frame_side_data_inventory_fields() -> Vec<String> {
    FrameSideDataKind::KNOWN
        .iter()
        .map(|kind| {
            let descriptor = kind
                .descriptor()
                .expect("known frame side data should have a descriptor");
            format!(
                "{}:{}:{}:{}:{}",
                kind.ffmpeg_value()
                    .expect("known frame side data should have a raw value"),
                kind.ffmpeg_constant()
                    .expect("known frame side data should have a constant"),
                kind.descriptor_name()
                    .expect("known frame side data should have a name"),
                descriptor.name(),
                descriptor.properties().bits()
            )
        })
        .collect()
}

fn frame_side_data_fields(frame: &Frame) -> Vec<String> {
    let side_data = frame
        .side_data()
        .first()
        .expect("expected one frame side data entry");
    vec![
        frame.side_data().len().to_string(),
        side_data
            .descriptor_name()
            .expect("displaymatrix side data descriptor")
            .to_string(),
        side_data.data().len().to_string(),
        hex(side_data.data()),
        side_data.buffer().strong_count().to_string(),
        bool_field(side_data.is_writable()),
    ]
}

fn side_array_status_fields(success: bool, frame: &Frame) -> Vec<String> {
    let mut fields = vec![bool_field(success)];
    fields.extend(side_array_fields(frame));
    fields
}

fn side_array_fields(frame: &Frame) -> Vec<String> {
    vec![
        frame.side_data().len().to_string(),
        side_summary(frame.side_data()),
    ]
}

fn side_add_buffer_fields(
    success: bool,
    frame: &Frame,
    source: Option<&BufferRef>,
    kind: &FrameSideDataKind,
) -> Vec<String> {
    let entry = frame
        .side_data()
        .iter()
        .find(|side_data| side_data.kind_id() == kind);
    vec![
        bool_field(success),
        bool_field(source.is_some()),
        frame.side_data().len().to_string(),
        side_summary(frame.side_data()),
        source
            .map(|buffer| hex(buffer.as_slice()))
            .unwrap_or_else(|| "none".to_string()),
        source
            .map(|buffer| buffer.strong_count().to_string())
            .unwrap_or_else(|| "0".to_string()),
        source
            .map(|buffer| bool_field(buffer.is_writable()))
            .unwrap_or_else(|| "0".to_string()),
        entry
            .map(|side_data| side_data.buffer().strong_count().to_string())
            .unwrap_or_else(|| "0".to_string()),
        entry
            .map(|side_data| bool_field(side_data.is_writable()))
            .unwrap_or_else(|| "0".to_string()),
        match (source, entry) {
            (Some(source), Some(entry)) => bool_field(source.shares_storage(entry.buffer())),
            _ => "0".to_string(),
        },
    ]
}

fn side_clone_fields(
    ret: i32,
    frame: &Frame,
    source: &FrameSideData,
    kind: &FrameSideDataKind,
) -> Vec<String> {
    let entry = frame
        .side_data()
        .iter()
        .find(|side_data| side_data.kind_id() == kind);
    vec![
        bool_field(ret >= 0),
        ret.to_string(),
        frame.side_data().len().to_string(),
        side_summary(frame.side_data()),
        source.metadata().get("gain").unwrap_or("none").to_string(),
        entry
            .and_then(|side_data| side_data.metadata().get("gain"))
            .unwrap_or("none")
            .to_string(),
        source.buffer().strong_count().to_string(),
        bool_field(source.is_writable()),
        entry
            .map(|side_data| side_data.buffer().strong_count().to_string())
            .unwrap_or_else(|| "0".to_string()),
        entry
            .map(|side_data| bool_field(side_data.is_writable()))
            .unwrap_or_else(|| "0".to_string()),
        entry
            .map(|side_data| bool_field(source.buffer().shares_storage(side_data.buffer())))
            .unwrap_or_else(|| "0".to_string()),
    ]
}

fn side_summary(side_data: &[FrameSideData]) -> String {
    if side_data.is_empty() {
        return "none".to_string();
    }

    side_data
        .iter()
        .map(|side_data| {
            format!(
                "{}:{}:{}",
                side_data.descriptor_name().unwrap_or(side_data.kind()),
                side_data.data().len(),
                hex(side_data.data())
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn first_plane_share_fields(left: &Frame, right: &Frame) -> Vec<String> {
    let left = first_plane_buffer(left);
    let right = first_plane_buffer(right);
    buffer_share_fields(left, right)
}

fn first_side_data_share_fields(left: &Frame, right: &Frame) -> Vec<String> {
    let left = left
        .side_data()
        .first()
        .expect("left frame should have side data")
        .buffer();
    let right = right
        .side_data()
        .last()
        .expect("right frame should have side data")
        .buffer();
    buffer_share_fields(left, right)
}

fn hw_frames_context_share_fields(left: &Frame, right: &Frame) -> Vec<String> {
    let left = left
        .hw_frames_context()
        .expect("left frame should have hw context");
    let right = right
        .hw_frames_context()
        .expect("right frame should have hw context");
    buffer_share_fields(left, right)
}

fn buffer_share_fields(left: &BufferRef, right: &BufferRef) -> Vec<String> {
    vec![
        bool_field(left.shares_storage(right)),
        left.strong_count().to_string(),
        right.strong_count().to_string(),
        bool_field(left.is_writable()),
        bool_field(right.is_writable()),
    ]
}

fn first_plane_buffer(frame: &Frame) -> &BufferRef {
    match frame.data() {
        FrameData::Video(video) => &video.plane_buffers()[0],
        FrameData::Audio(audio) => &audio.plane_buffers()[0],
        FrameData::Empty => panic!("empty frame has no plane buffer"),
    }
}

fn join_usizes(values: &[usize]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn join_hex_planes(planes: &[Vec<u8>]) -> String {
    if planes.is_empty() {
        return "none".to_string();
    }
    planes
        .iter()
        .map(|plane| hex(plane))
        .collect::<Vec<_>>()
        .join(",")
}

fn hex(data: &[u8]) -> String {
    let mut output = String::with_capacity(data.len() * 2);
    for byte in data {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn opaque_ref_summary(buffer: Option<&BufferRef>) -> String {
    match buffer {
        Some(buffer) => format!(
            "{}:{}:{}:{}",
            buffer.len(),
            hex(buffer.as_slice()),
            buffer.strong_count(),
            bool_field(buffer.is_writable())
        ),
        None => "none".to_string(),
    }
}

fn bool_field(value: bool) -> String {
    if value { "1" } else { "0" }.to_string()
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let mut parts = line.split('|');
        let name = parts.next().expect("row name").to_string();
        let fields = parts.map(str::to_string).collect::<Vec<_>>();
        assert!(!fields.is_empty(), "oracle row `{line}` has no fields");
        assert!(
            rows.insert(name, fields).is_none(),
            "duplicate oracle row `{line}`"
        );
    }
    rows
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
}

fn compile_and_run_oracle(
    include_dir: &Path,
    libavutil: &Path,
    source: &Path,
    executable: &Path,
) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "gcc -I {} {} {} -lm -pthread -ldl -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavutil)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavutil frame oracle")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gcc -I {} {} {} -lm -pthread -ldl -o {} && {}",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavutil.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavutil frame oracle")
    };

    assert!(
        output.status.success(),
        "libavutil frame oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
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
#include <libavutil/avutil.h>
#include <libavutil/buffer.h>
#include <libavutil/channel_layout.h>
#include <libavutil/dict.h>
#include <libavutil/frame.h>
#include <libavutil/imgutils.h>
#include <libavutil/pixdesc.h>
#include <libavutil/pixfmt.h>
#include <libavutil/samplefmt.h>

typedef struct SideKind {
    enum AVFrameSideDataType type;
    const char *constant;
} SideKind;

static void fail_if(int condition, const char *message)
{
    if (condition) {
        fprintf(stderr, "%s\n", message);
        exit(1);
    }
}

static void print_hex(const uint8_t *data, size_t len)
{
    for (size_t i = 0; i < len; i++)
        printf("%02x", data[i]);
}

static void print_buffer_ref_summary(const AVBufferRef *ref)
{
    if (!ref) {
        printf("none");
        return;
    }
    printf("%zu:", ref->size);
    print_hex(ref->data, ref->size);
    printf(":%d:%d", av_buffer_get_ref_count(ref),
           av_buffer_is_writable((AVBufferRef *)ref));
}

static void print_line_sizes(const int *line_sizes, int count)
{
    if (count <= 0) {
        printf("none");
        return;
    }
    for (int i = 0; i < count; i++) {
        if (i)
            printf(",");
        printf("%d", line_sizes[i]);
    }
}

static void print_video_planes(const AVFrame *frame)
{
    if (frame->width <= 0 || frame->height <= 0 || frame->data[0] == NULL) {
        printf("none");
        return;
    }

    if (frame->format == AV_PIX_FMT_GRAY8) {
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0], frame->width);
        return;
    }

    printf("unsupported");
}

static void print_audio_planes(const AVFrame *frame)
{
    if (frame->nb_samples <= 0 || frame->ch_layout.nb_channels <= 0 ||
        frame->data[0] == NULL) {
        printf("none");
        return;
    }

    int bytes = av_get_bytes_per_sample(frame->format);
    fail_if(bytes <= 0, "audio bytes-per-sample lookup failed");
    int plane_count = av_sample_fmt_is_planar(frame->format)
                          ? frame->ch_layout.nb_channels
                          : 1;
    int visible = av_sample_fmt_is_planar(frame->format)
                      ? frame->nb_samples * bytes
                      : frame->nb_samples * frame->ch_layout.nb_channels * bytes;

    for (int plane = 0; plane < plane_count; plane++) {
        if (plane)
            printf(",");
        print_hex(frame->data[plane], visible);
    }
}

static void print_side_summary(const AVFrame *frame)
{
    if (frame->nb_side_data == 0) {
        printf("none");
        return;
    }

    for (int i = 0; i < frame->nb_side_data; i++) {
        const AVFrameSideData *sd = frame->side_data[i];
        const char *name = av_frame_side_data_name(sd->type);
        if (i)
            printf(";");
        printf("%s:%zu:", name ? name : "unknown", sd->size);
        print_hex(sd->data, sd->size);
    }
}

static void print_metadata_summary(const AVDictionary *metadata)
{
    const char *keys[] = { "encoder", "title", "keep", "artist" };
    int printed = 0;

    for (size_t i = 0; i < sizeof(keys) / sizeof(keys[0]); i++) {
        const AVDictionaryEntry *entry =
            av_dict_get(metadata, keys[i], NULL, 0);
        if (!entry)
            continue;
        if (printed)
            printf(";");
        printf("%s=%s", keys[i], entry->value);
        printed = 1;
    }

    if (!printed)
        printf("none");
}

static void print_side_array_summary(AVFrameSideData * const *sd, int nb_sd)
{
    if (nb_sd == 0) {
        printf("none");
        return;
    }

    for (int i = 0; i < nb_sd; i++) {
        const AVFrameSideData *entry = sd[i];
        const char *name = av_frame_side_data_name(entry->type);
        if (i)
            printf(";");
        printf("%s:%zu:", name ? name : "unknown", entry->size);
        print_hex(entry->data, entry->size);
    }
}

static void print_side_array_row(const char *name, int success,
                                 AVFrameSideData * const *sd, int nb_sd)
{
    printf("%s|%d|%d|", name, success, nb_sd);
    print_side_array_summary(sd, nb_sd);
    printf("\n");
}

static const AVFrameSideData *find_side_array_entry(AVFrameSideData * const *sd,
                                                    int nb_sd,
                                                    enum AVFrameSideDataType type)
{
    for (int i = 0; i < nb_sd; i++) {
        if (sd[i]->type == type)
            return sd[i];
    }
    return NULL;
}

static void print_side_add_buffer_row(const char *name, int success,
                                      AVFrameSideData * const *sd, int nb_sd,
                                      const AVBufferRef *source,
                                      enum AVFrameSideDataType type)
{
    const AVFrameSideData *entry = find_side_array_entry(sd, nb_sd, type);
    printf("%s|%d|%d|%d|", name, success, source != NULL, nb_sd);
    print_side_array_summary(sd, nb_sd);
    printf("|");
    if (source)
        print_hex(source->data, source->size);
    else
        printf("none");
    printf("|%d|%d|%d|%d|%d\n",
           source ? av_buffer_get_ref_count((AVBufferRef *)source) : 0,
           source ? av_buffer_is_writable((AVBufferRef *)source) : 0,
           entry && entry->buf ? av_buffer_get_ref_count(entry->buf) : 0,
           entry && entry->buf ? av_buffer_is_writable(entry->buf) : 0,
           source && entry ? source->data == entry->data : 0);
}

static const char *side_metadata_value(const AVFrameSideData *sd,
                                       const char *key)
{
    const AVDictionaryEntry *entry =
        sd ? av_dict_get(sd->metadata, key, NULL, 0) : NULL;
    return entry ? entry->value : "none";
}

static void print_side_clone_row(const char *name, int ret,
                                 AVFrameSideData * const *sd, int nb_sd,
                                 const AVFrameSideData *source,
                                 enum AVFrameSideDataType type)
{
    const AVFrameSideData *entry = find_side_array_entry(sd, nb_sd, type);
    printf("%s|%d|%d|%d|", name, ret >= 0, ret, nb_sd);
    print_side_array_summary(sd, nb_sd);
    printf("|%s|%s|%d|%d|%d|%d|%d\n",
           side_metadata_value(source, "gain"),
           side_metadata_value(entry, "gain"),
           source && source->buf ? av_buffer_get_ref_count(source->buf) : 0,
           source && source->buf
               ? av_buffer_is_writable((AVBufferRef *)source->buf)
               : 0,
           entry && entry->buf ? av_buffer_get_ref_count(entry->buf) : 0,
           entry && entry->buf ? av_buffer_is_writable(entry->buf) : 0,
           source && entry ? source->data == entry->data : 0);
}

static void print_frame(const char *name, const AVFrame *frame)
{
    const char *kind = "empty";
    const char *format = "none";
    int plane_count = 0;

    if (frame->width > 0 && frame->height > 0) {
        kind = "video";
        format = av_get_pix_fmt_name(frame->format);
        plane_count = 1;
    } else if (frame->nb_samples > 0 && frame->ch_layout.nb_channels > 0) {
        kind = "audio";
        format = av_get_sample_fmt_name(frame->format);
        plane_count = av_sample_fmt_is_planar(frame->format)
                          ? frame->ch_layout.nb_channels
                          : 1;
    }

    printf("%s|%" PRId64 "|%" PRId64 "|%" PRId64 "|%d/%d|%d/%d|%zu|%zu|%zu|%zu|%d|%c|%d|%d|%d|%d|%d|%d|%d|%d|%" PRId64 "|%d|%" PRIuPTR "|",
           name, frame->pts, frame->pkt_dts, frame->duration,
           frame->time_base.num, frame->time_base.den,
           frame->sample_aspect_ratio.num, frame->sample_aspect_ratio.den,
           frame->crop_top, frame->crop_bottom, frame->crop_left,
           frame->crop_right, frame->pict_type,
           av_get_picture_type_char(frame->pict_type), frame->quality,
           frame->repeat_pict, frame->flags, frame->color_range,
           frame->color_primaries, frame->color_trc, frame->colorspace,
           frame->chroma_location, frame->best_effort_timestamp,
           frame->decode_error_flags, (uintptr_t)frame->opaque);
    print_buffer_ref_summary(frame->opaque_ref);
    printf("|%d|", frame->alpha_mode);
    print_metadata_summary(frame->metadata);
    printf("|%s|%s|%d|%d|%d|%d|%d|", kind, format ? format : "none",
           frame->width, frame->height, frame->nb_samples, frame->sample_rate,
           frame->ch_layout.nb_channels);
    print_line_sizes(frame->linesize, plane_count);
    printf("|");
    if (strcmp(kind, "video") == 0)
        print_video_planes(frame);
    else if (strcmp(kind, "audio") == 0)
        print_audio_planes(frame);
    else
        printf("none");
    printf("|%d|%d|", av_frame_is_writable((AVFrame *)frame),
           frame->nb_side_data);
    print_side_summary(frame);
    printf("|%d\n", frame->hw_frames_ctx != NULL);
}

static void print_share(const char *name, const AVFrame *left,
                        const AVFrame *right)
{
    fail_if(!left->buf[0] || !right->buf[0], "missing frame buffer ref");
    printf("%s|%d|%d|%d|%d|%d\n", name, left->data[0] == right->data[0],
           av_buffer_get_ref_count(left->buf[0]),
           av_buffer_get_ref_count(right->buf[0]),
           av_buffer_is_writable(left->buf[0]),
           av_buffer_is_writable(right->buf[0]));
}

static void print_buffer_ref_share(const char *name, const AVBufferRef *left,
                                   const AVBufferRef *right)
{
    fail_if(!left || !right, "missing buffer refs to compare");
    printf("%s|%d|%d|%d|%d|%d\n", name, left->data == right->data,
           av_buffer_get_ref_count(left), av_buffer_get_ref_count(right),
           av_buffer_is_writable(left), av_buffer_is_writable(right));
}

static void print_side_share(const char *name, const AVFrame *left,
                             const AVFrame *right)
{
    fail_if(left->nb_side_data <= 0 || right->nb_side_data <= 0,
            "missing frame side data refs");
    print_buffer_ref_share(name, left->side_data[0]->buf,
                           right->side_data[right->nb_side_data - 1]->buf);
}

static void print_hw_share(const char *name, const AVFrame *left,
                           const AVFrame *right)
{
    print_buffer_ref_share(name, left->hw_frames_ctx, right->hw_frames_ctx);
}

static void print_side_kind_inventory(void)
{
    const SideKind kinds[] = {
        { AV_FRAME_DATA_PANSCAN, "AV_FRAME_DATA_PANSCAN" },
        { AV_FRAME_DATA_A53_CC, "AV_FRAME_DATA_A53_CC" },
        { AV_FRAME_DATA_STEREO3D, "AV_FRAME_DATA_STEREO3D" },
        { AV_FRAME_DATA_MATRIXENCODING, "AV_FRAME_DATA_MATRIXENCODING" },
        { AV_FRAME_DATA_DOWNMIX_INFO, "AV_FRAME_DATA_DOWNMIX_INFO" },
        { AV_FRAME_DATA_REPLAYGAIN, "AV_FRAME_DATA_REPLAYGAIN" },
        { AV_FRAME_DATA_DISPLAYMATRIX, "AV_FRAME_DATA_DISPLAYMATRIX" },
        { AV_FRAME_DATA_AFD, "AV_FRAME_DATA_AFD" },
        { AV_FRAME_DATA_MOTION_VECTORS, "AV_FRAME_DATA_MOTION_VECTORS" },
        { AV_FRAME_DATA_SKIP_SAMPLES, "AV_FRAME_DATA_SKIP_SAMPLES" },
        { AV_FRAME_DATA_AUDIO_SERVICE_TYPE, "AV_FRAME_DATA_AUDIO_SERVICE_TYPE" },
        { AV_FRAME_DATA_MASTERING_DISPLAY_METADATA, "AV_FRAME_DATA_MASTERING_DISPLAY_METADATA" },
        { AV_FRAME_DATA_GOP_TIMECODE, "AV_FRAME_DATA_GOP_TIMECODE" },
        { AV_FRAME_DATA_SPHERICAL, "AV_FRAME_DATA_SPHERICAL" },
        { AV_FRAME_DATA_CONTENT_LIGHT_LEVEL, "AV_FRAME_DATA_CONTENT_LIGHT_LEVEL" },
        { AV_FRAME_DATA_ICC_PROFILE, "AV_FRAME_DATA_ICC_PROFILE" },
        { AV_FRAME_DATA_S12M_TIMECODE, "AV_FRAME_DATA_S12M_TIMECODE" },
        { AV_FRAME_DATA_DYNAMIC_HDR_PLUS, "AV_FRAME_DATA_DYNAMIC_HDR_PLUS" },
        { AV_FRAME_DATA_REGIONS_OF_INTEREST, "AV_FRAME_DATA_REGIONS_OF_INTEREST" },
        { AV_FRAME_DATA_VIDEO_ENC_PARAMS, "AV_FRAME_DATA_VIDEO_ENC_PARAMS" },
        { AV_FRAME_DATA_SEI_UNREGISTERED, "AV_FRAME_DATA_SEI_UNREGISTERED" },
        { AV_FRAME_DATA_FILM_GRAIN_PARAMS, "AV_FRAME_DATA_FILM_GRAIN_PARAMS" },
        { AV_FRAME_DATA_DETECTION_BBOXES, "AV_FRAME_DATA_DETECTION_BBOXES" },
        { AV_FRAME_DATA_DOVI_RPU_BUFFER, "AV_FRAME_DATA_DOVI_RPU_BUFFER" },
        { AV_FRAME_DATA_DOVI_METADATA, "AV_FRAME_DATA_DOVI_METADATA" },
        { AV_FRAME_DATA_DYNAMIC_HDR_VIVID, "AV_FRAME_DATA_DYNAMIC_HDR_VIVID" },
        { AV_FRAME_DATA_AMBIENT_VIEWING_ENVIRONMENT, "AV_FRAME_DATA_AMBIENT_VIEWING_ENVIRONMENT" },
        { AV_FRAME_DATA_VIDEO_HINT, "AV_FRAME_DATA_VIDEO_HINT" },
        { AV_FRAME_DATA_LCEVC, "AV_FRAME_DATA_LCEVC" },
        { AV_FRAME_DATA_VIEW_ID, "AV_FRAME_DATA_VIEW_ID" },
        { AV_FRAME_DATA_3D_REFERENCE_DISPLAYS, "AV_FRAME_DATA_3D_REFERENCE_DISPLAYS" },
        { AV_FRAME_DATA_EXIF, "AV_FRAME_DATA_EXIF" },
    };

    printf("frame:side-kind-inventory");
    for (size_t i = 0; i < sizeof(kinds) / sizeof(kinds[0]); i++) {
        const AVSideDataDescriptor *desc =
            av_frame_side_data_desc(kinds[i].type);
        const char *name = av_frame_side_data_name(kinds[i].type);
        printf("|%d:%s:%s:%s:%u", kinds[i].type, kinds[i].constant,
               name ? name : "", desc ? desc->name : "",
               desc ? desc->props : 0);
    }
    printf("\n");
}

static void fill_video_gray(AVFrame *frame, const uint8_t *data)
{
    for (int row = 0; row < frame->height; row++)
        memcpy(frame->data[0] + row * frame->linesize[0],
               data + row * frame->width, frame->width);
}

static void print_side_data_row(const char *name, const AVFrame *frame,
                                const AVFrameSideData *sd)
{
    const char *side_name = av_frame_side_data_name(sd->type);
    printf("%s|%d|%s|%zu|", name, frame->nb_side_data,
           side_name ? side_name : "unknown", sd->size);
    print_hex(sd->data, sd->size);
    printf("|%d|%d\n", sd->buf ? av_buffer_get_ref_count(sd->buf) : 0,
           sd->buf ? av_buffer_is_writable(sd->buf) : 0);
}

int main(void)
{
    AVFrame *empty = av_frame_alloc();
    fail_if(!empty, "av_frame_alloc failed");
    print_frame("frame:alloc-default", empty);
    av_frame_free(&empty);

    print_side_kind_inventory();
    printf("frame:side-flags|%u|%u|%u\n",
           AV_FRAME_SIDE_DATA_FLAG_UNIQUE,
           AV_FRAME_SIDE_DATA_FLAG_REPLACE,
           AV_FRAME_SIDE_DATA_FLAG_NEW_REF);
    printf("frame:side-props|%u|%u|%u|%u|%u\n",
           AV_SIDE_DATA_PROP_GLOBAL, AV_SIDE_DATA_PROP_MULTI,
           AV_SIDE_DATA_PROP_SIZE_DEPENDENT,
           AV_SIDE_DATA_PROP_COLOR_DEPENDENT,
           AV_SIDE_DATA_PROP_CHANNEL_DEPENDENT);

    AVFrame *video = av_frame_alloc();
    fail_if(!video, "video av_frame_alloc failed");
    video->format = AV_PIX_FMT_GRAY8;
    video->width = 2;
    video->height = 3;
    video->pts = 123;
    video->pkt_dts = 122;
    video->duration = 121;
    video->time_base = (AVRational){ 1, 90000 };
    video->sample_aspect_ratio = (AVRational){ 16, 9 };
    video->crop_top = 1;
    video->crop_bottom = 2;
    video->crop_left = 3;
    video->crop_right = 4;
    video->pict_type = AV_PICTURE_TYPE_P;
    video->quality = 23;
    video->repeat_pict = 2;
    video->flags = AV_FRAME_FLAG_KEY | AV_FRAME_FLAG_INTERLACED |
                   AV_FRAME_FLAG_TOP_FIELD_FIRST;
    video->color_range = AVCOL_RANGE_JPEG;
    video->color_primaries = AVCOL_PRI_BT2020;
    video->color_trc = AVCOL_TRC_SMPTE2084;
    video->colorspace = AVCOL_SPC_BT2020_NCL;
    video->chroma_location = AVCHROMA_LOC_TOPLEFT;
    video->best_effort_timestamp = 124;
    video->decode_error_flags =
        FF_DECODE_ERROR_INVALID_BITSTREAM |
        FF_DECODE_ERROR_CONCEALMENT_ACTIVE;
    video->opaque = (void *)(uintptr_t)0x1234;
    video->opaque_ref = av_buffer_alloc(2);
    fail_if(!video->opaque_ref, "video opaque_ref allocation failed");
    video->opaque_ref->data[0] = 0xde;
    video->opaque_ref->data[1] = 0xad;
    video->alpha_mode = AVALPHA_MODE_PREMULTIPLIED;
    av_dict_set(&video->metadata, "encoder", "oracle", 0);
    fail_if(av_frame_get_buffer(video, 1) < 0,
            "video av_frame_get_buffer failed");
    static const uint8_t video_payload[] = { 1, 2, 3, 4, 5, 6 };
    fill_video_gray(video, video_payload);
    print_frame("frame:video-buffer", video);

    AVFrame *video_ref = av_frame_alloc();
    fail_if(!video_ref, "video_ref av_frame_alloc failed");
    fail_if(av_frame_ref(video_ref, video) < 0, "av_frame_ref failed");
    print_frame("frame:video-ref-src", video);
    print_frame("frame:video-ref-dst", video_ref);
    print_share("frame:video-ref-shares", video, video_ref);

    int make_ret = av_frame_make_writable(video_ref);
    printf("frame:video-make-writable-ret|%d\n", make_ret);
    fail_if(make_ret < 0, "av_frame_make_writable failed");
    print_frame("frame:video-after-make-writable-src", video);
    print_frame("frame:video-after-make-writable-dst", video_ref);
    print_share("frame:video-after-make-writable-shares", video, video_ref);

    AVFrame *move_dst = av_frame_alloc();
    fail_if(!move_dst, "move_dst av_frame_alloc failed");
    av_frame_move_ref(move_dst, video_ref);
    print_frame("frame:move-dst", move_dst);
    print_frame("frame:move-src", video_ref);

    av_frame_unref(video);
    print_frame("frame:unref", video);

    AVFrame *audio = av_frame_alloc();
    fail_if(!audio, "audio av_frame_alloc failed");
    audio->format = AV_SAMPLE_FMT_S16;
    audio->sample_rate = 48000;
    audio->nb_samples = 3;
    av_channel_layout_default(&audio->ch_layout, 2);
    fail_if(av_frame_get_buffer(audio, 1) < 0,
            "audio av_frame_get_buffer failed");
    for (int i = 0; i < 12; i++)
        audio->data[0][i] = (uint8_t)(i + 1);
    print_frame("frame:audio-buffer", audio);

    AVFrame *copy_src = av_frame_alloc();
    fail_if(!copy_src, "copy_src av_frame_alloc failed");
    copy_src->format = AV_PIX_FMT_GRAY8;
    copy_src->width = 2;
    copy_src->height = 1;
    copy_src->pts = 321;
    copy_src->pkt_dts = 320;
    copy_src->duration = 319;
    copy_src->time_base = (AVRational){ 1, 48000 };
    copy_src->sample_aspect_ratio = (AVRational){ 64, 45 };
    copy_src->crop_top = 1;
    copy_src->crop_bottom = 2;
    copy_src->crop_left = 3;
    copy_src->crop_right = 4;
    copy_src->pict_type = AV_PICTURE_TYPE_BI;
    copy_src->quality = 66;
    copy_src->repeat_pict = 4;
    copy_src->flags = AV_FRAME_FLAG_KEY | AV_FRAME_FLAG_LOSSLESS;
    copy_src->color_range = AVCOL_RANGE_JPEG;
    copy_src->color_primaries = AVCOL_PRI_BT2020;
    copy_src->color_trc = AVCOL_TRC_SMPTE2084;
    copy_src->colorspace = AVCOL_SPC_BT2020_NCL;
    copy_src->chroma_location = AVCHROMA_LOC_TOPLEFT;
    copy_src->best_effort_timestamp = 322;
    copy_src->decode_error_flags =
        FF_DECODE_ERROR_MISSING_REFERENCE |
        FF_DECODE_ERROR_DECODE_SLICES;
    copy_src->opaque = (void *)(uintptr_t)0x2222;
    copy_src->opaque_ref = av_buffer_alloc(3);
    fail_if(!copy_src->opaque_ref, "copy_src opaque_ref allocation failed");
    copy_src->opaque_ref->data[0] = 0x22;
    copy_src->opaque_ref->data[1] = 0x23;
    copy_src->opaque_ref->data[2] = 0x24;
    copy_src->alpha_mode = AVALPHA_MODE_STRAIGHT;
    av_dict_set(&copy_src->metadata, "title", "source", 0);
    av_dict_set(&copy_src->metadata, "artist", "libavutil", 0);
    fail_if(av_frame_get_buffer(copy_src, 1) < 0,
            "copy_src av_frame_get_buffer failed");
    static const uint8_t copy_src_payload[] = { 1, 2 };
    fill_video_gray(copy_src, copy_src_payload);
    copy_src->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!copy_src->hw_frames_ctx, "copy_src hw context allocation failed");
    copy_src->hw_frames_ctx->data[0] = 0xEE;
    AVFrameSideData *copy_src_sd = av_frame_new_side_data(
        copy_src, AV_FRAME_DATA_DISPLAYMATRIX, 36);
    fail_if(!copy_src_sd, "copy_src side data allocation failed");
    for (int i = 0; i < 36; i++)
        copy_src_sd->data[i] = (uint8_t)(i + 1);

    AVFrame *copy_dst = av_frame_alloc();
    fail_if(!copy_dst, "copy_dst av_frame_alloc failed");
    copy_dst->format = AV_PIX_FMT_GRAY8;
    copy_dst->width = 2;
    copy_dst->height = 1;
    copy_dst->pts = 999;
    copy_dst->pkt_dts = 998;
    copy_dst->duration = 997;
    copy_dst->time_base = (AVRational){ 1, 1000 };
    copy_dst->sample_aspect_ratio = (AVRational){ 1, 1 };
    copy_dst->crop_top = 9;
    copy_dst->crop_bottom = 8;
    copy_dst->crop_left = 7;
    copy_dst->crop_right = 6;
    copy_dst->pict_type = AV_PICTURE_TYPE_I;
    copy_dst->quality = 11;
    copy_dst->repeat_pict = 1;
    copy_dst->flags = AV_FRAME_FLAG_CORRUPT;
    copy_dst->color_range = AVCOL_RANGE_MPEG;
    copy_dst->color_primaries = AVCOL_PRI_SMPTE170M;
    copy_dst->color_trc = AVCOL_TRC_BT709;
    copy_dst->colorspace = AVCOL_SPC_SMPTE170M;
    copy_dst->chroma_location = AVCHROMA_LOC_CENTER;
    copy_dst->best_effort_timestamp = 1000;
    copy_dst->decode_error_flags = FF_DECODE_ERROR_INVALID_BITSTREAM;
    copy_dst->opaque = (void *)(uintptr_t)0x3333;
    copy_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!copy_dst->opaque_ref, "copy_dst opaque_ref allocation failed");
    copy_dst->opaque_ref->data[0] = 0x33;
    copy_dst->alpha_mode = AVALPHA_MODE_PREMULTIPLIED;
    av_dict_set(&copy_dst->metadata, "title", "destination", 0);
    av_dict_set(&copy_dst->metadata, "keep", "destination", 0);
    fail_if(av_frame_get_buffer(copy_dst, 1) < 0,
            "copy_dst av_frame_get_buffer failed");
    static const uint8_t copy_dst_payload[] = { 9, 8 };
    fill_video_gray(copy_dst, copy_dst_payload);
    copy_dst->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!copy_dst->hw_frames_ctx, "copy_dst hw context allocation failed");
    copy_dst->hw_frames_ctx->data[0] = 0xAA;
    AVFrameSideData *copy_dst_sd = av_frame_new_side_data(
        copy_dst, AV_FRAME_DATA_DISPLAYMATRIX, 36);
    fail_if(!copy_dst_sd, "copy_dst side data allocation failed");
    for (int i = 0; i < 36; i++)
        copy_dst_sd->data[i] = 0x99;

    int copy_props_ret = av_frame_copy_props(copy_dst, copy_src);
    printf("frame:copy-props-ret|%d\n", copy_props_ret);
    fail_if(copy_props_ret < 0, "av_frame_copy_props failed");
    print_frame("frame:copy-props-src", copy_src);
    print_frame("frame:copy-props-dst", copy_dst);
    print_share("frame:copy-props-plane-shares", copy_src, copy_dst);
    print_side_share("frame:copy-props-side-shares", copy_src, copy_dst);
    print_hw_share("frame:copy-props-hw-shares", copy_src, copy_dst);

    AVFrame *side_frame = av_frame_alloc();
    fail_if(!side_frame, "side_frame av_frame_alloc failed");
    AVFrameSideData *sd = av_frame_new_side_data(
        side_frame, AV_FRAME_DATA_DISPLAYMATRIX, 36);
    fail_if(!sd, "av_frame_new_side_data failed");
    for (int i = 0; i < 36; i++)
        sd->data[i] = (uint8_t)(i + 1);
    print_side_data_row("frame:side-data-new", side_frame, sd);
    AVFrameSideData *found =
        av_frame_get_side_data(side_frame, AV_FRAME_DATA_DISPLAYMATRIX);
    av_frame_remove_side_data(side_frame, AV_FRAME_DATA_DISPLAYMATRIX);
    printf("frame:side-data-remove|%d|%d\n", found == sd,
           side_frame->nb_side_data);

    AVFrameSideData **side_array = NULL;
    int nb_side_array = 0;
    AVFrameSideData *array_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_REPLAYGAIN, 1, 0);
    fail_if(!array_entry, "frame side array new failed");
    array_entry->data[0] = 0x11;
    print_side_array_row("frame:side-array-new", array_entry != NULL,
                         side_array, nb_side_array);

    AVFrameSideData *duplicate_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_REPLAYGAIN, 1, 0);
    print_side_array_row("frame:side-array-duplicate",
                         duplicate_entry != NULL, side_array, nb_side_array);

    AVFrameSideData *replace_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_REPLAYGAIN, 2,
        AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    fail_if(!replace_entry, "frame side array replace failed");
    replace_entry->data[0] = 0x33;
    replace_entry->data[1] = 0x44;
    print_side_array_row("frame:side-array-replace", replace_entry != NULL,
                         side_array, nb_side_array);

    AVFrameSideData *display_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_DISPLAYMATRIX, 1, 0);
    fail_if(!display_entry, "frame side array display add failed");
    display_entry->data[0] = 0x55;
    AVFrameSideData *unique_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_REPLAYGAIN, 1,
        AV_FRAME_SIDE_DATA_FLAG_UNIQUE);
    fail_if(!unique_entry, "frame side array unique add failed");
    unique_entry->data[0] = 0x66;
    print_side_array_row("frame:side-array-unique", unique_entry != NULL,
                         side_array, nb_side_array);

    AVFrameSideData *sei_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_SEI_UNREGISTERED, 16, 0);
    fail_if(!sei_entry, "frame side array multi add failed");
    memset(sei_entry->data, 0x77, sei_entry->size);
    AVFrameSideData *sei_replace_entry = av_frame_side_data_new(
        &side_array, &nb_side_array, AV_FRAME_DATA_SEI_UNREGISTERED, 16,
        AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    fail_if(!sei_replace_entry, "frame side array multi replace add failed");
    memset(sei_replace_entry->data, 0x88, sei_replace_entry->size);
    print_side_array_row("frame:side-array-multi-replace",
                         sei_replace_entry != NULL, side_array,
                         nb_side_array);

    av_frame_side_data_remove_by_props(&side_array, &nb_side_array,
                                       AV_SIDE_DATA_PROP_MULTI);
    printf("frame:side-array-remove-multi|%d|", nb_side_array);
    print_side_array_summary(side_array, nb_side_array);
    printf("\n");
    av_frame_side_data_free(&side_array, &nb_side_array);

    AVFrameSideData **side_add_array = NULL;
    int nb_side_add_array = 0;
    AVBufferRef *take_buf = av_buffer_alloc(3);
    fail_if(!take_buf, "frame side add take buffer allocation failed");
    take_buf->data[0] = 0xa1;
    take_buf->data[1] = 0xa2;
    take_buf->data[2] = 0xa3;
    AVFrameSideData *take_entry = av_frame_side_data_add(
        &side_add_array, &nb_side_add_array, AV_FRAME_DATA_REPLAYGAIN,
        &take_buf, 0);
    print_side_add_buffer_row("frame:side-add-take", take_entry != NULL,
                              side_add_array, nb_side_add_array, take_buf,
                              AV_FRAME_DATA_REPLAYGAIN);

    AVBufferRef *duplicate_buf = av_buffer_alloc(1);
    fail_if(!duplicate_buf, "frame side add duplicate buffer allocation failed");
    duplicate_buf->data[0] = 0xb1;
    AVFrameSideData *duplicate_add_entry = av_frame_side_data_add(
        &side_add_array, &nb_side_add_array, AV_FRAME_DATA_REPLAYGAIN,
        &duplicate_buf, 0);
    print_side_add_buffer_row("frame:side-add-duplicate",
                              duplicate_add_entry != NULL, side_add_array,
                              nb_side_add_array, duplicate_buf,
                              AV_FRAME_DATA_REPLAYGAIN);
    av_buffer_unref(&duplicate_buf);

    AVBufferRef *replace_buf = av_buffer_alloc(2);
    fail_if(!replace_buf, "frame side add replace buffer allocation failed");
    replace_buf->data[0] = 0xc1;
    replace_buf->data[1] = 0xc2;
    AVFrameSideData *replace_add_entry = av_frame_side_data_add(
        &side_add_array, &nb_side_add_array, AV_FRAME_DATA_REPLAYGAIN,
        &replace_buf, AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    print_side_add_buffer_row("frame:side-add-replace",
                              replace_add_entry != NULL, side_add_array,
                              nb_side_add_array, replace_buf,
                              AV_FRAME_DATA_REPLAYGAIN);

    AVBufferRef *ref_buf = av_buffer_alloc(2);
    fail_if(!ref_buf, "frame side add new-ref buffer allocation failed");
    ref_buf->data[0] = 0xd1;
    ref_buf->data[1] = 0xd2;
    AVFrameSideData *ref_add_entry = av_frame_side_data_add(
        &side_add_array, &nb_side_add_array, AV_FRAME_DATA_DISPLAYMATRIX,
        &ref_buf, AV_FRAME_SIDE_DATA_FLAG_NEW_REF);
    print_side_add_buffer_row("frame:side-add-new-ref",
                              ref_add_entry != NULL, side_add_array,
                              nb_side_add_array, ref_buf,
                              AV_FRAME_DATA_DISPLAYMATRIX);
    av_buffer_unref(&ref_buf);
    av_frame_side_data_free(&side_add_array, &nb_side_add_array);

    AVFrameSideData **clone_source_array = NULL;
    int nb_clone_source_array = 0;
    AVFrameSideData *clone_source = av_frame_side_data_new(
        &clone_source_array, &nb_clone_source_array, AV_FRAME_DATA_REPLAYGAIN,
        2, 0);
    fail_if(!clone_source, "frame side clone source allocation failed");
    clone_source->data[0] = 0xe1;
    clone_source->data[1] = 0xe2;
    fail_if(av_dict_set(&clone_source->metadata, "gain", "source", 0) < 0,
            "frame side clone source metadata failed");

    AVFrameSideData **side_clone_array = NULL;
    int nb_side_clone_array = 0;
    int clone_new_ret = av_frame_side_data_clone(
        &side_clone_array, &nb_side_clone_array, clone_source, 0);
    print_side_clone_row("frame:side-clone-new", clone_new_ret,
                         side_clone_array, nb_side_clone_array, clone_source,
                         AV_FRAME_DATA_REPLAYGAIN);

    int clone_duplicate_ret = av_frame_side_data_clone(
        &side_clone_array, &nb_side_clone_array, clone_source, 0);
    print_side_clone_row("frame:side-clone-duplicate", clone_duplicate_ret,
                         side_clone_array, nb_side_clone_array, clone_source,
                         AV_FRAME_DATA_REPLAYGAIN);

    AVFrameSideData **clone_replacement_array = NULL;
    int nb_clone_replacement_array = 0;
    AVFrameSideData *clone_replacement = av_frame_side_data_new(
        &clone_replacement_array, &nb_clone_replacement_array,
        AV_FRAME_DATA_REPLAYGAIN, 2, 0);
    fail_if(!clone_replacement,
            "frame side clone replacement allocation failed");
    clone_replacement->data[0] = 0xf1;
    clone_replacement->data[1] = 0xf2;
    fail_if(av_dict_set(&clone_replacement->metadata, "gain",
                        "replacement", 0) < 0,
            "frame side clone replacement metadata failed");
    int clone_replace_ret = av_frame_side_data_clone(
        &side_clone_array, &nb_side_clone_array, clone_replacement,
        AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    print_side_clone_row("frame:side-clone-replace", clone_replace_ret,
                         side_clone_array, nb_side_clone_array,
                         clone_replacement, AV_FRAME_DATA_REPLAYGAIN);

    AVFrameSideData *clone_display_entry = av_frame_side_data_new(
        &side_clone_array, &nb_side_clone_array, AV_FRAME_DATA_DISPLAYMATRIX,
        1, 0);
    fail_if(!clone_display_entry,
            "frame side clone display allocation failed");
    clone_display_entry->data[0] = 0xe5;
    int clone_unique_ret = av_frame_side_data_clone(
        &side_clone_array, &nb_side_clone_array, clone_source,
        AV_FRAME_SIDE_DATA_FLAG_UNIQUE);
    print_side_clone_row("frame:side-clone-unique", clone_unique_ret,
                         side_clone_array, nb_side_clone_array, clone_source,
                         AV_FRAME_DATA_REPLAYGAIN);

    AVFrameSideData **clone_multi_source_array = NULL;
    int nb_clone_multi_source_array = 0;
    AVFrameSideData *clone_multi_source = av_frame_side_data_new(
        &clone_multi_source_array, &nb_clone_multi_source_array,
        AV_FRAME_DATA_SEI_UNREGISTERED, 16, 0);
    fail_if(!clone_multi_source,
            "frame side clone multi source allocation failed");
    memset(clone_multi_source->data, 0x77, clone_multi_source->size);
    fail_if(av_dict_set(&clone_multi_source->metadata, "gain", "multi", 0) < 0,
            "frame side clone multi metadata failed");
    AVFrameSideData **clone_multi_array = NULL;
    int nb_clone_multi_array = 0;
    int clone_multi_new_ret = av_frame_side_data_clone(
        &clone_multi_array, &nb_clone_multi_array, clone_multi_source, 0);
    print_side_clone_row("frame:side-clone-multi-new", clone_multi_new_ret,
                         clone_multi_array, nb_clone_multi_array,
                         clone_multi_source, AV_FRAME_DATA_SEI_UNREGISTERED);
    int clone_multi_replace_ret = av_frame_side_data_clone(
        &clone_multi_array, &nb_clone_multi_array, clone_multi_source,
        AV_FRAME_SIDE_DATA_FLAG_REPLACE);
    print_side_clone_row("frame:side-clone-multi-replace",
                         clone_multi_replace_ret, clone_multi_array,
                         nb_clone_multi_array, clone_multi_source,
                         AV_FRAME_DATA_SEI_UNREGISTERED);

    av_frame_side_data_free(&clone_multi_array, &nb_clone_multi_array);
    av_frame_side_data_free(&clone_multi_source_array,
                            &nb_clone_multi_source_array);
    av_frame_side_data_free(&side_clone_array, &nb_side_clone_array);
    av_frame_side_data_free(&clone_replacement_array,
                            &nb_clone_replacement_array);
    av_frame_side_data_free(&clone_source_array, &nb_clone_source_array);

    av_frame_free(&side_frame);
    av_frame_free(&copy_dst);
    av_frame_free(&copy_src);
    av_frame_free(&audio);
    av_frame_free(&move_dst);
    av_frame_free(&video_ref);
    av_frame_free(&video);

    return 0;
}
"#
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
