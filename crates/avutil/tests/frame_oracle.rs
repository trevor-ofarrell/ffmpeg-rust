use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    frame_side_data_descriptor_for_value, frame_side_data_name_for_value, AudioFrame, AvErrorCode,
    BufferRef, ChannelLayout, ChannelLayoutSpec, Frame, FrameAlphaMode, FrameBufferTopology,
    FrameChromaLocation, FrameColorPrimaries, FrameColorRange, FrameColorSpace,
    FrameColorTransferCharacteristic, FrameCropFlags, FrameData, FrameDecodeErrorFlags, FrameFifo,
    FrameFlags, FramePictureType, FrameSideData, FrameSideDataFlags, FrameSideDataKind,
    FrameSideDataProperties, PixelFormat, Rational, SampleFormat, VideoFrame, AVPALETTE_SIZE,
    AV_NOPTS_VALUE, AV_NUM_DATA_POINTERS,
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
    let mut empty_make_writable = Frame::empty();
    let empty_make_writable_ret = empty_make_writable
        .try_make_writable()
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:empty-make-writable-ret".to_string(),
        vec![empty_make_writable_ret.to_string()],
    );
    rows.insert(
        "frame:empty-after-make-writable".to_string(),
        frame_fields(&empty_make_writable),
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
    let display_value = FrameSideDataKind::DisplayMatrix
        .ffmpeg_value()
        .expect("displaymatrix should have a raw FFmpeg value");
    let exif_value = FrameSideDataKind::Exif
        .ffmpeg_value()
        .expect("EXIF should have a raw FFmpeg value");
    let sentinel_value = FrameSideDataKind::KNOWN.len() as i32;
    rows.insert(
        "frame:side-name-boundaries".to_string(),
        vec![
            display_value.to_string(),
            frame_side_data_name_for_value(display_value)
                .expect("displaymatrix should have a name")
                .to_string(),
            exif_value.to_string(),
            frame_side_data_name_for_value(exif_value)
                .expect("EXIF should have a name")
                .to_string(),
            bool_field(frame_side_data_name_for_value(-1).is_none()),
            bool_field(frame_side_data_name_for_value(sentinel_value).is_none()),
            bool_field(frame_side_data_name_for_value(sentinel_value + 1).is_none()),
            bool_field(frame_side_data_name_for_value(i32::MAX).is_none()),
        ],
    );
    let display_desc =
        frame_side_data_descriptor_for_value(display_value).expect("displaymatrix descriptor");
    let exif_desc = frame_side_data_descriptor_for_value(exif_value).expect("EXIF descriptor");
    rows.insert(
        "frame:side-desc-boundaries".to_string(),
        vec![
            display_value.to_string(),
            display_desc.name().to_string(),
            display_desc.properties().bits().to_string(),
            exif_value.to_string(),
            exif_desc.name().to_string(),
            exif_desc.properties().bits().to_string(),
            bool_field(frame_side_data_descriptor_for_value(-1).is_none()),
            bool_field(frame_side_data_descriptor_for_value(sentinel_value).is_none()),
            bool_field(frame_side_data_descriptor_for_value(sentinel_value + 1).is_none()),
            bool_field(frame_side_data_descriptor_for_value(i32::MAX).is_none()),
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
    video.set_sample_rate(44_100);
    video.set_channel_layout(Some(ChannelLayout::mono()));
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
    rows.insert(
        "frame:plane-buffer-video-0".to_string(),
        frame_plane_buffer_fields(&video, 0),
    );
    rows.insert(
        "frame:plane-buffer-video-invalid".to_string(),
        frame_plane_buffer_fields(&video, 1),
    );

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

    let pal8 = Frame::video(
        VideoFrame::new_pal8_with_palette(3, 2, pal8_index_fixture(3, 2), pal8_palette_fixture())
            .unwrap(),
    );
    rows.insert("frame:pal8-buffer".to_string(), pal8_frame_fields(&pal8));
    rows.insert(
        "frame:pal8-plane-buffer-0".to_string(),
        pal8_plane_buffer_fields(&pal8, 0),
    );
    rows.insert(
        "frame:pal8-plane-buffer-1".to_string(),
        pal8_plane_buffer_fields(&pal8, 1),
    );
    rows.insert(
        "frame:pal8-plane-buffer-invalid".to_string(),
        pal8_plane_buffer_fields(&pal8, 2),
    );

    let mut pal8_ref = Frame::empty();
    pal8_ref.ref_from(&pal8);
    rows.insert("frame:pal8-ref-src".to_string(), pal8_frame_fields(&pal8));
    rows.insert(
        "frame:pal8-ref-dst".to_string(),
        pal8_frame_fields(&pal8_ref),
    );
    rows.insert(
        "frame:pal8-ref-plane0-shares".to_string(),
        pal8_plane_share_fields(&pal8, &pal8_ref, 0),
    );
    rows.insert(
        "frame:pal8-ref-plane1-shares".to_string(),
        pal8_plane_share_fields(&pal8, &pal8_ref, 1),
    );

    pal8_ref.make_writable();
    rows.insert(
        "frame:pal8-after-make-writable-src".to_string(),
        pal8_frame_fields(&pal8),
    );
    rows.insert(
        "frame:pal8-after-make-writable-dst".to_string(),
        pal8_frame_fields(&pal8_ref),
    );
    rows.insert(
        "frame:pal8-make-writable-plane0-shares".to_string(),
        pal8_plane_share_fields(&pal8, &pal8_ref, 0),
    );
    rows.insert(
        "frame:pal8-make-writable-plane1-shares".to_string(),
        pal8_plane_share_fields(&pal8, &pal8_ref, 1),
    );

    let pal8_crop_palette = BufferRef::from_vec(pal8_palette_fixture());
    let mut pal8_crop = Frame::video(
        VideoFrame::new_pal8_with_palette_refs(
            6,
            4,
            BufferRef::from_vec(pal8_index_fixture(6, 4)),
            pal8_crop_palette.clone(),
        )
        .unwrap(),
    );
    pal8_crop.set_crop_offsets(1, 1, 1, 1);
    pal8_crop.apply_cropping(FrameCropFlags::UNALIGNED).unwrap();
    rows.insert(
        "frame:pal8-crop-palette-preserved".to_string(),
        pal8_crop_palette_preserved_fields(&pal8_crop, &pal8_crop_palette),
    );

    let mut move_dst = Frame::empty();
    move_dst.move_ref_from(&mut video_ref);
    rows.insert("frame:move-dst".to_string(), frame_fields(&move_dst));
    rows.insert("frame:move-src".to_string(), frame_fields(&video_ref));

    video.unref();
    rows.insert("frame:unref".to_string(), frame_fields(&video));

    let mut rich_unref = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(1, 1, PixelFormat::Gray8, vec![vec![0x42]], 1)
            .unwrap(),
    )
    .with_hw_frames_context(BufferRef::copy_from_slice(&[0x77]));
    rich_unref.set_pts(Some(701));
    rich_unref
        .set_time_base(Rational::new(1, 25).unwrap())
        .unwrap();
    rich_unref.set_opaque_ref(Some(BufferRef::copy_from_slice(&[0x88, 0x99])));
    rich_unref
        .metadata_mut()
        .set("title", "before-unref")
        .unwrap();
    rich_unref
        .set_side_data_kind(FrameSideDataKind::DisplayMatrix, vec![0x55; 36])
        .unwrap();
    rich_unref.unref();
    rows.insert("frame:unref-rich".to_string(), frame_fields(&rich_unref));

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
    rows.insert(
        "frame:plane-buffer-audio-packed-0".to_string(),
        frame_plane_buffer_fields(&audio, 0),
    );
    rows.insert(
        "frame:plane-buffer-audio-packed-invalid".to_string(),
        frame_plane_buffer_fields(&audio, 1),
    );

    let planar_audio = Frame::audio(
        AudioFrame::new_with_channel_layout_and_aligned_line_sizes(
            48_000,
            ChannelLayout::stereo(),
            SampleFormat::S16P,
            2,
            vec![vec![1, 0, 2, 0], vec![3, 0, 4, 0]],
            1,
        )
        .unwrap(),
    );
    rows.insert(
        "frame:audio-planar-buffer".to_string(),
        frame_fields(&planar_audio),
    );
    rows.insert(
        "frame:plane-buffer-audio-planar-1".to_string(),
        frame_plane_buffer_fields(&planar_audio, 1),
    );

    let extended_audio = Frame::audio(
        AudioFrame::new(
            48_000,
            10,
            SampleFormat::S16P,
            1,
            (0..10).map(|plane| vec![plane as u8, 0]).collect(),
        )
        .unwrap(),
    );
    rows.insert(
        "frame:audio-extended-topology".to_string(),
        frame_buffer_topology_fields(&extended_audio),
    );
    rows.insert(
        "frame:plane-buffer-audio-extended-8".to_string(),
        frame_plane_buffer_fields(&extended_audio, 8),
    );
    rows.insert(
        "frame:plane-buffer-audio-extended-9".to_string(),
        frame_plane_buffer_fields(&extended_audio, 9),
    );
    rows.insert(
        "frame:plane-buffer-audio-extended-invalid".to_string(),
        frame_plane_buffer_fields(&extended_audio, 10),
    );
    let packed_ten_channel_audio =
        Frame::audio(AudioFrame::new(48_000, 10, SampleFormat::S16, 1, vec![vec![0; 20]]).unwrap());
    rows.insert(
        "frame:audio-packed-ten-topology".to_string(),
        frame_buffer_topology_fields(&packed_ten_channel_audio),
    );
    rows.insert(
        "frame:plane-buffer-audio-packed-ten-0".to_string(),
        frame_plane_buffer_fields(&packed_ten_channel_audio, 0),
    );
    rows.insert(
        "frame:plane-buffer-audio-packed-ten-invalid".to_string(),
        frame_plane_buffer_fields(&packed_ten_channel_audio, 1),
    );

    let mut copy_data_video_source = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![1, 2, 3, 4, 5, 6]],
            1,
        )
        .unwrap(),
    );
    copy_data_video_source.set_pts(Some(101));
    copy_data_video_source
        .metadata_mut()
        .set("title", "copy-data-source")
        .unwrap();
    let mut copy_data_video_destination = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![9, 9, 9, 8, 8, 8]],
            1,
        )
        .unwrap(),
    );
    copy_data_video_destination.set_pts(Some(202));
    copy_data_video_destination
        .metadata_mut()
        .set("title", "copy-data-destination")
        .unwrap();
    let copy_data_video_ret = copy_data_video_destination
        .copy_data_from(&copy_data_video_source)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:copy-data-video-ret".to_string(),
        vec![copy_data_video_ret.to_string()],
    );
    rows.insert(
        "frame:copy-data-video-src".to_string(),
        frame_fields(&copy_data_video_source),
    );
    rows.insert(
        "frame:copy-data-video-dst".to_string(),
        frame_fields(&copy_data_video_destination),
    );

    let copy_data_audio_source = Frame::audio(
        AudioFrame::new_with_channel_layout_and_aligned_line_sizes(
            44_100,
            ChannelLayout::stereo(),
            SampleFormat::S16,
            2,
            vec![vec![1, 0, 2, 0, 3, 0, 4, 0]],
            1,
        )
        .unwrap(),
    );
    let mut copy_data_audio_destination = Frame::audio(
        AudioFrame::new_with_channel_layout_and_aligned_line_sizes(
            96_000,
            ChannelLayout::stereo(),
            SampleFormat::S16,
            2,
            vec![vec![9, 0, 8, 0, 7, 0, 6, 0]],
            1,
        )
        .unwrap(),
    );
    copy_data_audio_destination.set_pts(Some(303));
    let copy_data_audio_ret = copy_data_audio_destination
        .copy_data_from(&copy_data_audio_source)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:copy-data-audio-ret".to_string(),
        vec![copy_data_audio_ret.to_string()],
    );
    rows.insert(
        "frame:copy-data-audio-src".to_string(),
        frame_fields(&copy_data_audio_source),
    );
    rows.insert(
        "frame:copy-data-audio-dst".to_string(),
        frame_fields(&copy_data_audio_destination),
    );

    let mut copy_data_larger_destination = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![7, 7, 7, 6, 6, 6]],
            1,
        )
        .unwrap(),
    );
    let copy_data_larger_source = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            2,
            2,
            PixelFormat::Gray8,
            vec![vec![1, 2, 3, 4]],
            1,
        )
        .unwrap(),
    );
    let copy_data_larger_ret = copy_data_larger_destination
        .copy_data_from(&copy_data_larger_source)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:copy-data-video-larger-dst-ret".to_string(),
        vec![copy_data_larger_ret.to_string()],
    );
    rows.insert(
        "frame:copy-data-video-larger-dst".to_string(),
        frame_fields(&copy_data_larger_destination),
    );

    let mut copy_data_too_small_destination = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            2,
            2,
            PixelFormat::Gray8,
            vec![vec![7, 7, 6, 6]],
            1,
        )
        .unwrap(),
    );
    let copy_data_too_small_source = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![1, 2, 3, 4, 5, 6]],
            1,
        )
        .unwrap(),
    );
    let copy_data_too_small_ret = copy_data_too_small_destination
        .copy_data_from(&copy_data_too_small_source)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:copy-data-video-too-small-ret".to_string(),
        vec![copy_data_too_small_ret.to_string()],
    );
    rows.insert(
        "frame:copy-data-video-too-small-dst".to_string(),
        frame_fields(&copy_data_too_small_destination),
    );

    let copy_data_kind_mismatch_source = Frame::audio(
        AudioFrame::new_with_channel_layout_and_aligned_line_sizes(
            44_100,
            ChannelLayout::stereo(),
            SampleFormat::S16,
            2,
            vec![vec![1, 0, 2, 0, 3, 0, 4, 0]],
            1,
        )
        .unwrap(),
    );
    let mut copy_data_kind_mismatch_destination = Frame::video(
        VideoFrame::new_with_aligned_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![9, 9, 9, 8, 8, 8]],
            1,
        )
        .unwrap(),
    );
    let copy_data_kind_mismatch_before = frame_fields(&copy_data_kind_mismatch_destination);
    let copy_data_kind_mismatch_ret = copy_data_kind_mismatch_destination
        .copy_data_from(&copy_data_kind_mismatch_source)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:copy-data-kind-mismatch-ret".to_string(),
        vec![copy_data_kind_mismatch_ret.to_string()],
    );
    rows.insert(
        "frame:copy-data-kind-mismatch-before".to_string(),
        copy_data_kind_mismatch_before,
    );
    rows.insert(
        "frame:copy-data-kind-mismatch-src".to_string(),
        frame_fields(&copy_data_kind_mismatch_source),
    );
    rows.insert(
        "frame:copy-data-kind-mismatch-after".to_string(),
        frame_fields(&copy_data_kind_mismatch_destination),
    );

    let crop_payload = gray8_incrementing_payload(6, 4);
    let crop_storage = gray8_strided_storage(6, 4, 64, &crop_payload);
    let mut crop_aligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            6,
            4,
            PixelFormat::Gray8,
            vec![crop_storage.clone()],
            vec![64],
        )
        .unwrap(),
    );
    crop_aligned.set_crop_offsets(1, 1, 1, 2);
    let crop_aligned_ret = crop_aligned
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-aligned-ret".to_string(),
        vec![crop_aligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-aligned".to_string(),
        frame_fields(&crop_aligned),
    );

    let mut crop_unaligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            6,
            4,
            PixelFormat::Gray8,
            vec![crop_storage.clone()],
            vec![64],
        )
        .unwrap(),
    );
    crop_unaligned.set_crop_offsets(1, 1, 1, 2);
    let crop_unaligned_ret = crop_unaligned
        .apply_cropping(FrameCropFlags::UNALIGNED)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-unaligned-ret".to_string(),
        vec![crop_unaligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-unaligned".to_string(),
        frame_fields(&crop_unaligned),
    );

    let rgb_crop_storage = packed_strided_storage(8, 4, 3, 192);
    let mut crop_rgb24_aligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Rgb24,
            vec![rgb_crop_storage.clone()],
            vec![192],
        )
        .unwrap(),
    );
    crop_rgb24_aligned.set_crop_offsets(1, 0, 1, 1);
    let crop_rgb24_aligned_ret = crop_rgb24_aligned
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-rgb24-aligned-ret".to_string(),
        vec![crop_rgb24_aligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-rgb24-aligned".to_string(),
        frame_fields(&crop_rgb24_aligned),
    );

    let mut crop_rgb24_unaligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Rgb24,
            vec![rgb_crop_storage],
            vec![192],
        )
        .unwrap(),
    );
    crop_rgb24_unaligned.set_crop_offsets(1, 0, 1, 1);
    let crop_rgb24_unaligned_ret = crop_rgb24_unaligned
        .apply_cropping(FrameCropFlags::UNALIGNED)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-rgb24-unaligned-ret".to_string(),
        vec![crop_rgb24_unaligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-rgb24-unaligned".to_string(),
        frame_fields(&crop_rgb24_unaligned),
    );

    let bgr_crop_storage = packed_strided_storage(8, 4, 3, 192);
    let mut crop_bgr24_aligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Bgr24,
            vec![bgr_crop_storage.clone()],
            vec![192],
        )
        .unwrap(),
    );
    crop_bgr24_aligned.set_crop_offsets(1, 0, 1, 1);
    let crop_bgr24_aligned_ret = crop_bgr24_aligned
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-bgr24-aligned-ret".to_string(),
        vec![crop_bgr24_aligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-bgr24-aligned".to_string(),
        frame_fields(&crop_bgr24_aligned),
    );

    let mut crop_bgr24_unaligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Bgr24,
            vec![bgr_crop_storage],
            vec![192],
        )
        .unwrap(),
    );
    crop_bgr24_unaligned.set_crop_offsets(1, 0, 1, 1);
    let crop_bgr24_unaligned_ret = crop_bgr24_unaligned
        .apply_cropping(FrameCropFlags::UNALIGNED)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-bgr24-unaligned-ret".to_string(),
        vec![crop_bgr24_unaligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-bgr24-unaligned".to_string(),
        frame_fields(&crop_bgr24_unaligned),
    );

    let yuv420p_crop_storage = yuv420p_strided_storage(8, 4, 64);
    let mut crop_yuv420p_default = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Yuv420p,
            yuv420p_crop_storage.clone(),
            vec![64, 64, 64],
        )
        .unwrap(),
    );
    crop_yuv420p_default.set_crop_offsets(2, 0, 2, 2);
    let crop_yuv420p_default_ret = crop_yuv420p_default
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-yuv420p-default-ret".to_string(),
        vec![crop_yuv420p_default_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-yuv420p-default".to_string(),
        frame_fields(&crop_yuv420p_default),
    );

    let mut crop_yuv420p_unaligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Yuv420p,
            yuv420p_crop_storage,
            vec![64, 64, 64],
        )
        .unwrap(),
    );
    crop_yuv420p_unaligned.set_crop_offsets(2, 0, 2, 2);
    let crop_yuv420p_unaligned_ret = crop_yuv420p_unaligned
        .apply_cropping(FrameCropFlags::UNALIGNED)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-yuv420p-unaligned-ret".to_string(),
        vec![crop_yuv420p_unaligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-yuv420p-unaligned".to_string(),
        frame_fields(&crop_yuv420p_unaligned),
    );

    let yuv420p_odd_crop_storage = yuv420p_strided_storage(8, 4, 64);
    let mut crop_yuv420p_odd_default = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Yuv420p,
            yuv420p_odd_crop_storage.clone(),
            vec![64, 64, 64],
        )
        .unwrap(),
    );
    crop_yuv420p_odd_default.set_crop_offsets(1, 0, 1, 1);
    let crop_yuv420p_odd_default_ret = crop_yuv420p_odd_default
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-yuv420p-odd-default-ret".to_string(),
        vec![crop_yuv420p_odd_default_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-yuv420p-odd-default".to_string(),
        frame_fields(&crop_yuv420p_odd_default),
    );

    let mut crop_yuv420p_odd_unaligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Yuv420p,
            yuv420p_odd_crop_storage,
            vec![64, 64, 64],
        )
        .unwrap(),
    );
    crop_yuv420p_odd_unaligned.set_crop_offsets(1, 0, 1, 1);
    let crop_yuv420p_odd_unaligned_ret = crop_yuv420p_odd_unaligned
        .apply_cropping(FrameCropFlags::UNALIGNED)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-yuv420p-odd-unaligned-ret".to_string(),
        vec![crop_yuv420p_odd_unaligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-yuv420p-odd-unaligned".to_string(),
        frame_fields(&crop_yuv420p_odd_unaligned),
    );

    for (pixel_format, name, log2_chroma_w, log2_chroma_h) in [
        (PixelFormat::Nv12, "nv12", 1usize, 1usize),
        (PixelFormat::Nv21, "nv21", 1usize, 1usize),
        (PixelFormat::Nv16, "nv16", 1usize, 0usize),
        (PixelFormat::Nv24, "nv24", 0usize, 0usize),
        (PixelFormat::Nv42, "nv42", 0usize, 0usize),
    ] {
        let line_sizes = if matches!(pixel_format, PixelFormat::Nv24 | PixelFormat::Nv42) {
            vec![64, 128]
        } else {
            vec![64, 64]
        };
        let semiplanar_crop_storage = semiplanar_yuv_strided_storage(
            8,
            4,
            line_sizes[0],
            line_sizes[1],
            log2_chroma_w,
            log2_chroma_h,
            (1, 2),
        );
        let mut crop_semiplanar_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage.clone(),
                line_sizes.clone(),
            )
            .unwrap(),
        );
        crop_semiplanar_default.set_crop_offsets(1, 0, 1, 1);
        let crop_semiplanar_default_ret = crop_semiplanar_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_semiplanar_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_semiplanar_default),
        );

        let mut crop_semiplanar_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage.clone(),
                line_sizes.clone(),
            )
            .unwrap(),
        );
        crop_semiplanar_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_semiplanar_unaligned_ret = crop_semiplanar_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_semiplanar_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_semiplanar_unaligned),
        );

        let mut crop_semiplanar_even_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage.clone(),
                line_sizes.clone(),
            )
            .unwrap(),
        );
        crop_semiplanar_even_default.set_crop_offsets(2, 0, 2, 2);
        let crop_semiplanar_even_default_ret = crop_semiplanar_even_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-even-default-ret"),
            vec![crop_semiplanar_even_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-even-default"),
            frame_fields(&crop_semiplanar_even_default),
        );

        let mut crop_semiplanar_even_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage,
                line_sizes,
            )
            .unwrap(),
        );
        crop_semiplanar_even_unaligned.set_crop_offsets(2, 0, 2, 2);
        let crop_semiplanar_even_unaligned_ret = crop_semiplanar_even_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-even-unaligned-ret"),
            vec![crop_semiplanar_even_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-even-unaligned"),
            frame_fields(&crop_semiplanar_even_unaligned),
        );
    }

    for (pixel_format, name, log2_chroma_w, log2_chroma_h) in [
        (PixelFormat::Nv20Le, "nv20le", 1usize, 0usize),
        (PixelFormat::Nv20Be, "nv20be", 1usize, 0usize),
        (PixelFormat::P010Le, "p010le", 1usize, 1usize),
        (PixelFormat::P010Be, "p010be", 1usize, 1usize),
        (PixelFormat::P012Le, "p012le", 1usize, 1usize),
        (PixelFormat::P012Be, "p012be", 1usize, 1usize),
        (PixelFormat::P016Le, "p016le", 1usize, 1usize),
        (PixelFormat::P016Be, "p016be", 1usize, 1usize),
        (PixelFormat::P210Le, "p210le", 1usize, 0usize),
        (PixelFormat::P210Be, "p210be", 1usize, 0usize),
        (PixelFormat::P212Le, "p212le", 1usize, 0usize),
        (PixelFormat::P212Be, "p212be", 1usize, 0usize),
        (PixelFormat::P216Le, "p216le", 1usize, 0usize),
        (PixelFormat::P216Be, "p216be", 1usize, 0usize),
        (PixelFormat::P410Le, "p410le", 0usize, 0usize),
        (PixelFormat::P410Be, "p410be", 0usize, 0usize),
        (PixelFormat::P412Le, "p412le", 0usize, 0usize),
        (PixelFormat::P412Be, "p412be", 0usize, 0usize),
        (PixelFormat::P416Le, "p416le", 0usize, 0usize),
        (PixelFormat::P416Be, "p416be", 0usize, 0usize),
    ] {
        let line_sizes = if matches!(
            pixel_format,
            PixelFormat::P410Le
                | PixelFormat::P410Be
                | PixelFormat::P412Le
                | PixelFormat::P412Be
                | PixelFormat::P416Le
                | PixelFormat::P416Be
        ) {
            vec![64, 128]
        } else {
            vec![64, 64]
        };
        let semiplanar_crop_storage = semiplanar_yuv_strided_storage(
            8,
            4,
            line_sizes[0],
            line_sizes[1],
            log2_chroma_w,
            log2_chroma_h,
            (2, 4),
        );
        let mut crop_semiplanar_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage.clone(),
                line_sizes.clone(),
            )
            .unwrap(),
        );
        crop_semiplanar_default.set_crop_offsets(1, 0, 1, 1);
        let crop_semiplanar_default_ret = crop_semiplanar_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_semiplanar_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_semiplanar_default),
        );

        let mut crop_semiplanar_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage.clone(),
                line_sizes.clone(),
            )
            .unwrap(),
        );
        crop_semiplanar_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_semiplanar_unaligned_ret = crop_semiplanar_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_semiplanar_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_semiplanar_unaligned),
        );

        let mut crop_semiplanar_even_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage.clone(),
                line_sizes.clone(),
            )
            .unwrap(),
        );
        crop_semiplanar_even_default.set_crop_offsets(2, 0, 2, 2);
        let crop_semiplanar_even_default_ret = crop_semiplanar_even_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-even-default-ret"),
            vec![crop_semiplanar_even_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-even-default"),
            frame_fields(&crop_semiplanar_even_default),
        );

        let mut crop_semiplanar_even_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                semiplanar_crop_storage,
                line_sizes,
            )
            .unwrap(),
        );
        crop_semiplanar_even_unaligned.set_crop_offsets(2, 0, 2, 2);
        let crop_semiplanar_even_unaligned_ret = crop_semiplanar_even_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-even-unaligned-ret"),
            vec![crop_semiplanar_even_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-even-unaligned"),
            frame_fields(&crop_semiplanar_even_unaligned),
        );
    }

    for (pixel_format, name, log2_chroma_w, log2_chroma_h) in [
        (PixelFormat::YuvJ420p, "yuvj420p", 1usize, 1usize),
        (PixelFormat::Yuv422p, "yuv422p", 1usize, 0usize),
        (PixelFormat::YuvJ422p, "yuvj422p", 1usize, 0usize),
        (PixelFormat::Yuv410p, "yuv410p", 2usize, 2usize),
        (PixelFormat::Yuv411p, "yuv411p", 2usize, 0usize),
        (PixelFormat::YuvJ411p, "yuvj411p", 2usize, 0usize),
        (PixelFormat::Yuv440p, "yuv440p", 0usize, 1usize),
        (PixelFormat::YuvJ440p, "yuvj440p", 0usize, 1usize),
        (PixelFormat::Yuv444p, "yuv444p", 0usize, 0usize),
        (PixelFormat::YuvJ444p, "yuvj444p", 0usize, 0usize),
        (PixelFormat::Gbrp, "gbrp", 0usize, 0usize),
    ] {
        let planar_crop_storage =
            planar_yuv_strided_storage(8, 4, 64, log2_chroma_w, log2_chroma_h);
        let mut crop_planar_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage.clone(),
                vec![64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_default.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_default_ret = crop_planar_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_planar_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_planar_default),
        );

        let mut crop_planar_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage,
                vec![64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_unaligned_ret = crop_planar_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_planar_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_planar_unaligned),
        );
    }

    let uyyvyy411_crop_storage = uyyvyy411_strided_storage(8, 4, 128);
    let mut crop_uyyvyy411_default = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Uyyvyy411,
            vec![uyyvyy411_crop_storage.clone()],
            vec![128],
        )
        .unwrap(),
    );
    crop_uyyvyy411_default.set_crop_offsets(1, 0, 1, 1);
    let crop_uyyvyy411_default_ret = crop_uyyvyy411_default
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-uyyvyy411-default-ret".to_string(),
        vec![crop_uyyvyy411_default_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-uyyvyy411-default".to_string(),
        frame_fields(&crop_uyyvyy411_default),
    );

    let mut crop_uyyvyy411_unaligned = Frame::video(
        VideoFrame::new_with_line_sizes(
            8,
            4,
            PixelFormat::Uyyvyy411,
            vec![uyyvyy411_crop_storage],
            vec![128],
        )
        .unwrap(),
    );
    crop_uyyvyy411_unaligned.set_crop_offsets(1, 0, 1, 1);
    let crop_uyyvyy411_unaligned_ret = crop_uyyvyy411_unaligned
        .apply_cropping(FrameCropFlags::UNALIGNED)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-uyyvyy411-unaligned-ret".to_string(),
        vec![crop_uyyvyy411_unaligned_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-uyyvyy411-unaligned".to_string(),
        frame_fields(&crop_uyyvyy411_unaligned),
    );

    for (pixel_format, name, log2_chroma_w, log2_chroma_h) in [
        (PixelFormat::Yuva420p, "yuva420p", 1usize, 1usize),
        (PixelFormat::Yuva422p, "yuva422p", 1usize, 0usize),
        (PixelFormat::Yuva444p, "yuva444p", 0usize, 0usize),
        (PixelFormat::Gbrap, "gbrap", 0usize, 0usize),
    ] {
        let planar_crop_storage = planar_alpha_strided_storage_with_sample_bytes(
            8,
            4,
            64,
            log2_chroma_w,
            log2_chroma_h,
            1,
        );
        let mut crop_planar_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage.clone(),
                vec![64, 64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_default.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_default_ret = crop_planar_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_planar_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_planar_default),
        );

        let mut crop_planar_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage,
                vec![64, 64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_unaligned_ret = crop_planar_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_planar_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_planar_unaligned),
        );
    }

    for (pixel_format, name, log2_chroma_w, log2_chroma_h, sample_bytes) in [
        (
            PixelFormat::Yuv420p9Le,
            "yuv420p9le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p9Be,
            "yuv420p9be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p9Le,
            "yuv422p9le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p9Be,
            "yuv422p9be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p9Le,
            "yuv444p9le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p9Be,
            "yuv444p9be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p10Le,
            "yuv420p10le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p10Be,
            "yuv420p10be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p10Le,
            "yuv422p10le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p10Be,
            "yuv422p10be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv440p10Le,
            "yuv440p10le",
            0usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv440p10Be,
            "yuv440p10be",
            0usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p10Le,
            "yuv444p10le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p10Be,
            "yuv444p10be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p10MsbLe,
            "yuv444p10msble",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p10MsbBe,
            "yuv444p10msbbe",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p12Le,
            "yuv420p12le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p12Be,
            "yuv420p12be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p12Le,
            "yuv422p12le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p12Be,
            "yuv422p12be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv440p12Le,
            "yuv440p12le",
            0usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv440p12Be,
            "yuv440p12be",
            0usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p12Le,
            "yuv444p12le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p12Be,
            "yuv444p12be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p12MsbLe,
            "yuv444p12msble",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p12MsbBe,
            "yuv444p12msbbe",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p14Le,
            "yuv420p14le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p14Be,
            "yuv420p14be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p14Le,
            "yuv422p14le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p14Be,
            "yuv422p14be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p14Le,
            "yuv444p14le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p14Be,
            "yuv444p14be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p16Le,
            "yuv420p16le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv420p16Be,
            "yuv420p16be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p16Le,
            "yuv422p16le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv422p16Be,
            "yuv422p16be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p16Le,
            "yuv444p16le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuv444p16Be,
            "yuv444p16be",
            0usize,
            0usize,
            2usize,
        ),
        (PixelFormat::Gbrp9Le, "gbrp9le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp9Be, "gbrp9be", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp10Le, "gbrp10le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp10Be, "gbrp10be", 0usize, 0usize, 2usize),
        (
            PixelFormat::Gbrp10MsbLe,
            "gbrp10msble",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Gbrp10MsbBe,
            "gbrp10msbbe",
            0usize,
            0usize,
            2usize,
        ),
        (PixelFormat::Gbrp12Le, "gbrp12le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp12Be, "gbrp12be", 0usize, 0usize, 2usize),
        (
            PixelFormat::Gbrp12MsbLe,
            "gbrp12msble",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Gbrp12MsbBe,
            "gbrp12msbbe",
            0usize,
            0usize,
            2usize,
        ),
        (PixelFormat::Gbrp14Le, "gbrp14le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp14Be, "gbrp14be", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp16Le, "gbrp16le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrp16Be, "gbrp16be", 0usize, 0usize, 2usize),
        (PixelFormat::GbrpF16Le, "gbrpf16le", 0usize, 0usize, 2usize),
        (PixelFormat::GbrpF16Be, "gbrpf16be", 0usize, 0usize, 2usize),
        (PixelFormat::GbrpF32Le, "gbrpf32le", 0usize, 0usize, 4usize),
        (PixelFormat::GbrpF32Be, "gbrpf32be", 0usize, 0usize, 4usize),
    ] {
        let planar_crop_storage = planar_yuv_strided_storage_with_sample_bytes(
            8,
            4,
            64,
            log2_chroma_w,
            log2_chroma_h,
            sample_bytes,
        );
        let mut crop_planar_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage.clone(),
                vec![64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_default.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_default_ret = crop_planar_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_planar_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_planar_default),
        );

        let mut crop_planar_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage,
                vec![64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_unaligned_ret = crop_planar_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_planar_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_planar_unaligned),
        );
    }

    for (pixel_format, name, log2_chroma_w, log2_chroma_h, sample_bytes) in [
        (
            PixelFormat::Yuva420p9Le,
            "yuva420p9le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuva420p9Be,
            "yuva420p9be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p9Le,
            "yuva422p9le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p9Be,
            "yuva422p9be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p9Le,
            "yuva444p9le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p9Be,
            "yuva444p9be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva420p10Le,
            "yuva420p10le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuva420p10Be,
            "yuva420p10be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p10Le,
            "yuva422p10le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p10Be,
            "yuva422p10be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p10Le,
            "yuva444p10le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p10Be,
            "yuva444p10be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p12Le,
            "yuva422p12le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p12Be,
            "yuva422p12be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p12Le,
            "yuva444p12le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p12Be,
            "yuva444p12be",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva420p16Le,
            "yuva420p16le",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuva420p16Be,
            "yuva420p16be",
            1usize,
            1usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p16Le,
            "yuva422p16le",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva422p16Be,
            "yuva422p16be",
            1usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p16Le,
            "yuva444p16le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::Yuva444p16Be,
            "yuva444p16be",
            0usize,
            0usize,
            2usize,
        ),
        (PixelFormat::Gbrap10Le, "gbrap10le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap10Be, "gbrap10be", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap12Le, "gbrap12le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap12Be, "gbrap12be", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap14Le, "gbrap14le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap14Be, "gbrap14be", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap16Le, "gbrap16le", 0usize, 0usize, 2usize),
        (PixelFormat::Gbrap16Be, "gbrap16be", 0usize, 0usize, 2usize),
        (
            PixelFormat::GbrapF16Le,
            "gbrapf16le",
            0usize,
            0usize,
            2usize,
        ),
        (
            PixelFormat::GbrapF16Be,
            "gbrapf16be",
            0usize,
            0usize,
            2usize,
        ),
        (PixelFormat::Gbrap32Le, "gbrap32le", 0usize, 0usize, 4usize),
        (PixelFormat::Gbrap32Be, "gbrap32be", 0usize, 0usize, 4usize),
        (
            PixelFormat::GbrapF32Le,
            "gbrapf32le",
            0usize,
            0usize,
            4usize,
        ),
        (
            PixelFormat::GbrapF32Be,
            "gbrapf32be",
            0usize,
            0usize,
            4usize,
        ),
    ] {
        let planar_crop_storage = planar_alpha_strided_storage_with_sample_bytes(
            8,
            4,
            64,
            log2_chroma_w,
            log2_chroma_h,
            sample_bytes,
        );
        let mut crop_planar_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage.clone(),
                vec![64, 64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_default.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_default_ret = crop_planar_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_planar_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_planar_default),
        );

        let mut crop_planar_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                planar_crop_storage,
                vec![64, 64, 64, 64],
            )
            .unwrap(),
        );
        crop_planar_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_planar_unaligned_ret = crop_planar_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_planar_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_planar_unaligned),
        );
    }

    for (pixel_format, name, bits_per_pixel) in [
        (PixelFormat::MonoWhite, "monow", 1usize),
        (PixelFormat::MonoBlack, "monob", 1usize),
        (PixelFormat::Rgb4, "rgb4", 4usize),
        (PixelFormat::Bgr4, "bgr4", 4usize),
    ] {
        let bitstream_crop_storage = bitstream_strided_storage(8, 4, bits_per_pixel, 64);
        let mut crop_bitstream_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                vec![bitstream_crop_storage.clone()],
                vec![64],
            )
            .unwrap(),
        );
        crop_bitstream_default.set_crop_offsets(1, 0, 1, 1);
        let crop_bitstream_default_ret = crop_bitstream_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-default-ret"),
            vec![crop_bitstream_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-default"),
            frame_fields(&crop_bitstream_default),
        );

        let mut crop_bitstream_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                vec![bitstream_crop_storage],
                vec![64],
            )
            .unwrap(),
        );
        crop_bitstream_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_bitstream_unaligned_ret = crop_bitstream_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned-ret"),
            vec![crop_bitstream_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{name}-unaligned"),
            frame_fields(&crop_bitstream_unaligned),
        );
    }

    for (pixel_format, bytes_per_pixel, line_size) in [
        (PixelFormat::Pal8, 1, 64),
        (PixelFormat::Rgb8, 1, 64),
        (PixelFormat::Bgr8, 1, 64),
        (PixelFormat::Rgb4Byte, 1, 64),
        (PixelFormat::Bgr4Byte, 1, 64),
        (PixelFormat::BayerBggr8, 1, 64),
        (PixelFormat::BayerRggb8, 1, 64),
        (PixelFormat::BayerGbrg8, 1, 64),
        (PixelFormat::BayerGrbg8, 1, 64),
        (PixelFormat::Yuyv422, 2, 64),
        (PixelFormat::Uyvy422, 2, 64),
        (PixelFormat::Yvyu422, 2, 64),
        (PixelFormat::Rgba, 4, 64),
        (PixelFormat::Bgra, 4, 64),
        (PixelFormat::Argb, 4, 64),
        (PixelFormat::Abgr, 4, 64),
        (PixelFormat::ZeroRgb, 4, 64),
        (PixelFormat::Rgb0, 4, 64),
        (PixelFormat::ZeroBgr, 4, 64),
        (PixelFormat::Bgr0, 4, 64),
        (PixelFormat::X2Rgb10Le, 4, 64),
        (PixelFormat::X2Rgb10Be, 4, 64),
        (PixelFormat::X2Bgr10Le, 4, 64),
        (PixelFormat::X2Bgr10Be, 4, 64),
        (PixelFormat::Rgb565Be, 2, 64),
        (PixelFormat::Rgb565Le, 2, 64),
        (PixelFormat::Rgb555Be, 2, 64),
        (PixelFormat::Rgb555Le, 2, 64),
        (PixelFormat::Bgr565Be, 2, 64),
        (PixelFormat::Bgr565Le, 2, 64),
        (PixelFormat::Bgr555Be, 2, 64),
        (PixelFormat::Bgr555Le, 2, 64),
        (PixelFormat::Rgb444Le, 2, 64),
        (PixelFormat::Rgb444Be, 2, 64),
        (PixelFormat::Bgr444Le, 2, 64),
        (PixelFormat::Bgr444Be, 2, 64),
        (PixelFormat::BayerBggr16Le, 2, 64),
        (PixelFormat::BayerBggr16Be, 2, 64),
        (PixelFormat::BayerRggb16Le, 2, 64),
        (PixelFormat::BayerRggb16Be, 2, 64),
        (PixelFormat::BayerGbrg16Le, 2, 64),
        (PixelFormat::BayerGbrg16Be, 2, 64),
        (PixelFormat::BayerGrbg16Le, 2, 64),
        (PixelFormat::BayerGrbg16Be, 2, 64),
        (PixelFormat::Ya8, 2, 64),
        (PixelFormat::Ya16Le, 4, 64),
        (PixelFormat::Ya16Be, 4, 64),
        (PixelFormat::Yaf16Le, 4, 64),
        (PixelFormat::Yaf16Be, 4, 64),
        (PixelFormat::Yaf32Le, 8, 64),
        (PixelFormat::Yaf32Be, 8, 64),
        (PixelFormat::Gray9Le, 2, 64),
        (PixelFormat::Gray9Be, 2, 64),
        (PixelFormat::Gray10Le, 2, 64),
        (PixelFormat::Gray10Be, 2, 64),
        (PixelFormat::Gray12Le, 2, 64),
        (PixelFormat::Gray12Be, 2, 64),
        (PixelFormat::Gray14Le, 2, 64),
        (PixelFormat::Gray14Be, 2, 64),
        (PixelFormat::Gray16Le, 2, 64),
        (PixelFormat::Gray16Be, 2, 64),
        (PixelFormat::Gray32Le, 4, 64),
        (PixelFormat::Gray32Be, 4, 64),
        (PixelFormat::GrayF16Le, 2, 64),
        (PixelFormat::GrayF16Be, 2, 64),
        (PixelFormat::GrayF32Le, 4, 64),
        (PixelFormat::GrayF32Be, 4, 64),
        (PixelFormat::Y210Le, 4, 64),
        (PixelFormat::Y210Be, 4, 64),
        (PixelFormat::Y212Le, 4, 64),
        (PixelFormat::Y212Be, 4, 64),
        (PixelFormat::Y216Le, 4, 64),
        (PixelFormat::Y216Be, 4, 64),
        (PixelFormat::Rgb48Le, 6, 192),
        (PixelFormat::Rgb48Be, 6, 192),
        (PixelFormat::RgbF16Le, 6, 192),
        (PixelFormat::RgbF16Be, 6, 192),
        (PixelFormat::Bgr48Le, 6, 192),
        (PixelFormat::Bgr48Be, 6, 192),
        (PixelFormat::Rgba64Le, 8, 64),
        (PixelFormat::Rgba64Be, 8, 64),
        (PixelFormat::RgbaF16Le, 8, 64),
        (PixelFormat::RgbaF16Be, 8, 64),
        (PixelFormat::Bgra64Le, 8, 64),
        (PixelFormat::Bgra64Be, 8, 64),
        (PixelFormat::Ayuv64Le, 8, 64),
        (PixelFormat::Ayuv64Be, 8, 64),
        (PixelFormat::RgbF32Le, 12, 192),
        (PixelFormat::RgbF32Be, 12, 192),
        (PixelFormat::Rgb96Le, 12, 192),
        (PixelFormat::Rgb96Be, 12, 192),
        (PixelFormat::RgbaF32Le, 16, 128),
        (PixelFormat::RgbaF32Be, 16, 128),
        (PixelFormat::Rgba128Le, 16, 128),
        (PixelFormat::Rgba128Be, 16, 128),
        (PixelFormat::Vuya, 4, 64),
        (PixelFormat::Vuyx, 4, 64),
        (PixelFormat::Xv30Le, 4, 64),
        (PixelFormat::Xv30Be, 4, 64),
        (PixelFormat::Xv36Le, 8, 64),
        (PixelFormat::Xv36Be, 8, 64),
        (PixelFormat::Xv48Le, 8, 64),
        (PixelFormat::Xv48Be, 8, 64),
        (PixelFormat::V30xLe, 4, 64),
        (PixelFormat::V30xBe, 4, 64),
        (PixelFormat::Ayuv, 4, 64),
        (PixelFormat::Uyva, 4, 64),
        (PixelFormat::Vyu444, 3, 192),
        (PixelFormat::Xyz12Le, 6, 192),
        (PixelFormat::Xyz12Be, 6, 192),
    ] {
        let row_name = pixel_format.name();
        let packed_crop_storage = packed_strided_storage(8, 4, bytes_per_pixel, line_size);
        let mut crop_packed_default = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                vec![packed_crop_storage.clone()],
                vec![line_size],
            )
            .unwrap(),
        );
        crop_packed_default.set_crop_offsets(1, 0, 1, 1);
        let crop_packed_default_ret = crop_packed_default
            .apply_cropping(FrameCropFlags::NONE)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{row_name}-default-ret"),
            vec![crop_packed_default_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{row_name}-default"),
            frame_fields(&crop_packed_default),
        );

        let mut crop_packed_unaligned = Frame::video(
            VideoFrame::new_with_line_sizes(
                8,
                4,
                pixel_format,
                vec![packed_crop_storage],
                vec![line_size],
            )
            .unwrap(),
        );
        crop_packed_unaligned.set_crop_offsets(1, 0, 1, 1);
        let crop_packed_unaligned_ret = crop_packed_unaligned
            .apply_cropping(FrameCropFlags::UNALIGNED)
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
        rows.insert(
            format!("frame:apply-crop-{row_name}-unaligned-ret"),
            vec![crop_packed_unaligned_ret.to_string()],
        );
        rows.insert(
            format!("frame:apply-crop-{row_name}-unaligned"),
            frame_fields(&crop_packed_unaligned),
        );
    }

    let mut invalid_crop = Frame::video(
        VideoFrame::new_with_line_sizes(6, 4, PixelFormat::Gray8, vec![crop_storage], vec![64])
            .unwrap(),
    );
    invalid_crop.set_crop_offsets(1, 0, 5, 1);
    let invalid_crop_ret = invalid_crop
        .apply_cropping(FrameCropFlags::NONE)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:apply-crop-invalid-ret".to_string(),
        vec![invalid_crop_ret.to_string()],
    );
    rows.insert(
        "frame:apply-crop-invalid".to_string(),
        frame_fields(&invalid_crop),
    );

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
    copy_source.set_sample_rate(22_050);
    copy_source.set_channel_layout(Some(ChannelLayout::stereo()));
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
    copy_destination.set_sample_rate(96_000);
    copy_destination.set_channel_layout_spec(Some(ChannelLayoutSpec::unspecified(6).unwrap()));
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

    let replace_source_side = BufferRef::copy_from_slice(&[0x44; 36]);
    let replace_source_hw = BufferRef::copy_from_slice(&[0x55]);
    let replace_source_opaque_ref = BufferRef::copy_from_slice(&[0x66, 0x67]);
    let replace_source_video = VideoFrame::new_with_aligned_line_sizes(
        2,
        2,
        PixelFormat::Gray8,
        vec![vec![10, 11, 12, 13]],
        1,
    )
    .unwrap();
    let mut replace_source =
        Frame::video(replace_source_video).with_hw_frames_context(replace_source_hw);
    replace_source.set_pts(Some(410));
    replace_source.set_pkt_dts(Some(409));
    replace_source.set_duration(408).unwrap();
    replace_source
        .set_time_base(Rational::new(1, 1_000).unwrap())
        .unwrap();
    replace_source.set_sample_rate(32_000);
    replace_source.set_channel_layout(Some(ChannelLayout::mono()));
    replace_source
        .set_sample_aspect_ratio(Rational::new(4, 3).unwrap())
        .unwrap();
    replace_source.set_crop_offsets(1, 0, 0, 1);
    replace_source.set_picture_type(FramePictureType::I);
    replace_source.set_quality(77);
    replace_source.set_repeat_pict(2);
    replace_source.set_flags(FrameFlags::KEY | FrameFlags::LOSSLESS);
    replace_source.set_color_range(FrameColorRange::Jpeg);
    replace_source.set_color_primaries(FrameColorPrimaries::Bt2020);
    replace_source.set_color_transfer_characteristic(FrameColorTransferCharacteristic::Smpte2084);
    replace_source.set_color_space(FrameColorSpace::Bt2020Ncl);
    replace_source.set_chroma_location(FrameChromaLocation::TopLeft);
    replace_source.set_best_effort_timestamp(Some(411));
    replace_source.set_decode_error_flags(FrameDecodeErrorFlags::MISSING_REFERENCE);
    replace_source.set_opaque_address(0x5151);
    replace_source.set_opaque_ref(Some(replace_source_opaque_ref));
    replace_source.set_alpha_mode(FrameAlphaMode::Straight);
    replace_source
        .metadata_mut()
        .set("title", "replace-source")
        .unwrap();
    replace_source
        .set_side_data_kind_buffer(FrameSideDataKind::DisplayMatrix, replace_source_side)
        .unwrap();

    let mut ref_destination = Frame::empty();
    ref_destination.ref_from(&replace_source);
    rows.insert("frame:ref-rich-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "frame:ref-rich-src".to_string(),
        frame_fields(&replace_source),
    );
    rows.insert(
        "frame:ref-rich-dst".to_string(),
        frame_fields(&ref_destination),
    );
    rows.insert(
        "frame:ref-rich-plane-shares".to_string(),
        first_plane_share_fields(&replace_source, &ref_destination),
    );
    rows.insert(
        "frame:ref-rich-side-shares".to_string(),
        first_side_data_share_fields(&replace_source, &ref_destination),
    );
    rows.insert(
        "frame:ref-rich-hw-shares".to_string(),
        hw_frames_context_share_fields(&replace_source, &ref_destination),
    );
    let ref_after_unref = {
        let mut source_copy = replace_source.clone();
        source_copy.unref();
        source_copy
    };
    rows.insert(
        "frame:ref-rich-src-after-unref".to_string(),
        frame_fields(&ref_after_unref),
    );
    rows.insert(
        "frame:ref-rich-dst-after-source-unref".to_string(),
        frame_fields(&ref_destination),
    );
    drop(ref_destination);

    let mut replace_clone = replace_source.clone_ref();
    rows.insert(
        "frame:clone-ref-src".to_string(),
        frame_fields(&replace_source),
    );
    rows.insert(
        "frame:clone-ref-dst".to_string(),
        frame_fields(&replace_clone),
    );
    rows.insert(
        "frame:clone-ref-plane-shares".to_string(),
        first_plane_share_fields(&replace_source, &replace_clone),
    );
    rows.insert(
        "frame:clone-ref-side-shares".to_string(),
        first_side_data_share_fields(&replace_source, &replace_clone),
    );
    rows.insert(
        "frame:clone-ref-hw-shares".to_string(),
        hw_frames_context_share_fields(&replace_source, &replace_clone),
    );
    replace_clone.take_hw_frames_context();
    rows.insert(
        "frame:clone-ref-after-take-hw-context".to_string(),
        frame_fields(&replace_clone),
    );
    let rich_make_writable_ret = replace_clone
        .try_make_writable()
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:rich-make-writable-ret".to_string(),
        vec![rich_make_writable_ret.to_string()],
    );
    rows.insert(
        "frame:rich-after-make-writable-src".to_string(),
        frame_fields(&replace_source),
    );
    rows.insert(
        "frame:rich-after-make-writable-dst".to_string(),
        frame_fields(&replace_clone),
    );
    rows.insert(
        "frame:rich-after-make-writable-plane-shares".to_string(),
        first_plane_share_fields(&replace_source, &replace_clone),
    );
    rows.insert(
        "frame:rich-after-make-writable-side-shares".to_string(),
        first_side_data_share_fields(&replace_source, &replace_clone),
    );
    rows.insert(
        "frame:rich-after-make-writable-opaque-ref-shares".to_string(),
        opaque_ref_share_fields(&replace_source, &replace_clone),
    );

    let replace_destination_video =
        VideoFrame::new_with_aligned_line_sizes(1, 1, PixelFormat::Gray8, vec![vec![9]], 1)
            .unwrap();
    let mut replace_destination = Frame::video(replace_destination_video)
        .with_hw_frames_context(BufferRef::copy_from_slice(&[0x99]));
    replace_destination.set_pts(Some(999));
    replace_destination
        .metadata_mut()
        .set("keep", "destination")
        .unwrap();
    replace_destination
        .set_side_data_kind(FrameSideDataKind::ReplayGain, vec![0x99; 16])
        .unwrap();
    let replace_ret = replace_destination
        .replace_from(&replace_source)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:replace-ret".to_string(),
        vec![replace_ret.to_string()],
    );
    rows.insert(
        "frame:replace-src".to_string(),
        frame_fields(&replace_source),
    );
    rows.insert(
        "frame:replace-dst".to_string(),
        frame_fields(&replace_destination),
    );
    rows.insert(
        "frame:replace-plane-shares".to_string(),
        first_plane_share_fields(&replace_source, &replace_destination),
    );
    rows.insert(
        "frame:replace-side-shares".to_string(),
        first_side_data_share_fields(&replace_source, &replace_destination),
    );
    rows.insert(
        "frame:replace-hw-shares".to_string(),
        hw_frames_context_share_fields(&replace_source, &replace_destination),
    );

    let mut replace_empty_destination = replace_source.clone_ref();
    let replace_empty_ret = replace_empty_destination
        .replace_from(&Frame::empty())
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:replace-empty-ret".to_string(),
        vec![replace_empty_ret.to_string()],
    );
    rows.insert(
        "frame:replace-empty-dst".to_string(),
        frame_fields(&replace_empty_destination),
    );

    let move_replace_destination_video =
        VideoFrame::new_with_aligned_line_sizes(1, 1, PixelFormat::Gray8, vec![vec![0xAA]], 1)
            .unwrap();
    let mut move_replace_destination = Frame::video(move_replace_destination_video)
        .with_hw_frames_context(BufferRef::copy_from_slice(&[0xAA]));
    move_replace_destination.set_pts(Some(1_234));
    move_replace_destination.set_opaque_ref(Some(BufferRef::copy_from_slice(&[0xAB])));
    move_replace_destination
        .metadata_mut()
        .set("keep", "move-destination")
        .unwrap();
    move_replace_destination
        .set_side_data_kind(FrameSideDataKind::ReplayGain, vec![0xAA; 16])
        .unwrap();
    move_replace_destination.move_ref_from(&mut replace_source);
    rows.insert(
        "frame:move-replace-dst".to_string(),
        frame_fields(&move_replace_destination),
    );
    rows.insert(
        "frame:move-replace-src".to_string(),
        frame_fields(&replace_source),
    );
    rows.insert(
        "frame:move-replace-plane-shares".to_string(),
        first_plane_share_fields(&replace_destination, &move_replace_destination),
    );
    rows.insert(
        "frame:move-replace-side-shares".to_string(),
        first_side_data_share_fields(&replace_destination, &move_replace_destination),
    );
    rows.insert(
        "frame:move-replace-hw-shares".to_string(),
        hw_frames_context_share_fields(&replace_destination, &move_replace_destination),
    );

    let mut fifo = FrameFifo::new();
    rows.insert(
        "frame:fifo-new-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let mut fifo_move_src = frame_with_fifo_props();
    let fifo_write_move_ret = fifo
        .write_move(&mut fifo_move_src)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:fifo-write-move-ret".to_string(),
        vec![fifo_write_move_ret.to_string()],
    );
    rows.insert(
        "frame:fifo-write-move-src".to_string(),
        frame_fields(&fifo_move_src),
    );
    rows.insert(
        "frame:fifo-after-write-move-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let fifo_peek0 = fifo.peek(0);
    rows.insert(
        "frame:fifo-peek0-ret".to_string(),
        vec![fifo_peek0
            .as_ref()
            .map(|_| 0)
            .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1))
            .to_string()],
    );
    rows.insert(
        "frame:fifo-peek0".to_string(),
        frame_fields(fifo_peek0.expect("peek 0 should succeed")),
    );
    let fifo_peek1_ret = fifo
        .peek(1)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:fifo-peek1-ret".to_string(),
        vec![fifo_peek1_ret.to_string()],
    );
    let mut fifo_move_dst = Frame::empty();
    let fifo_read_move_ret = fifo
        .read_move(&mut fifo_move_dst)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:fifo-read-move-ret".to_string(),
        vec![fifo_read_move_ret.to_string()],
    );
    rows.insert(
        "frame:fifo-read-move-dst".to_string(),
        frame_fields(&fifo_move_dst),
    );
    rows.insert(
        "frame:fifo-after-read-move-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let fifo_ref_src = frame_with_fifo_props();
    let fifo_write_ref_ret = fifo
        .write_ref(&fifo_ref_src)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:fifo-write-ref-ret".to_string(),
        vec![fifo_write_ref_ret.to_string()],
    );
    rows.insert(
        "frame:fifo-write-ref-src".to_string(),
        frame_fields(&fifo_ref_src),
    );
    rows.insert(
        "frame:fifo-after-write-ref-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let mut fifo_ref_dst = Frame::empty();
    let fifo_read_ref_ret = fifo
        .read_ref(&mut fifo_ref_dst)
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:fifo-read-ref-ret".to_string(),
        vec![fifo_read_ref_ret.to_string()],
    );
    rows.insert(
        "frame:fifo-read-ref-dst".to_string(),
        frame_fields(&fifo_ref_dst),
    );
    rows.insert(
        "frame:fifo-after-read-ref-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );

    let mut fifo_first = frame_with_fifo_props();
    fifo_first.set_pts(Some(611));
    fifo.write_move(&mut fifo_first).unwrap();
    let mut fifo_second = frame_with_fifo_props();
    fifo_second.set_pts(Some(612));
    fifo.write_move(&mut fifo_second).unwrap();
    rows.insert(
        "frame:fifo-before-drain-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    fifo.drain(1).unwrap();
    rows.insert(
        "frame:fifo-after-drain-one-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    rows.insert(
        "frame:fifo-after-drain-one-peek".to_string(),
        frame_fields(fifo.peek(0).expect("second drained frame should remain")),
    );
    fifo.drain(1).unwrap();
    rows.insert(
        "frame:fifo-after-drain-all-can-read".to_string(),
        vec![fifo.can_read().to_string()],
    );
    let fifo_read_empty_ret = fifo
        .read_move(&mut Frame::empty())
        .map(|_| 0)
        .unwrap_or_else(|err| err.code().map(AvErrorCode::raw).unwrap_or(-1));
    rows.insert(
        "frame:fifo-read-empty-ret".to_string(),
        vec![fifo_read_empty_ret.to_string()],
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

    let mut side_from_buf = Frame::empty();
    let mut frame_take_source = Some(BufferRef::copy_from_slice(&[0x91, 0x92, 0x93]));
    let frame_take_success = side_from_buf
        .new_side_data_kind_buffer(FrameSideDataKind::ReplayGain, &mut frame_take_source)
        .is_ok();
    rows.insert(
        "frame:side-data-from-buf-take".to_string(),
        side_add_buffer_fields(
            frame_take_success,
            &side_from_buf,
            frame_take_source.as_ref(),
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut frame_duplicate_source = Some(BufferRef::copy_from_slice(&[0x94]));
    let frame_duplicate_success = side_from_buf
        .new_side_data_kind_buffer(FrameSideDataKind::ReplayGain, &mut frame_duplicate_source)
        .is_ok();
    rows.insert(
        "frame:side-data-from-buf-duplicate".to_string(),
        side_add_buffer_fields(
            frame_duplicate_success,
            &side_from_buf,
            frame_duplicate_source.as_ref(),
            &FrameSideDataKind::ReplayGain,
        ),
    );
    let mut frame_multi_source = Some(BufferRef::copy_from_slice(&[0x95; 16]));
    let frame_multi_success = side_from_buf
        .new_side_data_kind_buffer(FrameSideDataKind::SeiUnregistered, &mut frame_multi_source)
        .is_ok();
    rows.insert(
        "frame:side-data-from-buf-multi".to_string(),
        side_add_buffer_fields(
            frame_multi_success,
            &side_from_buf,
            frame_multi_source.as_ref(),
            &FrameSideDataKind::SeiUnregistered,
        ),
    );
    let duplicate_found = side_from_buf.side_data_by_kind(&FrameSideDataKind::ReplayGain);
    rows.insert(
        "frame:side-data-get-duplicate".to_string(),
        vec![
            side_from_buf.side_data().len().to_string(),
            bool_field(
                duplicate_found
                    .map(|side_data| side_data.data() == [0x91, 0x92, 0x93].as_slice())
                    .unwrap_or(false),
            ),
            duplicate_found
                .map(|side_data| side_data.data().len().to_string())
                .unwrap_or_else(|| "0".to_string()),
            duplicate_found
                .map(|side_data| hex(side_data.data()))
                .unwrap_or_else(|| "none".to_string()),
        ],
    );
    let removed_duplicates =
        side_from_buf.remove_all_side_data_kind(&FrameSideDataKind::ReplayGain);
    rows.insert(
        "frame:side-data-remove-duplicate".to_string(),
        vec![
            removed_duplicates.len().to_string(),
            side_from_buf.side_data().len().to_string(),
            side_summary(side_from_buf.side_data()),
        ],
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
    let multi_found = side_array.side_data_by_kind(&FrameSideDataKind::SeiUnregistered);
    rows.insert(
        "frame:side-array-get-multi-first".to_string(),
        vec![
            side_array.side_data().len().to_string(),
            bool_field(
                multi_found
                    .map(|side_data| side_data.data() == [0x77; 16].as_slice())
                    .unwrap_or(false),
            ),
            multi_found
                .map(|side_data| side_data.data().len().to_string())
                .unwrap_or_else(|| "0".to_string()),
            multi_found
                .map(|side_data| hex(side_data.data()))
                .unwrap_or_else(|| "none".to_string()),
        ],
    );
    let missing_found = side_array.side_data_by_kind(&FrameSideDataKind::FilmGrainParams);
    rows.insert(
        "frame:side-array-get-missing".to_string(),
        vec![
            bool_field(missing_found.is_some()),
            side_array.side_data().len().to_string(),
        ],
    );
    side_array.remove_side_data_by_properties(FrameSideDataProperties::MULTI);
    rows.insert(
        "frame:side-array-remove-multi".to_string(),
        side_array_fields(&side_array),
    );
    side_array.clear_side_data();
    rows.insert(
        "frame:side-array-free".to_string(),
        vec![
            side_array.side_data().len().to_string(),
            bool_field(side_array.side_data().is_empty()),
        ],
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

fn gray8_incrementing_payload(width: usize, height: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(width * height);
    for row in 0..height {
        for column in 0..width {
            payload.push((row * 16 + column) as u8);
        }
    }
    payload
}

fn gray8_strided_storage(width: usize, height: usize, line_size: usize, visible: &[u8]) -> Vec<u8> {
    assert_eq!(visible.len(), width * height);
    let mut storage = vec![0; line_size * height];
    for row in 0..height {
        let src_start = row * width;
        let src_end = src_start + width;
        let dst_start = row * line_size;
        let dst_end = dst_start + width;
        storage[dst_start..dst_end].copy_from_slice(&visible[src_start..src_end]);
    }
    storage
}

fn packed_strided_storage(
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    line_size: usize,
) -> Vec<u8> {
    let visible_row_bytes = width * bytes_per_pixel;
    let mut storage = vec![0; line_size * height];
    for row in 0..height {
        let dst_start = row * line_size;
        for column in 0..visible_row_bytes {
            storage[dst_start + column] = (row * 16 + column) as u8;
        }
    }
    storage
}

fn uyyvyy411_row_bytes(width: usize) -> usize {
    width.div_ceil(4) * 6
}

fn uyyvyy411_strided_storage(width: usize, height: usize, line_size: usize) -> Vec<u8> {
    let visible_row_bytes = uyyvyy411_row_bytes(width);
    let mut storage = vec![0; line_size * height];
    for row in 0..height {
        let dst_start = row * line_size;
        for column in 0..visible_row_bytes {
            storage[dst_start + column] = (row * 16 + column) as u8;
        }
    }
    storage
}

fn bitstream_row_bytes(width: usize, bits_per_pixel: usize) -> usize {
    (width * bits_per_pixel).div_ceil(8)
}

fn bitstream_strided_storage(
    width: usize,
    height: usize,
    bits_per_pixel: usize,
    line_size: usize,
) -> Vec<u8> {
    let visible_row_bytes = bitstream_row_bytes(width, bits_per_pixel);
    let mut storage = vec![0; line_size * height];
    for row in 0..height {
        let dst_start = row * line_size;
        for column in 0..visible_row_bytes {
            storage[dst_start + column] = (row * 16 + column) as u8;
        }
    }
    storage
}

fn strided_plane_sample_storage(
    width: usize,
    height: usize,
    line_size: usize,
    base: u8,
    sample_bytes: usize,
) -> Vec<u8> {
    let mut storage = vec![0; line_size * height];
    for row in 0..height {
        let dst_start = row * line_size;
        for column in 0..width * sample_bytes {
            storage[dst_start + column] = base.wrapping_add((row * 16 + column) as u8);
        }
    }
    storage
}

fn yuv420p_strided_storage(width: usize, height: usize, line_size: usize) -> Vec<Vec<u8>> {
    planar_yuv_strided_storage(width, height, line_size, 1, 1)
}

fn planar_yuv_strided_storage(
    width: usize,
    height: usize,
    line_size: usize,
    log2_chroma_w: usize,
    log2_chroma_h: usize,
) -> Vec<Vec<u8>> {
    planar_yuv_strided_storage_with_sample_bytes(
        width,
        height,
        line_size,
        log2_chroma_w,
        log2_chroma_h,
        1,
    )
}

fn semiplanar_yuv_strided_storage(
    width: usize,
    height: usize,
    luma_line_size: usize,
    chroma_line_size: usize,
    log2_chroma_w: usize,
    log2_chroma_h: usize,
    sample_bytes: (usize, usize),
) -> Vec<Vec<u8>> {
    vec![
        strided_plane_sample_storage(width, height, luma_line_size, 0x10, sample_bytes.0),
        strided_plane_sample_storage(
            width >> log2_chroma_w,
            height >> log2_chroma_h,
            chroma_line_size,
            0x80,
            sample_bytes.1,
        ),
    ]
}

fn planar_yuv_strided_storage_with_sample_bytes(
    width: usize,
    height: usize,
    line_size: usize,
    log2_chroma_w: usize,
    log2_chroma_h: usize,
    sample_bytes: usize,
) -> Vec<Vec<u8>> {
    vec![
        strided_plane_sample_storage(width, height, line_size, 0x10, sample_bytes),
        strided_plane_sample_storage(
            width >> log2_chroma_w,
            height >> log2_chroma_h,
            line_size,
            0x80,
            sample_bytes,
        ),
        strided_plane_sample_storage(
            width >> log2_chroma_w,
            height >> log2_chroma_h,
            line_size,
            0xc0,
            sample_bytes,
        ),
    ]
}

fn planar_alpha_strided_storage_with_sample_bytes(
    width: usize,
    height: usize,
    line_size: usize,
    log2_chroma_w: usize,
    log2_chroma_h: usize,
    sample_bytes: usize,
) -> Vec<Vec<u8>> {
    let mut storage = planar_yuv_strided_storage_with_sample_bytes(
        width,
        height,
        line_size,
        log2_chroma_w,
        log2_chroma_h,
        sample_bytes,
    );
    storage.push(strided_plane_sample_storage(
        width,
        height,
        line_size,
        0xe0,
        sample_bytes,
    ));
    storage
}

fn frame_with_fifo_props() -> Frame {
    let video = VideoFrame::new_with_aligned_line_sizes(
        2,
        1,
        PixelFormat::Gray8,
        vec![vec![0xaa, 0xbb]],
        1,
    )
    .unwrap();
    let mut frame = Frame::video(video).with_hw_frames_context(BufferRef::copy_from_slice(&[0xee]));
    frame.set_pts(Some(410));
    frame.set_pkt_dts(Some(409));
    frame.set_duration(3).unwrap();
    frame
        .set_time_base(Rational::new(1, 90_000).unwrap())
        .unwrap();
    frame
        .set_sample_aspect_ratio(Rational::new(1, 1).unwrap())
        .unwrap();
    frame.set_picture_type(FramePictureType::P);
    frame.set_quality(17);
    frame.set_repeat_pict(1);
    frame.set_flags(FrameFlags::KEY | FrameFlags::LOSSLESS);
    frame.set_opaque_ref(Some(BufferRef::copy_from_slice(&[0xde, 0xad])));
    frame.metadata_mut().set("title", "fifo").unwrap();
    frame
        .set_side_data_kind(FrameSideDataKind::DisplayMatrix, vec![0x33; 36])
        .unwrap();
    frame
}

fn frame_fields(frame: &Frame) -> Vec<String> {
    let pts = frame.pts().unwrap_or(AV_NOPTS_VALUE);
    let pkt_dts = frame.pkt_dts().unwrap_or(AV_NOPTS_VALUE);
    let crop = frame.crop();
    let top_level_channels = frame.channel_count();
    let (kind, format, width, height, nb_samples, line_sizes, planes) = match frame.data() {
        FrameData::Empty => (
            "empty",
            "none",
            0usize,
            0usize,
            0usize,
            Vec::new(),
            Vec::new(),
        ),
        FrameData::Video(video) => (
            "video",
            video.pixel_format_name(),
            video.width(),
            video.height(),
            0usize,
            video.line_sizes().to_vec(),
            video.planes().to_vec(),
        ),
        FrameData::Audio(audio) if audio.samples_per_channel() > 0 && top_level_channels > 0 => (
            "audio",
            audio.sample_format_name(),
            0usize,
            0usize,
            audio.samples_per_channel(),
            audio.ffmpeg_line_sizes(),
            audio.planes().to_vec(),
        ),
        FrameData::Audio(_) => (
            "empty",
            "none",
            0usize,
            0usize,
            0usize,
            Vec::new(),
            Vec::new(),
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
        frame.sample_rate().to_string(),
        frame.channel_count().to_string(),
        join_usizes(&line_sizes),
        join_hex_planes(&planes),
        bool_field(frame.is_writable()),
        frame.side_data().len().to_string(),
        side_summary(frame.side_data()),
        bool_field(frame.hw_frames_context().is_some()),
    ]
}

fn frame_buffer_topology_fields(frame: &Frame) -> Vec<String> {
    let topology = frame.buffer_topology();
    frame_buffer_topology_values(topology)
}

fn frame_plane_buffer_fields(frame: &Frame, index: usize) -> Vec<String> {
    match frame.plane_buffer(index) {
        Some(buffer) => vec![
            "1".to_string(),
            hex(buffer.as_slice()),
            buffer.strong_count().to_string(),
            bool_field(buffer.is_writable()),
        ],
        None => vec![
            "0".to_string(),
            "none".to_string(),
            "0".to_string(),
            "0".to_string(),
        ],
    }
}

fn pal8_frame_fields(frame: &Frame) -> Vec<String> {
    match frame.data() {
        FrameData::Video(video) => vec![
            "video".to_string(),
            video.pixel_format_name().to_string(),
            video.width().to_string(),
            video.height().to_string(),
            join_usizes(video.line_sizes()),
            join_hex_planes(video.planes()),
            bool_field(video.is_writable()),
            video.plane_buffers().len().to_string(),
        ],
        _ => vec![
            "empty".to_string(),
            "none".to_string(),
            "0".to_string(),
            "0".to_string(),
            "none".to_string(),
            "none".to_string(),
            "0".to_string(),
            "0".to_string(),
        ],
    }
}

fn pal8_plane_buffer_fields(frame: &Frame, index: usize) -> Vec<String> {
    match frame.data() {
        FrameData::Video(video) => match video.plane_buffer(index) {
            Some(buffer) => vec![
                "1".to_string(),
                hex(&video.planes()[index]),
                buffer.strong_count().to_string(),
                bool_field(buffer.is_writable()),
            ],
            None => vec![
                "0".to_string(),
                "none".to_string(),
                "0".to_string(),
                "0".to_string(),
            ],
        },
        _ => vec![
            "0".to_string(),
            "none".to_string(),
            "0".to_string(),
            "0".to_string(),
        ],
    }
}

fn pal8_plane_share_fields(left: &Frame, right: &Frame, index: usize) -> Vec<String> {
    let left = pal8_plane_buffer(left, index);
    let right = pal8_plane_buffer(right, index);
    buffer_share_fields(left, right)
}

fn pal8_plane_buffer(frame: &Frame, index: usize) -> &BufferRef {
    match frame.data() {
        FrameData::Video(video) if video.pixel_format() == PixelFormat::Pal8 => video
            .plane_buffer(index)
            .unwrap_or_else(|| panic!("pal8 frame has no plane buffer {index}")),
        _ => panic!("frame is not a pal8 video frame"),
    }
}

fn pal8_crop_palette_preserved_fields(frame: &Frame, palette: &BufferRef) -> Vec<String> {
    let FrameData::Video(video) = frame.data() else {
        return vec!["0".to_string(), "0".to_string(), "none".to_string()];
    };
    vec![
        bool_field(video.plane_buffers()[1].shares_storage(palette)),
        video.line_sizes()[1].to_string(),
        join_hex_planes(video.planes()),
    ]
}

fn frame_buffer_topology_values(topology: FrameBufferTopology) -> Vec<String> {
    vec![
        AV_NUM_DATA_POINTERS.to_string(),
        topology.direct_data_slots().to_string(),
        topology.data_pointer_count().to_string(),
        topology.direct_buffer_refs().to_string(),
        topology.extended_buffer_refs().to_string(),
        topology.writable_direct_buffer_refs().to_string(),
        topology.writable_extended_buffer_refs().to_string(),
        bool_field(topology.uses_separate_extended_data()),
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

fn opaque_ref_share_fields(left: &Frame, right: &Frame) -> Vec<String> {
    let left = left
        .opaque_ref()
        .expect("left frame should have opaque_ref");
    let right = right
        .opaque_ref()
        .expect("right frame should have opaque_ref");
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

fn pal8_index_fixture(width: usize, height: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(width * height);
    for row in 0..height {
        for column in 0..width {
            data.push((row * 16 + column) as u8);
        }
    }
    data
}

fn pal8_palette_fixture() -> Vec<u8> {
    (0..AVPALETTE_SIZE)
        .map(|index| (index as u8).wrapping_mul(37).wrapping_add(11))
        .collect()
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
#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <libavutil/avutil.h>
#include <libavutil/buffer.h>
#include <libavutil/channel_layout.h>
#include <libavutil/container_fifo.h>
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

static int bitstream_row_bytes(enum AVPixelFormat format, int width)
{
    switch (format) {
    case AV_PIX_FMT_MONOWHITE:
    case AV_PIX_FMT_MONOBLACK:
        return (width + 7) / 8;
    case AV_PIX_FMT_RGB4:
    case AV_PIX_FMT_BGR4:
        return (width + 1) / 2;
    default:
        return 0;
    }
}

static int planar_yuv_sample_bytes(enum AVPixelFormat format,
                                   int *log2_chroma_w,
                                   int *log2_chroma_h)
{
    *log2_chroma_w = 0;
    *log2_chroma_h = 0;
    switch (format) {
    case AV_PIX_FMT_YUV420P:
    case AV_PIX_FMT_YUVA420P:
    case AV_PIX_FMT_YUVJ420P:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        return 1;
    case AV_PIX_FMT_YUV422P:
    case AV_PIX_FMT_YUVA422P:
    case AV_PIX_FMT_YUVJ422P:
        *log2_chroma_w = 1;
        return 1;
    case AV_PIX_FMT_YUV410P:
        *log2_chroma_w = 2;
        *log2_chroma_h = 2;
        return 1;
    case AV_PIX_FMT_YUV411P:
    case AV_PIX_FMT_YUVJ411P:
        *log2_chroma_w = 2;
        return 1;
    case AV_PIX_FMT_YUV440P:
    case AV_PIX_FMT_YUVJ440P:
        *log2_chroma_h = 1;
        return 1;
    case AV_PIX_FMT_YUV444P:
    case AV_PIX_FMT_YUVA444P:
    case AV_PIX_FMT_YUVJ444P:
    case AV_PIX_FMT_GBRP:
    case AV_PIX_FMT_GBRAP:
        return 1;
    case AV_PIX_FMT_YUV420P9LE:
    case AV_PIX_FMT_YUV420P9BE:
    case AV_PIX_FMT_YUVA420P9LE:
    case AV_PIX_FMT_YUVA420P9BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV422P9LE:
    case AV_PIX_FMT_YUV422P9BE:
    case AV_PIX_FMT_YUVA422P9LE:
    case AV_PIX_FMT_YUVA422P9BE:
        *log2_chroma_w = 1;
        return 2;
    case AV_PIX_FMT_YUV444P9LE:
    case AV_PIX_FMT_YUV444P9BE:
    case AV_PIX_FMT_YUVA444P9LE:
    case AV_PIX_FMT_YUVA444P9BE:
        return 2;
    case AV_PIX_FMT_YUV420P10LE:
    case AV_PIX_FMT_YUV420P10BE:
    case AV_PIX_FMT_YUVA420P10LE:
    case AV_PIX_FMT_YUVA420P10BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV422P10LE:
    case AV_PIX_FMT_YUV422P10BE:
    case AV_PIX_FMT_YUVA422P10LE:
    case AV_PIX_FMT_YUVA422P10BE:
        *log2_chroma_w = 1;
        return 2;
    case AV_PIX_FMT_YUV444P10LE:
    case AV_PIX_FMT_YUV444P10BE:
    case AV_PIX_FMT_YUVA444P10LE:
    case AV_PIX_FMT_YUVA444P10BE:
        return 2;
    case AV_PIX_FMT_YUV440P10LE:
    case AV_PIX_FMT_YUV440P10BE:
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV444P10MSBLE:
    case AV_PIX_FMT_YUV444P10MSBBE:
        return 2;
    case AV_PIX_FMT_YUV420P12LE:
    case AV_PIX_FMT_YUV420P12BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV422P12LE:
    case AV_PIX_FMT_YUV422P12BE:
    case AV_PIX_FMT_YUVA422P12LE:
    case AV_PIX_FMT_YUVA422P12BE:
        *log2_chroma_w = 1;
        return 2;
    case AV_PIX_FMT_YUV444P12LE:
    case AV_PIX_FMT_YUV444P12BE:
    case AV_PIX_FMT_YUVA444P12LE:
    case AV_PIX_FMT_YUVA444P12BE:
        return 2;
    case AV_PIX_FMT_YUV440P12LE:
    case AV_PIX_FMT_YUV440P12BE:
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV444P12MSBLE:
    case AV_PIX_FMT_YUV444P12MSBBE:
        return 2;
    case AV_PIX_FMT_YUV420P14LE:
    case AV_PIX_FMT_YUV420P14BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV422P14LE:
    case AV_PIX_FMT_YUV422P14BE:
        *log2_chroma_w = 1;
        return 2;
    case AV_PIX_FMT_YUV444P14LE:
    case AV_PIX_FMT_YUV444P14BE:
        return 2;
    case AV_PIX_FMT_YUV420P16LE:
    case AV_PIX_FMT_YUV420P16BE:
    case AV_PIX_FMT_YUVA420P16LE:
    case AV_PIX_FMT_YUVA420P16BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        return 2;
    case AV_PIX_FMT_YUV422P16LE:
    case AV_PIX_FMT_YUV422P16BE:
    case AV_PIX_FMT_YUVA422P16LE:
    case AV_PIX_FMT_YUVA422P16BE:
        *log2_chroma_w = 1;
        return 2;
    case AV_PIX_FMT_YUV444P16LE:
    case AV_PIX_FMT_YUV444P16BE:
    case AV_PIX_FMT_GBRP9LE:
    case AV_PIX_FMT_GBRP9BE:
    case AV_PIX_FMT_GBRP10LE:
    case AV_PIX_FMT_GBRP10BE:
    case AV_PIX_FMT_GBRP10MSBLE:
    case AV_PIX_FMT_GBRP10MSBBE:
    case AV_PIX_FMT_GBRP12LE:
    case AV_PIX_FMT_GBRP12BE:
    case AV_PIX_FMT_GBRP12MSBLE:
    case AV_PIX_FMT_GBRP12MSBBE:
    case AV_PIX_FMT_GBRP14LE:
    case AV_PIX_FMT_GBRP14BE:
    case AV_PIX_FMT_GBRP16LE:
    case AV_PIX_FMT_GBRP16BE:
    case AV_PIX_FMT_GBRPF16LE:
    case AV_PIX_FMT_GBRPF16BE:
    case AV_PIX_FMT_YUVA444P16LE:
    case AV_PIX_FMT_YUVA444P16BE:
    case AV_PIX_FMT_GBRAP10LE:
    case AV_PIX_FMT_GBRAP10BE:
    case AV_PIX_FMT_GBRAP12LE:
    case AV_PIX_FMT_GBRAP12BE:
    case AV_PIX_FMT_GBRAP14LE:
    case AV_PIX_FMT_GBRAP14BE:
    case AV_PIX_FMT_GBRAP16LE:
    case AV_PIX_FMT_GBRAP16BE:
    case AV_PIX_FMT_GBRAPF16LE:
    case AV_PIX_FMT_GBRAPF16BE:
        return 2;
    case AV_PIX_FMT_GBRPF32LE:
    case AV_PIX_FMT_GBRPF32BE:
    case AV_PIX_FMT_GBRAP32LE:
    case AV_PIX_FMT_GBRAP32BE:
    case AV_PIX_FMT_GBRAPF32LE:
    case AV_PIX_FMT_GBRAPF32BE:
        return 4;
    default:
        return 0;
    }
}

static int planar_yuv_plane_count(enum AVPixelFormat format)
{
    switch (format) {
    case AV_PIX_FMT_YUVA420P:
    case AV_PIX_FMT_YUVA422P:
    case AV_PIX_FMT_YUVA444P:
    case AV_PIX_FMT_YUVA420P9LE:
    case AV_PIX_FMT_YUVA420P9BE:
    case AV_PIX_FMT_YUVA422P9LE:
    case AV_PIX_FMT_YUVA422P9BE:
    case AV_PIX_FMT_YUVA444P9LE:
    case AV_PIX_FMT_YUVA444P9BE:
    case AV_PIX_FMT_YUVA420P10LE:
    case AV_PIX_FMT_YUVA420P10BE:
    case AV_PIX_FMT_YUVA422P10LE:
    case AV_PIX_FMT_YUVA422P10BE:
    case AV_PIX_FMT_YUVA444P10LE:
    case AV_PIX_FMT_YUVA444P10BE:
    case AV_PIX_FMT_YUVA422P12LE:
    case AV_PIX_FMT_YUVA422P12BE:
    case AV_PIX_FMT_YUVA444P12LE:
    case AV_PIX_FMT_YUVA444P12BE:
    case AV_PIX_FMT_YUVA420P16LE:
    case AV_PIX_FMT_YUVA420P16BE:
    case AV_PIX_FMT_YUVA422P16LE:
    case AV_PIX_FMT_YUVA422P16BE:
    case AV_PIX_FMT_YUVA444P16LE:
    case AV_PIX_FMT_YUVA444P16BE:
    case AV_PIX_FMT_GBRAP:
    case AV_PIX_FMT_GBRAP10LE:
    case AV_PIX_FMT_GBRAP10BE:
    case AV_PIX_FMT_GBRAP12LE:
    case AV_PIX_FMT_GBRAP12BE:
    case AV_PIX_FMT_GBRAP14LE:
    case AV_PIX_FMT_GBRAP14BE:
    case AV_PIX_FMT_GBRAP16LE:
    case AV_PIX_FMT_GBRAP16BE:
    case AV_PIX_FMT_GBRAP32LE:
    case AV_PIX_FMT_GBRAP32BE:
    case AV_PIX_FMT_GBRAPF16LE:
    case AV_PIX_FMT_GBRAPF16BE:
    case AV_PIX_FMT_GBRAPF32LE:
    case AV_PIX_FMT_GBRAPF32BE:
        return 4;
    default:
        break;
    }

    int log2_chroma_w = 0;
    int log2_chroma_h = 0;
    return planar_yuv_sample_bytes(format, &log2_chroma_w, &log2_chroma_h)
               ? 3
               : 0;
}

static int semiplanar_yuv_layout(enum AVPixelFormat format,
                                 int *log2_chroma_w,
                                 int *log2_chroma_h,
                                 int *luma_sample_bytes,
                                 int *chroma_pair_bytes)
{
    switch (format) {
    case AV_PIX_FMT_NV12:
    case AV_PIX_FMT_NV21:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        *luma_sample_bytes = 1;
        *chroma_pair_bytes = 2;
        return 1;
    case AV_PIX_FMT_NV16:
        *log2_chroma_w = 1;
        *log2_chroma_h = 0;
        *luma_sample_bytes = 1;
        *chroma_pair_bytes = 2;
        return 1;
    case AV_PIX_FMT_NV24:
    case AV_PIX_FMT_NV42:
        *log2_chroma_w = 0;
        *log2_chroma_h = 0;
        *luma_sample_bytes = 1;
        *chroma_pair_bytes = 2;
        return 1;
    case AV_PIX_FMT_NV20LE:
    case AV_PIX_FMT_NV20BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 0;
        *luma_sample_bytes = 2;
        *chroma_pair_bytes = 4;
        return 1;
    case AV_PIX_FMT_P010LE:
    case AV_PIX_FMT_P010BE:
    case AV_PIX_FMT_P012LE:
    case AV_PIX_FMT_P012BE:
    case AV_PIX_FMT_P016LE:
    case AV_PIX_FMT_P016BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 1;
        *luma_sample_bytes = 2;
        *chroma_pair_bytes = 4;
        return 1;
    case AV_PIX_FMT_P210LE:
    case AV_PIX_FMT_P210BE:
    case AV_PIX_FMT_P212LE:
    case AV_PIX_FMT_P212BE:
    case AV_PIX_FMT_P216LE:
    case AV_PIX_FMT_P216BE:
        *log2_chroma_w = 1;
        *log2_chroma_h = 0;
        *luma_sample_bytes = 2;
        *chroma_pair_bytes = 4;
        return 1;
    case AV_PIX_FMT_P410LE:
    case AV_PIX_FMT_P410BE:
    case AV_PIX_FMT_P412LE:
    case AV_PIX_FMT_P412BE:
    case AV_PIX_FMT_P416LE:
    case AV_PIX_FMT_P416BE:
        *log2_chroma_w = 0;
        *log2_chroma_h = 0;
        *luma_sample_bytes = 2;
        *chroma_pair_bytes = 4;
        return 1;
    default:
        return 0;
    }
}

static void print_video_planes(const AVFrame *frame)
{
    if (frame->width <= 0 || frame->height <= 0 || frame->data[0] == NULL) {
        printf("none");
        return;
    }

    int log2_chroma_w = 0;
    int log2_chroma_h = 0;
    int planar_yuv_bytes = planar_yuv_sample_bytes(
        frame->format, &log2_chroma_w, &log2_chroma_h);
    if (planar_yuv_bytes) {
        int plane_count = planar_yuv_plane_count(frame->format);
        for (int plane = 0; plane < plane_count; plane++) {
            int full_size_plane = plane == 0 || plane == 3;
            int width = full_size_plane ? frame->width
                                        : frame->width >> log2_chroma_w;
            int height = full_size_plane ? frame->height
                                         : frame->height >> log2_chroma_h;
            if (plane)
                printf(",");
            for (int row = 0; row < height; row++)
                print_hex(frame->data[plane] + row * frame->linesize[plane],
                          width * planar_yuv_bytes);
        }
        return;
    }

    int luma_sample_bytes = 0;
    int chroma_pair_bytes = 0;
    if (semiplanar_yuv_layout(frame->format, &log2_chroma_w, &log2_chroma_h,
                              &luma_sample_bytes, &chroma_pair_bytes)) {
        int luma_row_bytes = frame->width * luma_sample_bytes;
        int chroma_row_bytes = (frame->width >> log2_chroma_w) *
                               chroma_pair_bytes;
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0],
                      luma_row_bytes);
        printf(",");
        for (int row = 0; row < (frame->height >> log2_chroma_h); row++)
            print_hex(frame->data[1] + row * frame->linesize[1],
                      chroma_row_bytes);
        return;
    }

    if (frame->format == AV_PIX_FMT_GRAY8) {
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0], frame->width);
        return;
    }
    if (frame->format == AV_PIX_FMT_UYYVYY411) {
        int visible_row_bytes = ((frame->width + 3) / 4) * 6;
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0],
                      visible_row_bytes);
        return;
    }
    int bitstream_bytes = bitstream_row_bytes(frame->format, frame->width);
    if (bitstream_bytes > 0) {
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0],
                      bitstream_bytes);
        return;
    }
    int bytes_per_pixel = 0;
    switch (frame->format) {
    case AV_PIX_FMT_RGB8:
    case AV_PIX_FMT_PAL8:
    case AV_PIX_FMT_BGR8:
    case AV_PIX_FMT_RGB4_BYTE:
    case AV_PIX_FMT_BGR4_BYTE:
    case AV_PIX_FMT_BAYER_BGGR8:
    case AV_PIX_FMT_BAYER_RGGB8:
    case AV_PIX_FMT_BAYER_GBRG8:
    case AV_PIX_FMT_BAYER_GRBG8:
        bytes_per_pixel = 1;
        break;
    case AV_PIX_FMT_RGB565BE:
    case AV_PIX_FMT_RGB565LE:
    case AV_PIX_FMT_RGB555BE:
    case AV_PIX_FMT_RGB555LE:
    case AV_PIX_FMT_BGR565BE:
    case AV_PIX_FMT_BGR565LE:
    case AV_PIX_FMT_BGR555BE:
    case AV_PIX_FMT_BGR555LE:
    case AV_PIX_FMT_RGB444LE:
    case AV_PIX_FMT_RGB444BE:
    case AV_PIX_FMT_BGR444LE:
    case AV_PIX_FMT_BGR444BE:
    case AV_PIX_FMT_BAYER_BGGR16LE:
    case AV_PIX_FMT_BAYER_BGGR16BE:
    case AV_PIX_FMT_BAYER_RGGB16LE:
    case AV_PIX_FMT_BAYER_RGGB16BE:
    case AV_PIX_FMT_BAYER_GBRG16LE:
    case AV_PIX_FMT_BAYER_GBRG16BE:
    case AV_PIX_FMT_BAYER_GRBG16LE:
    case AV_PIX_FMT_BAYER_GRBG16BE:
    case AV_PIX_FMT_YA8:
    case AV_PIX_FMT_GRAY9LE:
    case AV_PIX_FMT_GRAY9BE:
    case AV_PIX_FMT_GRAY10LE:
    case AV_PIX_FMT_GRAY10BE:
    case AV_PIX_FMT_GRAY12LE:
    case AV_PIX_FMT_GRAY12BE:
    case AV_PIX_FMT_GRAY14LE:
    case AV_PIX_FMT_GRAY14BE:
    case AV_PIX_FMT_GRAY16LE:
    case AV_PIX_FMT_GRAY16BE:
    case AV_PIX_FMT_GRAYF16LE:
    case AV_PIX_FMT_GRAYF16BE:
    case AV_PIX_FMT_YUYV422:
    case AV_PIX_FMT_UYVY422:
    case AV_PIX_FMT_YVYU422:
        bytes_per_pixel = 2;
        break;
    case AV_PIX_FMT_RGB24:
    case AV_PIX_FMT_BGR24:
    case AV_PIX_FMT_VYU444:
        bytes_per_pixel = 3;
        break;
    case AV_PIX_FMT_RGB48LE:
    case AV_PIX_FMT_RGB48BE:
    case AV_PIX_FMT_RGBF16LE:
    case AV_PIX_FMT_RGBF16BE:
    case AV_PIX_FMT_BGR48LE:
    case AV_PIX_FMT_BGR48BE:
        bytes_per_pixel = 6;
        break;
    case AV_PIX_FMT_RGBA:
    case AV_PIX_FMT_BGRA:
    case AV_PIX_FMT_ARGB:
    case AV_PIX_FMT_ABGR:
    case AV_PIX_FMT_0RGB:
    case AV_PIX_FMT_RGB0:
    case AV_PIX_FMT_0BGR:
    case AV_PIX_FMT_BGR0:
    case AV_PIX_FMT_X2RGB10LE:
    case AV_PIX_FMT_X2RGB10BE:
    case AV_PIX_FMT_X2BGR10LE:
    case AV_PIX_FMT_X2BGR10BE:
    case AV_PIX_FMT_VUYA:
    case AV_PIX_FMT_VUYX:
    case AV_PIX_FMT_XV30LE:
    case AV_PIX_FMT_XV30BE:
    case AV_PIX_FMT_V30XLE:
    case AV_PIX_FMT_V30XBE:
    case AV_PIX_FMT_AYUV:
    case AV_PIX_FMT_UYVA:
    case AV_PIX_FMT_YA16LE:
    case AV_PIX_FMT_YA16BE:
    case AV_PIX_FMT_YAF16LE:
    case AV_PIX_FMT_YAF16BE:
    case AV_PIX_FMT_GRAY32LE:
    case AV_PIX_FMT_GRAY32BE:
    case AV_PIX_FMT_GRAYF32LE:
    case AV_PIX_FMT_GRAYF32BE:
    case AV_PIX_FMT_Y210LE:
    case AV_PIX_FMT_Y210BE:
    case AV_PIX_FMT_Y212LE:
    case AV_PIX_FMT_Y212BE:
    case AV_PIX_FMT_Y216LE:
    case AV_PIX_FMT_Y216BE:
        bytes_per_pixel = 4;
        break;
    case AV_PIX_FMT_RGBF32LE:
    case AV_PIX_FMT_RGBF32BE:
    case AV_PIX_FMT_RGB96LE:
    case AV_PIX_FMT_RGB96BE:
        bytes_per_pixel = 12;
        break;
    case AV_PIX_FMT_XYZ12LE:
    case AV_PIX_FMT_XYZ12BE:
        bytes_per_pixel = 6;
        break;
    case AV_PIX_FMT_YAF32LE:
    case AV_PIX_FMT_YAF32BE:
    case AV_PIX_FMT_RGBAF16LE:
    case AV_PIX_FMT_RGBAF16BE:
    case AV_PIX_FMT_RGBA64LE:
    case AV_PIX_FMT_RGBA64BE:
    case AV_PIX_FMT_BGRA64LE:
    case AV_PIX_FMT_BGRA64BE:
    case AV_PIX_FMT_AYUV64LE:
    case AV_PIX_FMT_AYUV64BE:
    case AV_PIX_FMT_XV36LE:
    case AV_PIX_FMT_XV36BE:
    case AV_PIX_FMT_XV48LE:
    case AV_PIX_FMT_XV48BE:
        bytes_per_pixel = 8;
        break;
    case AV_PIX_FMT_RGBAF32LE:
    case AV_PIX_FMT_RGBAF32BE:
    case AV_PIX_FMT_RGBA128LE:
    case AV_PIX_FMT_RGBA128BE:
        bytes_per_pixel = 16;
        break;
    default:
        break;
    }
    if (bytes_per_pixel > 0) {
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0],
                      frame->width * bytes_per_pixel);
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

static int frame_data_pointer_count(const AVFrame *frame)
{
    if (frame->width > 0 && frame->height > 0) {
        int count = 0;
        for (int i = 0; i < AV_NUM_DATA_POINTERS; i++)
            if (frame->data[i] != NULL)
                count++;
        return count;
    }

    if (frame->nb_samples > 0 && frame->ch_layout.nb_channels > 0) {
        return av_sample_fmt_is_planar(frame->format)
                   ? frame->ch_layout.nb_channels
                   : 1;
    }

    return 0;
}

static void print_frame_buffer_topology(const char *name, const AVFrame *frame)
{
    int direct_data_slots = 0;
    int extended_data_pointers = 0;
    int direct_buffer_refs = 0;
    int writable_direct_buffer_refs = 0;
    int writable_extended_buffer_refs = 0;
    int data_pointer_count = frame_data_pointer_count(frame);

    for (int i = 0; i < AV_NUM_DATA_POINTERS; i++) {
        if (frame->data[i] != NULL)
            direct_data_slots++;
        if (frame->buf[i] != NULL) {
            direct_buffer_refs++;
            if (av_buffer_is_writable(frame->buf[i]))
                writable_direct_buffer_refs++;
        }
    }

    if (frame->extended_data != NULL) {
        for (int i = 0; i < data_pointer_count; i++)
            if (frame->extended_data[i] != NULL)
                extended_data_pointers++;
    }

    for (int i = 0; i < frame->nb_extended_buf; i++) {
        if (frame->extended_buf[i] != NULL &&
            av_buffer_is_writable(frame->extended_buf[i]))
            writable_extended_buffer_refs++;
    }

    printf("%s|%d|%d|%d|%d|%d|%d|%d|%d\n", name, AV_NUM_DATA_POINTERS,
           direct_data_slots, extended_data_pointers, direct_buffer_refs,
           frame->nb_extended_buf, writable_direct_buffer_refs,
           writable_extended_buffer_refs,
           frame->extended_data != NULL && frame->extended_data != frame->data);
}

static void print_frame_plane_buffer(const char *name, const AVFrame *frame,
                                     int plane)
{
    AVBufferRef *ref = av_frame_get_plane_buffer(frame, plane);
    const uint8_t *data = NULL;
    size_t visible_size = 0;

    if (!ref) {
        printf("%s|0|none|0|0\n", name);
        return;
    }

    if (frame->width > 0 && frame->height > 0 &&
        frame->format == AV_PIX_FMT_GRAY8 && plane == 0) {
        printf("%s|1|", name);
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0],
                      frame->width);
        printf("|%d|%d\n", av_buffer_get_ref_count(ref),
               av_buffer_is_writable(ref));
        return;
    }

    if (frame->nb_samples > 0 && frame->ch_layout.nb_channels > 0) {
        int bytes_per_sample = av_get_bytes_per_sample(frame->format);
        int planar = av_sample_fmt_is_planar(frame->format);
        if (bytes_per_sample > 0 && planar &&
            plane >= 0 && plane < frame->ch_layout.nb_channels) {
            data = frame->extended_data ? frame->extended_data[plane]
                                        : frame->data[plane];
            visible_size = (size_t)bytes_per_sample * frame->nb_samples;
        } else if (bytes_per_sample > 0 && !planar && plane == 0) {
            data = frame->data[0];
            visible_size = (size_t)bytes_per_sample * frame->nb_samples *
                           frame->ch_layout.nb_channels;
        }
    }

    printf("%s|1|", name);
    if (data && visible_size > 0)
        print_hex(data, visible_size);
    else
        printf("none");
    printf("|%d|%d\n", av_buffer_get_ref_count(ref),
           av_buffer_is_writable(ref));
}

static void print_pal8_planes(const AVFrame *frame)
{
    if (frame->format != AV_PIX_FMT_PAL8 || frame->data[0] == NULL ||
        frame->data[1] == NULL) {
        printf("none");
        return;
    }

    for (int row = 0; row < frame->height; row++)
        print_hex(frame->data[0] + row * frame->linesize[0], frame->width);
    printf(",");
    print_hex(frame->data[1], AVPALETTE_SIZE);
}

static void print_pal8_frame(const char *name, const AVFrame *frame)
{
    printf("%s|video|%s|%d|%d|%d,%d|", name,
           av_get_pix_fmt_name(frame->format), frame->width, frame->height,
           frame->linesize[0], frame->linesize[1]);
    print_pal8_planes(frame);
    printf("|%d|%d\n", av_frame_is_writable((AVFrame *)frame),
           frame->data[1] != NULL ? 2 : 1);
}

static void print_pal8_plane_buffer(const char *name, const AVFrame *frame,
                                    int plane)
{
    AVBufferRef *ref = av_frame_get_plane_buffer(frame, plane);
    if (!ref || frame->format != AV_PIX_FMT_PAL8 || plane < 0 || plane > 1) {
        printf("%s|0|none|0|0\n", name);
        return;
    }

    printf("%s|1|", name);
    if (plane == 0) {
        for (int row = 0; row < frame->height; row++)
            print_hex(frame->data[0] + row * frame->linesize[0],
                      frame->width);
    } else {
        print_hex(frame->data[1], AVPALETTE_SIZE);
    }
    printf("|%d|%d\n", av_buffer_get_ref_count(ref),
           av_buffer_is_writable(ref));
}

static void print_pal8_share(const char *name, const AVFrame *left,
                             const AVFrame *right, int plane)
{
    AVBufferRef *left_ref = av_frame_get_plane_buffer((AVFrame *)left, plane);
    AVBufferRef *right_ref = av_frame_get_plane_buffer((AVFrame *)right, plane);
    fail_if(!left_ref || !right_ref, "missing pal8 plane buffer ref");
    printf("%s|%d|%d|%d|%d|%d\n",
           name, left->data[plane] == right->data[plane],
           av_buffer_get_ref_count(left_ref),
           av_buffer_get_ref_count(right_ref),
           av_buffer_is_writable(left_ref),
           av_buffer_is_writable(right_ref));
}

static void print_pal8_crop_palette_preserved(const char *name,
                                              const AVFrame *frame,
                                              const uint8_t *palette_before)
{
    printf("%s|%d|%d|", name, frame->data[1] == palette_before,
           frame->linesize[1]);
    print_pal8_planes(frame);
    printf("\n");
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

static int video_plane_count_for_format(enum AVPixelFormat format)
{
    int log2_chroma_w = 0;
    int log2_chroma_h = 0;
    if (planar_yuv_sample_bytes(format, &log2_chroma_w, &log2_chroma_h))
        return planar_yuv_plane_count(format);
    int luma_sample_bytes = 0;
    int chroma_pair_bytes = 0;
    if (semiplanar_yuv_layout(format, &log2_chroma_w, &log2_chroma_h,
                              &luma_sample_bytes, &chroma_pair_bytes))
        return 2;
    return 1;
}

static void print_frame(const char *name, const AVFrame *frame)
{
    const char *kind = "empty";
    const char *format = "none";
    int plane_count = 0;

    if (frame->width > 0 && frame->height > 0) {
        kind = "video";
        format = av_get_pix_fmt_name(frame->format);
        plane_count = video_plane_count_for_format(frame->format);
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

static void fill_video_packed(AVFrame *frame, int bytes_per_pixel);
static void fill_video_uyyvyy411(AVFrame *frame);
static void fill_video_bitstream(AVFrame *frame);
static void fill_video_planar_yuv(AVFrame *frame);
static void fill_video_semiplanar_yuv(AVFrame *frame);
static void fill_video_yuv420p(AVFrame *frame);

static void exercise_bitstream_crop_pair(const char *name,
                                         enum AVPixelFormat format)
{
    char row[96];
    AVFrame *crop_default = av_frame_alloc();
    fail_if(!crop_default, "bitstream default crop allocation failed");
    crop_default->format = format;
    crop_default->width = 8;
    crop_default->height = 4;
    fail_if(av_frame_get_buffer(crop_default, 64) < 0,
            "bitstream default crop get_buffer failed");
    fill_video_bitstream(crop_default);
    crop_default->crop_top = 1;
    crop_default->crop_left = 1;
    crop_default->crop_right = 1;
    int crop_default_ret = av_frame_apply_cropping(crop_default, 0);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default-ret", name);
    printf("%s|%d\n", row, crop_default_ret);
    fail_if(crop_default_ret < 0, "bitstream default crop apply failed");
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default", name);
    print_frame(row, crop_default);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "bitstream unaligned crop allocation failed");
    crop_unaligned->format = format;
    crop_unaligned->width = 8;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "bitstream unaligned crop get_buffer failed");
    fill_video_bitstream(crop_unaligned);
    crop_unaligned->crop_top = 1;
    crop_unaligned->crop_left = 1;
    crop_unaligned->crop_right = 1;
    int crop_unaligned_ret = av_frame_apply_cropping(
        crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned-ret", name);
    printf("%s|%d\n", row, crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0, "bitstream unaligned crop apply failed");
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned", name);
    print_frame(row, crop_unaligned);

    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_default);
}

static void exercise_packed_crop_pair(const char *name,
                                      enum AVPixelFormat format,
                                      int bytes_per_pixel)
{
    char row[96];
    AVFrame *crop_default = av_frame_alloc();
    fail_if(!crop_default, "packed default crop allocation failed");
    crop_default->format = format;
    crop_default->width = 8;
    crop_default->height = 4;
    fail_if(av_frame_get_buffer(crop_default, 64) < 0,
            "packed default crop get_buffer failed");
    fill_video_packed(crop_default, bytes_per_pixel);
    crop_default->crop_top = 1;
    crop_default->crop_left = 1;
    crop_default->crop_right = 1;
    int crop_default_ret = av_frame_apply_cropping(crop_default, 0);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default-ret", name);
    printf("%s|%d\n", row, crop_default_ret);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default", name);
    print_frame(row, crop_default);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "packed unaligned crop allocation failed");
    crop_unaligned->format = format;
    crop_unaligned->width = 8;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "packed unaligned crop get_buffer failed");
    fill_video_packed(crop_unaligned, bytes_per_pixel);
    crop_unaligned->crop_top = 1;
    crop_unaligned->crop_left = 1;
    crop_unaligned->crop_right = 1;
    int crop_unaligned_ret = av_frame_apply_cropping(
        crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned-ret", name);
    printf("%s|%d\n", row, crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0, "packed unaligned crop apply failed");
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned", name);
    print_frame(row, crop_unaligned);

    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_default);
}

static void exercise_uyyvyy411_crop_pair(void)
{
    AVFrame *crop_default = av_frame_alloc();
    fail_if(!crop_default, "uyyvyy411 default crop allocation failed");
    crop_default->format = AV_PIX_FMT_UYYVYY411;
    crop_default->width = 8;
    crop_default->height = 4;
    fail_if(av_frame_get_buffer(crop_default, 64) < 0,
            "uyyvyy411 default crop get_buffer failed");
    fill_video_uyyvyy411(crop_default);
    crop_default->crop_top = 1;
    crop_default->crop_left = 1;
    crop_default->crop_right = 1;
    int crop_default_ret = av_frame_apply_cropping(crop_default, 0);
    printf("frame:apply-crop-uyyvyy411-default-ret|%d\n",
           crop_default_ret);
    print_frame("frame:apply-crop-uyyvyy411-default", crop_default);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "uyyvyy411 unaligned crop allocation failed");
    crop_unaligned->format = AV_PIX_FMT_UYYVYY411;
    crop_unaligned->width = 8;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "uyyvyy411 unaligned crop get_buffer failed");
    fill_video_uyyvyy411(crop_unaligned);
    crop_unaligned->crop_top = 1;
    crop_unaligned->crop_left = 1;
    crop_unaligned->crop_right = 1;
    int crop_unaligned_ret = av_frame_apply_cropping(
        crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    printf("frame:apply-crop-uyyvyy411-unaligned-ret|%d\n",
           crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0,
            "uyyvyy411 unaligned crop apply failed");
    print_frame("frame:apply-crop-uyyvyy411-unaligned", crop_unaligned);

    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_default);
}

static void exercise_yuv420p_crop_pair(void)
{
    AVFrame *crop_default = av_frame_alloc();
    fail_if(!crop_default, "yuv420p default crop allocation failed");
    crop_default->format = AV_PIX_FMT_YUV420P;
    crop_default->width = 8;
    crop_default->height = 4;
    fail_if(av_frame_get_buffer(crop_default, 64) < 0,
            "yuv420p default crop get_buffer failed");
    fill_video_yuv420p(crop_default);
    crop_default->crop_top = 2;
    crop_default->crop_left = 2;
    crop_default->crop_right = 2;
    int crop_default_ret = av_frame_apply_cropping(crop_default, 0);
    printf("frame:apply-crop-yuv420p-default-ret|%d\n",
           crop_default_ret);
    fail_if(crop_default_ret < 0, "yuv420p default crop apply failed");
    print_frame("frame:apply-crop-yuv420p-default", crop_default);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "yuv420p unaligned crop allocation failed");
    crop_unaligned->format = AV_PIX_FMT_YUV420P;
    crop_unaligned->width = 8;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "yuv420p unaligned crop get_buffer failed");
    fill_video_yuv420p(crop_unaligned);
    crop_unaligned->crop_top = 2;
    crop_unaligned->crop_left = 2;
    crop_unaligned->crop_right = 2;
    int crop_unaligned_ret = av_frame_apply_cropping(
        crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    printf("frame:apply-crop-yuv420p-unaligned-ret|%d\n",
           crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0, "yuv420p unaligned crop apply failed");
    print_frame("frame:apply-crop-yuv420p-unaligned", crop_unaligned);

    AVFrame *crop_odd_default = av_frame_alloc();
    fail_if(!crop_odd_default, "yuv420p odd default crop allocation failed");
    crop_odd_default->format = AV_PIX_FMT_YUV420P;
    crop_odd_default->width = 8;
    crop_odd_default->height = 4;
    fail_if(av_frame_get_buffer(crop_odd_default, 64) < 0,
            "yuv420p odd default crop get_buffer failed");
    fill_video_yuv420p(crop_odd_default);
    crop_odd_default->crop_top = 1;
    crop_odd_default->crop_left = 1;
    crop_odd_default->crop_right = 1;
    int crop_odd_default_ret = av_frame_apply_cropping(crop_odd_default, 0);
    printf("frame:apply-crop-yuv420p-odd-default-ret|%d\n",
           crop_odd_default_ret);
    fail_if(crop_odd_default_ret < 0,
            "yuv420p odd default crop apply failed");
    print_frame("frame:apply-crop-yuv420p-odd-default",
                crop_odd_default);

    AVFrame *crop_odd_unaligned = av_frame_alloc();
    fail_if(!crop_odd_unaligned,
            "yuv420p odd unaligned crop allocation failed");
    crop_odd_unaligned->format = AV_PIX_FMT_YUV420P;
    crop_odd_unaligned->width = 8;
    crop_odd_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_odd_unaligned, 64) < 0,
            "yuv420p odd unaligned crop get_buffer failed");
    fill_video_yuv420p(crop_odd_unaligned);
    crop_odd_unaligned->crop_top = 1;
    crop_odd_unaligned->crop_left = 1;
    crop_odd_unaligned->crop_right = 1;
    int crop_odd_unaligned_ret = av_frame_apply_cropping(
        crop_odd_unaligned, AV_FRAME_CROP_UNALIGNED);
    printf("frame:apply-crop-yuv420p-odd-unaligned-ret|%d\n",
           crop_odd_unaligned_ret);
    fail_if(crop_odd_unaligned_ret < 0,
            "yuv420p odd unaligned crop apply failed");
    print_frame("frame:apply-crop-yuv420p-odd-unaligned",
                crop_odd_unaligned);

    av_frame_free(&crop_odd_unaligned);
    av_frame_free(&crop_odd_default);
    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_default);
}

static void exercise_planar_yuv_crop_pair(enum AVPixelFormat format,
                                          const char *name)
{
    char row[128];
    AVFrame *crop_default = av_frame_alloc();
    fail_if(!crop_default, "planar YUV default crop allocation failed");
    crop_default->format = format;
    crop_default->width = 8;
    crop_default->height = 4;
    fail_if(av_frame_get_buffer(crop_default, 64) < 0,
            "planar YUV default crop get_buffer failed");
    fill_video_planar_yuv(crop_default);
    crop_default->crop_top = 1;
    crop_default->crop_left = 1;
    crop_default->crop_right = 1;
    int crop_default_ret = av_frame_apply_cropping(crop_default, 0);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default-ret", name);
    printf("%s|%d\n", row, crop_default_ret);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default", name);
    print_frame(row, crop_default);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "planar YUV unaligned crop allocation failed");
    crop_unaligned->format = format;
    crop_unaligned->width = 8;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "planar YUV unaligned crop get_buffer failed");
    fill_video_planar_yuv(crop_unaligned);
    crop_unaligned->crop_top = 1;
    crop_unaligned->crop_left = 1;
    crop_unaligned->crop_right = 1;
    int crop_unaligned_ret = av_frame_apply_cropping(
        crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned-ret", name);
    printf("%s|%d\n", row, crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0,
            "planar YUV unaligned crop apply failed");
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned", name);
    print_frame(row, crop_unaligned);

    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_default);
}

static void exercise_semiplanar_crop_pair(enum AVPixelFormat format,
                                          const char *name)
{
    char row[128];
    AVFrame *crop_default = av_frame_alloc();
    fail_if(!crop_default, "semi-planar default crop allocation failed");
    crop_default->format = format;
    crop_default->width = 8;
    crop_default->height = 4;
    fail_if(av_frame_get_buffer(crop_default, 64) < 0,
            "semi-planar default crop get_buffer failed");
    fill_video_semiplanar_yuv(crop_default);
    crop_default->crop_top = 1;
    crop_default->crop_left = 1;
    crop_default->crop_right = 1;
    int crop_default_ret = av_frame_apply_cropping(crop_default, 0);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default-ret", name);
    printf("%s|%d\n", row, crop_default_ret);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-default", name);
    print_frame(row, crop_default);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "semi-planar unaligned crop allocation failed");
    crop_unaligned->format = format;
    crop_unaligned->width = 8;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "semi-planar unaligned crop get_buffer failed");
    fill_video_semiplanar_yuv(crop_unaligned);
    crop_unaligned->crop_top = 1;
    crop_unaligned->crop_left = 1;
    crop_unaligned->crop_right = 1;
    int crop_unaligned_ret = av_frame_apply_cropping(
        crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned-ret", name);
    printf("%s|%d\n", row, crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0,
            "semi-planar unaligned crop apply failed");
    snprintf(row, sizeof(row), "frame:apply-crop-%s-unaligned", name);
    print_frame(row, crop_unaligned);

    AVFrame *crop_even_default = av_frame_alloc();
    fail_if(!crop_even_default,
            "semi-planar even default crop allocation failed");
    crop_even_default->format = format;
    crop_even_default->width = 8;
    crop_even_default->height = 4;
    fail_if(av_frame_get_buffer(crop_even_default, 64) < 0,
            "semi-planar even default crop get_buffer failed");
    fill_video_semiplanar_yuv(crop_even_default);
    crop_even_default->crop_top = 2;
    crop_even_default->crop_left = 2;
    crop_even_default->crop_right = 2;
    int crop_even_default_ret = av_frame_apply_cropping(crop_even_default, 0);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-even-default-ret",
             name);
    printf("%s|%d\n", row, crop_even_default_ret);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-even-default", name);
    print_frame(row, crop_even_default);

    AVFrame *crop_even_unaligned = av_frame_alloc();
    fail_if(!crop_even_unaligned,
            "semi-planar even unaligned crop allocation failed");
    crop_even_unaligned->format = format;
    crop_even_unaligned->width = 8;
    crop_even_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_even_unaligned, 64) < 0,
            "semi-planar even unaligned crop get_buffer failed");
    fill_video_semiplanar_yuv(crop_even_unaligned);
    crop_even_unaligned->crop_top = 2;
    crop_even_unaligned->crop_left = 2;
    crop_even_unaligned->crop_right = 2;
    int crop_even_unaligned_ret = av_frame_apply_cropping(
        crop_even_unaligned, AV_FRAME_CROP_UNALIGNED);
    snprintf(row, sizeof(row), "frame:apply-crop-%s-even-unaligned-ret",
             name);
    printf("%s|%d\n", row, crop_even_unaligned_ret);
    fail_if(crop_even_unaligned_ret < 0,
            "semi-planar even unaligned crop apply failed");
    snprintf(row, sizeof(row), "frame:apply-crop-%s-even-unaligned", name);
    print_frame(row, crop_even_unaligned);

    av_frame_free(&crop_even_unaligned);
    av_frame_free(&crop_even_default);
    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_default);
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

static void print_opaque_ref_share(const char *name, const AVFrame *left,
                                   const AVFrame *right)
{
    print_buffer_ref_share(name, left->opaque_ref, right->opaque_ref);
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

static void fill_video_pal8(AVFrame *frame)
{
    for (int row = 0; row < frame->height; row++) {
        uint8_t *dst = frame->data[0] + row * frame->linesize[0];
        for (int column = 0; column < frame->width; column++)
            dst[column] = (uint8_t)(row * 16 + column);
    }
    for (int index = 0; index < AVPALETTE_SIZE; index++)
        frame->data[1][index] = (uint8_t)(index * 37 + 11);
}

static void fill_video_packed(AVFrame *frame, int bytes_per_pixel)
{
    int visible_row_bytes = frame->width * bytes_per_pixel;
    for (int row = 0; row < frame->height; row++) {
        uint8_t *dst = frame->data[0] + row * frame->linesize[0];
        for (int column = 0; column < visible_row_bytes; column++)
            dst[column] = (uint8_t)(row * 16 + column);
    }
}

static void fill_video_uyyvyy411(AVFrame *frame)
{
    int visible_row_bytes = ((frame->width + 3) / 4) * 6;
    for (int row = 0; row < frame->height; row++) {
        uint8_t *dst = frame->data[0] + row * frame->linesize[0];
        for (int column = 0; column < visible_row_bytes; column++)
            dst[column] = (uint8_t)(row * 16 + column);
    }
}

static void fill_video_bitstream(AVFrame *frame)
{
    int visible_row_bytes = bitstream_row_bytes(frame->format, frame->width);
    fail_if(visible_row_bytes <= 0, "unsupported bitstream fill format");
    for (int row = 0; row < frame->height; row++) {
        uint8_t *dst = frame->data[0] + row * frame->linesize[0];
        for (int column = 0; column < visible_row_bytes; column++)
            dst[column] = (uint8_t)(row * 16 + column);
    }
}

static void fill_video_yuv420p(AVFrame *frame)
{
    fill_video_planar_yuv(frame);
}

static void fill_video_planar_yuv(AVFrame *frame)
{
    int log2_chroma_w = 0;
    int log2_chroma_h = 0;
    int sample_bytes = planar_yuv_sample_bytes(
        frame->format, &log2_chroma_w, &log2_chroma_h);
    fail_if(!sample_bytes, "unsupported planar YUV fill format");
    int plane_count = planar_yuv_plane_count(frame->format);

    for (int row = 0; row < frame->height; row++) {
        uint8_t *dst = frame->data[0] + row * frame->linesize[0];
        int visible_row_bytes = frame->width * sample_bytes;
        for (int column = 0; column < visible_row_bytes; column++)
            dst[column] = (uint8_t)(0x10 + row * 16 + column);
    }

    int chroma_width = frame->width >> log2_chroma_w;
    int chroma_height = frame->height >> log2_chroma_h;
    for (int row = 0; row < chroma_height; row++) {
        uint8_t *u = frame->data[1] + row * frame->linesize[1];
        uint8_t *v = frame->data[2] + row * frame->linesize[2];
        int visible_row_bytes = chroma_width * sample_bytes;
        for (int column = 0; column < visible_row_bytes; column++) {
            u[column] = (uint8_t)(0x80 + row * 16 + column);
            v[column] = (uint8_t)(0xc0 + row * 16 + column);
        }
    }

    if (plane_count == 4) {
        for (int row = 0; row < frame->height; row++) {
            uint8_t *a = frame->data[3] + row * frame->linesize[3];
            int visible_row_bytes = frame->width * sample_bytes;
            for (int column = 0; column < visible_row_bytes; column++)
                a[column] = (uint8_t)(0xe0 + row * 16 + column);
        }
    }
}

static void fill_video_semiplanar_yuv(AVFrame *frame)
{
    int log2_chroma_w = 0;
    int log2_chroma_h = 0;
    int luma_sample_bytes = 0;
    int chroma_pair_bytes = 0;
    fail_if(!semiplanar_yuv_layout(frame->format, &log2_chroma_w,
                                   &log2_chroma_h, &luma_sample_bytes,
                                   &chroma_pair_bytes),
            "unsupported semi-planar fill format");

    for (int row = 0; row < frame->height; row++) {
        uint8_t *dst = frame->data[0] + row * frame->linesize[0];
        int visible_row_bytes = frame->width * luma_sample_bytes;
        for (int column = 0; column < visible_row_bytes; column++)
            dst[column] = (uint8_t)(0x10 + row * 16 + column);
    }

    int chroma_width = frame->width >> log2_chroma_w;
    int chroma_height = frame->height >> log2_chroma_h;
    for (int row = 0; row < chroma_height; row++) {
        uint8_t *dst = frame->data[1] + row * frame->linesize[1];
        int visible_row_bytes = chroma_width * chroma_pair_bytes;
        for (int column = 0; column < visible_row_bytes; column++)
            dst[column] = (uint8_t)(0x80 + row * 16 + column);
    }
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

static AVFrame *frame_with_fifo_props(void)
{
    AVFrame *frame = av_frame_alloc();
    fail_if(!frame, "frame fifo allocation failed");
    frame->format = AV_PIX_FMT_GRAY8;
    frame->width = 2;
    frame->height = 1;
    frame->pts = 410;
    frame->pkt_dts = 409;
    frame->duration = 3;
    frame->time_base = (AVRational){ 1, 90000 };
    frame->sample_aspect_ratio = (AVRational){ 1, 1 };
    frame->pict_type = AV_PICTURE_TYPE_P;
    frame->quality = 17;
    frame->repeat_pict = 1;
    frame->flags = AV_FRAME_FLAG_KEY | AV_FRAME_FLAG_LOSSLESS;
    fail_if(av_frame_get_buffer(frame, 1) < 0,
            "frame fifo get buffer failed");
    frame->data[0][0] = 0xaa;
    frame->data[0][1] = 0xbb;
    frame->opaque_ref = av_buffer_alloc(2);
    fail_if(!frame->opaque_ref, "frame fifo opaque_ref allocation failed");
    frame->opaque_ref->data[0] = 0xde;
    frame->opaque_ref->data[1] = 0xad;
    frame->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!frame->hw_frames_ctx, "frame fifo hw allocation failed");
    frame->hw_frames_ctx->data[0] = 0xee;
    fail_if(av_dict_set(&frame->metadata, "title", "fifo", 0) < 0,
            "frame fifo metadata allocation failed");
    AVFrameSideData *sd = av_frame_new_side_data(
        frame, AV_FRAME_DATA_DISPLAYMATRIX, 36);
    fail_if(!sd, "frame fifo side data allocation failed");
    memset(sd->data, 0x33, sd->size);
    return frame;
}

static void exercise_frame_fifo_api(void)
{
    AVContainerFifo *fifo = av_container_fifo_alloc_avframe(123);
    fail_if(!fifo, "frame fifo allocation failed");
    printf("frame:fifo-new-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVFrame *move_src = frame_with_fifo_props();
    int write_move_ret = av_container_fifo_write(fifo, move_src, 0);
    printf("frame:fifo-write-move-ret|%d\n", write_move_ret);
    fail_if(write_move_ret < 0, "frame fifo write move failed");
    print_frame("frame:fifo-write-move-src", move_src);
    printf("frame:fifo-after-write-move-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    void *peeked = NULL;
    int peek0_ret = av_container_fifo_peek(fifo, &peeked, 0);
    printf("frame:fifo-peek0-ret|%d\n", peek0_ret);
    fail_if(peek0_ret < 0, "frame fifo peek 0 failed");
    print_frame("frame:fifo-peek0", (const AVFrame *)peeked);
    void *invalid_peek = NULL;
    int peek1_ret = av_container_fifo_peek(fifo, &invalid_peek, 1);
    printf("frame:fifo-peek1-ret|%d\n", peek1_ret);

    AVFrame *move_dst = av_frame_alloc();
    fail_if(!move_dst, "frame fifo move dst allocation failed");
    int read_move_ret = av_container_fifo_read(fifo, move_dst, 0);
    printf("frame:fifo-read-move-ret|%d\n", read_move_ret);
    fail_if(read_move_ret < 0, "frame fifo read move failed");
    print_frame("frame:fifo-read-move-dst", move_dst);
    printf("frame:fifo-after-read-move-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_frame_free(&move_dst);
    av_frame_free(&move_src);

    AVFrame *ref_src = frame_with_fifo_props();
    int write_ref_ret = av_container_fifo_write(
        fifo, ref_src, AV_CONTAINER_FIFO_FLAG_REF);
    printf("frame:fifo-write-ref-ret|%d\n", write_ref_ret);
    fail_if(write_ref_ret < 0, "frame fifo write ref failed");
    print_frame("frame:fifo-write-ref-src", ref_src);
    printf("frame:fifo-after-write-ref-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVFrame *ref_dst = av_frame_alloc();
    fail_if(!ref_dst, "frame fifo ref dst allocation failed");
    int read_ref_ret = av_container_fifo_read(
        fifo, ref_dst, AV_CONTAINER_FIFO_FLAG_REF);
    printf("frame:fifo-read-ref-ret|%d\n", read_ref_ret);
    fail_if(read_ref_ret < 0, "frame fifo read ref failed");
    print_frame("frame:fifo-read-ref-dst", ref_dst);
    printf("frame:fifo-after-read-ref-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_frame_free(&ref_dst);
    av_frame_free(&ref_src);

    AVFrame *first = frame_with_fifo_props();
    first->pts = 611;
    AVFrame *second = frame_with_fifo_props();
    second->pts = 612;
    fail_if(av_container_fifo_write(fifo, first, 0) < 0,
            "frame fifo first write failed");
    fail_if(av_container_fifo_write(fifo, second, 0) < 0,
            "frame fifo second write failed");
    printf("frame:fifo-before-drain-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    av_container_fifo_drain(fifo, 1);
    printf("frame:fifo-after-drain-one-can-read|%zu\n",
           av_container_fifo_can_read(fifo));
    peeked = NULL;
    fail_if(av_container_fifo_peek(fifo, &peeked, 0) < 0,
            "frame fifo drain peek failed");
    print_frame("frame:fifo-after-drain-one-peek",
                (const AVFrame *)peeked);
    av_container_fifo_drain(fifo, 1);
    printf("frame:fifo-after-drain-all-can-read|%zu\n",
           av_container_fifo_can_read(fifo));

    AVFrame *empty_read = av_frame_alloc();
    fail_if(!empty_read, "frame fifo empty read allocation failed");
    int read_empty_ret = av_container_fifo_read(fifo, empty_read, 0);
    printf("frame:fifo-read-empty-ret|%d\n", read_empty_ret);

    av_frame_free(&empty_read);
    av_frame_free(&second);
    av_frame_free(&first);
    av_container_fifo_free(&fifo);
}

int main(void)
{
    AVFrame *empty = av_frame_alloc();
    fail_if(!empty, "av_frame_alloc failed");
    print_frame("frame:alloc-default", empty);
    int empty_make_writable_ret = av_frame_make_writable(empty);
    printf("frame:empty-make-writable-ret|%d\n", empty_make_writable_ret);
    print_frame("frame:empty-after-make-writable", empty);
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
    printf("frame:side-name-boundaries|%d|%s|%d|%s|%d|%d|%d|%d\n",
           AV_FRAME_DATA_DISPLAYMATRIX,
           av_frame_side_data_name(AV_FRAME_DATA_DISPLAYMATRIX),
           AV_FRAME_DATA_EXIF, av_frame_side_data_name(AV_FRAME_DATA_EXIF),
           av_frame_side_data_name((enum AVFrameSideDataType)-1) == NULL,
           av_frame_side_data_name((enum AVFrameSideDataType)(AV_FRAME_DATA_EXIF + 1)) == NULL,
           av_frame_side_data_name((enum AVFrameSideDataType)(AV_FRAME_DATA_EXIF + 2)) == NULL,
           av_frame_side_data_name((enum AVFrameSideDataType)INT_MAX) == NULL);
    const AVSideDataDescriptor *display_desc =
        av_frame_side_data_desc(AV_FRAME_DATA_DISPLAYMATRIX);
    const AVSideDataDescriptor *exif_desc =
        av_frame_side_data_desc(AV_FRAME_DATA_EXIF);
    printf("frame:side-desc-boundaries|%d|%s|%u|%d|%s|%u|%d|%d|%d|%d\n",
           AV_FRAME_DATA_DISPLAYMATRIX,
           display_desc ? display_desc->name : "",
           display_desc ? display_desc->props : 0,
           AV_FRAME_DATA_EXIF,
           exif_desc ? exif_desc->name : "",
           exif_desc ? exif_desc->props : 0,
           av_frame_side_data_desc((enum AVFrameSideDataType)-1) == NULL,
           av_frame_side_data_desc((enum AVFrameSideDataType)(AV_FRAME_DATA_EXIF + 1)) == NULL,
           av_frame_side_data_desc((enum AVFrameSideDataType)(AV_FRAME_DATA_EXIF + 2)) == NULL,
           av_frame_side_data_desc((enum AVFrameSideDataType)INT_MAX) == NULL);

    AVFrame *video = av_frame_alloc();
    fail_if(!video, "video av_frame_alloc failed");
    video->format = AV_PIX_FMT_GRAY8;
    video->width = 2;
    video->height = 3;
    video->pts = 123;
    video->pkt_dts = 122;
    video->duration = 121;
    video->time_base = (AVRational){ 1, 90000 };
    video->sample_rate = 44100;
    av_channel_layout_default(&video->ch_layout, 1);
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
    print_frame_plane_buffer("frame:plane-buffer-video-0", video, 0);
    print_frame_plane_buffer("frame:plane-buffer-video-invalid", video, 1);

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

    AVFrame *pal8 = av_frame_alloc();
    fail_if(!pal8, "pal8 allocation failed");
    pal8->format = AV_PIX_FMT_PAL8;
    pal8->width = 3;
    pal8->height = 2;
    fail_if(av_frame_get_buffer(pal8, 1) < 0,
            "pal8 get_buffer failed");
    fill_video_pal8(pal8);
    print_pal8_frame("frame:pal8-buffer", pal8);
    print_pal8_plane_buffer("frame:pal8-plane-buffer-0", pal8, 0);
    print_pal8_plane_buffer("frame:pal8-plane-buffer-1", pal8, 1);
    print_pal8_plane_buffer("frame:pal8-plane-buffer-invalid", pal8, 2);

    AVFrame *pal8_ref = av_frame_alloc();
    fail_if(!pal8_ref, "pal8_ref allocation failed");
    fail_if(av_frame_ref(pal8_ref, pal8) < 0, "pal8 av_frame_ref failed");
    print_pal8_frame("frame:pal8-ref-src", pal8);
    print_pal8_frame("frame:pal8-ref-dst", pal8_ref);
    print_pal8_share("frame:pal8-ref-plane0-shares", pal8, pal8_ref, 0);
    print_pal8_share("frame:pal8-ref-plane1-shares", pal8, pal8_ref, 1);

    fail_if(av_frame_make_writable(pal8_ref) < 0,
            "pal8 av_frame_make_writable failed");
    print_pal8_frame("frame:pal8-after-make-writable-src", pal8);
    print_pal8_frame("frame:pal8-after-make-writable-dst", pal8_ref);
    print_pal8_share("frame:pal8-make-writable-plane0-shares",
                     pal8, pal8_ref, 0);
    print_pal8_share("frame:pal8-make-writable-plane1-shares",
                     pal8, pal8_ref, 1);

    AVFrame *pal8_crop = av_frame_alloc();
    fail_if(!pal8_crop, "pal8_crop allocation failed");
    pal8_crop->format = AV_PIX_FMT_PAL8;
    pal8_crop->width = 6;
    pal8_crop->height = 4;
    fail_if(av_frame_get_buffer(pal8_crop, 1) < 0,
            "pal8_crop get_buffer failed");
    fill_video_pal8(pal8_crop);
    const uint8_t *pal8_crop_palette_before = pal8_crop->data[1];
    pal8_crop->crop_top = 1;
    pal8_crop->crop_bottom = 1;
    pal8_crop->crop_left = 1;
    pal8_crop->crop_right = 1;
    fail_if(av_frame_apply_cropping(pal8_crop, AV_FRAME_CROP_UNALIGNED) < 0,
            "pal8_crop apply failed");
    print_pal8_crop_palette_preserved("frame:pal8-crop-palette-preserved",
                                      pal8_crop,
                                      pal8_crop_palette_before);

    av_frame_free(&pal8_crop);
    av_frame_free(&pal8_ref);
    av_frame_free(&pal8);

    AVFrame *move_dst = av_frame_alloc();
    fail_if(!move_dst, "move_dst av_frame_alloc failed");
    av_frame_move_ref(move_dst, video_ref);
    print_frame("frame:move-dst", move_dst);
    print_frame("frame:move-src", video_ref);

    av_frame_unref(video);
    print_frame("frame:unref", video);

    AVFrame *rich_unref = av_frame_alloc();
    fail_if(!rich_unref, "rich_unref av_frame_alloc failed");
    rich_unref->format = AV_PIX_FMT_GRAY8;
    rich_unref->width = 1;
    rich_unref->height = 1;
    rich_unref->pts = 701;
    rich_unref->time_base = (AVRational){ 1, 25 };
    rich_unref->opaque_ref = av_buffer_alloc(2);
    fail_if(!rich_unref->opaque_ref,
            "rich_unref opaque_ref allocation failed");
    rich_unref->opaque_ref->data[0] = 0x88;
    rich_unref->opaque_ref->data[1] = 0x99;
    rich_unref->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!rich_unref->hw_frames_ctx,
            "rich_unref hw_frames_ctx allocation failed");
    rich_unref->hw_frames_ctx->data[0] = 0x77;
    av_dict_set(&rich_unref->metadata, "title", "before-unref", 0);
    fail_if(av_frame_get_buffer(rich_unref, 1) < 0,
            "rich_unref av_frame_get_buffer failed");
    rich_unref->data[0][0] = 0x42;
    AVFrameSideData *rich_unref_sd = av_frame_new_side_data(
        rich_unref, AV_FRAME_DATA_DISPLAYMATRIX, 36);
    fail_if(!rich_unref_sd, "rich_unref side data allocation failed");
    memset(rich_unref_sd->data, 0x55, rich_unref_sd->size);
    av_frame_unref(rich_unref);
    print_frame("frame:unref-rich", rich_unref);

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
    print_frame_plane_buffer("frame:plane-buffer-audio-packed-0", audio, 0);
    print_frame_plane_buffer("frame:plane-buffer-audio-packed-invalid",
                             audio, 1);

    AVFrame *planar_audio = av_frame_alloc();
    fail_if(!planar_audio, "planar_audio av_frame_alloc failed");
    planar_audio->format = AV_SAMPLE_FMT_S16P;
    planar_audio->sample_rate = 48000;
    planar_audio->nb_samples = 2;
    av_channel_layout_default(&planar_audio->ch_layout, 2);
    fail_if(av_frame_get_buffer(planar_audio, 1) < 0,
            "planar_audio av_frame_get_buffer failed");
    planar_audio->data[0][0] = 1;
    planar_audio->data[0][1] = 0;
    planar_audio->data[0][2] = 2;
    planar_audio->data[0][3] = 0;
    planar_audio->data[1][0] = 3;
    planar_audio->data[1][1] = 0;
    planar_audio->data[1][2] = 4;
    planar_audio->data[1][3] = 0;
    print_frame("frame:audio-planar-buffer", planar_audio);
    print_frame_plane_buffer("frame:plane-buffer-audio-planar-1",
                             planar_audio, 1);

    AVFrame *extended_audio = av_frame_alloc();
    fail_if(!extended_audio, "extended_audio av_frame_alloc failed");
    extended_audio->format = AV_SAMPLE_FMT_S16P;
    extended_audio->sample_rate = 48000;
    extended_audio->nb_samples = 1;
    av_channel_layout_default(&extended_audio->ch_layout, 10);
    fail_if(av_frame_get_buffer(extended_audio, 1) < 0,
            "extended_audio av_frame_get_buffer failed");
    for (int i = 0; i < 10; i++) {
        extended_audio->extended_data[i][0] = (uint8_t)i;
        extended_audio->extended_data[i][1] = 0;
    }
    print_frame_buffer_topology("frame:audio-extended-topology",
                                extended_audio);
    print_frame_plane_buffer("frame:plane-buffer-audio-extended-8",
                             extended_audio, 8);
    print_frame_plane_buffer("frame:plane-buffer-audio-extended-9",
                             extended_audio, 9);
    print_frame_plane_buffer("frame:plane-buffer-audio-extended-invalid",
                             extended_audio, 10);

    AVFrame *packed_ten_audio = av_frame_alloc();
    fail_if(!packed_ten_audio, "packed_ten_audio av_frame_alloc failed");
    packed_ten_audio->format = AV_SAMPLE_FMT_S16;
    packed_ten_audio->sample_rate = 48000;
    packed_ten_audio->nb_samples = 1;
    av_channel_layout_default(&packed_ten_audio->ch_layout, 10);
    fail_if(av_frame_get_buffer(packed_ten_audio, 1) < 0,
            "packed_ten_audio av_frame_get_buffer failed");
    memset(packed_ten_audio->data[0], 0, 20);
    print_frame_buffer_topology("frame:audio-packed-ten-topology",
                                packed_ten_audio);
    print_frame_plane_buffer("frame:plane-buffer-audio-packed-ten-0",
                             packed_ten_audio, 0);
    print_frame_plane_buffer("frame:plane-buffer-audio-packed-ten-invalid",
                             packed_ten_audio, 1);

    AVFrame *copy_data_video_src = av_frame_alloc();
    fail_if(!copy_data_video_src, "copy_data_video_src allocation failed");
    copy_data_video_src->format = AV_PIX_FMT_GRAY8;
    copy_data_video_src->width = 3;
    copy_data_video_src->height = 2;
    copy_data_video_src->pts = 101;
    av_dict_set(&copy_data_video_src->metadata, "title",
                "copy-data-source", 0);
    fail_if(av_frame_get_buffer(copy_data_video_src, 1) < 0,
            "copy_data_video_src get_buffer failed");
    static const uint8_t copy_data_video_src_payload[] =
        { 1, 2, 3, 4, 5, 6 };
    fill_video_gray(copy_data_video_src, copy_data_video_src_payload);

    AVFrame *copy_data_video_dst = av_frame_alloc();
    fail_if(!copy_data_video_dst, "copy_data_video_dst allocation failed");
    copy_data_video_dst->format = AV_PIX_FMT_GRAY8;
    copy_data_video_dst->width = 3;
    copy_data_video_dst->height = 2;
    copy_data_video_dst->pts = 202;
    av_dict_set(&copy_data_video_dst->metadata, "title",
                "copy-data-destination", 0);
    fail_if(av_frame_get_buffer(copy_data_video_dst, 1) < 0,
            "copy_data_video_dst get_buffer failed");
    static const uint8_t copy_data_video_dst_payload[] =
        { 9, 9, 9, 8, 8, 8 };
    fill_video_gray(copy_data_video_dst, copy_data_video_dst_payload);

    int copy_data_video_ret =
        av_frame_copy(copy_data_video_dst, copy_data_video_src);
    printf("frame:copy-data-video-ret|%d\n", copy_data_video_ret);
    fail_if(copy_data_video_ret < 0, "copy_data_video av_frame_copy failed");
    print_frame("frame:copy-data-video-src", copy_data_video_src);
    print_frame("frame:copy-data-video-dst", copy_data_video_dst);

    AVFrame *copy_data_audio_src = av_frame_alloc();
    fail_if(!copy_data_audio_src, "copy_data_audio_src allocation failed");
    copy_data_audio_src->format = AV_SAMPLE_FMT_S16;
    copy_data_audio_src->sample_rate = 44100;
    copy_data_audio_src->nb_samples = 2;
    av_channel_layout_default(&copy_data_audio_src->ch_layout, 2);
    fail_if(av_frame_get_buffer(copy_data_audio_src, 1) < 0,
            "copy_data_audio_src get_buffer failed");
    static const uint8_t copy_data_audio_src_payload[] =
        { 1, 0, 2, 0, 3, 0, 4, 0 };
    memcpy(copy_data_audio_src->data[0], copy_data_audio_src_payload,
           sizeof(copy_data_audio_src_payload));

    AVFrame *copy_data_audio_dst = av_frame_alloc();
    fail_if(!copy_data_audio_dst, "copy_data_audio_dst allocation failed");
    copy_data_audio_dst->format = AV_SAMPLE_FMT_S16;
    copy_data_audio_dst->sample_rate = 96000;
    copy_data_audio_dst->nb_samples = 2;
    copy_data_audio_dst->pts = 303;
    av_channel_layout_default(&copy_data_audio_dst->ch_layout, 2);
    fail_if(av_frame_get_buffer(copy_data_audio_dst, 1) < 0,
            "copy_data_audio_dst get_buffer failed");
    static const uint8_t copy_data_audio_dst_payload[] =
        { 9, 0, 8, 0, 7, 0, 6, 0 };
    memcpy(copy_data_audio_dst->data[0], copy_data_audio_dst_payload,
           sizeof(copy_data_audio_dst_payload));

    int copy_data_audio_ret =
        av_frame_copy(copy_data_audio_dst, copy_data_audio_src);
    printf("frame:copy-data-audio-ret|%d\n", copy_data_audio_ret);
    fail_if(copy_data_audio_ret < 0, "copy_data_audio av_frame_copy failed");
    print_frame("frame:copy-data-audio-src", copy_data_audio_src);
    print_frame("frame:copy-data-audio-dst", copy_data_audio_dst);

    AVFrame *copy_data_larger_dst = av_frame_alloc();
    fail_if(!copy_data_larger_dst,
            "copy_data_larger_dst allocation failed");
    copy_data_larger_dst->format = AV_PIX_FMT_GRAY8;
    copy_data_larger_dst->width = 3;
    copy_data_larger_dst->height = 2;
    fail_if(av_frame_get_buffer(copy_data_larger_dst, 1) < 0,
            "copy_data_larger_dst get_buffer failed");
    static const uint8_t copy_data_larger_dst_payload[] =
        { 7, 7, 7, 6, 6, 6 };
    fill_video_gray(copy_data_larger_dst, copy_data_larger_dst_payload);

    AVFrame *copy_data_larger_src = av_frame_alloc();
    fail_if(!copy_data_larger_src,
            "copy_data_larger_src allocation failed");
    copy_data_larger_src->format = AV_PIX_FMT_GRAY8;
    copy_data_larger_src->width = 2;
    copy_data_larger_src->height = 2;
    fail_if(av_frame_get_buffer(copy_data_larger_src, 1) < 0,
            "copy_data_larger_src get_buffer failed");
    static const uint8_t copy_data_larger_src_payload[] = { 1, 2, 3, 4 };
    fill_video_gray(copy_data_larger_src, copy_data_larger_src_payload);

    int copy_data_larger_ret =
        av_frame_copy(copy_data_larger_dst, copy_data_larger_src);
    printf("frame:copy-data-video-larger-dst-ret|%d\n",
           copy_data_larger_ret);
    fail_if(copy_data_larger_ret < 0,
            "copy_data_larger av_frame_copy failed");
    print_frame("frame:copy-data-video-larger-dst", copy_data_larger_dst);

    AVFrame *copy_data_too_small_dst = av_frame_alloc();
    fail_if(!copy_data_too_small_dst,
            "copy_data_too_small_dst allocation failed");
    copy_data_too_small_dst->format = AV_PIX_FMT_GRAY8;
    copy_data_too_small_dst->width = 2;
    copy_data_too_small_dst->height = 2;
    fail_if(av_frame_get_buffer(copy_data_too_small_dst, 1) < 0,
            "copy_data_too_small_dst get_buffer failed");
    static const uint8_t copy_data_too_small_dst_payload[] = { 7, 7, 6, 6 };
    fill_video_gray(copy_data_too_small_dst,
                    copy_data_too_small_dst_payload);

    AVFrame *copy_data_too_small_src = av_frame_alloc();
    fail_if(!copy_data_too_small_src,
            "copy_data_too_small_src allocation failed");
    copy_data_too_small_src->format = AV_PIX_FMT_GRAY8;
    copy_data_too_small_src->width = 3;
    copy_data_too_small_src->height = 2;
    fail_if(av_frame_get_buffer(copy_data_too_small_src, 1) < 0,
            "copy_data_too_small_src get_buffer failed");
    static const uint8_t copy_data_too_small_src_payload[] =
        { 1, 2, 3, 4, 5, 6 };
    fill_video_gray(copy_data_too_small_src,
                    copy_data_too_small_src_payload);

    int copy_data_too_small_ret =
        av_frame_copy(copy_data_too_small_dst, copy_data_too_small_src);
    printf("frame:copy-data-video-too-small-ret|%d\n",
           copy_data_too_small_ret);
    print_frame("frame:copy-data-video-too-small-dst",
                copy_data_too_small_dst);

    AVFrame *copy_data_kind_mismatch_video = av_frame_alloc();
    fail_if(!copy_data_kind_mismatch_video,
            "copy_data_kind_mismatch_video allocation failed");
    copy_data_kind_mismatch_video->format = AV_PIX_FMT_GRAY8;
    copy_data_kind_mismatch_video->width = 3;
    copy_data_kind_mismatch_video->height = 2;
    fail_if(av_frame_get_buffer(copy_data_kind_mismatch_video, 1) < 0,
            "copy_data_kind_mismatch_video get_buffer failed");
    static const uint8_t copy_data_kind_mismatch_video_payload[] =
        { 9, 9, 9, 8, 8, 8 };
    fill_video_gray(copy_data_kind_mismatch_video,
                    copy_data_kind_mismatch_video_payload);

    AVFrame *copy_data_kind_mismatch_audio = av_frame_alloc();
    fail_if(!copy_data_kind_mismatch_audio,
            "copy_data_kind_mismatch_audio allocation failed");
    copy_data_kind_mismatch_audio->format = AV_SAMPLE_FMT_S16;
    copy_data_kind_mismatch_audio->sample_rate = 44100;
    copy_data_kind_mismatch_audio->nb_samples = 2;
    av_channel_layout_default(&copy_data_kind_mismatch_audio->ch_layout, 2);
    fail_if(av_frame_get_buffer(copy_data_kind_mismatch_audio, 1) < 0,
            "copy_data_kind_mismatch_audio get_buffer failed");
    static const uint8_t copy_data_kind_mismatch_audio_payload[] =
        { 1, 0, 2, 0, 3, 0, 4, 0 };
    memcpy(copy_data_kind_mismatch_audio->data[0],
           copy_data_kind_mismatch_audio_payload,
           sizeof(copy_data_kind_mismatch_audio_payload));

    print_frame("frame:copy-data-kind-mismatch-before",
                copy_data_kind_mismatch_video);
    int copy_data_kind_mismatch_ret =
        av_frame_copy(copy_data_kind_mismatch_video, copy_data_kind_mismatch_audio);
    printf("frame:copy-data-kind-mismatch-ret|%d\n",
           copy_data_kind_mismatch_ret);
    print_frame("frame:copy-data-kind-mismatch-src",
                copy_data_kind_mismatch_audio);
    print_frame("frame:copy-data-kind-mismatch-after",
                copy_data_kind_mismatch_video);

    static const uint8_t crop_payload[] = {
        0,  1,  2,  3,  4,  5,
        16, 17, 18, 19, 20, 21,
        32, 33, 34, 35, 36, 37,
        48, 49, 50, 51, 52, 53,
    };

    AVFrame *crop_aligned = av_frame_alloc();
    fail_if(!crop_aligned, "crop_aligned allocation failed");
    crop_aligned->format = AV_PIX_FMT_GRAY8;
    crop_aligned->width = 6;
    crop_aligned->height = 4;
    fail_if(av_frame_get_buffer(crop_aligned, 64) < 0,
            "crop_aligned get_buffer failed");
    fill_video_gray(crop_aligned, crop_payload);
    crop_aligned->crop_top = 1;
    crop_aligned->crop_bottom = 1;
    crop_aligned->crop_left = 1;
    crop_aligned->crop_right = 2;
    int crop_aligned_ret = av_frame_apply_cropping(crop_aligned, 0);
    printf("frame:apply-crop-aligned-ret|%d\n", crop_aligned_ret);
    fail_if(crop_aligned_ret < 0, "crop_aligned apply failed");
    print_frame("frame:apply-crop-aligned", crop_aligned);

    AVFrame *crop_unaligned = av_frame_alloc();
    fail_if(!crop_unaligned, "crop_unaligned allocation failed");
    crop_unaligned->format = AV_PIX_FMT_GRAY8;
    crop_unaligned->width = 6;
    crop_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_unaligned, 64) < 0,
            "crop_unaligned get_buffer failed");
    fill_video_gray(crop_unaligned, crop_payload);
    crop_unaligned->crop_top = 1;
    crop_unaligned->crop_bottom = 1;
    crop_unaligned->crop_left = 1;
    crop_unaligned->crop_right = 2;
    int crop_unaligned_ret =
        av_frame_apply_cropping(crop_unaligned, AV_FRAME_CROP_UNALIGNED);
    printf("frame:apply-crop-unaligned-ret|%d\n", crop_unaligned_ret);
    fail_if(crop_unaligned_ret < 0, "crop_unaligned apply failed");
    print_frame("frame:apply-crop-unaligned", crop_unaligned);

    AVFrame *crop_rgb24_aligned = av_frame_alloc();
    fail_if(!crop_rgb24_aligned, "crop_rgb24_aligned allocation failed");
    crop_rgb24_aligned->format = AV_PIX_FMT_RGB24;
    crop_rgb24_aligned->width = 8;
    crop_rgb24_aligned->height = 4;
    fail_if(av_frame_get_buffer(crop_rgb24_aligned, 64) < 0,
            "crop_rgb24_aligned get_buffer failed");
    fill_video_packed(crop_rgb24_aligned, 3);
    crop_rgb24_aligned->crop_top = 1;
    crop_rgb24_aligned->crop_left = 1;
    crop_rgb24_aligned->crop_right = 1;
    int crop_rgb24_aligned_ret =
        av_frame_apply_cropping(crop_rgb24_aligned, 0);
    printf("frame:apply-crop-rgb24-aligned-ret|%d\n",
           crop_rgb24_aligned_ret);
    fail_if(crop_rgb24_aligned_ret < 0, "crop_rgb24_aligned apply failed");
    print_frame("frame:apply-crop-rgb24-aligned", crop_rgb24_aligned);

    AVFrame *crop_rgb24_unaligned = av_frame_alloc();
    fail_if(!crop_rgb24_unaligned, "crop_rgb24_unaligned allocation failed");
    crop_rgb24_unaligned->format = AV_PIX_FMT_RGB24;
    crop_rgb24_unaligned->width = 8;
    crop_rgb24_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_rgb24_unaligned, 64) < 0,
            "crop_rgb24_unaligned get_buffer failed");
    fill_video_packed(crop_rgb24_unaligned, 3);
    crop_rgb24_unaligned->crop_top = 1;
    crop_rgb24_unaligned->crop_left = 1;
    crop_rgb24_unaligned->crop_right = 1;
    int crop_rgb24_unaligned_ret = av_frame_apply_cropping(
        crop_rgb24_unaligned, AV_FRAME_CROP_UNALIGNED);
    printf("frame:apply-crop-rgb24-unaligned-ret|%d\n",
           crop_rgb24_unaligned_ret);
    fail_if(crop_rgb24_unaligned_ret < 0,
            "crop_rgb24_unaligned apply failed");
    print_frame("frame:apply-crop-rgb24-unaligned", crop_rgb24_unaligned);

    AVFrame *crop_bgr24_aligned = av_frame_alloc();
    fail_if(!crop_bgr24_aligned, "crop_bgr24_aligned allocation failed");
    crop_bgr24_aligned->format = AV_PIX_FMT_BGR24;
    crop_bgr24_aligned->width = 8;
    crop_bgr24_aligned->height = 4;
    fail_if(av_frame_get_buffer(crop_bgr24_aligned, 64) < 0,
            "crop_bgr24_aligned get_buffer failed");
    fill_video_packed(crop_bgr24_aligned, 3);
    crop_bgr24_aligned->crop_top = 1;
    crop_bgr24_aligned->crop_left = 1;
    crop_bgr24_aligned->crop_right = 1;
    int crop_bgr24_aligned_ret =
        av_frame_apply_cropping(crop_bgr24_aligned, 0);
    printf("frame:apply-crop-bgr24-aligned-ret|%d\n",
           crop_bgr24_aligned_ret);
    fail_if(crop_bgr24_aligned_ret < 0, "crop_bgr24_aligned apply failed");
    print_frame("frame:apply-crop-bgr24-aligned", crop_bgr24_aligned);

    AVFrame *crop_bgr24_unaligned = av_frame_alloc();
    fail_if(!crop_bgr24_unaligned, "crop_bgr24_unaligned allocation failed");
    crop_bgr24_unaligned->format = AV_PIX_FMT_BGR24;
    crop_bgr24_unaligned->width = 8;
    crop_bgr24_unaligned->height = 4;
    fail_if(av_frame_get_buffer(crop_bgr24_unaligned, 64) < 0,
            "crop_bgr24_unaligned get_buffer failed");
    fill_video_packed(crop_bgr24_unaligned, 3);
    crop_bgr24_unaligned->crop_top = 1;
    crop_bgr24_unaligned->crop_left = 1;
    crop_bgr24_unaligned->crop_right = 1;
    int crop_bgr24_unaligned_ret = av_frame_apply_cropping(
        crop_bgr24_unaligned, AV_FRAME_CROP_UNALIGNED);
    printf("frame:apply-crop-bgr24-unaligned-ret|%d\n",
           crop_bgr24_unaligned_ret);
    fail_if(crop_bgr24_unaligned_ret < 0,
            "crop_bgr24_unaligned apply failed");
    print_frame("frame:apply-crop-bgr24-unaligned", crop_bgr24_unaligned);

    exercise_bitstream_crop_pair("monow", AV_PIX_FMT_MONOWHITE);
    exercise_bitstream_crop_pair("monob", AV_PIX_FMT_MONOBLACK);
    exercise_bitstream_crop_pair("rgb4", AV_PIX_FMT_RGB4);
    exercise_bitstream_crop_pair("bgr4", AV_PIX_FMT_BGR4);
    exercise_packed_crop_pair("rgba", AV_PIX_FMT_RGBA, 4);
    exercise_packed_crop_pair("bgra", AV_PIX_FMT_BGRA, 4);
    exercise_packed_crop_pair("argb", AV_PIX_FMT_ARGB, 4);
    exercise_packed_crop_pair("abgr", AV_PIX_FMT_ABGR, 4);
    exercise_packed_crop_pair("0rgb", AV_PIX_FMT_0RGB, 4);
    exercise_packed_crop_pair("rgb0", AV_PIX_FMT_RGB0, 4);
    exercise_packed_crop_pair("0bgr", AV_PIX_FMT_0BGR, 4);
    exercise_packed_crop_pair("bgr0", AV_PIX_FMT_BGR0, 4);
    exercise_packed_crop_pair("x2rgb10le", AV_PIX_FMT_X2RGB10LE, 4);
    exercise_packed_crop_pair("x2rgb10be", AV_PIX_FMT_X2RGB10BE, 4);
    exercise_packed_crop_pair("x2bgr10le", AV_PIX_FMT_X2BGR10LE, 4);
    exercise_packed_crop_pair("x2bgr10be", AV_PIX_FMT_X2BGR10BE, 4);
    exercise_packed_crop_pair("pal8", AV_PIX_FMT_PAL8, 1);
    exercise_packed_crop_pair("rgb8", AV_PIX_FMT_RGB8, 1);
    exercise_packed_crop_pair("bgr8", AV_PIX_FMT_BGR8, 1);
    exercise_packed_crop_pair("rgb4_byte", AV_PIX_FMT_RGB4_BYTE, 1);
    exercise_packed_crop_pair("bgr4_byte", AV_PIX_FMT_BGR4_BYTE, 1);
    exercise_packed_crop_pair("bayer_bggr8", AV_PIX_FMT_BAYER_BGGR8, 1);
    exercise_packed_crop_pair("bayer_rggb8", AV_PIX_FMT_BAYER_RGGB8, 1);
    exercise_packed_crop_pair("bayer_gbrg8", AV_PIX_FMT_BAYER_GBRG8, 1);
    exercise_packed_crop_pair("bayer_grbg8", AV_PIX_FMT_BAYER_GRBG8, 1);
    exercise_packed_crop_pair("yuyv422", AV_PIX_FMT_YUYV422, 2);
    exercise_packed_crop_pair("uyvy422", AV_PIX_FMT_UYVY422, 2);
    exercise_packed_crop_pair("yvyu422", AV_PIX_FMT_YVYU422, 2);
    exercise_uyyvyy411_crop_pair();
    exercise_packed_crop_pair("rgb565be", AV_PIX_FMT_RGB565BE, 2);
    exercise_packed_crop_pair("rgb565le", AV_PIX_FMT_RGB565LE, 2);
    exercise_packed_crop_pair("rgb555be", AV_PIX_FMT_RGB555BE, 2);
    exercise_packed_crop_pair("rgb555le", AV_PIX_FMT_RGB555LE, 2);
    exercise_packed_crop_pair("bgr565be", AV_PIX_FMT_BGR565BE, 2);
    exercise_packed_crop_pair("bgr565le", AV_PIX_FMT_BGR565LE, 2);
    exercise_packed_crop_pair("bgr555be", AV_PIX_FMT_BGR555BE, 2);
    exercise_packed_crop_pair("bgr555le", AV_PIX_FMT_BGR555LE, 2);
    exercise_packed_crop_pair("rgb444le", AV_PIX_FMT_RGB444LE, 2);
    exercise_packed_crop_pair("rgb444be", AV_PIX_FMT_RGB444BE, 2);
    exercise_packed_crop_pair("bgr444le", AV_PIX_FMT_BGR444LE, 2);
    exercise_packed_crop_pair("bgr444be", AV_PIX_FMT_BGR444BE, 2);
    exercise_packed_crop_pair("bayer_bggr16le", AV_PIX_FMT_BAYER_BGGR16LE, 2);
    exercise_packed_crop_pair("bayer_bggr16be", AV_PIX_FMT_BAYER_BGGR16BE, 2);
    exercise_packed_crop_pair("bayer_rggb16le", AV_PIX_FMT_BAYER_RGGB16LE, 2);
    exercise_packed_crop_pair("bayer_rggb16be", AV_PIX_FMT_BAYER_RGGB16BE, 2);
    exercise_packed_crop_pair("bayer_gbrg16le", AV_PIX_FMT_BAYER_GBRG16LE, 2);
    exercise_packed_crop_pair("bayer_gbrg16be", AV_PIX_FMT_BAYER_GBRG16BE, 2);
    exercise_packed_crop_pair("bayer_grbg16le", AV_PIX_FMT_BAYER_GRBG16LE, 2);
    exercise_packed_crop_pair("bayer_grbg16be", AV_PIX_FMT_BAYER_GRBG16BE, 2);
    exercise_packed_crop_pair("ya8", AV_PIX_FMT_YA8, 2);
    exercise_packed_crop_pair("ya16le", AV_PIX_FMT_YA16LE, 4);
    exercise_packed_crop_pair("ya16be", AV_PIX_FMT_YA16BE, 4);
    exercise_packed_crop_pair("yaf16le", AV_PIX_FMT_YAF16LE, 4);
    exercise_packed_crop_pair("yaf16be", AV_PIX_FMT_YAF16BE, 4);
    exercise_packed_crop_pair("yaf32le", AV_PIX_FMT_YAF32LE, 8);
    exercise_packed_crop_pair("yaf32be", AV_PIX_FMT_YAF32BE, 8);
    exercise_packed_crop_pair("gray9le", AV_PIX_FMT_GRAY9LE, 2);
    exercise_packed_crop_pair("gray9be", AV_PIX_FMT_GRAY9BE, 2);
    exercise_packed_crop_pair("gray10le", AV_PIX_FMT_GRAY10LE, 2);
    exercise_packed_crop_pair("gray10be", AV_PIX_FMT_GRAY10BE, 2);
    exercise_packed_crop_pair("gray12le", AV_PIX_FMT_GRAY12LE, 2);
    exercise_packed_crop_pair("gray12be", AV_PIX_FMT_GRAY12BE, 2);
    exercise_packed_crop_pair("gray14le", AV_PIX_FMT_GRAY14LE, 2);
    exercise_packed_crop_pair("gray14be", AV_PIX_FMT_GRAY14BE, 2);
    exercise_packed_crop_pair("gray16le", AV_PIX_FMT_GRAY16LE, 2);
    exercise_packed_crop_pair("gray16be", AV_PIX_FMT_GRAY16BE, 2);
    exercise_packed_crop_pair("gray32le", AV_PIX_FMT_GRAY32LE, 4);
    exercise_packed_crop_pair("gray32be", AV_PIX_FMT_GRAY32BE, 4);
    exercise_packed_crop_pair("grayf16le", AV_PIX_FMT_GRAYF16LE, 2);
    exercise_packed_crop_pair("grayf16be", AV_PIX_FMT_GRAYF16BE, 2);
    exercise_packed_crop_pair("grayf32le", AV_PIX_FMT_GRAYF32LE, 4);
    exercise_packed_crop_pair("grayf32be", AV_PIX_FMT_GRAYF32BE, 4);
    exercise_packed_crop_pair("y210le", AV_PIX_FMT_Y210LE, 4);
    exercise_packed_crop_pair("y210be", AV_PIX_FMT_Y210BE, 4);
    exercise_packed_crop_pair("y212le", AV_PIX_FMT_Y212LE, 4);
    exercise_packed_crop_pair("y212be", AV_PIX_FMT_Y212BE, 4);
    exercise_packed_crop_pair("y216le", AV_PIX_FMT_Y216LE, 4);
    exercise_packed_crop_pair("y216be", AV_PIX_FMT_Y216BE, 4);
    exercise_packed_crop_pair("rgb48le", AV_PIX_FMT_RGB48LE, 6);
    exercise_packed_crop_pair("rgb48be", AV_PIX_FMT_RGB48BE, 6);
    exercise_packed_crop_pair("rgbf16le", AV_PIX_FMT_RGBF16LE, 6);
    exercise_packed_crop_pair("rgbf16be", AV_PIX_FMT_RGBF16BE, 6);
    exercise_packed_crop_pair("bgr48le", AV_PIX_FMT_BGR48LE, 6);
    exercise_packed_crop_pair("bgr48be", AV_PIX_FMT_BGR48BE, 6);
    exercise_packed_crop_pair("rgba64le", AV_PIX_FMT_RGBA64LE, 8);
    exercise_packed_crop_pair("rgba64be", AV_PIX_FMT_RGBA64BE, 8);
    exercise_packed_crop_pair("rgbaf16le", AV_PIX_FMT_RGBAF16LE, 8);
    exercise_packed_crop_pair("rgbaf16be", AV_PIX_FMT_RGBAF16BE, 8);
    exercise_packed_crop_pair("bgra64le", AV_PIX_FMT_BGRA64LE, 8);
    exercise_packed_crop_pair("bgra64be", AV_PIX_FMT_BGRA64BE, 8);
    exercise_packed_crop_pair("ayuv64le", AV_PIX_FMT_AYUV64LE, 8);
    exercise_packed_crop_pair("ayuv64be", AV_PIX_FMT_AYUV64BE, 8);
    exercise_packed_crop_pair("rgbf32le", AV_PIX_FMT_RGBF32LE, 12);
    exercise_packed_crop_pair("rgbf32be", AV_PIX_FMT_RGBF32BE, 12);
    exercise_packed_crop_pair("rgb96le", AV_PIX_FMT_RGB96LE, 12);
    exercise_packed_crop_pair("rgb96be", AV_PIX_FMT_RGB96BE, 12);
    exercise_packed_crop_pair("rgbaf32le", AV_PIX_FMT_RGBAF32LE, 16);
    exercise_packed_crop_pair("rgbaf32be", AV_PIX_FMT_RGBAF32BE, 16);
    exercise_packed_crop_pair("rgba128le", AV_PIX_FMT_RGBA128LE, 16);
    exercise_packed_crop_pair("rgba128be", AV_PIX_FMT_RGBA128BE, 16);
    exercise_packed_crop_pair("vuya", AV_PIX_FMT_VUYA, 4);
    exercise_packed_crop_pair("vuyx", AV_PIX_FMT_VUYX, 4);
    exercise_packed_crop_pair("xv30le", AV_PIX_FMT_XV30LE, 4);
    exercise_packed_crop_pair("xv30be", AV_PIX_FMT_XV30BE, 4);
    exercise_packed_crop_pair("xv36le", AV_PIX_FMT_XV36LE, 8);
    exercise_packed_crop_pair("xv36be", AV_PIX_FMT_XV36BE, 8);
    exercise_packed_crop_pair("xv48le", AV_PIX_FMT_XV48LE, 8);
    exercise_packed_crop_pair("xv48be", AV_PIX_FMT_XV48BE, 8);
    exercise_packed_crop_pair("v30xle", AV_PIX_FMT_V30XLE, 4);
    exercise_packed_crop_pair("v30xbe", AV_PIX_FMT_V30XBE, 4);
    exercise_packed_crop_pair("ayuv", AV_PIX_FMT_AYUV, 4);
    exercise_packed_crop_pair("uyva", AV_PIX_FMT_UYVA, 4);
    exercise_packed_crop_pair("vyu444", AV_PIX_FMT_VYU444, 3);
    exercise_packed_crop_pair("xyz12le", AV_PIX_FMT_XYZ12LE, 6);
    exercise_packed_crop_pair("xyz12be", AV_PIX_FMT_XYZ12BE, 6);
    exercise_yuv420p_crop_pair();
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV12, "nv12");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV21, "nv21");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV16, "nv16");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV24, "nv24");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV42, "nv42");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV20LE, "nv20le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_NV20BE, "nv20be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P010LE, "p010le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P010BE, "p010be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P012LE, "p012le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P012BE, "p012be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P016LE, "p016le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P016BE, "p016be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P210LE, "p210le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P210BE, "p210be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P212LE, "p212le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P212BE, "p212be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P216LE, "p216le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P216BE, "p216be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P410LE, "p410le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P410BE, "p410be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P412LE, "p412le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P412BE, "p412be");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P416LE, "p416le");
    exercise_semiplanar_crop_pair(AV_PIX_FMT_P416BE, "p416be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVJ420P, "yuvj420p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P, "yuv422p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVJ422P, "yuvj422p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV410P, "yuv410p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV411P, "yuv411p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVJ411P, "yuvj411p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV440P, "yuv440p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVJ440P, "yuvj440p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P, "yuv444p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVJ444P, "yuvj444p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P9LE, "yuv420p9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P9BE, "yuv420p9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P9LE, "yuv422p9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P9BE, "yuv422p9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P9LE, "yuv444p9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P9BE, "yuv444p9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P10LE, "yuv420p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P10BE, "yuv420p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P10LE, "yuv422p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P10BE, "yuv422p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV440P10LE, "yuv440p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV440P10BE, "yuv440p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P10LE, "yuv444p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P10BE, "yuv444p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P10MSBLE, "yuv444p10msble");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P10MSBBE, "yuv444p10msbbe");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P12LE, "yuv420p12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P12BE, "yuv420p12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P12LE, "yuv422p12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P12BE, "yuv422p12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV440P12LE, "yuv440p12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV440P12BE, "yuv440p12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P12LE, "yuv444p12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P12BE, "yuv444p12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P12MSBLE, "yuv444p12msble");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P12MSBBE, "yuv444p12msbbe");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P14LE, "yuv420p14le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P14BE, "yuv420p14be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P14LE, "yuv422p14le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P14BE, "yuv422p14be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P14LE, "yuv444p14le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P14BE, "yuv444p14be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P16LE, "yuv420p16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV420P16BE, "yuv420p16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P16LE, "yuv422p16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV422P16BE, "yuv422p16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P16LE, "yuv444p16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUV444P16BE, "yuv444p16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP, "gbrp");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP9LE, "gbrp9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP9BE, "gbrp9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP10LE, "gbrp10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP10BE, "gbrp10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP10MSBLE, "gbrp10msble");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP10MSBBE, "gbrp10msbbe");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP12LE, "gbrp12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP12BE, "gbrp12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP12MSBLE, "gbrp12msble");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP12MSBBE, "gbrp12msbbe");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP14LE, "gbrp14le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP14BE, "gbrp14be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP16LE, "gbrp16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRP16BE, "gbrp16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRPF16LE, "gbrpf16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRPF16BE, "gbrpf16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRPF32LE, "gbrpf32le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRPF32BE, "gbrpf32be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P, "yuva420p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P, "yuva422p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P, "yuva444p");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P9LE, "yuva420p9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P9BE, "yuva420p9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P9LE, "yuva422p9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P9BE, "yuva422p9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P9LE, "yuva444p9le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P9BE, "yuva444p9be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P10LE, "yuva420p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P10BE, "yuva420p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P10LE, "yuva422p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P10BE, "yuva422p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P10LE, "yuva444p10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P10BE, "yuva444p10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P12LE, "yuva422p12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P12BE, "yuva422p12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P12LE, "yuva444p12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P12BE, "yuva444p12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P16LE, "yuva420p16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA420P16BE, "yuva420p16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P16LE, "yuva422p16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA422P16BE, "yuva422p16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P16LE, "yuva444p16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_YUVA444P16BE, "yuva444p16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP, "gbrap");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP10LE, "gbrap10le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP10BE, "gbrap10be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP12LE, "gbrap12le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP12BE, "gbrap12be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP14LE, "gbrap14le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP14BE, "gbrap14be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP16LE, "gbrap16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP16BE, "gbrap16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP32LE, "gbrap32le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAP32BE, "gbrap32be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAPF16LE, "gbrapf16le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAPF16BE, "gbrapf16be");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAPF32LE, "gbrapf32le");
    exercise_planar_yuv_crop_pair(AV_PIX_FMT_GBRAPF32BE, "gbrapf32be");

    AVFrame *invalid_crop = av_frame_alloc();
    fail_if(!invalid_crop, "invalid_crop allocation failed");
    invalid_crop->format = AV_PIX_FMT_GRAY8;
    invalid_crop->width = 6;
    invalid_crop->height = 4;
    fail_if(av_frame_get_buffer(invalid_crop, 64) < 0,
            "invalid_crop get_buffer failed");
    fill_video_gray(invalid_crop, crop_payload);
    invalid_crop->crop_top = 1;
    invalid_crop->crop_bottom = 0;
    invalid_crop->crop_left = 5;
    invalid_crop->crop_right = 1;
    int invalid_crop_ret = av_frame_apply_cropping(invalid_crop, 0);
    printf("frame:apply-crop-invalid-ret|%d\n", invalid_crop_ret);
    print_frame("frame:apply-crop-invalid", invalid_crop);

    AVFrame *copy_src = av_frame_alloc();
    fail_if(!copy_src, "copy_src av_frame_alloc failed");
    copy_src->format = AV_PIX_FMT_GRAY8;
    copy_src->width = 2;
    copy_src->height = 1;
    copy_src->pts = 321;
    copy_src->pkt_dts = 320;
    copy_src->duration = 319;
    copy_src->time_base = (AVRational){ 1, 48000 };
    copy_src->sample_rate = 22050;
    av_channel_layout_default(&copy_src->ch_layout, 2);
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
    copy_dst->sample_rate = 96000;
    av_channel_layout_default(&copy_dst->ch_layout, 6);
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

    AVFrame *replace_src = av_frame_alloc();
    fail_if(!replace_src, "replace_src av_frame_alloc failed");
    replace_src->format = AV_PIX_FMT_GRAY8;
    replace_src->width = 2;
    replace_src->height = 2;
    replace_src->pts = 410;
    replace_src->pkt_dts = 409;
    replace_src->duration = 408;
    replace_src->time_base = (AVRational){ 1, 1000 };
    replace_src->sample_rate = 32000;
    av_channel_layout_default(&replace_src->ch_layout, 1);
    replace_src->sample_aspect_ratio = (AVRational){ 4, 3 };
    replace_src->crop_top = 1;
    replace_src->crop_bottom = 0;
    replace_src->crop_left = 0;
    replace_src->crop_right = 1;
    replace_src->pict_type = AV_PICTURE_TYPE_I;
    replace_src->quality = 77;
    replace_src->repeat_pict = 2;
    replace_src->flags = AV_FRAME_FLAG_KEY | AV_FRAME_FLAG_LOSSLESS;
    replace_src->color_range = AVCOL_RANGE_JPEG;
    replace_src->color_primaries = AVCOL_PRI_BT2020;
    replace_src->color_trc = AVCOL_TRC_SMPTE2084;
    replace_src->colorspace = AVCOL_SPC_BT2020_NCL;
    replace_src->chroma_location = AVCHROMA_LOC_TOPLEFT;
    replace_src->best_effort_timestamp = 411;
    replace_src->decode_error_flags = FF_DECODE_ERROR_MISSING_REFERENCE;
    replace_src->opaque = (void *)(uintptr_t)0x5151;
    replace_src->opaque_ref = av_buffer_alloc(2);
    fail_if(!replace_src->opaque_ref,
            "replace_src opaque_ref allocation failed");
    replace_src->opaque_ref->data[0] = 0x66;
    replace_src->opaque_ref->data[1] = 0x67;
    replace_src->alpha_mode = AVALPHA_MODE_STRAIGHT;
    av_dict_set(&replace_src->metadata, "title", "replace-source", 0);
    fail_if(av_frame_get_buffer(replace_src, 1) < 0,
            "replace_src av_frame_get_buffer failed");
    static const uint8_t replace_src_payload[] = { 10, 11, 12, 13 };
    fill_video_gray(replace_src, replace_src_payload);
    replace_src->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!replace_src->hw_frames_ctx,
            "replace_src hw context allocation failed");
    replace_src->hw_frames_ctx->data[0] = 0x55;
    AVFrameSideData *replace_src_sd = av_frame_new_side_data(
        replace_src, AV_FRAME_DATA_DISPLAYMATRIX, 36);
    fail_if(!replace_src_sd, "replace_src side data allocation failed");
    memset(replace_src_sd->data, 0x44, replace_src_sd->size);

    AVFrame *ref_dst = av_frame_alloc();
    fail_if(!ref_dst, "ref_dst av_frame_alloc failed");
    int ref_ret = av_frame_ref(ref_dst, replace_src);
    printf("frame:ref-rich-ret|%d\n", ref_ret);
    fail_if(ref_ret < 0, "ref_dst av_frame_ref failed");
    print_frame("frame:ref-rich-src", replace_src);
    print_frame("frame:ref-rich-dst", ref_dst);
    print_share("frame:ref-rich-plane-shares", replace_src, ref_dst);
    print_side_share("frame:ref-rich-side-shares", replace_src, ref_dst);
    print_hw_share("frame:ref-rich-hw-shares", replace_src, ref_dst);
    AVFrame *replace_src_after_unref = av_frame_clone(replace_src);
    fail_if(!replace_src_after_unref, "replace_src_after_unref av_frame_clone failed");
    av_frame_unref(replace_src_after_unref);
    print_frame("frame:ref-rich-src-after-unref", replace_src_after_unref);
    print_frame("frame:ref-rich-dst-after-source-unref", ref_dst);
    av_frame_free(&replace_src_after_unref);
    av_frame_free(&ref_dst);

    AVFrame *replace_clone = av_frame_clone(replace_src);
    fail_if(!replace_clone, "av_frame_clone failed");
    print_frame("frame:clone-ref-src", replace_src);
    print_frame("frame:clone-ref-dst", replace_clone);
    print_share("frame:clone-ref-plane-shares", replace_src, replace_clone);
    print_side_share("frame:clone-ref-side-shares", replace_src,
                     replace_clone);
    print_hw_share("frame:clone-ref-hw-shares", replace_src, replace_clone);
    av_buffer_unref(&replace_clone->hw_frames_ctx);
    print_frame("frame:clone-ref-after-take-hw-context", replace_clone);
    int rich_make_writable_ret = av_frame_make_writable(replace_clone);
    printf("frame:rich-make-writable-ret|%d\n", rich_make_writable_ret);
    fail_if(rich_make_writable_ret < 0,
            "rich av_frame_make_writable failed");
    print_frame("frame:rich-after-make-writable-src", replace_src);
    print_frame("frame:rich-after-make-writable-dst", replace_clone);
    print_share("frame:rich-after-make-writable-plane-shares", replace_src,
                replace_clone);
    print_side_share("frame:rich-after-make-writable-side-shares",
                     replace_src, replace_clone);
    print_opaque_ref_share("frame:rich-after-make-writable-opaque-ref-shares",
                           replace_src, replace_clone);

    AVFrame *replace_dst = av_frame_alloc();
    fail_if(!replace_dst, "replace_dst av_frame_alloc failed");
    replace_dst->format = AV_PIX_FMT_GRAY8;
    replace_dst->width = 1;
    replace_dst->height = 1;
    replace_dst->pts = 999;
    av_dict_set(&replace_dst->metadata, "keep", "destination", 0);
    fail_if(av_frame_get_buffer(replace_dst, 1) < 0,
            "replace_dst av_frame_get_buffer failed");
    replace_dst->data[0][0] = 9;
    replace_dst->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!replace_dst->hw_frames_ctx,
            "replace_dst hw context allocation failed");
    replace_dst->hw_frames_ctx->data[0] = 0x99;
    AVFrameSideData *replace_dst_sd = av_frame_new_side_data(
        replace_dst, AV_FRAME_DATA_REPLAYGAIN, 16);
    fail_if(!replace_dst_sd, "replace_dst side data allocation failed");
    memset(replace_dst_sd->data, 0x99, replace_dst_sd->size);
    int replace_ret = av_frame_replace(replace_dst, replace_src);
    printf("frame:replace-ret|%d\n", replace_ret);
    fail_if(replace_ret < 0, "av_frame_replace failed");
    print_frame("frame:replace-src", replace_src);
    print_frame("frame:replace-dst", replace_dst);
    print_share("frame:replace-plane-shares", replace_src, replace_dst);
    print_side_share("frame:replace-side-shares", replace_src, replace_dst);
    print_hw_share("frame:replace-hw-shares", replace_src, replace_dst);

    AVFrame *replace_empty_dst = av_frame_clone(replace_src);
    fail_if(!replace_empty_dst, "replace_empty_dst clone failed");
    AVFrame *replace_empty_src = av_frame_alloc();
    fail_if(!replace_empty_src, "replace_empty_src av_frame_alloc failed");
    int replace_empty_ret =
        av_frame_replace(replace_empty_dst, replace_empty_src);
    printf("frame:replace-empty-ret|%d\n", replace_empty_ret);
    fail_if(replace_empty_ret < 0, "av_frame_replace empty failed");
    print_frame("frame:replace-empty-dst", replace_empty_dst);

    AVFrame *move_replace_dst = av_frame_alloc();
    fail_if(!move_replace_dst, "move_replace_dst av_frame_alloc failed");
    move_replace_dst->format = AV_PIX_FMT_GRAY8;
    move_replace_dst->width = 1;
    move_replace_dst->height = 1;
    move_replace_dst->pts = 1234;
    av_dict_set(&move_replace_dst->metadata, "keep", "move-destination", 0);
    fail_if(av_frame_get_buffer(move_replace_dst, 1) < 0,
            "move_replace_dst av_frame_get_buffer failed");
    move_replace_dst->data[0][0] = 0xAA;
    move_replace_dst->hw_frames_ctx = av_buffer_alloc(1);
    fail_if(!move_replace_dst->hw_frames_ctx,
            "move_replace_dst hw context allocation failed");
    move_replace_dst->hw_frames_ctx->data[0] = 0xAA;
    move_replace_dst->opaque_ref = av_buffer_alloc(1);
    fail_if(!move_replace_dst->opaque_ref,
            "move_replace_dst opaque_ref allocation failed");
    move_replace_dst->opaque_ref->data[0] = 0xAB;
    AVFrameSideData *move_replace_dst_sd = av_frame_new_side_data(
        move_replace_dst, AV_FRAME_DATA_REPLAYGAIN, 16);
    fail_if(!move_replace_dst_sd,
            "move_replace_dst side data allocation failed");
    memset(move_replace_dst_sd->data, 0xAA, move_replace_dst_sd->size);
    av_frame_move_ref(move_replace_dst, replace_src);
    print_frame("frame:move-replace-dst", move_replace_dst);
    print_frame("frame:move-replace-src", replace_src);
    print_share("frame:move-replace-plane-shares", replace_dst,
                move_replace_dst);
    print_side_share("frame:move-replace-side-shares", replace_dst,
                     move_replace_dst);
    print_hw_share("frame:move-replace-hw-shares", replace_dst,
                   move_replace_dst);

    exercise_frame_fifo_api();

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

    AVFrame *side_from_buf = av_frame_alloc();
    fail_if(!side_from_buf, "side_from_buf av_frame_alloc failed");
    AVBufferRef *frame_take_buf = av_buffer_alloc(3);
    fail_if(!frame_take_buf, "frame side data from buf allocation failed");
    frame_take_buf->data[0] = 0x91;
    frame_take_buf->data[1] = 0x92;
    frame_take_buf->data[2] = 0x93;
    AVFrameSideData *frame_take_entry = av_frame_new_side_data_from_buf(
        side_from_buf, AV_FRAME_DATA_REPLAYGAIN, frame_take_buf);
    print_side_add_buffer_row(
        "frame:side-data-from-buf-take", frame_take_entry != NULL,
        side_from_buf->side_data, side_from_buf->nb_side_data,
        frame_take_entry ? NULL : frame_take_buf, AV_FRAME_DATA_REPLAYGAIN);

    AVBufferRef *frame_duplicate_buf = av_buffer_alloc(1);
    fail_if(!frame_duplicate_buf,
            "frame side data from buf duplicate allocation failed");
    frame_duplicate_buf->data[0] = 0x94;
    AVFrameSideData *frame_duplicate_entry = av_frame_new_side_data_from_buf(
        side_from_buf, AV_FRAME_DATA_REPLAYGAIN, frame_duplicate_buf);
    print_side_add_buffer_row(
        "frame:side-data-from-buf-duplicate",
        frame_duplicate_entry != NULL, side_from_buf->side_data,
        side_from_buf->nb_side_data,
        frame_duplicate_entry ? NULL : frame_duplicate_buf,
        AV_FRAME_DATA_REPLAYGAIN);
    if (!frame_duplicate_entry)
        av_buffer_unref(&frame_duplicate_buf);

    AVBufferRef *frame_multi_buf = av_buffer_alloc(16);
    fail_if(!frame_multi_buf,
            "frame side data from buf multi allocation failed");
    memset(frame_multi_buf->data, 0x95, frame_multi_buf->size);
    AVFrameSideData *frame_multi_entry = av_frame_new_side_data_from_buf(
        side_from_buf, AV_FRAME_DATA_SEI_UNREGISTERED, frame_multi_buf);
    print_side_add_buffer_row(
        "frame:side-data-from-buf-multi", frame_multi_entry != NULL,
        side_from_buf->side_data, side_from_buf->nb_side_data,
        frame_multi_entry ? NULL : frame_multi_buf,
        AV_FRAME_DATA_SEI_UNREGISTERED);
    AVFrameSideData *frame_duplicate_found =
        av_frame_get_side_data(side_from_buf, AV_FRAME_DATA_REPLAYGAIN);
    printf("frame:side-data-get-duplicate|%d|%d|%zu|",
           side_from_buf->nb_side_data,
           frame_duplicate_found == frame_take_entry,
           frame_duplicate_found ? frame_duplicate_found->size : 0);
    if (frame_duplicate_found)
        print_hex(frame_duplicate_found->data, frame_duplicate_found->size);
    else
        printf("none");
    printf("\n");
    int before_frame_duplicate_remove = side_from_buf->nb_side_data;
    av_frame_remove_side_data(side_from_buf, AV_FRAME_DATA_REPLAYGAIN);
    printf("frame:side-data-remove-duplicate|%d|%d|",
           before_frame_duplicate_remove - side_from_buf->nb_side_data,
           side_from_buf->nb_side_data);
    print_side_array_summary(side_from_buf->side_data,
                             side_from_buf->nb_side_data);
    printf("\n");

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

    const AVFrameSideData *array_multi_found = av_frame_side_data_get(
        side_array, nb_side_array, AV_FRAME_DATA_SEI_UNREGISTERED);
    printf("frame:side-array-get-multi-first|%d|%d|%zu|",
           nb_side_array, array_multi_found == sei_entry,
           array_multi_found ? array_multi_found->size : 0);
    if (array_multi_found)
        print_hex(array_multi_found->data, array_multi_found->size);
    else
        printf("none");
    printf("\n");
    const AVFrameSideData *array_missing_found = av_frame_side_data_get(
        side_array, nb_side_array, AV_FRAME_DATA_FILM_GRAIN_PARAMS);
    printf("frame:side-array-get-missing|%d|%d\n",
           array_missing_found != NULL, nb_side_array);

    av_frame_side_data_remove_by_props(&side_array, &nb_side_array,
                                       AV_SIDE_DATA_PROP_MULTI);
    printf("frame:side-array-remove-multi|%d|", nb_side_array);
    print_side_array_summary(side_array, nb_side_array);
    printf("\n");
    av_frame_side_data_free(&side_array, &nb_side_array);
    printf("frame:side-array-free|%d|%d\n", nb_side_array,
           side_array == NULL);

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

    av_frame_free(&move_replace_dst);
    av_frame_free(&replace_empty_src);
    av_frame_free(&replace_empty_dst);
    av_frame_free(&replace_dst);
    av_frame_free(&replace_clone);
    av_frame_free(&replace_src);
    av_frame_free(&side_from_buf);
    av_frame_free(&side_frame);
    av_frame_free(&copy_dst);
    av_frame_free(&copy_src);
    av_frame_free(&copy_data_too_small_src);
    av_frame_free(&copy_data_too_small_dst);
    av_frame_free(&copy_data_larger_src);
    av_frame_free(&copy_data_larger_dst);
    av_frame_free(&copy_data_audio_dst);
    av_frame_free(&copy_data_audio_src);
    av_frame_free(&copy_data_video_dst);
    av_frame_free(&copy_data_video_src);
    av_frame_free(&copy_data_kind_mismatch_audio);
    av_frame_free(&copy_data_kind_mismatch_video);
    av_frame_free(&invalid_crop);
    av_frame_free(&crop_bgr24_unaligned);
    av_frame_free(&crop_bgr24_aligned);
    av_frame_free(&crop_rgb24_unaligned);
    av_frame_free(&crop_rgb24_aligned);
    av_frame_free(&crop_unaligned);
    av_frame_free(&crop_aligned);
    av_frame_free(&packed_ten_audio);
    av_frame_free(&extended_audio);
    av_frame_free(&planar_audio);
    av_frame_free(&audio);
    av_frame_free(&move_dst);
    av_frame_free(&video_ref);
    av_frame_free(&rich_unref);
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
