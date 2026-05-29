use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use avutil::{
    AvErrorCode, BufferPool, BufferPoolAllocation, BufferPoolCallbacks, BufferRef,
    AV_BUFFER_FLAG_READONLY, AV_BUFFER_REF_ABI_LAYOUT,
};

type ReleaseRows = Arc<Mutex<Vec<(usize, Vec<u8>)>>>;

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_buffer_refs_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/buffer.h").is_file(),
        "missing pinned FFmpeg libavutil buffer headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-buffer");
    fs::create_dir_all(&work_dir).expect("create avutil-buffer oracle work dir");
    let source = work_dir.join("buffer_oracle.c");
    let executable = work_dir.join("buffer_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-buffer oracle C source");

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
        "buffer:abi-avbufferref-layout".to_string(),
        buffer_abi_layout_fields(&AV_BUFFER_REF_ABI_LAYOUT),
    );
    rows.insert(
        "buffer:flag-readonly".to_string(),
        vec![AV_BUFFER_FLAG_READONLY.to_string()],
    );

    rows.insert(
        "buffer:alloc".to_string(),
        vec!["4".to_string(), "1".to_string(), "1".to_string()],
    );
    rows.insert("buffer:alloc-huge".to_string(), vec!["1".to_string()]);

    let alloc_zero = BufferRef::from_vec(Vec::new());
    rows.insert("buffer:alloc-zero".to_string(), buffer_fields(&alloc_zero));

    let allocz = BufferRef::zeroed(4).unwrap();
    rows.insert("buffer:allocz".to_string(), buffer_fields(&allocz));

    rows.insert(
        "buffer:allocz-huge".to_string(),
        vec![bool_field(
            BufferRef::zeroed(usize::MAX).unwrap_err().code() == Some(AvErrorCode::ENOMEM),
        )],
    );

    let allocz_zero = BufferRef::zeroed(0).unwrap();
    rows.insert(
        "buffer:allocz-zero".to_string(),
        buffer_fields(&allocz_zero),
    );

    let ref_src = BufferRef::from_vec(vec![1, 2, 3]);
    let ref_dst = ref_src.clone();
    rows.insert("buffer:ref-src".to_string(), buffer_fields(&ref_src));
    rows.insert("buffer:ref-dst".to_string(), buffer_fields(&ref_dst));
    rows.insert(
        "buffer:ref-shares".to_string(),
        vec![bool_field(ref_src.shares_storage(&ref_dst))],
    );

    let mut unique = BufferRef::from_vec(vec![4, 5, 6]);
    let unique_before = unique.as_ptr();
    unique.make_mut();
    rows.insert(
        "buffer:make-writable-unique-ret".to_string(),
        vec![
            "0".to_string(),
            bool_field(std::ptr::eq(unique_before, unique.as_ptr())),
        ],
    );
    rows.insert(
        "buffer:make-writable-unique".to_string(),
        buffer_fields(&unique),
    );

    let shared_src = BufferRef::from_vec(vec![9, 8, 7]);
    let mut shared_dst = shared_src.clone();
    shared_dst.make_mut();
    rows.insert(
        "buffer:make-writable-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:make-writable-shared-src".to_string(),
        buffer_fields(&shared_src),
    );
    rows.insert(
        "buffer:make-writable-shared-dst".to_string(),
        buffer_fields(&shared_dst),
    );
    rows.insert(
        "buffer:make-writable-shared-shares".to_string(),
        vec![bool_field(shared_src.shares_storage(&shared_dst))],
    );

    let zero_shared_src = BufferRef::zeroed(0).unwrap();
    let mut zero_shared_dst = BufferRef::ref_from(&zero_shared_src);
    zero_shared_dst.make_mut();
    rows.insert(
        "buffer:make-writable-zero-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:make-writable-zero-shared-src".to_string(),
        buffer_fields(&zero_shared_src),
    );
    rows.insert(
        "buffer:make-writable-zero-shared-dst".to_string(),
        buffer_fields(&zero_shared_dst),
    );
    rows.insert(
        "buffer:make-writable-zero-shared-shares".to_string(),
        vec![
            bool_field(zero_shared_src.shares_storage(&zero_shared_dst)),
            zero_shared_src.strong_count().to_string(),
        ],
    );

    let released = Arc::new(Mutex::new(Vec::<usize>::new()));
    let capture = Arc::clone(&released);
    let mut readonly = BufferRef::from_external_slice_with_opaque_readonly(
        vec![5, 6, 7].into(),
        77usize,
        move |opaque| {
            capture.lock().unwrap().push(opaque);
        },
    );
    rows.insert(
        "buffer:readonly".to_string(),
        buffer_fields_with_opaque(&readonly),
    );
    readonly.make_mut();
    let released_values = released.lock().unwrap();
    rows.insert(
        "buffer:readonly-make-writable-ret".to_string(),
        vec![
            "0".to_string(),
            released_values.len().to_string(),
            released_values[0].to_string(),
        ],
    );
    drop(released_values);
    rows.insert(
        "buffer:readonly-after".to_string(),
        buffer_fields_with_opaque(&readonly),
    );

    let create_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_capture = Arc::clone(&create_released);
    let create = BufferRef::from_vec_with_opaque_release_callback(
        vec![31, 32, 33],
        123usize,
        move |opaque, bytes| {
            create_capture.lock().unwrap().push((opaque, bytes));
        },
    );
    rows.insert(
        "buffer:create-writable".to_string(),
        buffer_fields_with_opaque(&create),
    );
    drop(create);
    rows.insert(
        "buffer:create-writable-release".to_string(),
        release_fields(&create_released),
    );

    let create_zero_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_zero_capture = Arc::clone(&create_zero_released);
    let create_zero = BufferRef::from_vec_with_opaque_release_callback(
        Vec::new(),
        321usize,
        move |opaque, bytes| {
            create_zero_capture.lock().unwrap().push((opaque, bytes));
        },
    );
    rows.insert(
        "buffer:create-zero".to_string(),
        buffer_fields_with_opaque(&create_zero),
    );
    drop(create_zero);
    rows.insert(
        "buffer:create-zero-release".to_string(),
        release_fields(&create_zero_released),
    );

    let mut create_default_opaque = BufferRef::from_vec_with_opaque(vec![34, 35, 36], 322usize);
    rows.insert(
        "buffer:create-default-opaque".to_string(),
        buffer_fields_with_opaque(&create_default_opaque),
    );
    create_default_opaque.make_mut();
    rows.insert(
        "buffer:create-default-opaque-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-default-opaque-make-writable".to_string(),
        buffer_fields_with_opaque(&create_default_opaque),
    );

    let create_default_shared_src = BufferRef::from_vec_with_opaque(vec![37, 38, 39], 323usize);
    let mut create_default_shared_dst = BufferRef::ref_from(&create_default_shared_src);
    create_default_shared_dst.make_mut();
    rows.insert(
        "buffer:create-default-opaque-shared-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-default-opaque-shared-src".to_string(),
        buffer_fields_with_opaque(&create_default_shared_src),
    );
    rows.insert(
        "buffer:create-default-opaque-shared-dst".to_string(),
        buffer_fields_with_opaque(&create_default_shared_dst),
    );
    rows.insert(
        "buffer:create-default-opaque-shared-shares".to_string(),
        vec![bool_field(
            create_default_shared_src.shares_storage(&create_default_shared_dst),
        )],
    );

    let mut create_default_readonly =
        BufferRef::from_vec_with_opaque_readonly(vec![40, 41, 42], 324usize);
    create_default_readonly.make_mut();
    rows.insert(
        "buffer:create-default-opaque-readonly-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-default-opaque-readonly-after".to_string(),
        buffer_fields_with_opaque(&create_default_readonly),
    );

    let mut create_default_realloc =
        Some(BufferRef::from_vec_with_opaque(vec![43, 44, 45], 325usize));
    let create_default_realloc_before = create_default_realloc.as_ref().unwrap().as_ptr();
    BufferRef::realloc(&mut create_default_realloc, 5).unwrap();
    let create_default_realloc = create_default_realloc.expect("realloc keeps destination");
    rows.insert(
        "buffer:create-default-opaque-realloc-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-default-opaque-realloc".to_string(),
        buffer_prefix_fields(&create_default_realloc, 3),
    );
    rows.insert(
        "buffer:create-default-opaque-realloc-opaque".to_string(),
        vec![create_default_realloc
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:create-default-opaque-realloc-replaced".to_string(),
        vec![bool_field(!std::ptr::eq(
            create_default_realloc_before,
            create_default_realloc.as_ptr(),
        ))],
    );

    let create_zero_readonly_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_zero_readonly_capture = Arc::clone(&create_zero_readonly_released);
    let mut create_zero_readonly = BufferRef::from_vec_with_opaque_release_callback_readonly(
        Vec::new(),
        654usize,
        move |opaque, bytes| {
            create_zero_readonly_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    rows.insert(
        "buffer:create-zero-readonly".to_string(),
        buffer_fields_with_opaque(&create_zero_readonly),
    );
    create_zero_readonly.make_mut();
    rows.insert(
        "buffer:create-zero-readonly-make-writable-ret".to_string(),
        vec![
            "0".to_string(),
            create_zero_readonly_released
                .lock()
                .unwrap()
                .len()
                .to_string(),
        ],
    );
    rows.insert(
        "buffer:create-zero-readonly-after".to_string(),
        buffer_fields_with_opaque(&create_zero_readonly),
    );
    rows.insert(
        "buffer:create-zero-readonly-release".to_string(),
        release_fields(&create_zero_readonly_released),
    );

    let create_readonly_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_readonly_capture = Arc::clone(&create_readonly_released);
    let mut create_readonly = BufferRef::from_vec_with_opaque_release_callback_readonly(
        vec![21, 22, 23],
        432usize,
        move |opaque, bytes| {
            create_readonly_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    rows.insert(
        "buffer:create-readonly".to_string(),
        buffer_fields_with_opaque(&create_readonly),
    );
    create_readonly.make_mut();
    rows.insert(
        "buffer:create-readonly-make-writable-ret".to_string(),
        vec![
            "0".to_string(),
            create_readonly_released.lock().unwrap().len().to_string(),
        ],
    );
    rows.insert(
        "buffer:create-readonly-after".to_string(),
        buffer_fields_with_opaque(&create_readonly),
    );
    rows.insert(
        "buffer:create-readonly-release".to_string(),
        release_fields(&create_readonly_released),
    );

    let create_readonly_shared_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_readonly_shared_capture = Arc::clone(&create_readonly_shared_released);
    let create_readonly_shared_src = BufferRef::from_vec_with_opaque_release_callback_readonly(
        vec![24, 25, 26],
        433usize,
        move |opaque, bytes| {
            create_readonly_shared_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    let mut create_readonly_shared_dst = BufferRef::ref_from(&create_readonly_shared_src);
    rows.insert(
        "buffer:create-readonly-shared-ref-src".to_string(),
        buffer_fields_with_opaque(&create_readonly_shared_src),
    );
    rows.insert(
        "buffer:create-readonly-shared-ref-dst".to_string(),
        buffer_fields_with_opaque(&create_readonly_shared_dst),
    );
    rows.insert(
        "buffer:create-readonly-shared-ref-shares".to_string(),
        vec![
            bool_field(create_readonly_shared_src.shares_storage(&create_readonly_shared_dst)),
            create_readonly_shared_src.strong_count().to_string(),
        ],
    );
    create_readonly_shared_dst.make_mut();
    rows.insert(
        "buffer:create-readonly-shared-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-readonly-shared-src".to_string(),
        buffer_fields_with_opaque(&create_readonly_shared_src),
    );
    rows.insert(
        "buffer:create-readonly-shared-dst".to_string(),
        buffer_fields_with_opaque(&create_readonly_shared_dst),
    );
    rows.insert(
        "buffer:create-readonly-shared-shares".to_string(),
        vec![bool_field(
            create_readonly_shared_src.shares_storage(&create_readonly_shared_dst),
        )],
    );
    drop(create_readonly_shared_dst);
    rows.insert(
        "buffer:create-readonly-shared-release-before-src-drop".to_string(),
        vec![create_readonly_shared_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(create_readonly_shared_src);
    rows.insert(
        "buffer:create-readonly-shared-release".to_string(),
        release_fields(&create_readonly_shared_released),
    );

    let create_shared_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_shared_capture = Arc::clone(&create_shared_released);
    let create_shared_src = BufferRef::from_vec_with_opaque_release_callback(
        vec![40, 41, 42],
        456usize,
        move |opaque, bytes| {
            create_shared_capture.lock().unwrap().push((opaque, bytes));
        },
    );
    let mut create_shared_dst = create_shared_src.clone();
    rows.insert(
        "buffer:create-shared-ref-src".to_string(),
        buffer_fields_with_opaque(&create_shared_src),
    );
    rows.insert(
        "buffer:create-shared-ref-dst".to_string(),
        buffer_fields_with_opaque(&create_shared_dst),
    );
    rows.insert(
        "buffer:create-shared-ref-shares".to_string(),
        vec![
            bool_field(create_shared_src.shares_storage(&create_shared_dst)),
            create_shared_src.strong_count().to_string(),
        ],
    );
    create_shared_dst.make_mut();
    rows.insert(
        "buffer:create-shared-make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-shared-src".to_string(),
        buffer_fields_with_opaque(&create_shared_src),
    );
    rows.insert(
        "buffer:create-shared-dst".to_string(),
        buffer_fields_with_opaque(&create_shared_dst),
    );
    rows.insert(
        "buffer:create-shared-shares".to_string(),
        vec![bool_field(
            create_shared_src.shares_storage(&create_shared_dst),
        )],
    );
    drop(create_shared_dst);
    rows.insert(
        "buffer:create-shared-release-before-src-drop".to_string(),
        vec![create_shared_released.lock().unwrap().len().to_string()],
    );
    drop(create_shared_src);
    rows.insert(
        "buffer:create-shared-release".to_string(),
        release_fields(&create_shared_released),
    );

    let create_shared_realloc_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_shared_realloc_capture = Arc::clone(&create_shared_realloc_released);
    let create_shared_realloc_src = BufferRef::from_vec_with_opaque_release_callback(
        vec![44, 45, 46],
        567usize,
        move |opaque, bytes| {
            create_shared_realloc_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    let mut create_shared_realloc_dst = Some(BufferRef::ref_from(&create_shared_realloc_src));
    BufferRef::realloc(&mut create_shared_realloc_dst, 5).unwrap();
    let create_shared_realloc_dst =
        create_shared_realloc_dst.expect("create shared realloc result");
    rows.insert(
        "buffer:create-shared-realloc-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-shared-realloc-src".to_string(),
        buffer_fields_with_opaque(&create_shared_realloc_src),
    );
    rows.insert(
        "buffer:create-shared-realloc-dst".to_string(),
        buffer_prefix_fields(&create_shared_realloc_dst, 3),
    );
    rows.insert(
        "buffer:create-shared-realloc-dst-opaque".to_string(),
        vec![create_shared_realloc_dst
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:create-shared-realloc-shares".to_string(),
        vec![bool_field(
            create_shared_realloc_src.shares_storage(&create_shared_realloc_dst),
        )],
    );
    rows.insert(
        "buffer:create-shared-realloc-release-before-src-drop".to_string(),
        vec![create_shared_realloc_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(create_shared_realloc_dst);
    rows.insert(
        "buffer:create-shared-realloc-release-before-src-unref".to_string(),
        vec![create_shared_realloc_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(create_shared_realloc_src);
    rows.insert(
        "buffer:create-shared-realloc-release".to_string(),
        release_fields(&create_shared_realloc_released),
    );

    let create_shared_shrink_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_shared_shrink_capture = Arc::clone(&create_shared_shrink_released);
    let create_shared_shrink_src = BufferRef::from_vec_with_opaque_release_callback(
        vec![47, 48, 49, 50],
        568usize,
        move |opaque, bytes| {
            create_shared_shrink_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    let mut create_shared_shrink_dst = Some(BufferRef::ref_from(&create_shared_shrink_src));
    BufferRef::realloc(&mut create_shared_shrink_dst, 2).unwrap();
    let create_shared_shrink_dst = create_shared_shrink_dst.expect("create shared shrink result");
    rows.insert(
        "buffer:create-shared-shrink-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-shared-shrink-src".to_string(),
        buffer_fields_with_opaque(&create_shared_shrink_src),
    );
    rows.insert(
        "buffer:create-shared-shrink-dst".to_string(),
        buffer_prefix_fields(&create_shared_shrink_dst, 2),
    );
    rows.insert(
        "buffer:create-shared-shrink-dst-opaque".to_string(),
        vec![create_shared_shrink_dst
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:create-shared-shrink-shares".to_string(),
        vec![bool_field(
            create_shared_shrink_src.shares_storage(&create_shared_shrink_dst),
        )],
    );
    rows.insert(
        "buffer:create-shared-shrink-release-before-src-drop".to_string(),
        vec![create_shared_shrink_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(create_shared_shrink_dst);
    rows.insert(
        "buffer:create-shared-shrink-release-before-src-unref".to_string(),
        vec![create_shared_shrink_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(create_shared_shrink_src);
    rows.insert(
        "buffer:create-shared-shrink-release".to_string(),
        release_fields(&create_shared_shrink_released),
    );

    let create_realloc_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_realloc_capture = Arc::clone(&create_realloc_released);
    let mut create_realloc = Some(BufferRef::from_vec_with_opaque_release_callback(
        vec![50, 51, 52],
        789usize,
        move |opaque, bytes| {
            create_realloc_capture.lock().unwrap().push((opaque, bytes));
        },
    ));
    let create_realloc_before = create_realloc
        .as_ref()
        .expect("create realloc input")
        .as_ptr();
    BufferRef::realloc(&mut create_realloc, 5).unwrap();
    let create_realloc = create_realloc.expect("create realloc result");
    rows.insert(
        "buffer:create-realloc-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-realloc".to_string(),
        buffer_prefix_fields(&create_realloc, 3),
    );
    rows.insert(
        "buffer:create-realloc-opaque".to_string(),
        vec![create_realloc
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:create-realloc-replaced".to_string(),
        vec![bool_field(create_realloc_before != create_realloc.as_ptr())],
    );
    rows.insert(
        "buffer:create-realloc-release".to_string(),
        release_fields(&create_realloc_released),
    );

    let create_realloc_shrink_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_realloc_shrink_capture = Arc::clone(&create_realloc_shrink_released);
    let mut create_realloc_shrink = Some(BufferRef::from_vec_with_opaque_release_callback(
        vec![53, 54, 55, 56],
        790usize,
        move |opaque, bytes| {
            create_realloc_shrink_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    ));
    let create_realloc_shrink_before = create_realloc_shrink
        .as_ref()
        .expect("create shrink realloc input")
        .as_ptr();
    BufferRef::realloc(&mut create_realloc_shrink, 2).unwrap();
    let create_realloc_shrink = create_realloc_shrink.expect("create shrink realloc result");
    rows.insert(
        "buffer:create-realloc-shrink-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-realloc-shrink".to_string(),
        buffer_prefix_fields(&create_realloc_shrink, 2),
    );
    rows.insert(
        "buffer:create-realloc-shrink-opaque".to_string(),
        vec![create_realloc_shrink
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:create-realloc-shrink-replaced".to_string(),
        vec![bool_field(
            create_realloc_shrink_before != create_realloc_shrink.as_ptr(),
        )],
    );
    rows.insert(
        "buffer:create-realloc-shrink-release-before-unref".to_string(),
        release_fields(&create_realloc_shrink_released),
    );
    drop(create_realloc_shrink);
    rows.insert(
        "buffer:create-realloc-shrink-release-after-unref".to_string(),
        release_fields(&create_realloc_shrink_released),
    );

    let mut grow = Some(BufferRef::from_vec(vec![1, 2, 3]));
    let grow_data_before = grow.as_ref().expect("grow input").as_ptr();
    BufferRef::realloc(&mut grow, 5).unwrap();
    let grow = grow.expect("grow realloc result");
    rows.insert("buffer:realloc-grow-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "buffer:realloc-grow".to_string(),
        buffer_prefix_fields(&grow, 3),
    );
    rows.insert(
        "buffer:realloc-grow-replaced".to_string(),
        vec![bool_field(grow_data_before != grow.as_ptr())],
    );
    let mut grow = Some(grow);
    BufferRef::realloc(&mut grow, 2).unwrap();
    let grow = grow.expect("shrink realloc result");
    rows.insert(
        "buffer:realloc-shrink-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert("buffer:realloc-shrink".to_string(), buffer_fields(&grow));

    let mut realloc_zero = Some(BufferRef::from_vec(vec![9, 10, 11]));
    BufferRef::realloc(&mut realloc_zero, 0).unwrap();
    rows.insert("buffer:realloc-zero-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "buffer:realloc-zero".to_string(),
        buffer_status_fields(realloc_zero.as_ref().expect("zero realloc result")),
    );

    let realloc_src = BufferRef::from_vec(vec![7, 7, 7]);
    let mut realloc_dst = realloc_src.clone();
    realloc_dst.resize(5).unwrap();
    rows.insert(
        "buffer:realloc-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:realloc-shared-src".to_string(),
        buffer_fields(&realloc_src),
    );
    rows.insert(
        "buffer:realloc-shared-dst".to_string(),
        buffer_prefix_fields(&realloc_dst, 3),
    );
    rows.insert(
        "buffer:realloc-shared-shares".to_string(),
        vec![bool_field(realloc_src.shares_storage(&realloc_dst))],
    );

    let realloc_same_src = BufferRef::from_vec(vec![4, 6, 8]);
    let mut realloc_same_dst = Some(BufferRef::ref_from(&realloc_same_src));
    BufferRef::realloc(&mut realloc_same_dst, realloc_same_src.len()).unwrap();
    let realloc_same_dst = realloc_same_dst.expect("same-size realloc result");
    rows.insert(
        "buffer:realloc-same-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:realloc-same-shared-src".to_string(),
        buffer_fields(&realloc_same_src),
    );
    rows.insert(
        "buffer:realloc-same-shared-dst".to_string(),
        buffer_fields(&realloc_same_dst),
    );
    rows.insert(
        "buffer:realloc-same-shared-shares".to_string(),
        vec![
            bool_field(realloc_same_src.shares_storage(&realloc_same_dst)),
            realloc_same_src.strong_count().to_string(),
        ],
    );

    let create_realloc_same_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let create_realloc_same_capture = Arc::clone(&create_realloc_same_released);
    let mut create_realloc_same = Some(BufferRef::from_vec_with_opaque_release_callback(
        vec![60, 61, 62],
        654usize,
        move |opaque, bytes| {
            create_realloc_same_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    ));
    let create_realloc_same_ptr = create_realloc_same
        .as_ref()
        .expect("create same realloc input")
        .as_ptr();
    BufferRef::realloc(&mut create_realloc_same, 3).unwrap();
    let create_realloc_same = create_realloc_same.expect("create same realloc result");
    rows.insert(
        "buffer:create-realloc-same-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:create-realloc-same".to_string(),
        buffer_fields_with_opaque(&create_realloc_same),
    );
    rows.insert(
        "buffer:create-realloc-same-sameptr".to_string(),
        vec![bool_field(
            create_realloc_same_ptr == create_realloc_same.as_ptr(),
        )],
    );
    rows.insert(
        "buffer:create-realloc-same-release-before-unref".to_string(),
        vec![create_realloc_same_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(create_realloc_same);
    rows.insert(
        "buffer:create-realloc-same-release".to_string(),
        release_fields(&create_realloc_same_released),
    );

    let readonly_realloc_same_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_realloc_same_capture = Arc::clone(&readonly_realloc_same_released);
    let mut readonly_realloc_same =
        Some(BufferRef::from_vec_with_opaque_release_callback_readonly(
            vec![70, 71, 72],
            88usize,
            move |opaque, bytes| {
                readonly_realloc_same_capture
                    .lock()
                    .unwrap()
                    .push((opaque, bytes));
            },
        ));
    let readonly_realloc_same_ptr = readonly_realloc_same
        .as_ref()
        .expect("readonly same realloc input")
        .as_ptr();
    BufferRef::realloc(&mut readonly_realloc_same, 3).unwrap();
    let readonly_realloc_same = readonly_realloc_same.expect("readonly same realloc result");
    rows.insert(
        "buffer:readonly-realloc-same-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:readonly-realloc-same".to_string(),
        buffer_fields_with_opaque(&readonly_realloc_same),
    );
    rows.insert(
        "buffer:readonly-realloc-same-sameptr".to_string(),
        vec![bool_field(
            readonly_realloc_same_ptr == readonly_realloc_same.as_ptr(),
        )],
    );
    rows.insert(
        "buffer:readonly-realloc-same-release-before-unref".to_string(),
        vec![readonly_realloc_same_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(readonly_realloc_same);
    rows.insert(
        "buffer:readonly-realloc-same-release".to_string(),
        release_fields(&readonly_realloc_same_released),
    );

    let readonly_realloc_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_realloc_capture = Arc::clone(&readonly_realloc_released);
    let mut readonly_realloc = Some(BufferRef::from_vec_with_opaque_release_callback_readonly(
        vec![90, 91, 92],
        889usize,
        move |opaque, bytes| {
            readonly_realloc_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    ));
    let readonly_realloc_ptr = readonly_realloc
        .as_ref()
        .expect("readonly realloc input")
        .as_ptr();
    BufferRef::realloc(&mut readonly_realloc, 5).unwrap();
    let readonly_realloc = readonly_realloc.expect("readonly realloc result");
    rows.insert(
        "buffer:readonly-realloc-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:readonly-realloc".to_string(),
        buffer_prefix_fields(&readonly_realloc, 3),
    );
    rows.insert(
        "buffer:readonly-realloc-opaque".to_string(),
        vec![readonly_realloc
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:readonly-realloc-replaced".to_string(),
        vec![bool_field(
            readonly_realloc_ptr != readonly_realloc.as_ptr(),
        )],
    );
    rows.insert(
        "buffer:readonly-realloc-release-before-unref".to_string(),
        release_fields(&readonly_realloc_released),
    );
    drop(readonly_realloc);
    rows.insert(
        "buffer:readonly-realloc-release-after-unref".to_string(),
        release_fields(&readonly_realloc_released),
    );

    let readonly_realloc_shrink_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_realloc_shrink_capture = Arc::clone(&readonly_realloc_shrink_released);
    let mut readonly_realloc_shrink =
        Some(BufferRef::from_vec_with_opaque_release_callback_readonly(
            vec![93, 94, 95, 96],
            890usize,
            move |opaque, bytes| {
                readonly_realloc_shrink_capture
                    .lock()
                    .unwrap()
                    .push((opaque, bytes));
            },
        ));
    let readonly_realloc_shrink_ptr = readonly_realloc_shrink
        .as_ref()
        .expect("readonly shrink realloc input")
        .as_ptr();
    BufferRef::realloc(&mut readonly_realloc_shrink, 2).unwrap();
    let readonly_realloc_shrink = readonly_realloc_shrink.expect("readonly shrink realloc result");
    rows.insert(
        "buffer:readonly-realloc-shrink-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:readonly-realloc-shrink".to_string(),
        buffer_prefix_fields(&readonly_realloc_shrink, 2),
    );
    rows.insert(
        "buffer:readonly-realloc-shrink-opaque".to_string(),
        vec![readonly_realloc_shrink
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:readonly-realloc-shrink-replaced".to_string(),
        vec![bool_field(
            readonly_realloc_shrink_ptr != readonly_realloc_shrink.as_ptr(),
        )],
    );
    rows.insert(
        "buffer:readonly-realloc-shrink-release-before-unref".to_string(),
        release_fields(&readonly_realloc_shrink_released),
    );
    drop(readonly_realloc_shrink);
    rows.insert(
        "buffer:readonly-realloc-shrink-release-after-unref".to_string(),
        release_fields(&readonly_realloc_shrink_released),
    );

    let readonly_shared_realloc_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_shared_realloc_capture = Arc::clone(&readonly_shared_realloc_released);
    let readonly_shared_realloc_src = BufferRef::from_vec_with_opaque_release_callback_readonly(
        vec![80, 81, 82],
        998usize,
        move |opaque, bytes| {
            readonly_shared_realloc_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    let mut readonly_shared_realloc_dst = Some(BufferRef::ref_from(&readonly_shared_realloc_src));
    BufferRef::realloc(&mut readonly_shared_realloc_dst, 5).unwrap();
    let readonly_shared_realloc_dst =
        readonly_shared_realloc_dst.expect("readonly shared realloc result");
    rows.insert(
        "buffer:readonly-shared-realloc-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:readonly-shared-realloc-src".to_string(),
        buffer_fields_with_opaque(&readonly_shared_realloc_src),
    );
    rows.insert(
        "buffer:readonly-shared-realloc-dst".to_string(),
        buffer_prefix_fields(&readonly_shared_realloc_dst, 3),
    );
    rows.insert(
        "buffer:readonly-shared-realloc-dst-opaque".to_string(),
        vec![readonly_shared_realloc_dst
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:readonly-shared-realloc-shares".to_string(),
        vec![bool_field(
            readonly_shared_realloc_src.shares_storage(&readonly_shared_realloc_dst),
        )],
    );
    rows.insert(
        "buffer:readonly-shared-realloc-release-before-src-drop".to_string(),
        vec![readonly_shared_realloc_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(readonly_shared_realloc_dst);
    rows.insert(
        "buffer:readonly-shared-realloc-release-before-src-unref".to_string(),
        vec![readonly_shared_realloc_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(readonly_shared_realloc_src);
    rows.insert(
        "buffer:readonly-shared-realloc-release".to_string(),
        release_fields(&readonly_shared_realloc_released),
    );

    let readonly_shared_shrink_released = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_shared_shrink_capture = Arc::clone(&readonly_shared_shrink_released);
    let readonly_shared_shrink_src = BufferRef::from_vec_with_opaque_release_callback_readonly(
        vec![83, 84, 85, 86],
        1002usize,
        move |opaque, bytes| {
            readonly_shared_shrink_capture
                .lock()
                .unwrap()
                .push((opaque, bytes));
        },
    );
    let mut readonly_shared_shrink_dst = Some(BufferRef::ref_from(&readonly_shared_shrink_src));
    BufferRef::realloc(&mut readonly_shared_shrink_dst, 2).unwrap();
    let readonly_shared_shrink_dst =
        readonly_shared_shrink_dst.expect("readonly shared shrink result");
    rows.insert(
        "buffer:readonly-shared-shrink-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:readonly-shared-shrink-src".to_string(),
        buffer_fields_with_opaque(&readonly_shared_shrink_src),
    );
    rows.insert(
        "buffer:readonly-shared-shrink-dst".to_string(),
        buffer_prefix_fields(&readonly_shared_shrink_dst, 2),
    );
    rows.insert(
        "buffer:readonly-shared-shrink-dst-opaque".to_string(),
        vec![readonly_shared_shrink_dst
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string()],
    );
    rows.insert(
        "buffer:readonly-shared-shrink-shares".to_string(),
        vec![bool_field(
            readonly_shared_shrink_src.shares_storage(&readonly_shared_shrink_dst),
        )],
    );
    rows.insert(
        "buffer:readonly-shared-shrink-release-before-src-drop".to_string(),
        vec![readonly_shared_shrink_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(readonly_shared_shrink_dst);
    rows.insert(
        "buffer:readonly-shared-shrink-release-before-src-unref".to_string(),
        vec![readonly_shared_shrink_released
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    drop(readonly_shared_shrink_src);
    rows.insert(
        "buffer:readonly-shared-shrink-release".to_string(),
        release_fields(&readonly_shared_shrink_released),
    );

    let replace_src = BufferRef::from_vec(vec![3, 4, 5]);
    let replace_dst = replace_src.clone();
    rows.insert("buffer:replace-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "buffer:replace-src".to_string(),
        buffer_fields(&replace_src),
    );
    rows.insert(
        "buffer:replace-dst".to_string(),
        buffer_fields(&replace_dst),
    );
    rows.insert(
        "buffer:replace-shares".to_string(),
        vec![bool_field(replace_src.shares_storage(&replace_dst))],
    );

    let mut replace_null_src = Some(BufferRef::from_vec(vec![8, 9]));
    BufferRef::replace(&mut replace_null_src, None);
    rows.insert(
        "buffer:replace-null-src".to_string(),
        vec!["0".to_string(), bool_field(replace_null_src.is_none())],
    );

    let mut replace_null_null = None;
    BufferRef::replace(&mut replace_null_null, None);
    rows.insert(
        "buffer:replace-null-null".to_string(),
        vec!["0".to_string(), bool_field(replace_null_null.is_none())],
    );

    let replace_null_source = BufferRef::from_vec(vec![6, 7, 8]);
    let mut replace_null_dst = None;
    BufferRef::replace(&mut replace_null_dst, Some(&replace_null_source));
    let replace_null_dst = replace_null_dst.expect("replace into null dst");
    rows.insert(
        "buffer:replace-null-dst".to_string(),
        buffer_fields(&replace_null_dst),
    );
    rows.insert(
        "buffer:replace-null-dst-shares".to_string(),
        vec![bool_field(
            replace_null_source.shares_storage(&replace_null_dst),
        )],
    );

    let replace_equiv_src = BufferRef::from_vec(vec![1, 4, 9]);
    let mut replace_equiv_dst = Some(BufferRef::ref_from(&replace_equiv_src));
    BufferRef::replace(&mut replace_equiv_dst, Some(&replace_equiv_src));
    let replace_equiv_dst = replace_equiv_dst.expect("replace equivalent dst");
    rows.insert(
        "buffer:replace-equivalent-ret".to_string(),
        vec![
            "0".to_string(),
            replace_equiv_src.strong_count().to_string(),
            bool_field(replace_equiv_src.shares_storage(&replace_equiv_dst)),
        ],
    );

    let mut replace_self = Some(BufferRef::from_vec(vec![2, 4, 6]));
    let replace_self_before = replace_self.as_ref().expect("self replace input").as_ptr();
    let replace_self_source = BufferRef::ref_from(replace_self.as_ref().expect("self replace ref"));
    BufferRef::replace(&mut replace_self, Some(&replace_self_source));
    drop(replace_self_source);
    let replace_self = replace_self.expect("self replace keeps destination");
    rows.insert(
        "buffer:replace-self-ret".to_string(),
        vec![
            "0".to_string(),
            bool_field(std::ptr::eq(replace_self_before, replace_self.as_ptr())),
        ],
    );
    rows.insert(
        "buffer:replace-self".to_string(),
        buffer_fields(&replace_self),
    );

    let mut unref_null_input = None;
    BufferRef::unref(&mut unref_null_input);
    rows.insert(
        "buffer:unref-null-input".to_string(),
        vec![bool_field(unref_null_input.is_none())],
    );

    let mut realloc_null = None;
    BufferRef::realloc(&mut realloc_null, 4).unwrap();
    rows.insert("buffer:realloc-null-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "buffer:realloc-null".to_string(),
        buffer_status_fields(realloc_null.as_ref().expect("realloc null result")),
    );
    let mut realloc_null = realloc_null;
    BufferRef::realloc(&mut realloc_null, 6).unwrap();
    let realloc_null = realloc_null.expect("nullable realloc grow result");
    rows.insert(
        "buffer:realloc-null-grow-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:realloc-null-grow".to_string(),
        buffer_status_fields(&realloc_null),
    );

    let mut realloc_null_zero = None;
    BufferRef::realloc(&mut realloc_null_zero, 0).unwrap();
    rows.insert(
        "buffer:realloc-null-zero-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:realloc-null-zero".to_string(),
        buffer_status_fields(
            realloc_null_zero
                .as_ref()
                .expect("nullable zero realloc result"),
        ),
    );

    let mut realloc_invalid = Some(BufferRef::from_vec(vec![91, 92, 93]));
    let realloc_invalid_err = BufferRef::realloc(&mut realloc_invalid, usize::MAX).unwrap_err();
    rows.insert(
        "buffer:realloc-invalid-huge-ret".to_string(),
        vec![realloc_invalid_err
            .code()
            .expect("huge realloc maps to ENOMEM")
            .raw()
            .to_string()],
    );
    rows.insert(
        "buffer:realloc-invalid-huge".to_string(),
        buffer_fields(
            realloc_invalid
                .as_ref()
                .expect("huge realloc preserves dst"),
        ),
    );

    let mut realloc_null_invalid = None;
    let realloc_null_invalid_err =
        BufferRef::realloc(&mut realloc_null_invalid, usize::MAX).unwrap_err();
    rows.insert(
        "buffer:realloc-null-invalid-huge".to_string(),
        vec![
            realloc_null_invalid_err
                .code()
                .expect("huge null realloc maps to ENOMEM")
                .raw()
                .to_string(),
            bool_field(realloc_null_invalid.is_none()),
        ],
    );

    let offset_src = BufferRef::from_vec(vec![10, 11, 12, 13]);
    let offset_ref = offset_src.ref_slice(1, 2).unwrap();
    rows.insert(
        "buffer:offset-ref-src".to_string(),
        buffer_fields(&offset_src),
    );
    rows.insert(
        "buffer:offset-ref-view".to_string(),
        buffer_fields(&offset_ref),
    );
    rows.insert(
        "buffer:offset-ref-delta".to_string(),
        vec![((offset_ref.as_ptr() as usize) - (offset_src.as_ptr() as usize)).to_string()],
    );

    {
        let offset_ref_clone = BufferRef::ref_from(&offset_ref);
        rows.insert(
            "buffer:offset-ref-clone".to_string(),
            buffer_fields(&offset_ref_clone),
        );
        rows.insert(
            "buffer:offset-ref-clone-shape".to_string(),
            vec![
                bool_field(offset_ref_clone.shares_storage(&offset_ref)),
                bool_field(offset_ref_clone.as_ptr() == offset_ref.as_ptr()),
                ((offset_ref_clone.as_ptr() as usize) - (offset_src.as_ptr() as usize)).to_string(),
                offset_ref.strong_count().to_string(),
            ],
        );
    }

    let mut offset_make_writable = offset_ref.clone();
    offset_make_writable.make_mut();
    rows.insert(
        "buffer:offset-make-writable".to_string(),
        buffer_fields(&offset_make_writable),
    );
    rows.insert(
        "buffer:offset-make-writable-shares".to_string(),
        vec![bool_field(offset_make_writable.shares_storage(&offset_ref))],
    );

    let mut offset_realloc = offset_src.ref_slice(1, 2).unwrap();
    offset_realloc.resize(3).unwrap();
    rows.insert(
        "buffer:offset-realloc-grow".to_string(),
        buffer_prefix_fields(&offset_realloc, 2),
    );
    rows.insert(
        "buffer:offset-realloc-shares".to_string(),
        vec![bool_field(offset_realloc.shares_storage(&offset_src))],
    );

    let offset_unique_base = BufferRef::from_vec(vec![30, 31, 32, 33]);
    let offset_unique_base_ptr = offset_unique_base.as_ptr() as usize;
    let mut offset_unique_make_writable = offset_unique_base.ref_slice(1, 2).unwrap();
    drop(offset_unique_base);
    let offset_unique_before = offset_unique_make_writable.as_ptr();
    offset_unique_make_writable.make_mut();
    rows.insert(
        "buffer:offset-unique-make-writable-ret".to_string(),
        vec![
            "0".to_string(),
            bool_field(offset_unique_before == offset_unique_make_writable.as_ptr()),
            ((offset_unique_make_writable.as_ptr() as usize) - offset_unique_base_ptr).to_string(),
        ],
    );
    rows.insert(
        "buffer:offset-unique-make-writable".to_string(),
        buffer_fields(&offset_unique_make_writable),
    );

    let offset_unique_realloc_base = BufferRef::from_vec(vec![34, 35, 36, 37]);
    let mut offset_unique_realloc = Some(offset_unique_realloc_base.ref_slice(1, 2).unwrap());
    drop(offset_unique_realloc_base);
    let offset_unique_realloc_before = offset_unique_realloc
        .as_ref()
        .expect("unique offset realloc input")
        .as_ptr();
    BufferRef::realloc(&mut offset_unique_realloc, 3).unwrap();
    let offset_unique_realloc = offset_unique_realloc.expect("unique offset realloc result");
    rows.insert(
        "buffer:offset-unique-realloc-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:offset-unique-realloc".to_string(),
        buffer_prefix_fields(&offset_unique_realloc, 2),
    );
    rows.insert(
        "buffer:offset-unique-realloc-replaced".to_string(),
        vec![
            bool_field(offset_unique_realloc_before != offset_unique_realloc.as_ptr()),
            offset_unique_realloc.offset().to_string(),
        ],
    );

    let replace_offset_base = BufferRef::from_vec(vec![21, 22, 23, 24]);
    let replace_offset_src = replace_offset_base.ref_slice(1, 2).unwrap();
    let mut replace_offset_dst = Some(BufferRef::ref_from(&replace_offset_base));
    drop(replace_offset_base);
    BufferRef::replace(&mut replace_offset_dst, Some(&replace_offset_src));
    let replace_offset_dst = replace_offset_dst.expect("replace offset dst");
    rows.insert(
        "buffer:replace-offset-equivalent".to_string(),
        buffer_fields(&replace_offset_dst),
    );
    rows.insert(
        "buffer:replace-offset-equivalent-shares".to_string(),
        vec![
            bool_field(replace_offset_src.shares_storage(&replace_offset_dst)),
            replace_offset_src.strong_count().to_string(),
        ],
    );

    rows.insert("buffer:unref-null".to_string(), vec!["1".to_string()]);
    rows.insert(
        "buffer:unref-null-repeat".to_string(),
        vec!["1".to_string()],
    );

    let mut null_pool = None;
    BufferPool::uninit(&mut null_pool);
    rows.insert(
        "pool:uninit-null".to_string(),
        vec![bool_field(null_pool.is_none())],
    );

    let zero_pool = BufferPool::new(0, 0).unwrap();
    let zero_first = zero_pool.get().unwrap();
    rows.insert("pool-zero:first".to_string(), buffer_fields(&zero_first));
    rows.insert(
        "pool-zero:first-opaque".to_string(),
        vec![bool_field(zero_first.pool_opaque_ref::<usize>().is_none())],
    );
    drop(zero_first);
    let zero_reuse = zero_pool.get().unwrap();
    rows.insert("pool-zero:reuse".to_string(), buffer_fields(&zero_reuse));
    rows.insert(
        "pool-zero:reuse-opaque".to_string(),
        vec![bool_field(zero_reuse.pool_opaque_ref::<usize>().is_none())],
    );
    drop(zero_reuse);
    drop(zero_pool);

    let default_pool = BufferPool::new(3, 0).unwrap();
    let mut default_first = default_pool.get().unwrap();
    rows.insert(
        "pool-default:first-status".to_string(),
        buffer_status_fields(&default_first),
    );
    rows.insert(
        "pool-default:first-opaque".to_string(),
        vec![bool_field(
            default_first.pool_opaque_ref::<usize>().is_none(),
        )],
    );
    default_first
        .make_mut()
        .copy_from_slice(&[0x21, 0x22, 0x23]);
    drop(default_first);
    let default_reuse = default_pool.get().unwrap();
    rows.insert(
        "pool-default:reuse".to_string(),
        buffer_fields(&default_reuse),
    );
    rows.insert(
        "pool-default:reuse-opaque".to_string(),
        vec![bool_field(
            default_reuse.pool_opaque_ref::<usize>().is_none(),
        )],
    );
    drop(default_reuse);
    drop(default_pool);

    let init2_default_pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let init2_default_pool_free_capture = Arc::clone(&init2_default_pool_frees);
    let init2_default_pool = BufferPool::with_callbacks(
        2,
        0,
        BufferPoolCallbacks::default().with_pool_free(move || {
            init2_default_pool_free_capture.lock().unwrap().push(88);
        }),
    )
    .unwrap();
    let mut init2_default_first = init2_default_pool.get().unwrap();
    rows.insert(
        "pool-init2-default:first-status".to_string(),
        buffer_status_fields(&init2_default_first),
    );
    rows.insert(
        "pool-init2-default:first-opaque".to_string(),
        vec![bool_field(
            init2_default_first.pool_opaque_ref::<usize>().is_none(),
        )],
    );
    init2_default_first
        .make_mut()
        .copy_from_slice(&[0x31, 0x32]);
    drop(init2_default_first);
    let init2_default_reuse = init2_default_pool.get().unwrap();
    rows.insert(
        "pool-init2-default:reuse".to_string(),
        buffer_fields(&init2_default_reuse),
    );
    rows.insert(
        "pool-init2-default:reuse-opaque".to_string(),
        vec![bool_field(
            init2_default_reuse.pool_opaque_ref::<usize>().is_none(),
        )],
    );
    drop(init2_default_reuse);
    drop(init2_default_pool);
    let init2_default_pool_free_values = init2_default_pool_frees.lock().unwrap();
    rows.insert(
        "pool-init2-default:pool-free".to_string(),
        vec![
            init2_default_pool_free_values.len().to_string(),
            init2_default_pool_free_values[0].to_string(),
        ],
    );
    drop(init2_default_pool_free_values);

    let legacy_allocations = Arc::new(Mutex::new(Vec::<usize>::new()));
    let legacy_releases = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let legacy_allocate_capture = Arc::clone(&legacy_allocations);
    let legacy_release_capture = Arc::clone(&legacy_releases);
    let legacy_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::new(
            move |allocated_len| {
                legacy_allocate_capture.lock().unwrap().push(allocated_len);
                Ok(vec![0x51, 0x52, 0x53][..allocated_len].to_vec())
            },
            move |storage| {
                legacy_release_capture.lock().unwrap().push(storage);
            },
        ),
    )
    .unwrap();
    let mut legacy_first = legacy_pool.get().unwrap();
    rows.insert(
        "pool-legacy-custom:first".to_string(),
        buffer_fields(&legacy_first),
    );
    rows.insert(
        "pool-legacy-custom:first-opaque".to_string(),
        vec![bool_field(
            legacy_first.pool_opaque_ref::<usize>().is_none(),
        )],
    );
    legacy_first.make_mut().copy_from_slice(&[0xa1, 0xa2, 0xa3]);
    drop(legacy_first);
    let legacy_reuse = legacy_pool.get().unwrap();
    rows.insert(
        "pool-legacy-custom:reuse".to_string(),
        buffer_fields(&legacy_reuse),
    );
    rows.insert(
        "pool-legacy-custom:reuse-opaque".to_string(),
        vec![bool_field(
            legacy_reuse.pool_opaque_ref::<usize>().is_none(),
        )],
    );
    rows.insert(
        "pool-legacy-custom:reuse-allocs".to_string(),
        vec![legacy_allocations.lock().unwrap().len().to_string()],
    );
    drop(legacy_reuse);
    drop(legacy_pool);
    let legacy_release_values = legacy_releases.lock().unwrap();
    rows.insert(
        "pool-legacy-custom:uninit-release".to_string(),
        vec![
            legacy_release_values.len().to_string(),
            hex(&legacy_release_values[0]),
        ],
    );
    drop(legacy_release_values);

    let multi_spare_allocations = Arc::new(Mutex::new(0u8));
    let multi_spare_releases = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let multi_spare_allocate_capture = Arc::clone(&multi_spare_allocations);
    let multi_spare_release_capture = Arc::clone(&multi_spare_releases);
    let multi_spare_pool = BufferPool::with_callbacks(
        2,
        0,
        BufferPoolCallbacks::new(
            move |allocated_len| {
                let mut next = multi_spare_allocate_capture.lock().unwrap();
                let base = 0x70 + *next;
                *next += 1;
                Ok(vec![base; allocated_len])
            },
            move |storage| {
                multi_spare_release_capture.lock().unwrap().push(storage);
            },
        ),
    )
    .unwrap();
    let mut multi_spare_first = multi_spare_pool.get().unwrap();
    let mut multi_spare_second = multi_spare_pool.get().unwrap();
    multi_spare_first.make_mut().copy_from_slice(&[0xa1, 0xa2]);
    multi_spare_second.make_mut().copy_from_slice(&[0xb1, 0xb2]);
    drop(multi_spare_first);
    drop(multi_spare_second);
    rows.insert(
        "pool-multi-spare:after-drop".to_string(),
        vec![multi_spare_releases.lock().unwrap().len().to_string()],
    );
    let multi_spare_reuse_first = multi_spare_pool.get().unwrap();
    let multi_spare_reuse_second = multi_spare_pool.get().unwrap();
    rows.insert(
        "pool-multi-spare:reuse-first".to_string(),
        buffer_fields(&multi_spare_reuse_first),
    );
    rows.insert(
        "pool-multi-spare:reuse-second".to_string(),
        buffer_fields(&multi_spare_reuse_second),
    );
    rows.insert(
        "pool-multi-spare:reuse-allocs".to_string(),
        vec![multi_spare_allocations.lock().unwrap().to_string()],
    );
    drop(multi_spare_reuse_first);
    drop(multi_spare_reuse_second);
    drop(multi_spare_pool);
    let multi_spare_release_values = multi_spare_releases.lock().unwrap();
    rows.insert(
        "pool-multi-spare:uninit-releases".to_string(),
        vec![
            multi_spare_release_values.len().to_string(),
            hex(&multi_spare_release_values[0]),
            hex(&multi_spare_release_values[1]),
        ],
    );
    drop(multi_spare_release_values);

    struct PoolToken {
        id: usize,
        size: usize,
    }

    let allocations = Arc::new(Mutex::new(Vec::<usize>::new()));
    let releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let allocation_capture = Arc::clone(&allocations);
    let release_capture = Arc::clone(&releases);
    let pool_free_capture = Arc::clone(&pool_frees);
    let pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            move |allocated_len| {
                allocation_capture.lock().unwrap().push(allocated_len);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 55,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool token should be preserved");
                release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_free_capture.lock().unwrap().push(55);
        }),
    )
    .unwrap();
    let mut pool_first = pool.get().unwrap();
    rows.insert("pool:first".to_string(), buffer_fields(&pool_first));
    let first_token = pool_first
        .pool_opaque_ref::<PoolToken>()
        .expect("first pool token");
    rows.insert(
        "pool:opaque-first".to_string(),
        vec![first_token.id.to_string(), first_token.size.to_string()],
    );
    pool_first.make_mut().copy_from_slice(&[0xaa, 0xbb, 0xcc]);
    drop(pool_first);
    let pool_reuse = pool.get().unwrap();
    rows.insert("pool:reuse".to_string(), buffer_fields(&pool_reuse));
    let reuse_token = pool_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("reused pool token");
    rows.insert(
        "pool:opaque-reuse".to_string(),
        vec![reuse_token.id.to_string(), reuse_token.size.to_string()],
    );
    rows.insert(
        "pool:reuse-allocs".to_string(),
        vec![allocations.lock().unwrap().len().to_string()],
    );
    drop(pool_reuse);
    drop(pool);
    let release_values = releases.lock().unwrap();
    rows.insert(
        "pool:uninit-releases".to_string(),
        vec![
            release_values.len().to_string(),
            release_values[0].0.to_string(),
            hex(&release_values[0].1),
        ],
    );
    drop(release_values);
    rows.insert(
        "pool:uninit-pool-free".to_string(),
        vec![pool_frees.lock().unwrap().len().to_string()],
    );

    let pool_unique_writable_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_unique_writable_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_unique_writable_release_capture = Arc::clone(&pool_unique_writable_releases);
    let pool_unique_writable_free_capture = Arc::clone(&pool_unique_writable_frees);
    let pool_unique_writable = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 62,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool unique writable token should be preserved");
                pool_unique_writable_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_unique_writable_free_capture.lock().unwrap().push(62);
        }),
    )
    .unwrap();
    let mut pool_unique_writable_ref = pool_unique_writable.get().unwrap();
    pool_unique_writable_ref
        .make_mut()
        .copy_from_slice(&[0x62, 0x63, 0x64]);
    let pool_unique_writable_ptr = pool_unique_writable_ref.as_ptr();
    pool_unique_writable_ref.make_mut();
    rows.insert(
        "pool-unique-writable:ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "pool-unique-writable:after".to_string(),
        buffer_fields(&pool_unique_writable_ref),
    );
    let pool_unique_writable_token = pool_unique_writable_ref
        .pool_opaque_ref::<PoolToken>()
        .expect("pool unique writable token");
    rows.insert(
        "pool-unique-writable:opaque".to_string(),
        vec![
            pool_unique_writable_token.id.to_string(),
            pool_unique_writable_token.size.to_string(),
        ],
    );
    rows.insert(
        "pool-unique-writable:same-data".to_string(),
        vec![bool_field(
            pool_unique_writable_ref.as_ptr() == pool_unique_writable_ptr,
        )],
    );
    rows.insert(
        "pool-unique-writable:after-make-writable".to_string(),
        vec![
            pool_unique_writable_releases
                .lock()
                .unwrap()
                .len()
                .to_string(),
            pool_unique_writable_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(pool_unique_writable_ref);
    rows.insert(
        "pool-unique-writable:after-unref".to_string(),
        vec![
            pool_unique_writable_releases
                .lock()
                .unwrap()
                .len()
                .to_string(),
            pool_unique_writable_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_unique_writable_reuse = pool_unique_writable.get().unwrap();
    rows.insert(
        "pool-unique-writable:reuse".to_string(),
        buffer_fields(&pool_unique_writable_reuse),
    );
    let pool_unique_writable_reuse_token = pool_unique_writable_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool unique writable reuse token");
    rows.insert(
        "pool-unique-writable:opaque-reuse".to_string(),
        vec![
            pool_unique_writable_reuse_token.id.to_string(),
            pool_unique_writable_reuse_token.size.to_string(),
        ],
    );
    drop(pool_unique_writable_reuse);
    drop(pool_unique_writable);
    let pool_unique_writable_release_values = pool_unique_writable_releases.lock().unwrap();
    let pool_unique_writable_free_values = pool_unique_writable_frees.lock().unwrap();
    rows.insert(
        "pool-unique-writable:uninit-release".to_string(),
        vec![
            pool_unique_writable_release_values.len().to_string(),
            pool_unique_writable_release_values[0].0.to_string(),
            hex(&pool_unique_writable_release_values[0].1),
            pool_unique_writable_free_values.len().to_string(),
            pool_unique_writable_free_values[0].to_string(),
        ],
    );
    drop(pool_unique_writable_free_values);
    drop(pool_unique_writable_release_values);

    let pool_cow_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_cow_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_cow_release_capture = Arc::clone(&pool_cow_releases);
    let pool_cow_free_capture = Arc::clone(&pool_cow_frees);
    let pool_cow = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 56,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool COW token should be preserved");
                pool_cow_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_cow_free_capture.lock().unwrap().push(56);
        }),
    )
    .unwrap();
    let pool_cow_src = pool_cow.get().unwrap();
    let mut pool_cow_dst = BufferRef::ref_from(&pool_cow_src);
    pool_cow_dst.make_mut();
    rows.insert(
        "pool-cow:make-writable-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert("pool-cow:src".to_string(), buffer_fields(&pool_cow_src));
    rows.insert("pool-cow:dst".to_string(), buffer_fields(&pool_cow_dst));
    rows.insert(
        "pool-cow:dst-opaque-null".to_string(),
        vec![bool_field(pool_cow_dst.opaque_ref::<PoolToken>().is_none())],
    );
    rows.insert(
        "pool-cow:shares".to_string(),
        vec![bool_field(pool_cow_src.shares_storage(&pool_cow_dst))],
    );
    pool_cow_dst.make_mut().copy_from_slice(&[0xab, 0xbc, 0xcd]);
    rows.insert(
        "pool-cow:dst-mutated".to_string(),
        buffer_fields(&pool_cow_dst),
    );
    drop(pool_cow_dst);
    rows.insert(
        "pool-cow:after-dst-unref".to_string(),
        vec![
            pool_cow_releases.lock().unwrap().len().to_string(),
            pool_cow_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(pool_cow_src);
    rows.insert(
        "pool-cow:after-src-unref".to_string(),
        vec![
            pool_cow_releases.lock().unwrap().len().to_string(),
            pool_cow_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_cow_reuse = pool_cow.get().unwrap();
    rows.insert("pool-cow:reuse".to_string(), buffer_fields(&pool_cow_reuse));
    let pool_cow_reuse_token = pool_cow_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool COW reuse token");
    rows.insert(
        "pool-cow:opaque-reuse".to_string(),
        vec![
            pool_cow_reuse_token.id.to_string(),
            pool_cow_reuse_token.size.to_string(),
        ],
    );
    drop(pool_cow_reuse);
    drop(pool_cow);
    let pool_cow_release_values = pool_cow_releases.lock().unwrap();
    let pool_cow_free_values = pool_cow_frees.lock().unwrap();
    rows.insert(
        "pool-cow:uninit-release".to_string(),
        vec![
            pool_cow_release_values.len().to_string(),
            pool_cow_release_values[0].0.to_string(),
            hex(&pool_cow_release_values[0].1),
            pool_cow_free_values.len().to_string(),
            pool_cow_free_values[0].to_string(),
        ],
    );
    drop(pool_cow_free_values);
    drop(pool_cow_release_values);

    let pool_realloc_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_realloc_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_realloc_release_capture = Arc::clone(&pool_realloc_releases);
    let pool_realloc_free_capture = Arc::clone(&pool_realloc_frees);
    let pool_realloc_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 57,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool realloc token should be preserved");
                pool_realloc_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_realloc_free_capture.lock().unwrap().push(57);
        }),
    )
    .unwrap();
    let mut pool_realloc = Some(pool_realloc_pool.get().unwrap());
    pool_realloc
        .as_mut()
        .unwrap()
        .make_mut()
        .copy_from_slice(&[0x10, 0x11, 0x12]);
    BufferRef::realloc(&mut pool_realloc, 5).unwrap();
    let mut pool_realloc_dst = pool_realloc.expect("pool realloc destination");
    rows.insert("pool-realloc:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-realloc:dst".to_string(),
        buffer_prefix_fields(&pool_realloc_dst, 3),
    );
    rows.insert(
        "pool-realloc:dst-opaque-null".to_string(),
        vec![bool_field(
            pool_realloc_dst.opaque_ref::<PoolToken>().is_none(),
        )],
    );
    rows.insert(
        "pool-realloc:after-realloc".to_string(),
        vec![
            pool_realloc_releases.lock().unwrap().len().to_string(),
            pool_realloc_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_realloc_reuse = pool_realloc_pool.get().unwrap();
    rows.insert(
        "pool-realloc:reuse".to_string(),
        buffer_fields(&pool_realloc_reuse),
    );
    let pool_realloc_reuse_token = pool_realloc_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool realloc reuse token");
    rows.insert(
        "pool-realloc:opaque-reuse".to_string(),
        vec![
            pool_realloc_reuse_token.id.to_string(),
            pool_realloc_reuse_token.size.to_string(),
        ],
    );
    pool_realloc_dst.make_mut()[0] = 0xee;
    rows.insert(
        "pool-realloc:dst-mutated".to_string(),
        buffer_prefix_fields(&pool_realloc_dst, 3),
    );
    drop(pool_realloc_dst);
    rows.insert(
        "pool-realloc:after-dst-unref".to_string(),
        vec![
            pool_realloc_releases.lock().unwrap().len().to_string(),
            pool_realloc_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(pool_realloc_reuse);
    drop(pool_realloc_pool);
    let pool_realloc_release_values = pool_realloc_releases.lock().unwrap();
    let pool_realloc_free_values = pool_realloc_frees.lock().unwrap();
    rows.insert(
        "pool-realloc:uninit-release".to_string(),
        vec![
            pool_realloc_release_values.len().to_string(),
            pool_realloc_release_values[0].0.to_string(),
            hex(&pool_realloc_release_values[0].1),
            pool_realloc_free_values.len().to_string(),
            pool_realloc_free_values[0].to_string(),
        ],
    );
    drop(pool_realloc_free_values);
    drop(pool_realloc_release_values);

    let pool_replace_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_replace_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_replace_release_capture = Arc::clone(&pool_replace_releases);
    let pool_replace_free_capture = Arc::clone(&pool_replace_frees);
    let pool_replace_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 58,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool replace token should be preserved");
                pool_replace_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_replace_free_capture.lock().unwrap().push(58);
        }),
    )
    .unwrap();
    let pool_replace_source = BufferRef::from_vec(vec![0x91, 0x92]);
    let mut pool_replace_dst = Some(pool_replace_pool.get().unwrap());
    pool_replace_dst
        .as_mut()
        .unwrap()
        .make_mut()
        .copy_from_slice(&[0x20, 0x21, 0x22]);
    BufferRef::replace(&mut pool_replace_dst, Some(&pool_replace_source));
    let pool_replace_dst = pool_replace_dst.expect("pool replace destination");
    rows.insert("pool-replace:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-replace:dst".to_string(),
        buffer_fields(&pool_replace_dst),
    );
    rows.insert(
        "pool-replace:dst-opaque-null".to_string(),
        vec![bool_field(
            pool_replace_dst.opaque_ref::<PoolToken>().is_none(),
        )],
    );
    rows.insert(
        "pool-replace:shares".to_string(),
        vec![bool_field(
            pool_replace_dst.shares_storage(&pool_replace_source),
        )],
    );
    rows.insert(
        "pool-replace:after-replace".to_string(),
        vec![
            pool_replace_releases.lock().unwrap().len().to_string(),
            pool_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_replace_reuse = pool_replace_pool.get().unwrap();
    rows.insert(
        "pool-replace:reuse".to_string(),
        buffer_fields(&pool_replace_reuse),
    );
    let pool_replace_reuse_token = pool_replace_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool replace reuse token");
    rows.insert(
        "pool-replace:opaque-reuse".to_string(),
        vec![
            pool_replace_reuse_token.id.to_string(),
            pool_replace_reuse_token.size.to_string(),
        ],
    );
    drop(pool_replace_dst);
    rows.insert(
        "pool-replace:after-dst-unref".to_string(),
        vec![
            pool_replace_releases.lock().unwrap().len().to_string(),
            pool_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(pool_replace_source);
    drop(pool_replace_reuse);
    drop(pool_replace_pool);
    let pool_replace_release_values = pool_replace_releases.lock().unwrap();
    let pool_replace_free_values = pool_replace_frees.lock().unwrap();
    rows.insert(
        "pool-replace:uninit-release".to_string(),
        vec![
            pool_replace_release_values.len().to_string(),
            pool_replace_release_values[0].0.to_string(),
            hex(&pool_replace_release_values[0].1),
            pool_replace_free_values.len().to_string(),
            pool_replace_free_values[0].to_string(),
        ],
    );
    drop(pool_replace_free_values);
    drop(pool_replace_release_values);

    let pool_null_replace_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_null_replace_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_null_replace_release_capture = Arc::clone(&pool_null_replace_releases);
    let pool_null_replace_free_capture = Arc::clone(&pool_null_replace_frees);
    let pool_null_replace_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 63,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool null replace token should be preserved");
                pool_null_replace_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_null_replace_free_capture.lock().unwrap().push(63);
        }),
    )
    .unwrap();
    let mut pool_null_replace_dst = Some(pool_null_replace_pool.get().unwrap());
    pool_null_replace_dst
        .as_mut()
        .unwrap()
        .make_mut()
        .copy_from_slice(&[0x63, 0x64, 0x65]);
    BufferRef::replace(&mut pool_null_replace_dst, None);
    rows.insert("pool-null-replace:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-null-replace:dst-null".to_string(),
        vec![bool_field(pool_null_replace_dst.is_none())],
    );
    rows.insert(
        "pool-null-replace:after-replace".to_string(),
        vec![
            pool_null_replace_releases.lock().unwrap().len().to_string(),
            pool_null_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_null_replace_reuse = pool_null_replace_pool.get().unwrap();
    rows.insert(
        "pool-null-replace:reuse".to_string(),
        buffer_fields(&pool_null_replace_reuse),
    );
    let pool_null_replace_reuse_token = pool_null_replace_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool null replace reuse token");
    rows.insert(
        "pool-null-replace:opaque-reuse".to_string(),
        vec![
            pool_null_replace_reuse_token.id.to_string(),
            pool_null_replace_reuse_token.size.to_string(),
        ],
    );
    drop(pool_null_replace_reuse);
    drop(pool_null_replace_pool);
    let pool_null_replace_release_values = pool_null_replace_releases.lock().unwrap();
    let pool_null_replace_free_values = pool_null_replace_frees.lock().unwrap();
    rows.insert(
        "pool-null-replace:uninit-release".to_string(),
        vec![
            pool_null_replace_release_values.len().to_string(),
            pool_null_replace_release_values[0].0.to_string(),
            hex(&pool_null_replace_release_values[0].1),
            pool_null_replace_free_values.len().to_string(),
            pool_null_replace_free_values[0].to_string(),
        ],
    );
    drop(pool_null_replace_free_values);
    drop(pool_null_replace_release_values);

    let pool_source_replace_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_source_replace_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_source_replace_release_capture = Arc::clone(&pool_source_replace_releases);
    let pool_source_replace_free_capture = Arc::clone(&pool_source_replace_frees);
    let pool_source_replace_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 59,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool source replace token should be preserved");
                pool_source_replace_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_source_replace_free_capture.lock().unwrap().push(59);
        }),
    )
    .unwrap();
    let mut pool_source_replace_source = pool_source_replace_pool.get().unwrap();
    pool_source_replace_source
        .make_mut()
        .copy_from_slice(&[0x41, 0x42, 0x43]);
    let mut pool_source_replace_dst = Some(BufferRef::from_vec(vec![0x66, 0x67]));
    BufferRef::replace(
        &mut pool_source_replace_dst,
        Some(&pool_source_replace_source),
    );
    let pool_source_replace_dst = pool_source_replace_dst.expect("pool source replace destination");
    rows.insert("pool-source-replace:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-source-replace:dst".to_string(),
        buffer_fields(&pool_source_replace_dst),
    );
    let pool_source_replace_dst_token = pool_source_replace_dst
        .pool_opaque_ref::<PoolToken>()
        .expect("pool source replace destination token");
    rows.insert(
        "pool-source-replace:dst-opaque".to_string(),
        vec![
            pool_source_replace_dst_token.id.to_string(),
            pool_source_replace_dst_token.size.to_string(),
        ],
    );
    rows.insert(
        "pool-source-replace:shares".to_string(),
        vec![bool_field(
            pool_source_replace_dst.shares_storage(&pool_source_replace_source),
        )],
    );
    rows.insert(
        "pool-source-replace:after-replace".to_string(),
        vec![
            pool_source_replace_releases
                .lock()
                .unwrap()
                .len()
                .to_string(),
            pool_source_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(pool_source_replace_source);
    rows.insert(
        "pool-source-replace:after-src-unref".to_string(),
        vec![
            pool_source_replace_releases
                .lock()
                .unwrap()
                .len()
                .to_string(),
            pool_source_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(pool_source_replace_dst);
    rows.insert(
        "pool-source-replace:after-dst-unref".to_string(),
        vec![
            pool_source_replace_releases
                .lock()
                .unwrap()
                .len()
                .to_string(),
            pool_source_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_source_replace_reuse = pool_source_replace_pool.get().unwrap();
    rows.insert(
        "pool-source-replace:reuse".to_string(),
        buffer_fields(&pool_source_replace_reuse),
    );
    let pool_source_replace_reuse_token = pool_source_replace_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool source replace reuse token");
    rows.insert(
        "pool-source-replace:opaque-reuse".to_string(),
        vec![
            pool_source_replace_reuse_token.id.to_string(),
            pool_source_replace_reuse_token.size.to_string(),
        ],
    );
    drop(pool_source_replace_reuse);
    drop(pool_source_replace_pool);
    let pool_source_replace_release_values = pool_source_replace_releases.lock().unwrap();
    let pool_source_replace_free_values = pool_source_replace_frees.lock().unwrap();
    rows.insert(
        "pool-source-replace:uninit-release".to_string(),
        vec![
            pool_source_replace_release_values.len().to_string(),
            pool_source_replace_release_values[0].0.to_string(),
            hex(&pool_source_replace_release_values[0].1),
            pool_source_replace_free_values.len().to_string(),
            pool_source_replace_free_values[0].to_string(),
        ],
    );
    drop(pool_source_replace_free_values);
    drop(pool_source_replace_release_values);

    let pool_pair_destination_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_pair_destination_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_pair_destination_release_capture = Arc::clone(&pool_pair_destination_releases);
    let pool_pair_destination_free_capture = Arc::clone(&pool_pair_destination_frees);
    let pool_pair_destination_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 60,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool pair destination token should be preserved");
                pool_pair_destination_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_pair_destination_free_capture.lock().unwrap().push(60);
        }),
    )
    .unwrap();
    let pool_pair_source_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_pair_source_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_pair_source_release_capture = Arc::clone(&pool_pair_source_releases);
    let pool_pair_source_free_capture = Arc::clone(&pool_pair_source_frees);
    let pool_pair_source_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![4, 5, 6],
                    PoolToken {
                        id: 61,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("pool pair source token should be preserved");
                pool_pair_source_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_pair_source_free_capture.lock().unwrap().push(61);
        }),
    )
    .unwrap();
    let mut pool_pair_source = pool_pair_source_pool.get().unwrap();
    pool_pair_source
        .make_mut()
        .copy_from_slice(&[0x51, 0x52, 0x53]);
    let mut pool_pair_destination = Some(pool_pair_destination_pool.get().unwrap());
    pool_pair_destination
        .as_mut()
        .unwrap()
        .make_mut()
        .copy_from_slice(&[0x31, 0x32, 0x33]);
    BufferRef::replace(&mut pool_pair_destination, Some(&pool_pair_source));
    let pool_pair_destination = pool_pair_destination.expect("pool pair replacement destination");
    rows.insert("pool-pair-replace:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-pair-replace:dst".to_string(),
        buffer_fields(&pool_pair_destination),
    );
    let pool_pair_destination_token = pool_pair_destination
        .pool_opaque_ref::<PoolToken>()
        .expect("pool pair replacement destination token");
    rows.insert(
        "pool-pair-replace:dst-opaque".to_string(),
        vec![
            pool_pair_destination_token.id.to_string(),
            pool_pair_destination_token.size.to_string(),
        ],
    );
    rows.insert(
        "pool-pair-replace:shares".to_string(),
        vec![bool_field(
            pool_pair_destination.shares_storage(&pool_pair_source),
        )],
    );
    rows.insert(
        "pool-pair-replace:after-replace".to_string(),
        vec![
            (pool_pair_destination_releases.lock().unwrap().len()
                + pool_pair_source_releases.lock().unwrap().len())
            .to_string(),
            (pool_pair_destination_frees.lock().unwrap().len()
                + pool_pair_source_frees.lock().unwrap().len())
            .to_string(),
        ],
    );
    let pool_pair_destination_reuse = pool_pair_destination_pool.get().unwrap();
    rows.insert(
        "pool-pair-replace:dst-reuse".to_string(),
        buffer_fields(&pool_pair_destination_reuse),
    );
    let pool_pair_destination_reuse_token = pool_pair_destination_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool pair destination reuse token");
    rows.insert(
        "pool-pair-replace:opaque-reuse-dst".to_string(),
        vec![
            pool_pair_destination_reuse_token.id.to_string(),
            pool_pair_destination_reuse_token.size.to_string(),
        ],
    );
    drop(pool_pair_source);
    rows.insert(
        "pool-pair-replace:after-src-unref".to_string(),
        vec![
            (pool_pair_destination_releases.lock().unwrap().len()
                + pool_pair_source_releases.lock().unwrap().len())
            .to_string(),
            (pool_pair_destination_frees.lock().unwrap().len()
                + pool_pair_source_frees.lock().unwrap().len())
            .to_string(),
        ],
    );
    drop(pool_pair_destination);
    rows.insert(
        "pool-pair-replace:after-dst-unref".to_string(),
        vec![
            (pool_pair_destination_releases.lock().unwrap().len()
                + pool_pair_source_releases.lock().unwrap().len())
            .to_string(),
            (pool_pair_destination_frees.lock().unwrap().len()
                + pool_pair_source_frees.lock().unwrap().len())
            .to_string(),
        ],
    );
    let pool_pair_source_reuse = pool_pair_source_pool.get().unwrap();
    rows.insert(
        "pool-pair-replace:src-reuse".to_string(),
        buffer_fields(&pool_pair_source_reuse),
    );
    let pool_pair_source_reuse_token = pool_pair_source_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("pool pair source reuse token");
    rows.insert(
        "pool-pair-replace:opaque-reuse-src".to_string(),
        vec![
            pool_pair_source_reuse_token.id.to_string(),
            pool_pair_source_reuse_token.size.to_string(),
        ],
    );
    drop(pool_pair_destination_reuse);
    drop(pool_pair_destination_pool);
    let pool_pair_destination_release_values = pool_pair_destination_releases.lock().unwrap();
    let pool_pair_destination_free_values = pool_pair_destination_frees.lock().unwrap();
    let pool_pair_destination_release_count = pool_pair_destination_release_values.len();
    let pool_pair_destination_free_count = pool_pair_destination_free_values.len();
    rows.insert(
        "pool-pair-replace:dst-uninit-release".to_string(),
        vec![
            pool_pair_destination_release_count.to_string(),
            pool_pair_destination_release_values[0].0.to_string(),
            hex(&pool_pair_destination_release_values[0].1),
            pool_pair_destination_free_count.to_string(),
            pool_pair_destination_free_values[0].to_string(),
        ],
    );
    drop(pool_pair_destination_free_values);
    drop(pool_pair_destination_release_values);
    drop(pool_pair_source_reuse);
    drop(pool_pair_source_pool);
    let pool_pair_source_release_values = pool_pair_source_releases.lock().unwrap();
    let pool_pair_source_free_values = pool_pair_source_frees.lock().unwrap();
    rows.insert(
        "pool-pair-replace:src-uninit-release".to_string(),
        vec![
            (pool_pair_destination_release_count + pool_pair_source_release_values.len())
                .to_string(),
            pool_pair_source_release_values[0].0.to_string(),
            hex(&pool_pair_source_release_values[0].1),
            (pool_pair_destination_free_count + pool_pair_source_free_values.len()).to_string(),
            pool_pair_source_free_values[0].to_string(),
        ],
    );
    drop(pool_pair_source_free_values);
    drop(pool_pair_source_release_values);

    let pool_same_replace_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let pool_same_replace_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let pool_same_replace_release_capture = Arc::clone(&pool_same_replace_releases);
    let pool_same_replace_free_capture = Arc::clone(&pool_same_replace_frees);
    let pool_same_replace = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 3);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2, 3],
                    PoolToken {
                        id: 64,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("same-pool replacement token should be preserved");
                pool_same_replace_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            pool_same_replace_free_capture.lock().unwrap().push(64);
        }),
    )
    .unwrap();
    let mut pool_same_replace_source = pool_same_replace.get().unwrap();
    pool_same_replace_source
        .make_mut()
        .copy_from_slice(&[0x51, 0x52, 0x53]);
    let mut pool_same_replace_destination = Some(pool_same_replace.get().unwrap());
    pool_same_replace_destination
        .as_mut()
        .unwrap()
        .make_mut()
        .copy_from_slice(&[0x31, 0x32, 0x33]);
    BufferRef::replace(
        &mut pool_same_replace_destination,
        Some(&pool_same_replace_source),
    );
    let pool_same_replace_replaced =
        pool_same_replace_destination.expect("same-pool replacement keeps destination");
    rows.insert("pool-same-replace:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-same-replace:dst".to_string(),
        buffer_fields(&pool_same_replace_replaced),
    );
    let pool_same_replace_replaced_token = pool_same_replace_replaced
        .pool_opaque_ref::<PoolToken>()
        .expect("same-pool replacement destination token");
    rows.insert(
        "pool-same-replace:dst-opaque".to_string(),
        vec![
            pool_same_replace_replaced_token.id.to_string(),
            pool_same_replace_replaced_token.size.to_string(),
        ],
    );
    rows.insert(
        "pool-same-replace:shares".to_string(),
        vec![bool_field(
            pool_same_replace_replaced.shares_storage(&pool_same_replace_source),
        )],
    );
    rows.insert(
        "pool-same-replace:after-replace".to_string(),
        vec![
            pool_same_replace_releases.lock().unwrap().len().to_string(),
            pool_same_replace_frees.lock().unwrap().len().to_string(),
        ],
    );
    let pool_same_replace_reuse_dst = pool_same_replace.get().unwrap();
    rows.insert(
        "pool-same-replace:reuse-dst".to_string(),
        buffer_fields(&pool_same_replace_reuse_dst),
    );
    let pool_same_replace_reuse_dst_token = pool_same_replace_reuse_dst
        .pool_opaque_ref::<PoolToken>()
        .expect("same-pool replacement reuse token");
    rows.insert(
        "pool-same-replace:opaque-reuse-dst".to_string(),
        vec![
            pool_same_replace_reuse_dst_token.id.to_string(),
            pool_same_replace_reuse_dst_token.size.to_string(),
        ],
    );
    drop(pool_same_replace_reuse_dst);
    drop(pool_same_replace_source);
    drop(pool_same_replace_replaced);
    let pool_same_replace_reuse_first = pool_same_replace.get().unwrap();
    let pool_same_replace_reuse_second = pool_same_replace.get().unwrap();
    rows.insert(
        "pool-same-replace:reuse-first".to_string(),
        buffer_fields(&pool_same_replace_reuse_first),
    );
    rows.insert(
        "pool-same-replace:reuse-second".to_string(),
        buffer_fields(&pool_same_replace_reuse_second),
    );
    let pool_same_replace_reuse_first_token = pool_same_replace_reuse_first
        .pool_opaque_ref::<PoolToken>()
        .expect("same-pool replacement reuse-first token");
    rows.insert(
        "pool-same-replace:opaque-reuse-first".to_string(),
        vec![
            pool_same_replace_reuse_first_token.id.to_string(),
            pool_same_replace_reuse_first_token.size.to_string(),
        ],
    );
    let pool_same_replace_reuse_second_token = pool_same_replace_reuse_second
        .pool_opaque_ref::<PoolToken>()
        .expect("same-pool replacement reuse-second token");
    rows.insert(
        "pool-same-replace:opaque-reuse-second".to_string(),
        vec![
            pool_same_replace_reuse_second_token.id.to_string(),
            pool_same_replace_reuse_second_token.size.to_string(),
        ],
    );
    drop(pool_same_replace_reuse_second);
    drop(pool_same_replace_reuse_first);
    drop(pool_same_replace);
    let pool_same_replace_release_values = pool_same_replace_releases.lock().unwrap();
    let pool_same_replace_free_values = pool_same_replace_frees.lock().unwrap();
    rows.insert(
        "pool-same-replace:uninit-release".to_string(),
        vec![
            pool_same_replace_release_values.len().to_string(),
            pool_same_replace_release_values[0].0.to_string(),
            hex(&pool_same_replace_release_values[0].1),
            pool_same_replace_release_values[1].0.to_string(),
            hex(&pool_same_replace_release_values[1].1),
            pool_same_replace_free_values.len().to_string(),
            pool_same_replace_free_values[0].to_string(),
        ],
    );
    drop(pool_same_replace_free_values);
    drop(pool_same_replace_release_values);

    let offset_pool_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let offset_pool_release_capture = Arc::clone(&offset_pool_releases);
    let offset_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                BufferPoolAllocation::with_opaque_visible_range(
                    vec![0xee, 0x31, 0x32, 0x33],
                    1,
                    allocated_len,
                    PoolToken {
                        id: 88,
                        size: allocated_len + 1,
                    },
                )
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("offset pool token should be preserved");
                offset_pool_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        ),
    )
    .unwrap();
    let offset_first = offset_pool.get().unwrap();
    rows.insert(
        "pool-offset:first".to_string(),
        buffer_fields(&offset_first),
    );
    let offset_first_token = offset_first
        .pool_opaque_ref::<PoolToken>()
        .expect("offset first pool token");
    rows.insert(
        "pool-offset:opaque-first".to_string(),
        vec![
            offset_first_token.id.to_string(),
            offset_first_token.size.to_string(),
        ],
    );
    drop(offset_first);
    rows.insert(
        "pool-offset:after-first-unref".to_string(),
        vec![offset_pool_releases.lock().unwrap().len().to_string()],
    );
    let offset_reuse = offset_pool.get().unwrap();
    rows.insert(
        "pool-offset:reuse".to_string(),
        buffer_fields(&offset_reuse),
    );
    let offset_reuse_token = offset_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("offset reuse pool token");
    rows.insert(
        "pool-offset:opaque-reuse".to_string(),
        vec![
            offset_reuse_token.id.to_string(),
            offset_reuse_token.size.to_string(),
        ],
    );
    drop(offset_reuse);
    drop(offset_pool);
    let offset_pool_release_values = offset_pool_releases.lock().unwrap();
    rows.insert(
        "pool-offset:uninit-release".to_string(),
        vec![
            offset_pool_release_values.len().to_string(),
            offset_pool_release_values[0].0.to_string(),
            hex(&offset_pool_release_values[0].1),
        ],
    );
    drop(offset_pool_release_values);

    let offset_pool_unique_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let offset_pool_unique_release_capture = Arc::clone(&offset_pool_unique_releases);
    let offset_pool_unique = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                BufferPoolAllocation::with_opaque_visible_range(
                    vec![0xee, 0x31, 0x32, 0x33],
                    1,
                    allocated_len,
                    PoolToken {
                        id: 90,
                        size: allocated_len + 1,
                    },
                )
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("unique offset pool token should be preserved");
                offset_pool_unique_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        ),
    )
    .unwrap();
    let mut offset_unique = offset_pool_unique.get().unwrap();
    offset_unique.make_mut()[0] = 0xaa;
    rows.insert("pool-offset-unique:ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "pool-offset-unique:after".to_string(),
        buffer_fields(&offset_unique),
    );
    drop(offset_unique);
    rows.insert(
        "pool-offset-unique:after-first-unref".to_string(),
        vec![offset_pool_unique_releases
            .lock()
            .unwrap()
            .len()
            .to_string()],
    );
    let offset_unique_reuse = offset_pool_unique.get().unwrap();
    rows.insert(
        "pool-offset-unique:reuse".to_string(),
        buffer_fields(&offset_unique_reuse),
    );
    drop(offset_unique_reuse);
    drop(offset_pool_unique);
    let offset_unique_release_values = offset_pool_unique_releases.lock().unwrap();
    rows.insert(
        "pool-offset-unique:uninit-release".to_string(),
        vec![
            offset_unique_release_values.len().to_string(),
            offset_unique_release_values[0].0.to_string(),
            hex(&offset_unique_release_values[0].1),
        ],
    );
    drop(offset_unique_release_values);

    let readonly_offset_pool_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_offset_pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let readonly_offset_pool_release_capture = Arc::clone(&readonly_offset_pool_releases);
    let readonly_offset_pool_free_capture = Arc::clone(&readonly_offset_pool_frees);
    let readonly_offset_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                BufferPoolAllocation::with_opaque_readonly_visible_range(
                    vec![0xee, 0x31, 0x32, 0x33],
                    1,
                    allocated_len,
                    PoolToken {
                        id: 89,
                        size: allocated_len + 1,
                    },
                )
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("readonly offset pool token should be preserved");
                readonly_offset_pool_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            readonly_offset_pool_free_capture.lock().unwrap().push(89);
        }),
    )
    .unwrap();
    let readonly_offset_first = readonly_offset_pool.get().unwrap();
    rows.insert(
        "pool-readonly-offset:first".to_string(),
        buffer_fields(&readonly_offset_first),
    );
    let readonly_offset_first_token = readonly_offset_first
        .pool_opaque_ref::<PoolToken>()
        .expect("readonly offset first pool token");
    rows.insert(
        "pool-readonly-offset:opaque-first".to_string(),
        vec![
            readonly_offset_first_token.id.to_string(),
            readonly_offset_first_token.size.to_string(),
        ],
    );
    drop(readonly_offset_first);
    rows.insert(
        "pool-readonly-offset:after-first-unref".to_string(),
        vec![
            readonly_offset_pool_releases
                .lock()
                .unwrap()
                .len()
                .to_string(),
            readonly_offset_pool_frees.lock().unwrap().len().to_string(),
        ],
    );
    let mut readonly_offset_reuse = readonly_offset_pool.get().unwrap();
    rows.insert(
        "pool-readonly-offset:reuse".to_string(),
        buffer_fields(&readonly_offset_reuse),
    );
    let readonly_offset_reuse_token = readonly_offset_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("readonly offset reuse pool token");
    rows.insert(
        "pool-readonly-offset:opaque-reuse".to_string(),
        vec![
            readonly_offset_reuse_token.id.to_string(),
            readonly_offset_reuse_token.size.to_string(),
        ],
    );
    readonly_offset_reuse.make_mut()[0] = 0xaa;
    drop(readonly_offset_reuse);
    drop(readonly_offset_pool);
    let readonly_offset_release_values = readonly_offset_pool_releases.lock().unwrap();
    let readonly_offset_pool_free_values = readonly_offset_pool_frees.lock().unwrap();
    rows.insert(
        "pool-readonly-offset:uninit-release".to_string(),
        vec![
            readonly_offset_release_values.len().to_string(),
            readonly_offset_release_values[0].0.to_string(),
            hex(&readonly_offset_release_values[0].1),
            readonly_offset_pool_free_values.len().to_string(),
            readonly_offset_pool_free_values[0].to_string(),
        ],
    );
    drop(readonly_offset_pool_free_values);
    drop(readonly_offset_release_values);

    let readonly_pool_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let readonly_pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let readonly_pool_release_capture = Arc::clone(&readonly_pool_releases);
    let readonly_pool_free_capture = Arc::clone(&readonly_pool_frees);
    let readonly_pool = BufferPool::with_callbacks(
        3,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                Ok(BufferPoolAllocation::with_opaque_readonly(
                    vec![0x41, 0x42, 0x43],
                    PoolToken {
                        id: 77,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("readonly pool token should be preserved");
                readonly_pool_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            readonly_pool_free_capture.lock().unwrap().push(77);
        }),
    )
    .unwrap();
    let readonly_first = readonly_pool.get().unwrap();
    rows.insert(
        "pool-readonly:first".to_string(),
        buffer_fields(&readonly_first),
    );
    let readonly_first_token = readonly_first
        .pool_opaque_ref::<PoolToken>()
        .expect("readonly first pool token");
    rows.insert(
        "pool-readonly:opaque-first".to_string(),
        vec![
            readonly_first_token.id.to_string(),
            readonly_first_token.size.to_string(),
        ],
    );
    drop(readonly_first);
    rows.insert(
        "pool-readonly:after-first-unref".to_string(),
        vec![
            readonly_pool_releases.lock().unwrap().len().to_string(),
            readonly_pool_frees.lock().unwrap().len().to_string(),
        ],
    );
    let mut readonly_reuse = readonly_pool.get().unwrap();
    rows.insert(
        "pool-readonly:reuse".to_string(),
        buffer_fields(&readonly_reuse),
    );
    let readonly_reuse_token = readonly_reuse
        .pool_opaque_ref::<PoolToken>()
        .expect("readonly reuse pool token");
    rows.insert(
        "pool-readonly:opaque-reuse".to_string(),
        vec![
            readonly_reuse_token.id.to_string(),
            readonly_reuse_token.size.to_string(),
        ],
    );
    readonly_reuse.make_mut()[0] = 0xaa;
    drop(readonly_reuse);
    drop(readonly_pool);
    let readonly_release_values = readonly_pool_releases.lock().unwrap();
    let readonly_pool_free_values = readonly_pool_frees.lock().unwrap();
    rows.insert(
        "pool-readonly:uninit-release".to_string(),
        vec![
            readonly_release_values.len().to_string(),
            readonly_release_values[0].0.to_string(),
            hex(&readonly_release_values[0].1),
            readonly_pool_free_values.len().to_string(),
            readonly_pool_free_values[0].to_string(),
        ],
    );
    drop(readonly_pool_free_values);
    drop(readonly_release_values);

    let huge_default_pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let huge_default_pool_free_capture = Arc::clone(&huge_default_pool_frees);
    let huge_default_pool = BufferPool::with_callbacks(
        usize::MAX,
        0,
        BufferPoolCallbacks::default().with_pool_free(move || {
            huge_default_pool_free_capture.lock().unwrap().push(99);
        }),
    )
    .unwrap();
    let huge_default_err = huge_default_pool.get().unwrap_err();
    rows.insert(
        "pool-default-huge:get".to_string(),
        vec![
            bool_field(huge_default_err.code() == Some(AvErrorCode::ENOMEM)),
            huge_default_pool_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(huge_default_pool);
    let huge_default_pool_free_values = huge_default_pool_frees.lock().unwrap();
    rows.insert(
        "pool-default-huge:uninit".to_string(),
        vec![
            huge_default_pool_free_values.len().to_string(),
            huge_default_pool_free_values[0].to_string(),
        ],
    );
    drop(huge_default_pool_free_values);

    let outstanding_releases = Arc::new(Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
    let outstanding_pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let outstanding_release_capture = Arc::clone(&outstanding_releases);
    let outstanding_pool_free_capture = Arc::clone(&outstanding_pool_frees);
    let outstanding_pool = BufferPool::with_callbacks(
        2,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            |allocated_len| {
                assert_eq!(allocated_len, 2);
                Ok(BufferPoolAllocation::with_opaque(
                    vec![1, 2],
                    PoolToken {
                        id: 66,
                        size: allocated_len,
                    },
                ))
            },
            move |allocation| {
                let token = allocation
                    .opaque_ref::<PoolToken>()
                    .expect("outstanding pool token should be preserved");
                outstanding_release_capture
                    .lock()
                    .unwrap()
                    .push((token.id, allocation.as_slice().to_vec()));
            },
        )
        .with_pool_free(move || {
            outstanding_pool_free_capture.lock().unwrap().push(66);
        }),
    )
    .unwrap();
    let mut outstanding = outstanding_pool.get().unwrap();
    outstanding.make_mut().copy_from_slice(&[0x11, 0x22]);
    drop(outstanding_pool);
    rows.insert(
        "pool:outstanding-after-uninit".to_string(),
        vec![
            outstanding_releases.lock().unwrap().len().to_string(),
            outstanding_pool_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(outstanding);
    let outstanding_release_values = outstanding_releases.lock().unwrap();
    rows.insert(
        "pool:outstanding-after-drop".to_string(),
        vec![
            outstanding_release_values.len().to_string(),
            outstanding_release_values[0].0.to_string(),
            hex(&outstanding_release_values[0].1),
            outstanding_pool_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(outstanding_release_values);

    let failed_allocations = Arc::new(Mutex::new(Vec::<usize>::new()));
    let failed_releases = Arc::new(Mutex::new(Vec::<BufferPoolAllocation>::new()));
    let failed_pool_frees = Arc::new(Mutex::new(Vec::<usize>::new()));
    let failed_allocation_capture = Arc::clone(&failed_allocations);
    let failed_release_capture = Arc::clone(&failed_releases);
    let failed_pool_free_capture = Arc::clone(&failed_pool_frees);
    let fail_pool = BufferPool::with_callbacks(
        4,
        0,
        BufferPoolCallbacks::with_allocation_callbacks(
            move |allocated_len| {
                failed_allocation_capture
                    .lock()
                    .unwrap()
                    .push(allocated_len);
                Err(avutil::AvError::external("pool allocation failed"))
            },
            move |allocation| {
                failed_release_capture.lock().unwrap().push(allocation);
            },
        )
        .with_pool_free(move || {
            failed_pool_free_capture.lock().unwrap().push(77);
        }),
    )
    .unwrap();
    rows.insert(
        "pool:alloc-fail".to_string(),
        vec![
            bool_field(fail_pool.get().is_err()),
            failed_allocations.lock().unwrap().len().to_string(),
            failed_releases.lock().unwrap().len().to_string(),
            failed_pool_frees.lock().unwrap().len().to_string(),
        ],
    );
    drop(fail_pool);
    let failed_pool_free_values = failed_pool_frees.lock().unwrap();
    rows.insert(
        "pool:alloc-fail-uninit".to_string(),
        vec![
            failed_pool_free_values.len().to_string(),
            failed_pool_free_values[0].to_string(),
        ],
    );
    drop(failed_pool_free_values);

    rows
}

fn buffer_fields(buffer: &BufferRef) -> Vec<String> {
    vec![
        buffer.len().to_string(),
        hex(buffer.as_slice()),
        buffer.strong_count().to_string(),
        bool_field(buffer.is_writable()),
    ]
}

fn buffer_abi_layout_fields(layout: &avutil::BufferAbiLayout) -> Vec<String> {
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

fn buffer_status_fields(buffer: &BufferRef) -> Vec<String> {
    vec![
        buffer.len().to_string(),
        buffer.strong_count().to_string(),
        bool_field(buffer.is_writable()),
    ]
}

fn buffer_prefix_fields(buffer: &BufferRef, prefix_len: usize) -> Vec<String> {
    vec![
        buffer.len().to_string(),
        hex(&buffer.as_slice()[..prefix_len]),
        buffer.strong_count().to_string(),
        bool_field(buffer.is_writable()),
    ]
}

fn buffer_fields_with_opaque(buffer: &BufferRef) -> Vec<String> {
    let mut fields = buffer_fields(buffer);
    fields.push(
        buffer
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string(),
    );
    fields
}

fn release_fields(released: &ReleaseRows) -> Vec<String> {
    let released = released.lock().unwrap();
    let (opaque, bytes) = released.first().expect("expected release row");
    vec![released.len().to_string(), opaque.to_string(), hex(bytes)]
}

fn hex(data: &[u8]) -> String {
    let mut output = String::with_capacity(data.len() * 2);
    for byte in data {
        output.push_str(&format!("{byte:02x}"));
    }
    output
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
            .expect("run WSL libavutil buffer oracle")
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
            .expect("run libavutil buffer oracle")
    };

    assert!(
        output.status.success(),
        "libavutil buffer oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
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
#include <libavutil/buffer.h>
#include <libavutil/mem.h>

static int release_count = 0;
static uintptr_t last_opaque = 0;
static int create_release_count = 0;
static uintptr_t last_create_opaque = 0;
static size_t last_create_release_size = 0;
static uint8_t last_create_release[32];
static int pool_alloc_count = 0;
static int pool_release_count = 0;
static int pool_free_count = 0;
static uintptr_t last_pool_release_id = 0;
static uintptr_t last_pool_free_id = 0;
static size_t last_pool_release_size = 0;
static uint8_t last_pool_release[32];
#define MAX_POOL_RELEASE_EVENTS 16
static uintptr_t pool_release_ids[MAX_POOL_RELEASE_EVENTS];
static size_t pool_release_sizes[MAX_POOL_RELEASE_EVENTS];
static uint8_t pool_release_data[MAX_POOL_RELEASE_EVENTS][32];

typedef struct PoolOpaque {
    uintptr_t id;
    size_t size;
} PoolOpaque;

static void fail_if(int condition, const char *message) {
    if (condition) {
        fprintf(stderr, "%s\n", message);
        exit(1);
    }
}

static void test_free(void *opaque, uint8_t *data) {
    release_count++;
    last_opaque = (uintptr_t)opaque;
    av_free(data);
}

static void reset_create_release(void) {
    create_release_count = 0;
    last_create_opaque = 0;
    last_create_release_size = 0;
    for (size_t i = 0; i < sizeof(last_create_release); i++)
        last_create_release[i] = 0;
}

static void test_create_free(void *opaque, uint8_t *data) {
    create_release_count++;
    last_create_opaque = (uintptr_t)opaque;
    fail_if(last_create_release_size > sizeof(last_create_release),
            "create release fixture too large");
    for (size_t i = 0; i < last_create_release_size; i++)
        last_create_release[i] = data[i];
    av_free(data);
}

static void reset_pool_counters(void) {
    pool_alloc_count = 0;
    pool_release_count = 0;
    pool_free_count = 0;
    last_pool_release_id = 0;
    last_pool_free_id = 0;
    last_pool_release_size = 0;
    for (size_t i = 0; i < sizeof(last_pool_release); i++)
        last_pool_release[i] = 0;
    for (size_t event = 0; event < MAX_POOL_RELEASE_EVENTS; event++) {
        pool_release_ids[event] = 0;
        pool_release_sizes[event] = 0;
        for (size_t i = 0; i < sizeof(pool_release_data[event]); i++)
            pool_release_data[event][i] = 0;
    }
}

static void test_pool_free(void *opaque, uint8_t *data) {
    PoolOpaque *pool_opaque = opaque;
    int event = pool_release_count++;
    last_pool_release_id = pool_opaque->id;
    last_pool_release_size = pool_opaque->size;
    fail_if(last_pool_release_size > sizeof(last_pool_release),
            "pool release fixture too large");
    for (size_t i = 0; i < last_pool_release_size; i++)
        last_pool_release[i] = data[i];
    if (event < MAX_POOL_RELEASE_EVENTS) {
        pool_release_ids[event] = pool_opaque->id;
        pool_release_sizes[event] = pool_opaque->size;
        fail_if(pool_opaque->size > sizeof(pool_release_data[event]),
                "pool release event fixture too large");
        for (size_t i = 0; i < pool_opaque->size; i++)
            pool_release_data[event][i] = data[i];
    }
    av_free(data);
}

static void test_pool_owner_free(void *opaque) {
    PoolOpaque *pool_opaque = opaque;
    pool_free_count++;
    last_pool_free_id = pool_opaque->id;
}

static AVBufferRef *test_pool_alloc(void *opaque, size_t size) {
    uint8_t *data = av_malloc(size);
    fail_if(!data, "av_malloc pool data failed");
    pool_alloc_count++;
    for (size_t i = 0; i < size; i++)
        data[i] = (uint8_t)(i + 1);
    return av_buffer_create(data, size, test_pool_free, opaque, 0);
}

static void test_pool_free_legacy(void *opaque, uint8_t *data) {
    (void)opaque;
    int event = pool_release_count++;
    last_pool_release_id = 0;
    last_pool_release_size = 3;
    for (size_t i = 0; i < last_pool_release_size; i++)
        last_pool_release[i] = data[i];
    if (event < MAX_POOL_RELEASE_EVENTS) {
        pool_release_ids[event] = 0;
        pool_release_sizes[event] = last_pool_release_size;
        for (size_t i = 0; i < last_pool_release_size; i++)
            pool_release_data[event][i] = data[i];
    }
    av_free(data);
}

static AVBufferRef *test_pool_alloc_legacy(size_t size) {
    uint8_t *data = av_malloc(size);
    fail_if(!data, "av_malloc legacy pool data failed");
    pool_alloc_count++;
    for (size_t i = 0; i < size; i++)
        data[i] = (uint8_t)(0x51 + i);
    return av_buffer_create(data, size, test_pool_free_legacy, NULL, 0);
}

static AVBufferRef *test_pool_alloc_multi_spare(void *opaque, size_t size) {
    uint8_t *data = av_malloc(size);
    fail_if(!data, "av_malloc multi-spare pool data failed");
    int allocation_index = pool_alloc_count++;
    for (size_t i = 0; i < size; i++)
        data[i] = (uint8_t)(0x70 + allocation_index);
    return av_buffer_create(data, size, test_pool_free, opaque, 0);
}

static AVBufferRef *test_pool_alloc_offset(void *opaque, size_t size) {
    uint8_t *data = av_malloc(size + 1);
    AVBufferRef *ret;
    fail_if(!data, "av_malloc offset pool data failed");
    pool_alloc_count++;
    data[0] = 0xee;
    for (size_t i = 0; i < size; i++)
        data[i + 1] = (uint8_t)(0x31 + i);
    ret = av_buffer_create(data, size + 1, test_pool_free, opaque, 0);
    fail_if(!ret, "av_buffer_create offset pool failed");
    ret->data = data + 1;
    ret->size = size;
    return ret;
}

static AVBufferRef *test_pool_alloc_readonly_offset(void *opaque, size_t size) {
    uint8_t *data = av_malloc(size + 1);
    AVBufferRef *ret;
    fail_if(!data, "av_malloc readonly offset pool data failed");
    pool_alloc_count++;
    data[0] = 0xee;
    for (size_t i = 0; i < size; i++)
        data[i + 1] = (uint8_t)(0x31 + i);
    ret = av_buffer_create(data, size + 1, test_pool_free, opaque,
                           AV_BUFFER_FLAG_READONLY);
    fail_if(!ret, "av_buffer_create readonly offset pool failed");
    ret->data = data + 1;
    ret->size = size;
    return ret;
}

static AVBufferRef *test_pool_alloc_readonly(void *opaque, size_t size) {
    uint8_t *data = av_malloc(size);
    fail_if(!data, "av_malloc readonly pool data failed");
    pool_alloc_count++;
    for (size_t i = 0; i < size; i++)
        data[i] = (uint8_t)(0x41 + i);
    return av_buffer_create(data, size, test_pool_free, opaque,
                            AV_BUFFER_FLAG_READONLY);
}

static AVBufferRef *test_pool_alloc_fail(void *opaque, size_t size) {
    (void)opaque;
    (void)size;
    pool_alloc_count++;
    return NULL;
}

static void fill_bytes(AVBufferRef *buf, const uint8_t *data, size_t size) {
    fail_if(!buf || buf->size < size, "short buffer in fill_bytes");
    for (size_t i = 0; i < size; i++)
        buf->data[i] = data[i];
}

static void print_hex(const uint8_t *data, size_t size) {
    for (size_t i = 0; i < size; i++)
        printf("%02x", data[i]);
}

static void print_status(const char *label, const AVBufferRef *buf) {
    printf("%s|%zu|%d|%d\n",
           label,
           buf ? buf->size : 0,
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0);
}

static void print_buffer(const char *label, const AVBufferRef *buf) {
    printf("%s|%zu|", label, buf ? buf->size : 0);
    if (buf)
        print_hex(buf->data, buf->size);
    printf("|%d|%d\n",
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0);
}

static void print_buffer_prefix(const char *label, const AVBufferRef *buf, size_t prefix) {
    printf("%s|%zu|", label, buf ? buf->size : 0);
    if (buf)
        print_hex(buf->data, prefix);
    printf("|%d|%d\n",
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0);
}

static void print_buffer_opaque(const char *label, const AVBufferRef *buf) {
    printf("%s|%zu|", label, buf ? buf->size : 0);
    if (buf)
        print_hex(buf->data, buf->size);
    printf("|%d|%d|%llu\n",
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0,
           (unsigned long long)(uintptr_t)(buf ? av_buffer_get_opaque(buf) : NULL));
}

static void print_create_release(const char *label) {
    printf("%s|%d|%llu|",
           label,
           create_release_count,
           (unsigned long long)last_create_opaque);
    print_hex(last_create_release, last_create_release_size);
    printf("\n");
}

#define PRINT_ABI_FIELD(type, field) \
    printf("|%s|%zu|%zu", #field, offsetof(type, field), sizeof(((type *)0)->field))

static void print_buffer_abi_layout(void) {
    printf("buffer:abi-avbufferref-layout|AVBufferRef|%zu|%zu|3",
           sizeof(AVBufferRef), (size_t)_Alignof(AVBufferRef));
    PRINT_ABI_FIELD(AVBufferRef, buffer);
    PRINT_ABI_FIELD(AVBufferRef, data);
    PRINT_ABI_FIELD(AVBufferRef, size);
    printf("\n");
}

static void print_buffer_flag_constants(void) {
    printf("buffer:flag-readonly|%d\n", AV_BUFFER_FLAG_READONLY);
}

int main(void) {
    print_buffer_abi_layout();
    print_buffer_flag_constants();

    AVBufferRef *buf = av_buffer_alloc(4);
    fail_if(!buf, "av_buffer_alloc failed");
    print_status("buffer:alloc", buf);
    av_buffer_unref(&buf);

    buf = av_buffer_alloc(SIZE_MAX);
    printf("buffer:alloc-huge|%d\n", buf == NULL);
    av_buffer_unref(&buf);

    buf = av_buffer_alloc(0);
    fail_if(!buf, "av_buffer_alloc zero failed");
    print_buffer("buffer:alloc-zero", buf);
    av_buffer_unref(&buf);

    buf = av_buffer_allocz(4);
    fail_if(!buf, "av_buffer_allocz failed");
    print_buffer("buffer:allocz", buf);
    av_buffer_unref(&buf);

    buf = av_buffer_allocz(SIZE_MAX);
    printf("buffer:allocz-huge|%d\n", buf == NULL);
    av_buffer_unref(&buf);

    buf = av_buffer_allocz(0);
    fail_if(!buf, "av_buffer_allocz zero failed");
    print_buffer("buffer:allocz-zero", buf);
    av_buffer_unref(&buf);

    static const uint8_t ref_bytes[] = { 1, 2, 3 };
    AVBufferRef *ref_src = av_buffer_allocz(3);
    fail_if(!ref_src, "av_buffer_allocz ref_src failed");
    fill_bytes(ref_src, ref_bytes, sizeof(ref_bytes));
    AVBufferRef *ref_dst = av_buffer_ref(ref_src);
    fail_if(!ref_dst, "av_buffer_ref failed");
    print_buffer("buffer:ref-src", ref_src);
    print_buffer("buffer:ref-dst", ref_dst);
    printf("buffer:ref-shares|%d\n", ref_src->data == ref_dst->data);
    av_buffer_unref(&ref_dst);
    av_buffer_unref(&ref_src);

    static const uint8_t unique_bytes[] = { 4, 5, 6 };
    AVBufferRef *unique = av_buffer_allocz(3);
    fail_if(!unique, "av_buffer_allocz unique failed");
    fill_bytes(unique, unique_bytes, sizeof(unique_bytes));
    uint8_t *unique_before = unique->data;
    int ret = av_buffer_make_writable(&unique);
    printf("buffer:make-writable-unique-ret|%d|%d\n",
           ret, unique_before == unique->data);
    print_buffer("buffer:make-writable-unique", unique);
    av_buffer_unref(&unique);

    static const uint8_t shared_bytes[] = { 9, 8, 7 };
    AVBufferRef *shared_src = av_buffer_allocz(3);
    fail_if(!shared_src, "av_buffer_allocz shared_src failed");
    fill_bytes(shared_src, shared_bytes, sizeof(shared_bytes));
    AVBufferRef *shared_dst = av_buffer_ref(shared_src);
    fail_if(!shared_dst, "av_buffer_ref shared failed");
    ret = av_buffer_make_writable(&shared_dst);
    printf("buffer:make-writable-shared-ret|%d\n", ret);
    print_buffer("buffer:make-writable-shared-src", shared_src);
    print_buffer("buffer:make-writable-shared-dst", shared_dst);
    printf("buffer:make-writable-shared-shares|%d\n",
           shared_src->data == shared_dst->data);
    av_buffer_unref(&shared_dst);
    av_buffer_unref(&shared_src);

    AVBufferRef *zero_shared_src = av_buffer_allocz(0);
    fail_if(!zero_shared_src, "av_buffer_allocz zero_shared_src failed");
    AVBufferRef *zero_shared_dst = av_buffer_ref(zero_shared_src);
    fail_if(!zero_shared_dst, "av_buffer_ref zero_shared failed");
    ret = av_buffer_make_writable(&zero_shared_dst);
    printf("buffer:make-writable-zero-shared-ret|%d\n", ret);
    print_buffer("buffer:make-writable-zero-shared-src", zero_shared_src);
    print_buffer("buffer:make-writable-zero-shared-dst", zero_shared_dst);
    printf("buffer:make-writable-zero-shared-shares|%d|%d\n",
           zero_shared_src->buffer == zero_shared_dst->buffer,
           av_buffer_get_ref_count(zero_shared_src));
    av_buffer_unref(&zero_shared_dst);
    av_buffer_unref(&zero_shared_src);

    uint8_t *readonly_data = av_malloc(3);
    fail_if(!readonly_data, "av_malloc readonly failed");
    readonly_data[0] = 5;
    readonly_data[1] = 6;
    readonly_data[2] = 7;
    AVBufferRef *readonly = av_buffer_create(readonly_data, 3, test_free,
                                             (void *)(uintptr_t)77,
                                             AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly, "av_buffer_create readonly failed");
    print_buffer_opaque("buffer:readonly", readonly);
    ret = av_buffer_make_writable(&readonly);
    printf("buffer:readonly-make-writable-ret|%d|%d|%llu\n",
           ret, release_count, (unsigned long long)last_opaque);
    print_buffer_opaque("buffer:readonly-after", readonly);
    av_buffer_unref(&readonly);

    reset_create_release();
    static const uint8_t create_bytes[] = { 31, 32, 33 };
    uint8_t *create_data = av_malloc(sizeof(create_bytes));
    fail_if(!create_data, "av_malloc create_data failed");
    for (size_t i = 0; i < sizeof(create_bytes); i++)
        create_data[i] = create_bytes[i];
    last_create_release_size = sizeof(create_bytes);
    AVBufferRef *create = av_buffer_create(create_data, sizeof(create_bytes),
                                           test_create_free,
                                           (void *)(uintptr_t)123, 0);
    fail_if(!create, "av_buffer_create writable failed");
    print_buffer_opaque("buffer:create-writable", create);
    av_buffer_unref(&create);
    print_create_release("buffer:create-writable-release");

    reset_create_release();
    uint8_t *create_zero_data = av_malloc(1);
    fail_if(!create_zero_data, "av_malloc create_zero_data failed");
    create_zero_data[0] = 0xab;
    last_create_release_size = 0;
    AVBufferRef *create_zero = av_buffer_create(create_zero_data, 0,
                                                test_create_free,
                                                (void *)(uintptr_t)321, 0);
    fail_if(!create_zero, "av_buffer_create zero failed");
    print_buffer_opaque("buffer:create-zero", create_zero);
    av_buffer_unref(&create_zero);
    print_create_release("buffer:create-zero-release");

    static const uint8_t create_default_opaque_bytes[] = { 34, 35, 36 };
    uint8_t *create_default_opaque_data =
        av_malloc(sizeof(create_default_opaque_bytes));
    fail_if(!create_default_opaque_data,
            "av_malloc create_default_opaque_data failed");
    for (size_t i = 0; i < sizeof(create_default_opaque_bytes); i++)
        create_default_opaque_data[i] = create_default_opaque_bytes[i];
    AVBufferRef *create_default_opaque =
        av_buffer_create(create_default_opaque_data,
                         sizeof(create_default_opaque_bytes),
                         NULL, (void *)(uintptr_t)322, 0);
    fail_if(!create_default_opaque,
            "av_buffer_create default opaque failed");
    print_buffer_opaque("buffer:create-default-opaque",
                        create_default_opaque);
    ret = av_buffer_make_writable(&create_default_opaque);
    printf("buffer:create-default-opaque-make-writable-ret|%d\n", ret);
    print_buffer_opaque("buffer:create-default-opaque-make-writable",
                        create_default_opaque);
    av_buffer_unref(&create_default_opaque);

    static const uint8_t create_default_shared_bytes[] = { 37, 38, 39 };
    uint8_t *create_default_shared_data =
        av_malloc(sizeof(create_default_shared_bytes));
    fail_if(!create_default_shared_data,
            "av_malloc create_default_shared_data failed");
    for (size_t i = 0; i < sizeof(create_default_shared_bytes); i++)
        create_default_shared_data[i] = create_default_shared_bytes[i];
    AVBufferRef *create_default_shared_src =
        av_buffer_create(create_default_shared_data,
                         sizeof(create_default_shared_bytes),
                         NULL, (void *)(uintptr_t)323, 0);
    fail_if(!create_default_shared_src,
            "av_buffer_create default opaque shared src failed");
    AVBufferRef *create_default_shared_dst =
        av_buffer_ref(create_default_shared_src);
    fail_if(!create_default_shared_dst,
            "av_buffer_ref default opaque shared failed");
    ret = av_buffer_make_writable(&create_default_shared_dst);
    printf("buffer:create-default-opaque-shared-make-writable-ret|%d\n",
           ret);
    print_buffer_opaque("buffer:create-default-opaque-shared-src",
                        create_default_shared_src);
    print_buffer_opaque("buffer:create-default-opaque-shared-dst",
                        create_default_shared_dst);
    printf("buffer:create-default-opaque-shared-shares|%d\n",
           create_default_shared_src->data == create_default_shared_dst->data);
    av_buffer_unref(&create_default_shared_dst);
    av_buffer_unref(&create_default_shared_src);

    static const uint8_t create_default_readonly_bytes[] = { 40, 41, 42 };
    uint8_t *create_default_readonly_data =
        av_malloc(sizeof(create_default_readonly_bytes));
    fail_if(!create_default_readonly_data,
            "av_malloc create_default_readonly_data failed");
    for (size_t i = 0; i < sizeof(create_default_readonly_bytes); i++)
        create_default_readonly_data[i] = create_default_readonly_bytes[i];
    AVBufferRef *create_default_readonly =
        av_buffer_create(create_default_readonly_data,
                         sizeof(create_default_readonly_bytes),
                         NULL, (void *)(uintptr_t)324,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!create_default_readonly,
            "av_buffer_create default opaque readonly failed");
    ret = av_buffer_make_writable(&create_default_readonly);
    printf("buffer:create-default-opaque-readonly-make-writable-ret|%d\n",
           ret);
    print_buffer_opaque("buffer:create-default-opaque-readonly-after",
                        create_default_readonly);
    av_buffer_unref(&create_default_readonly);

    static const uint8_t create_default_realloc_bytes[] = { 43, 44, 45 };
    uint8_t *create_default_realloc_data =
        av_malloc(sizeof(create_default_realloc_bytes));
    fail_if(!create_default_realloc_data,
            "av_malloc create_default_realloc_data failed");
    for (size_t i = 0; i < sizeof(create_default_realloc_bytes); i++)
        create_default_realloc_data[i] = create_default_realloc_bytes[i];
    AVBufferRef *create_default_realloc =
        av_buffer_create(create_default_realloc_data,
                         sizeof(create_default_realloc_bytes),
                         NULL, (void *)(uintptr_t)325, 0);
    fail_if(!create_default_realloc,
            "av_buffer_create default opaque realloc failed");
    uint8_t *create_default_realloc_before = create_default_realloc->data;
    ret = av_buffer_realloc(&create_default_realloc, 5);
    printf("buffer:create-default-opaque-realloc-ret|%d\n", ret);
    print_buffer_prefix("buffer:create-default-opaque-realloc",
                        create_default_realloc, 3);
    printf("buffer:create-default-opaque-realloc-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(create_default_realloc));
    printf("buffer:create-default-opaque-realloc-replaced|%d\n",
           create_default_realloc_before != create_default_realloc->data);
    av_buffer_unref(&create_default_realloc);

    reset_create_release();
    uint8_t *create_zero_readonly_data = av_malloc(1);
    fail_if(!create_zero_readonly_data,
            "av_malloc create_zero_readonly_data failed");
    create_zero_readonly_data[0] = 0xcd;
    last_create_release_size = 0;
    AVBufferRef *create_zero_readonly =
        av_buffer_create(create_zero_readonly_data, 0, test_create_free,
                         (void *)(uintptr_t)654, AV_BUFFER_FLAG_READONLY);
    fail_if(!create_zero_readonly, "av_buffer_create zero readonly failed");
    print_buffer_opaque("buffer:create-zero-readonly", create_zero_readonly);
    ret = av_buffer_make_writable(&create_zero_readonly);
    printf("buffer:create-zero-readonly-make-writable-ret|%d|%d\n",
           ret, create_release_count);
    print_buffer_opaque("buffer:create-zero-readonly-after",
                        create_zero_readonly);
    print_create_release("buffer:create-zero-readonly-release");
    av_buffer_unref(&create_zero_readonly);

    reset_create_release();
    static const uint8_t create_readonly_bytes[] = { 21, 22, 23 };
    uint8_t *create_readonly_data = av_malloc(sizeof(create_readonly_bytes));
    fail_if(!create_readonly_data, "av_malloc create_readonly_data failed");
    for (size_t i = 0; i < sizeof(create_readonly_bytes); i++)
        create_readonly_data[i] = create_readonly_bytes[i];
    last_create_release_size = sizeof(create_readonly_bytes);
    AVBufferRef *create_readonly =
        av_buffer_create(create_readonly_data, sizeof(create_readonly_bytes),
                         test_create_free, (void *)(uintptr_t)432,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!create_readonly, "av_buffer_create readonly owner failed");
    print_buffer_opaque("buffer:create-readonly", create_readonly);
    ret = av_buffer_make_writable(&create_readonly);
    printf("buffer:create-readonly-make-writable-ret|%d|%d\n",
           ret, create_release_count);
    print_buffer_opaque("buffer:create-readonly-after", create_readonly);
    print_create_release("buffer:create-readonly-release");
    av_buffer_unref(&create_readonly);

    reset_create_release();
    static const uint8_t create_readonly_shared_bytes[] = { 24, 25, 26 };
    uint8_t *create_readonly_shared_data =
        av_malloc(sizeof(create_readonly_shared_bytes));
    fail_if(!create_readonly_shared_data,
            "av_malloc create_readonly_shared_data failed");
    for (size_t i = 0; i < sizeof(create_readonly_shared_bytes); i++)
        create_readonly_shared_data[i] = create_readonly_shared_bytes[i];
    last_create_release_size = sizeof(create_readonly_shared_bytes);
    AVBufferRef *create_readonly_shared_src =
        av_buffer_create(create_readonly_shared_data,
                         sizeof(create_readonly_shared_bytes),
                         test_create_free, (void *)(uintptr_t)433,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!create_readonly_shared_src,
            "av_buffer_create readonly shared owner failed");
    AVBufferRef *create_readonly_shared_dst =
        av_buffer_ref(create_readonly_shared_src);
    fail_if(!create_readonly_shared_dst,
            "av_buffer_ref readonly shared owner failed");
    print_buffer_opaque("buffer:create-readonly-shared-ref-src",
                        create_readonly_shared_src);
    print_buffer_opaque("buffer:create-readonly-shared-ref-dst",
                        create_readonly_shared_dst);
    printf("buffer:create-readonly-shared-ref-shares|%d|%d\n",
           create_readonly_shared_src->data == create_readonly_shared_dst->data,
           av_buffer_get_ref_count(create_readonly_shared_src));
    ret = av_buffer_make_writable(&create_readonly_shared_dst);
    printf("buffer:create-readonly-shared-make-writable-ret|%d\n", ret);
    print_buffer_opaque("buffer:create-readonly-shared-src",
                        create_readonly_shared_src);
    print_buffer_opaque("buffer:create-readonly-shared-dst",
                        create_readonly_shared_dst);
    printf("buffer:create-readonly-shared-shares|%d\n",
           create_readonly_shared_src->data == create_readonly_shared_dst->data);
    av_buffer_unref(&create_readonly_shared_dst);
    printf("buffer:create-readonly-shared-release-before-src-drop|%d\n",
           create_release_count);
    av_buffer_unref(&create_readonly_shared_src);
    print_create_release("buffer:create-readonly-shared-release");

    reset_create_release();
    static const uint8_t create_shared_bytes[] = { 40, 41, 42 };
    uint8_t *create_shared_data = av_malloc(sizeof(create_shared_bytes));
    fail_if(!create_shared_data, "av_malloc create_shared_data failed");
    for (size_t i = 0; i < sizeof(create_shared_bytes); i++)
        create_shared_data[i] = create_shared_bytes[i];
    last_create_release_size = sizeof(create_shared_bytes);
    AVBufferRef *create_shared_src =
        av_buffer_create(create_shared_data, sizeof(create_shared_bytes),
                         test_create_free, (void *)(uintptr_t)456, 0);
    fail_if(!create_shared_src, "av_buffer_create shared src failed");
    AVBufferRef *create_shared_dst = av_buffer_ref(create_shared_src);
    fail_if(!create_shared_dst, "av_buffer_ref create_shared failed");
    print_buffer_opaque("buffer:create-shared-ref-src", create_shared_src);
    print_buffer_opaque("buffer:create-shared-ref-dst", create_shared_dst);
    printf("buffer:create-shared-ref-shares|%d|%d\n",
           create_shared_src->data == create_shared_dst->data,
           av_buffer_get_ref_count(create_shared_src));
    ret = av_buffer_make_writable(&create_shared_dst);
    printf("buffer:create-shared-make-writable-ret|%d\n", ret);
    print_buffer_opaque("buffer:create-shared-src", create_shared_src);
    print_buffer_opaque("buffer:create-shared-dst", create_shared_dst);
    printf("buffer:create-shared-shares|%d\n",
           create_shared_src->data == create_shared_dst->data);
    av_buffer_unref(&create_shared_dst);
    printf("buffer:create-shared-release-before-src-drop|%d\n",
           create_release_count);
    av_buffer_unref(&create_shared_src);
    print_create_release("buffer:create-shared-release");

    reset_create_release();
    static const uint8_t create_shared_realloc_bytes[] = { 44, 45, 46 };
    uint8_t *create_shared_realloc_data =
        av_malloc(sizeof(create_shared_realloc_bytes));
    fail_if(!create_shared_realloc_data,
            "av_malloc create_shared_realloc_data failed");
    for (size_t i = 0; i < sizeof(create_shared_realloc_bytes); i++)
        create_shared_realloc_data[i] = create_shared_realloc_bytes[i];
    last_create_release_size = sizeof(create_shared_realloc_bytes);
    AVBufferRef *create_shared_realloc_src =
        av_buffer_create(create_shared_realloc_data,
                         sizeof(create_shared_realloc_bytes),
                         test_create_free, (void *)(uintptr_t)567, 0);
    fail_if(!create_shared_realloc_src,
            "av_buffer_create shared realloc src failed");
    AVBufferRef *create_shared_realloc_dst =
        av_buffer_ref(create_shared_realloc_src);
    fail_if(!create_shared_realloc_dst,
            "av_buffer_ref create_shared_realloc failed");
    ret = av_buffer_realloc(&create_shared_realloc_dst, 5);
    printf("buffer:create-shared-realloc-ret|%d\n", ret);
    print_buffer_opaque("buffer:create-shared-realloc-src",
                        create_shared_realloc_src);
    print_buffer_prefix("buffer:create-shared-realloc-dst",
                        create_shared_realloc_dst, 3);
    printf("buffer:create-shared-realloc-dst-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(create_shared_realloc_dst));
    printf("buffer:create-shared-realloc-shares|%d\n",
           create_shared_realloc_src->buffer ==
               create_shared_realloc_dst->buffer);
    printf("buffer:create-shared-realloc-release-before-src-drop|%d\n",
           create_release_count);
    av_buffer_unref(&create_shared_realloc_dst);
    printf("buffer:create-shared-realloc-release-before-src-unref|%d\n",
           create_release_count);
    av_buffer_unref(&create_shared_realloc_src);
    print_create_release("buffer:create-shared-realloc-release");

    reset_create_release();
    static const uint8_t create_shared_shrink_bytes[] = { 47, 48, 49, 50 };
    uint8_t *create_shared_shrink_data =
        av_malloc(sizeof(create_shared_shrink_bytes));
    fail_if(!create_shared_shrink_data,
            "av_malloc create_shared_shrink_data failed");
    for (size_t i = 0; i < sizeof(create_shared_shrink_bytes); i++)
        create_shared_shrink_data[i] = create_shared_shrink_bytes[i];
    last_create_release_size = sizeof(create_shared_shrink_bytes);
    AVBufferRef *create_shared_shrink_src =
        av_buffer_create(create_shared_shrink_data,
                         sizeof(create_shared_shrink_bytes),
                         test_create_free, (void *)(uintptr_t)568, 0);
    fail_if(!create_shared_shrink_src,
            "av_buffer_create shared shrink src failed");
    AVBufferRef *create_shared_shrink_dst =
        av_buffer_ref(create_shared_shrink_src);
    fail_if(!create_shared_shrink_dst,
            "av_buffer_ref create_shared_shrink failed");
    ret = av_buffer_realloc(&create_shared_shrink_dst, 2);
    printf("buffer:create-shared-shrink-ret|%d\n", ret);
    print_buffer_opaque("buffer:create-shared-shrink-src",
                        create_shared_shrink_src);
    print_buffer_prefix("buffer:create-shared-shrink-dst",
                        create_shared_shrink_dst, 2);
    printf("buffer:create-shared-shrink-dst-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(create_shared_shrink_dst));
    printf("buffer:create-shared-shrink-shares|%d\n",
           create_shared_shrink_src->buffer ==
               create_shared_shrink_dst->buffer);
    printf("buffer:create-shared-shrink-release-before-src-drop|%d\n",
           create_release_count);
    av_buffer_unref(&create_shared_shrink_dst);
    printf("buffer:create-shared-shrink-release-before-src-unref|%d\n",
           create_release_count);
    av_buffer_unref(&create_shared_shrink_src);
    print_create_release("buffer:create-shared-shrink-release");

    reset_create_release();
    static const uint8_t create_realloc_bytes[] = { 50, 51, 52 };
    uint8_t *create_realloc_data = av_malloc(sizeof(create_realloc_bytes));
    fail_if(!create_realloc_data, "av_malloc create_realloc_data failed");
    for (size_t i = 0; i < sizeof(create_realloc_bytes); i++)
        create_realloc_data[i] = create_realloc_bytes[i];
    last_create_release_size = sizeof(create_realloc_bytes);
    AVBufferRef *create_realloc =
        av_buffer_create(create_realloc_data, sizeof(create_realloc_bytes),
                         test_create_free, (void *)(uintptr_t)789, 0);
    fail_if(!create_realloc, "av_buffer_create realloc failed");
    ret = av_buffer_realloc(&create_realloc, 5);
    printf("buffer:create-realloc-ret|%d\n", ret);
    print_buffer_prefix("buffer:create-realloc", create_realloc, 3);
    printf("buffer:create-realloc-opaque|%llu\n",
           (unsigned long long)(uintptr_t)av_buffer_get_opaque(create_realloc));
    printf("buffer:create-realloc-replaced|%d\n",
           create_realloc_data != create_realloc->data);
    print_create_release("buffer:create-realloc-release");
    av_buffer_unref(&create_realloc);

    reset_create_release();
    static const uint8_t create_realloc_shrink_bytes[] = { 53, 54, 55, 56 };
    uint8_t *create_realloc_shrink_data =
        av_malloc(sizeof(create_realloc_shrink_bytes));
    fail_if(!create_realloc_shrink_data,
            "av_malloc create_realloc_shrink_data failed");
    for (size_t i = 0; i < sizeof(create_realloc_shrink_bytes); i++)
        create_realloc_shrink_data[i] = create_realloc_shrink_bytes[i];
    last_create_release_size = sizeof(create_realloc_shrink_bytes);
    AVBufferRef *create_realloc_shrink =
        av_buffer_create(create_realloc_shrink_data,
                         sizeof(create_realloc_shrink_bytes),
                         test_create_free, (void *)(uintptr_t)790, 0);
    fail_if(!create_realloc_shrink,
            "av_buffer_create shrink realloc failed");
    uint8_t *create_realloc_shrink_before = create_realloc_shrink->data;
    ret = av_buffer_realloc(&create_realloc_shrink, 2);
    printf("buffer:create-realloc-shrink-ret|%d\n", ret);
    print_buffer_prefix("buffer:create-realloc-shrink",
                        create_realloc_shrink, 2);
    printf("buffer:create-realloc-shrink-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(create_realloc_shrink));
    printf("buffer:create-realloc-shrink-replaced|%d\n",
           create_realloc_shrink_before != create_realloc_shrink->data);
    print_create_release("buffer:create-realloc-shrink-release-before-unref");
    av_buffer_unref(&create_realloc_shrink);
    print_create_release("buffer:create-realloc-shrink-release-after-unref");

    static const uint8_t grow_bytes[] = { 1, 2, 3 };
    AVBufferRef *grow = av_buffer_allocz(3);
    fail_if(!grow, "av_buffer_allocz grow failed");
    fill_bytes(grow, grow_bytes, sizeof(grow_bytes));
    uint8_t *grow_data_before = grow->data;
    ret = av_buffer_realloc(&grow, 5);
    printf("buffer:realloc-grow-ret|%d\n", ret);
    print_buffer_prefix("buffer:realloc-grow", grow, 3);
    printf("buffer:realloc-grow-replaced|%d\n",
           grow_data_before != grow->data);
    ret = av_buffer_realloc(&grow, 2);
    printf("buffer:realloc-shrink-ret|%d\n", ret);
    print_buffer("buffer:realloc-shrink", grow);
    av_buffer_unref(&grow);

    static const uint8_t realloc_zero_bytes[] = { 9, 10, 11 };
    AVBufferRef *realloc_zero = av_buffer_allocz(3);
    fail_if(!realloc_zero, "av_buffer_allocz realloc_zero failed");
    fill_bytes(realloc_zero, realloc_zero_bytes, sizeof(realloc_zero_bytes));
    ret = av_buffer_realloc(&realloc_zero, 0);
    printf("buffer:realloc-zero-ret|%d\n", ret);
    print_status("buffer:realloc-zero", realloc_zero);
    av_buffer_unref(&realloc_zero);

    static const uint8_t realloc_shared_bytes[] = { 7, 7, 7 };
    AVBufferRef *realloc_src = av_buffer_allocz(3);
    fail_if(!realloc_src, "av_buffer_allocz realloc_src failed");
    fill_bytes(realloc_src, realloc_shared_bytes, sizeof(realloc_shared_bytes));
    AVBufferRef *realloc_dst = av_buffer_ref(realloc_src);
    fail_if(!realloc_dst, "av_buffer_ref realloc failed");
    ret = av_buffer_realloc(&realloc_dst, 5);
    printf("buffer:realloc-shared-ret|%d\n", ret);
    print_buffer("buffer:realloc-shared-src", realloc_src);
    print_buffer_prefix("buffer:realloc-shared-dst", realloc_dst, 3);
    printf("buffer:realloc-shared-shares|%d\n",
           realloc_src->data == realloc_dst->data);
    av_buffer_unref(&realloc_dst);
    av_buffer_unref(&realloc_src);

    static const uint8_t realloc_same_bytes[] = { 4, 6, 8 };
    AVBufferRef *realloc_same_src = av_buffer_allocz(3);
    fail_if(!realloc_same_src, "av_buffer_allocz realloc_same_src failed");
    fill_bytes(realloc_same_src, realloc_same_bytes,
               sizeof(realloc_same_bytes));
    AVBufferRef *realloc_same_dst = av_buffer_ref(realloc_same_src);
    fail_if(!realloc_same_dst, "av_buffer_ref realloc_same failed");
    ret = av_buffer_realloc(&realloc_same_dst, realloc_same_dst->size);
    printf("buffer:realloc-same-shared-ret|%d\n", ret);
    print_buffer("buffer:realloc-same-shared-src", realloc_same_src);
    print_buffer("buffer:realloc-same-shared-dst", realloc_same_dst);
    printf("buffer:realloc-same-shared-shares|%d|%d\n",
           realloc_same_src->data == realloc_same_dst->data,
           av_buffer_get_ref_count(realloc_same_src));
    av_buffer_unref(&realloc_same_dst);
    av_buffer_unref(&realloc_same_src);

    reset_create_release();
    static const uint8_t create_realloc_same_bytes[] = { 60, 61, 62 };
    uint8_t *create_realloc_same_data =
        av_malloc(sizeof(create_realloc_same_bytes));
    fail_if(!create_realloc_same_data,
            "av_malloc create_realloc_same_data failed");
    for (size_t i = 0; i < sizeof(create_realloc_same_bytes); i++)
        create_realloc_same_data[i] = create_realloc_same_bytes[i];
    last_create_release_size = sizeof(create_realloc_same_bytes);
    AVBufferRef *create_realloc_same =
        av_buffer_create(create_realloc_same_data,
                         sizeof(create_realloc_same_bytes),
                         test_create_free, (void *)(uintptr_t)654, 0);
    fail_if(!create_realloc_same, "av_buffer_create realloc same failed");
    uint8_t *create_realloc_same_before = create_realloc_same->data;
    ret = av_buffer_realloc(&create_realloc_same,
                            create_realloc_same->size);
    printf("buffer:create-realloc-same-ret|%d\n", ret);
    print_buffer_opaque("buffer:create-realloc-same", create_realloc_same);
    printf("buffer:create-realloc-same-sameptr|%d\n",
           create_realloc_same_before == create_realloc_same->data);
    printf("buffer:create-realloc-same-release-before-unref|%d\n",
           create_release_count);
    av_buffer_unref(&create_realloc_same);
    print_create_release("buffer:create-realloc-same-release");

    reset_create_release();
    static const uint8_t readonly_realloc_same_bytes[] = { 70, 71, 72 };
    uint8_t *readonly_realloc_same_data =
        av_malloc(sizeof(readonly_realloc_same_bytes));
    fail_if(!readonly_realloc_same_data,
            "av_malloc readonly_realloc_same_data failed");
    for (size_t i = 0; i < sizeof(readonly_realloc_same_bytes); i++)
        readonly_realloc_same_data[i] = readonly_realloc_same_bytes[i];
    last_create_release_size = sizeof(readonly_realloc_same_bytes);
    AVBufferRef *readonly_realloc_same =
        av_buffer_create(readonly_realloc_same_data,
                         sizeof(readonly_realloc_same_bytes),
                         test_create_free, (void *)(uintptr_t)88,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly_realloc_same, "av_buffer_create readonly realloc same failed");
    uint8_t *readonly_realloc_same_before = readonly_realloc_same->data;
    ret = av_buffer_realloc(&readonly_realloc_same,
                            readonly_realloc_same->size);
    printf("buffer:readonly-realloc-same-ret|%d\n", ret);
    print_buffer_opaque("buffer:readonly-realloc-same",
                        readonly_realloc_same);
    printf("buffer:readonly-realloc-same-sameptr|%d\n",
           readonly_realloc_same_before == readonly_realloc_same->data);
    printf("buffer:readonly-realloc-same-release-before-unref|%d\n",
           create_release_count);
    av_buffer_unref(&readonly_realloc_same);
    print_create_release("buffer:readonly-realloc-same-release");

    reset_create_release();
    static const uint8_t readonly_realloc_bytes[] = { 90, 91, 92 };
    uint8_t *readonly_realloc_data =
        av_malloc(sizeof(readonly_realloc_bytes));
    fail_if(!readonly_realloc_data,
            "av_malloc readonly_realloc_data failed");
    for (size_t i = 0; i < sizeof(readonly_realloc_bytes); i++)
        readonly_realloc_data[i] = readonly_realloc_bytes[i];
    last_create_release_size = sizeof(readonly_realloc_bytes);
    AVBufferRef *readonly_realloc =
        av_buffer_create(readonly_realloc_data,
                         sizeof(readonly_realloc_bytes),
                         test_create_free, (void *)(uintptr_t)889,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly_realloc, "av_buffer_create readonly realloc failed");
    uint8_t *readonly_realloc_before = readonly_realloc->data;
    ret = av_buffer_realloc(&readonly_realloc, 5);
    printf("buffer:readonly-realloc-ret|%d\n", ret);
    print_buffer_prefix("buffer:readonly-realloc", readonly_realloc, 3);
    printf("buffer:readonly-realloc-opaque|%llu\n",
           (unsigned long long)(uintptr_t)av_buffer_get_opaque(readonly_realloc));
    printf("buffer:readonly-realloc-replaced|%d\n",
           readonly_realloc_before != readonly_realloc->data);
    print_create_release("buffer:readonly-realloc-release-before-unref");
    av_buffer_unref(&readonly_realloc);
    print_create_release("buffer:readonly-realloc-release-after-unref");

    reset_create_release();
    static const uint8_t readonly_realloc_shrink_bytes[] = { 93, 94, 95, 96 };
    uint8_t *readonly_realloc_shrink_data =
        av_malloc(sizeof(readonly_realloc_shrink_bytes));
    fail_if(!readonly_realloc_shrink_data,
            "av_malloc readonly_realloc_shrink_data failed");
    for (size_t i = 0; i < sizeof(readonly_realloc_shrink_bytes); i++)
        readonly_realloc_shrink_data[i] = readonly_realloc_shrink_bytes[i];
    last_create_release_size = sizeof(readonly_realloc_shrink_bytes);
    AVBufferRef *readonly_realloc_shrink =
        av_buffer_create(readonly_realloc_shrink_data,
                         sizeof(readonly_realloc_shrink_bytes),
                         test_create_free, (void *)(uintptr_t)890,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly_realloc_shrink,
            "av_buffer_create readonly shrink realloc failed");
    uint8_t *readonly_realloc_shrink_before = readonly_realloc_shrink->data;
    ret = av_buffer_realloc(&readonly_realloc_shrink, 2);
    printf("buffer:readonly-realloc-shrink-ret|%d\n", ret);
    print_buffer_prefix("buffer:readonly-realloc-shrink",
                        readonly_realloc_shrink, 2);
    printf("buffer:readonly-realloc-shrink-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(readonly_realloc_shrink));
    printf("buffer:readonly-realloc-shrink-replaced|%d\n",
           readonly_realloc_shrink_before != readonly_realloc_shrink->data);
    print_create_release("buffer:readonly-realloc-shrink-release-before-unref");
    av_buffer_unref(&readonly_realloc_shrink);
    print_create_release("buffer:readonly-realloc-shrink-release-after-unref");

    reset_create_release();
    static const uint8_t readonly_shared_realloc_bytes[] = { 80, 81, 82 };
    uint8_t *readonly_shared_realloc_data =
        av_malloc(sizeof(readonly_shared_realloc_bytes));
    fail_if(!readonly_shared_realloc_data,
            "av_malloc readonly_shared_realloc_data failed");
    for (size_t i = 0; i < sizeof(readonly_shared_realloc_bytes); i++)
        readonly_shared_realloc_data[i] = readonly_shared_realloc_bytes[i];
    last_create_release_size = sizeof(readonly_shared_realloc_bytes);
    AVBufferRef *readonly_shared_realloc_src =
        av_buffer_create(readonly_shared_realloc_data,
                         sizeof(readonly_shared_realloc_bytes),
                         test_create_free, (void *)(uintptr_t)998,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly_shared_realloc_src,
            "av_buffer_create readonly shared realloc failed");
    AVBufferRef *readonly_shared_realloc_dst =
        av_buffer_ref(readonly_shared_realloc_src);
    fail_if(!readonly_shared_realloc_dst,
            "av_buffer_ref readonly shared realloc failed");
    ret = av_buffer_realloc(&readonly_shared_realloc_dst, 5);
    printf("buffer:readonly-shared-realloc-ret|%d\n", ret);
    print_buffer_opaque("buffer:readonly-shared-realloc-src",
                        readonly_shared_realloc_src);
    print_buffer_prefix("buffer:readonly-shared-realloc-dst",
                        readonly_shared_realloc_dst, 3);
    printf("buffer:readonly-shared-realloc-dst-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(readonly_shared_realloc_dst));
    printf("buffer:readonly-shared-realloc-shares|%d\n",
           readonly_shared_realloc_src->buffer ==
               readonly_shared_realloc_dst->buffer);
    printf("buffer:readonly-shared-realloc-release-before-src-drop|%d\n",
           create_release_count);
    av_buffer_unref(&readonly_shared_realloc_dst);
    printf("buffer:readonly-shared-realloc-release-before-src-unref|%d\n",
           create_release_count);
    av_buffer_unref(&readonly_shared_realloc_src);
    print_create_release("buffer:readonly-shared-realloc-release");

    reset_create_release();
    static const uint8_t readonly_shared_shrink_bytes[] = { 83, 84, 85, 86 };
    uint8_t *readonly_shared_shrink_data =
        av_malloc(sizeof(readonly_shared_shrink_bytes));
    fail_if(!readonly_shared_shrink_data,
            "av_malloc readonly_shared_shrink_data failed");
    for (size_t i = 0; i < sizeof(readonly_shared_shrink_bytes); i++)
        readonly_shared_shrink_data[i] = readonly_shared_shrink_bytes[i];
    last_create_release_size = sizeof(readonly_shared_shrink_bytes);
    AVBufferRef *readonly_shared_shrink_src =
        av_buffer_create(readonly_shared_shrink_data,
                         sizeof(readonly_shared_shrink_bytes),
                         test_create_free, (void *)(uintptr_t)1002,
                         AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly_shared_shrink_src,
            "av_buffer_create readonly shared shrink failed");
    AVBufferRef *readonly_shared_shrink_dst =
        av_buffer_ref(readonly_shared_shrink_src);
    fail_if(!readonly_shared_shrink_dst,
            "av_buffer_ref readonly shared shrink failed");
    ret = av_buffer_realloc(&readonly_shared_shrink_dst, 2);
    printf("buffer:readonly-shared-shrink-ret|%d\n", ret);
    print_buffer_opaque("buffer:readonly-shared-shrink-src",
                        readonly_shared_shrink_src);
    print_buffer_prefix("buffer:readonly-shared-shrink-dst",
                        readonly_shared_shrink_dst, 2);
    printf("buffer:readonly-shared-shrink-dst-opaque|%llu\n",
           (unsigned long long)(uintptr_t)
               av_buffer_get_opaque(readonly_shared_shrink_dst));
    printf("buffer:readonly-shared-shrink-shares|%d\n",
           readonly_shared_shrink_src->buffer ==
               readonly_shared_shrink_dst->buffer);
    printf("buffer:readonly-shared-shrink-release-before-src-drop|%d\n",
           create_release_count);
    av_buffer_unref(&readonly_shared_shrink_dst);
    printf("buffer:readonly-shared-shrink-release-before-src-unref|%d\n",
           create_release_count);
    av_buffer_unref(&readonly_shared_shrink_src);
    print_create_release("buffer:readonly-shared-shrink-release");

    static const uint8_t replace_src_bytes[] = { 3, 4, 5 };
    AVBufferRef *replace_src = av_buffer_allocz(3);
    AVBufferRef *replace_dst = av_buffer_allocz(2);
    fail_if(!replace_src || !replace_dst, "av_buffer_allocz replace failed");
    fill_bytes(replace_src, replace_src_bytes, sizeof(replace_src_bytes));
    ret = av_buffer_replace(&replace_dst, replace_src);
    printf("buffer:replace-ret|%d\n", ret);
    print_buffer("buffer:replace-src", replace_src);
    print_buffer("buffer:replace-dst", replace_dst);
    printf("buffer:replace-shares|%d\n", replace_src->data == replace_dst->data);
    av_buffer_unref(&replace_dst);
    av_buffer_unref(&replace_src);

    AVBufferRef *replace_null_src = av_buffer_allocz(2);
    fail_if(!replace_null_src, "av_buffer_allocz replace_null_src failed");
    ret = av_buffer_replace(&replace_null_src, NULL);
    printf("buffer:replace-null-src|%d|%d\n", ret, replace_null_src == NULL);

    AVBufferRef *replace_null_null = NULL;
    ret = av_buffer_replace(&replace_null_null, NULL);
    printf("buffer:replace-null-null|%d|%d\n", ret, replace_null_null == NULL);

    static const uint8_t replace_null_source_bytes[] = { 6, 7, 8 };
    AVBufferRef *replace_null_source = av_buffer_allocz(3);
    AVBufferRef *replace_null_dst = NULL;
    fail_if(!replace_null_source, "av_buffer_allocz replace_null_source failed");
    fill_bytes(replace_null_source, replace_null_source_bytes,
               sizeof(replace_null_source_bytes));
    ret = av_buffer_replace(&replace_null_dst, replace_null_source);
    fail_if(ret < 0 || !replace_null_dst, "av_buffer_replace null dst failed");
    print_buffer("buffer:replace-null-dst", replace_null_dst);
    printf("buffer:replace-null-dst-shares|%d\n",
           replace_null_source->data == replace_null_dst->data);
    av_buffer_unref(&replace_null_dst);
    av_buffer_unref(&replace_null_source);

    static const uint8_t replace_equiv_bytes[] = { 1, 4, 9 };
    AVBufferRef *replace_equiv_src = av_buffer_allocz(3);
    fail_if(!replace_equiv_src, "av_buffer_allocz replace_equiv_src failed");
    fill_bytes(replace_equiv_src, replace_equiv_bytes,
               sizeof(replace_equiv_bytes));
    AVBufferRef *replace_equiv_dst = av_buffer_ref(replace_equiv_src);
    fail_if(!replace_equiv_dst, "av_buffer_ref replace_equiv failed");
    ret = av_buffer_replace(&replace_equiv_dst, replace_equiv_src);
    printf("buffer:replace-equivalent-ret|%d|%d|%d\n",
           ret, av_buffer_get_ref_count(replace_equiv_src),
           replace_equiv_src->data == replace_equiv_dst->data);
    av_buffer_unref(&replace_equiv_dst);
    av_buffer_unref(&replace_equiv_src);

    static const uint8_t replace_self_bytes[] = { 2, 4, 6 };
    AVBufferRef *replace_self = av_buffer_allocz(3);
    fail_if(!replace_self, "av_buffer_allocz replace_self failed");
    fill_bytes(replace_self, replace_self_bytes, sizeof(replace_self_bytes));
    uint8_t *replace_self_data = replace_self->data;
    ret = av_buffer_replace(&replace_self, replace_self);
    printf("buffer:replace-self-ret|%d|%d\n",
           ret, replace_self->data == replace_self_data);
    print_buffer("buffer:replace-self", replace_self);
    av_buffer_unref(&replace_self);

    AVBufferRef *unref_null_input = NULL;
    av_buffer_unref(NULL);
    av_buffer_unref(&unref_null_input);
    printf("buffer:unref-null-input|%d\n", unref_null_input == NULL);

    AVBufferRef *realloc_null = NULL;
    ret = av_buffer_realloc(&realloc_null, 4);
    printf("buffer:realloc-null-ret|%d\n", ret);
    print_status("buffer:realloc-null", realloc_null);
    ret = av_buffer_realloc(&realloc_null, 6);
    printf("buffer:realloc-null-grow-ret|%d\n", ret);
    print_status("buffer:realloc-null-grow", realloc_null);
    av_buffer_unref(&realloc_null);

    AVBufferRef *realloc_null_zero = NULL;
    ret = av_buffer_realloc(&realloc_null_zero, 0);
    printf("buffer:realloc-null-zero-ret|%d\n", ret);
    print_status("buffer:realloc-null-zero", realloc_null_zero);
    av_buffer_unref(&realloc_null_zero);

    static const uint8_t realloc_invalid_bytes[] = { 91, 92, 93 };
    AVBufferRef *realloc_invalid = av_buffer_allocz(3);
    fail_if(!realloc_invalid, "av_buffer_allocz realloc_invalid failed");
    fill_bytes(realloc_invalid, realloc_invalid_bytes,
               sizeof(realloc_invalid_bytes));
    ret = av_buffer_realloc(&realloc_invalid, SIZE_MAX);
    printf("buffer:realloc-invalid-huge-ret|%d\n", ret);
    print_buffer("buffer:realloc-invalid-huge", realloc_invalid);
    av_buffer_unref(&realloc_invalid);

    AVBufferRef *realloc_null_invalid = NULL;
    ret = av_buffer_realloc(&realloc_null_invalid, SIZE_MAX);
    printf("buffer:realloc-null-invalid-huge|%d|%d\n",
           ret, realloc_null_invalid == NULL);
    av_buffer_unref(&realloc_null_invalid);

    static const uint8_t offset_bytes[] = { 10, 11, 12, 13 };
    AVBufferRef *offset_src = av_buffer_allocz(4);
    fail_if(!offset_src, "av_buffer_allocz offset_src failed");
    fill_bytes(offset_src, offset_bytes, sizeof(offset_bytes));
    AVBufferRef *offset_ref = av_buffer_ref(offset_src);
    fail_if(!offset_ref, "av_buffer_ref offset_ref failed");
    offset_ref->data += 1;
    offset_ref->size = 2;
    print_buffer("buffer:offset-ref-src", offset_src);
    print_buffer("buffer:offset-ref-view", offset_ref);
    printf("buffer:offset-ref-delta|%td\n", offset_ref->data - offset_src->data);

    AVBufferRef *offset_ref_clone = av_buffer_ref(offset_ref);
    fail_if(!offset_ref_clone, "av_buffer_ref offset_ref_clone failed");
    print_buffer("buffer:offset-ref-clone", offset_ref_clone);
    printf("buffer:offset-ref-clone-shape|%d|%d|%td|%d\n",
           offset_ref_clone->buffer == offset_ref->buffer,
           offset_ref_clone->data == offset_ref->data,
           offset_ref_clone->data - offset_src->data,
           av_buffer_get_ref_count(offset_ref));
    av_buffer_unref(&offset_ref_clone);

    AVBufferRef *offset_make_writable = av_buffer_ref(offset_ref);
    fail_if(!offset_make_writable, "av_buffer_ref offset_make_writable failed");
    ret = av_buffer_make_writable(&offset_make_writable);
    fail_if(ret < 0, "av_buffer_make_writable offset failed");
    print_buffer("buffer:offset-make-writable", offset_make_writable);
    printf("buffer:offset-make-writable-shares|%d\n",
           offset_make_writable->data == offset_ref->data);
    av_buffer_unref(&offset_make_writable);

    AVBufferRef *offset_realloc = av_buffer_ref(offset_src);
    fail_if(!offset_realloc, "av_buffer_ref offset_realloc failed");
    offset_realloc->data += 1;
    offset_realloc->size = 2;
    ret = av_buffer_realloc(&offset_realloc, 3);
    fail_if(ret < 0, "av_buffer_realloc offset failed");
    print_buffer_prefix("buffer:offset-realloc-grow", offset_realloc, 2);
    printf("buffer:offset-realloc-shares|%d\n",
           offset_realloc->data == offset_src->data + 1);
    av_buffer_unref(&offset_realloc);
    av_buffer_unref(&offset_ref);
    av_buffer_unref(&offset_src);

    static const uint8_t offset_unique_bytes[] = { 30, 31, 32, 33 };
    AVBufferRef *offset_unique_base = av_buffer_allocz(4);
    fail_if(!offset_unique_base, "av_buffer_allocz offset_unique_base failed");
    fill_bytes(offset_unique_base, offset_unique_bytes, sizeof(offset_unique_bytes));
    AVBufferRef *offset_unique_make_writable = av_buffer_ref(offset_unique_base);
    fail_if(!offset_unique_make_writable,
            "av_buffer_ref offset_unique_make_writable failed");
    uint8_t *offset_unique_base_data = offset_unique_base->data;
    av_buffer_unref(&offset_unique_base);
    offset_unique_make_writable->data += 1;
    offset_unique_make_writable->size = 2;
    uint8_t *offset_unique_before = offset_unique_make_writable->data;
    ret = av_buffer_make_writable(&offset_unique_make_writable);
    fail_if(ret < 0, "av_buffer_make_writable unique offset failed");
    printf("buffer:offset-unique-make-writable-ret|%d|%d|%td\n",
           ret, offset_unique_before == offset_unique_make_writable->data,
           offset_unique_make_writable->data - offset_unique_base_data);
    print_buffer("buffer:offset-unique-make-writable",
                 offset_unique_make_writable);
    av_buffer_unref(&offset_unique_make_writable);

    static const uint8_t offset_unique_realloc_bytes[] = { 34, 35, 36, 37 };
    AVBufferRef *offset_unique_realloc_base = av_buffer_allocz(4);
    fail_if(!offset_unique_realloc_base,
            "av_buffer_allocz offset_unique_realloc_base failed");
    fill_bytes(offset_unique_realloc_base, offset_unique_realloc_bytes,
               sizeof(offset_unique_realloc_bytes));
    AVBufferRef *offset_unique_realloc =
        av_buffer_ref(offset_unique_realloc_base);
    fail_if(!offset_unique_realloc,
            "av_buffer_ref offset_unique_realloc failed");
    av_buffer_unref(&offset_unique_realloc_base);
    offset_unique_realloc->data += 1;
    offset_unique_realloc->size = 2;
    uint8_t *offset_unique_realloc_before = offset_unique_realloc->data;
    ret = av_buffer_realloc(&offset_unique_realloc, 3);
    fail_if(ret < 0, "av_buffer_realloc unique offset failed");
    printf("buffer:offset-unique-realloc-ret|%d\n", ret);
    print_buffer_prefix("buffer:offset-unique-realloc",
                        offset_unique_realloc, 2);
    printf("buffer:offset-unique-realloc-replaced|%d|%d\n",
           offset_unique_realloc_before != offset_unique_realloc->data,
           0);
    av_buffer_unref(&offset_unique_realloc);

    static const uint8_t replace_offset_bytes[] = { 21, 22, 23, 24 };
    AVBufferRef *replace_offset_base = av_buffer_allocz(4);
    fail_if(!replace_offset_base, "av_buffer_allocz replace_offset_base failed");
    fill_bytes(replace_offset_base, replace_offset_bytes,
               sizeof(replace_offset_bytes));
    AVBufferRef *replace_offset_src = av_buffer_ref(replace_offset_base);
    AVBufferRef *replace_offset_dst = av_buffer_ref(replace_offset_base);
    fail_if(!replace_offset_src || !replace_offset_dst,
            "av_buffer_ref replace_offset failed");
    replace_offset_src->data += 1;
    replace_offset_src->size = 2;
    av_buffer_unref(&replace_offset_base);
    ret = av_buffer_replace(&replace_offset_dst, replace_offset_src);
    fail_if(ret < 0, "av_buffer_replace offset equivalent failed");
    print_buffer("buffer:replace-offset-equivalent", replace_offset_dst);
    printf("buffer:replace-offset-equivalent-shares|%d|%d\n",
           replace_offset_dst->data == replace_offset_src->data,
           av_buffer_get_ref_count(replace_offset_src));
    av_buffer_unref(&replace_offset_dst);
    av_buffer_unref(&replace_offset_src);

    buf = av_buffer_allocz(1);
    fail_if(!buf, "av_buffer_allocz unref failed");
    av_buffer_unref(&buf);
    printf("buffer:unref-null|%d\n", buf == NULL);
    av_buffer_unref(&buf);
    printf("buffer:unref-null-repeat|%d\n", buf == NULL);

    AVBufferPool *pool_null = NULL;
    av_buffer_pool_uninit(&pool_null);
    printf("pool:uninit-null|%d\n", pool_null == NULL);

    AVBufferPool *zero_pool = av_buffer_pool_init(0, NULL);
    fail_if(!zero_pool, "av_buffer_pool_init zero failed");
    AVBufferRef *zero_first = av_buffer_pool_get(zero_pool);
    fail_if(!zero_first, "av_buffer_pool_get zero first failed");
    print_buffer("pool-zero:first", zero_first);
    printf("pool-zero:first-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(zero_first) == NULL);
    av_buffer_unref(&zero_first);
    AVBufferRef *zero_reuse = av_buffer_pool_get(zero_pool);
    fail_if(!zero_reuse, "av_buffer_pool_get zero reuse failed");
    print_buffer("pool-zero:reuse", zero_reuse);
    printf("pool-zero:reuse-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(zero_reuse) == NULL);
    av_buffer_unref(&zero_reuse);
    av_buffer_pool_uninit(&zero_pool);

    AVBufferPool *default_pool = av_buffer_pool_init(3, NULL);
    fail_if(!default_pool, "av_buffer_pool_init default failed");
    AVBufferRef *default_first = av_buffer_pool_get(default_pool);
    fail_if(!default_first, "av_buffer_pool_get default first failed");
    print_status("pool-default:first-status", default_first);
    printf("pool-default:first-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(default_first) == NULL);
    static const uint8_t default_pool_mutated[] = { 0x21, 0x22, 0x23 };
    fill_bytes(default_first, default_pool_mutated,
               sizeof(default_pool_mutated));
    av_buffer_unref(&default_first);
    AVBufferRef *default_reuse = av_buffer_pool_get(default_pool);
    fail_if(!default_reuse, "av_buffer_pool_get default reuse failed");
    print_buffer("pool-default:reuse", default_reuse);
    printf("pool-default:reuse-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(default_reuse) == NULL);
    av_buffer_unref(&default_reuse);
    av_buffer_pool_uninit(&default_pool);

    reset_pool_counters();
    PoolOpaque init2_default_opaque = { 88, 2 };
    AVBufferPool *init2_default_pool =
        av_buffer_pool_init2(2, &init2_default_opaque, NULL,
                             test_pool_owner_free);
    fail_if(!init2_default_pool, "av_buffer_pool_init2 default failed");
    AVBufferRef *init2_default_first = av_buffer_pool_get(init2_default_pool);
    fail_if(!init2_default_first,
            "av_buffer_pool_get init2 default first failed");
    print_status("pool-init2-default:first-status", init2_default_first);
    printf("pool-init2-default:first-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(init2_default_first) == NULL);
    static const uint8_t init2_default_mutated[] = { 0x31, 0x32 };
    fill_bytes(init2_default_first, init2_default_mutated,
               sizeof(init2_default_mutated));
    av_buffer_unref(&init2_default_first);
    AVBufferRef *init2_default_reuse = av_buffer_pool_get(init2_default_pool);
    fail_if(!init2_default_reuse,
            "av_buffer_pool_get init2 default reuse failed");
    print_buffer("pool-init2-default:reuse", init2_default_reuse);
    printf("pool-init2-default:reuse-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(init2_default_reuse) == NULL);
    av_buffer_unref(&init2_default_reuse);
    av_buffer_pool_uninit(&init2_default_pool);
    printf("pool-init2-default:pool-free|%d|%" PRIuPTR "\n",
           pool_free_count, last_pool_free_id);

    reset_pool_counters();
    AVBufferPool *legacy_pool =
        av_buffer_pool_init(3, test_pool_alloc_legacy);
    fail_if(!legacy_pool, "av_buffer_pool_init legacy failed");
    AVBufferRef *legacy_first = av_buffer_pool_get(legacy_pool);
    fail_if(!legacy_first, "av_buffer_pool_get legacy first failed");
    print_buffer("pool-legacy-custom:first", legacy_first);
    printf("pool-legacy-custom:first-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(legacy_first) == NULL);
    static const uint8_t legacy_mutated[] = { 0xa1, 0xa2, 0xa3 };
    fill_bytes(legacy_first, legacy_mutated, sizeof(legacy_mutated));
    av_buffer_unref(&legacy_first);
    AVBufferRef *legacy_reuse = av_buffer_pool_get(legacy_pool);
    fail_if(!legacy_reuse, "av_buffer_pool_get legacy reuse failed");
    print_buffer("pool-legacy-custom:reuse", legacy_reuse);
    printf("pool-legacy-custom:reuse-opaque|%d\n",
           av_buffer_pool_buffer_get_opaque(legacy_reuse) == NULL);
    printf("pool-legacy-custom:reuse-allocs|%d\n", pool_alloc_count);
    av_buffer_unref(&legacy_reuse);
    av_buffer_pool_uninit(&legacy_pool);
    printf("pool-legacy-custom:uninit-release|%d|",
           pool_release_count);
    print_hex(last_pool_release, last_pool_release_size);
    printf("\n");

    reset_pool_counters();
    PoolOpaque multi_spare_opaque = { 70, 2 };
    AVBufferPool *multi_spare_pool =
        av_buffer_pool_init2(2, &multi_spare_opaque,
                             test_pool_alloc_multi_spare, NULL);
    fail_if(!multi_spare_pool, "av_buffer_pool_init2 multi-spare failed");
    AVBufferRef *multi_spare_first = av_buffer_pool_get(multi_spare_pool);
    AVBufferRef *multi_spare_second = av_buffer_pool_get(multi_spare_pool);
    fail_if(!multi_spare_first || !multi_spare_second,
            "av_buffer_pool_get multi-spare failed");
    static const uint8_t multi_spare_first_bytes[] = { 0xa1, 0xa2 };
    static const uint8_t multi_spare_second_bytes[] = { 0xb1, 0xb2 };
    fill_bytes(multi_spare_first, multi_spare_first_bytes,
               sizeof(multi_spare_first_bytes));
    fill_bytes(multi_spare_second, multi_spare_second_bytes,
               sizeof(multi_spare_second_bytes));
    av_buffer_unref(&multi_spare_first);
    av_buffer_unref(&multi_spare_second);
    printf("pool-multi-spare:after-drop|%d\n", pool_release_count);
    AVBufferRef *multi_spare_reuse_first =
        av_buffer_pool_get(multi_spare_pool);
    AVBufferRef *multi_spare_reuse_second =
        av_buffer_pool_get(multi_spare_pool);
    fail_if(!multi_spare_reuse_first || !multi_spare_reuse_second,
            "av_buffer_pool_get multi-spare reuse failed");
    print_buffer("pool-multi-spare:reuse-first",
                 multi_spare_reuse_first);
    print_buffer("pool-multi-spare:reuse-second",
                 multi_spare_reuse_second);
    printf("pool-multi-spare:reuse-allocs|%d\n", pool_alloc_count);
    av_buffer_unref(&multi_spare_reuse_first);
    av_buffer_unref(&multi_spare_reuse_second);
    av_buffer_pool_uninit(&multi_spare_pool);
    printf("pool-multi-spare:uninit-releases|%d|",
           pool_release_count);
    print_hex(pool_release_data[0], pool_release_sizes[0]);
    printf("|");
    print_hex(pool_release_data[1], pool_release_sizes[1]);
    printf("\n");

    reset_pool_counters();
    PoolOpaque pool_opaque = { 55, 3 };
    AVBufferPool *pool = av_buffer_pool_init2(3, &pool_opaque,
                                              test_pool_alloc, test_pool_owner_free);
    fail_if(!pool, "av_buffer_pool_init2 failed");
    AVBufferRef *pool_first = av_buffer_pool_get(pool);
    fail_if(!pool_first, "av_buffer_pool_get first failed");
    print_buffer("pool:first", pool_first);
    PoolOpaque *pool_first_opaque = av_buffer_pool_buffer_get_opaque(pool_first);
    fail_if(!pool_first_opaque, "pool first opaque missing");
    printf("pool:opaque-first|%" PRIuPTR "|%zu\n",
           pool_first_opaque->id, pool_first_opaque->size);
    static const uint8_t pool_mutated[] = { 0xaa, 0xbb, 0xcc };
    fill_bytes(pool_first, pool_mutated, sizeof(pool_mutated));
    av_buffer_unref(&pool_first);
    AVBufferRef *pool_reuse = av_buffer_pool_get(pool);
    fail_if(!pool_reuse, "av_buffer_pool_get reuse failed");
    print_buffer("pool:reuse", pool_reuse);
    PoolOpaque *pool_reuse_opaque = av_buffer_pool_buffer_get_opaque(pool_reuse);
    fail_if(!pool_reuse_opaque, "pool reuse opaque missing");
    printf("pool:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_reuse_opaque->id, pool_reuse_opaque->size);
    printf("pool:reuse-allocs|%d\n", pool_alloc_count);
    av_buffer_unref(&pool_reuse);
    av_buffer_pool_uninit(&pool);
    printf("pool:uninit-releases|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("\n");
    printf("pool:uninit-pool-free|%d\n", pool_free_count);

    reset_pool_counters();
    PoolOpaque pool_unique_writable_opaque = { 62, 3 };
    AVBufferPool *pool_unique_writable =
        av_buffer_pool_init2(3, &pool_unique_writable_opaque,
                             test_pool_alloc, test_pool_owner_free);
    fail_if(!pool_unique_writable,
            "av_buffer_pool_init2 unique writable failed");
    AVBufferRef *pool_unique_writable_ref =
        av_buffer_pool_get(pool_unique_writable);
    fail_if(!pool_unique_writable_ref,
            "av_buffer_pool_get unique writable failed");
    static const uint8_t pool_unique_writable_bytes[] = { 0x62, 0x63, 0x64 };
    fill_bytes(pool_unique_writable_ref, pool_unique_writable_bytes,
               sizeof(pool_unique_writable_bytes));
    uint8_t *pool_unique_writable_data = pool_unique_writable_ref->data;
    ret = av_buffer_make_writable(&pool_unique_writable_ref);
    printf("pool-unique-writable:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_make_writable unique pool failed");
    print_buffer("pool-unique-writable:after", pool_unique_writable_ref);
    PoolOpaque *pool_unique_writable_ref_opaque =
        av_buffer_pool_buffer_get_opaque(pool_unique_writable_ref);
    fail_if(!pool_unique_writable_ref_opaque,
            "pool unique writable opaque missing");
    printf("pool-unique-writable:opaque|%" PRIuPTR "|%zu\n",
           pool_unique_writable_ref_opaque->id,
           pool_unique_writable_ref_opaque->size);
    printf("pool-unique-writable:same-data|%d\n",
           pool_unique_writable_ref->data == pool_unique_writable_data);
    printf("pool-unique-writable:after-make-writable|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_unique_writable_ref);
    printf("pool-unique-writable:after-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_unique_writable_reuse =
        av_buffer_pool_get(pool_unique_writable);
    fail_if(!pool_unique_writable_reuse,
            "av_buffer_pool_get unique writable reuse failed");
    print_buffer("pool-unique-writable:reuse", pool_unique_writable_reuse);
    PoolOpaque *pool_unique_writable_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_unique_writable_reuse);
    fail_if(!pool_unique_writable_reuse_opaque,
            "pool unique writable reuse opaque missing");
    printf("pool-unique-writable:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_unique_writable_reuse_opaque->id,
           pool_unique_writable_reuse_opaque->size);
    av_buffer_unref(&pool_unique_writable_reuse);
    av_buffer_pool_uninit(&pool_unique_writable);
    printf("pool-unique-writable:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_cow_opaque = { 56, 3 };
    AVBufferPool *pool_cow =
        av_buffer_pool_init2(3, &pool_cow_opaque, test_pool_alloc,
                             test_pool_owner_free);
    fail_if(!pool_cow, "av_buffer_pool_init2 cow failed");
    AVBufferRef *pool_cow_src = av_buffer_pool_get(pool_cow);
    fail_if(!pool_cow_src, "av_buffer_pool_get cow source failed");
    AVBufferRef *pool_cow_dst = av_buffer_ref(pool_cow_src);
    fail_if(!pool_cow_dst, "av_buffer_ref cow destination failed");
    ret = av_buffer_make_writable(&pool_cow_dst);
    printf("pool-cow:make-writable-ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_make_writable pool cow failed");
    print_buffer("pool-cow:src", pool_cow_src);
    print_buffer("pool-cow:dst", pool_cow_dst);
    printf("pool-cow:dst-opaque-null|%d\n",
           av_buffer_get_opaque(pool_cow_dst) == NULL);
    printf("pool-cow:shares|%d\n", pool_cow_src->data == pool_cow_dst->data);
    static const uint8_t pool_cow_mutated[] = { 0xab, 0xbc, 0xcd };
    fill_bytes(pool_cow_dst, pool_cow_mutated, sizeof(pool_cow_mutated));
    print_buffer("pool-cow:dst-mutated", pool_cow_dst);
    av_buffer_unref(&pool_cow_dst);
    printf("pool-cow:after-dst-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_cow_src);
    printf("pool-cow:after-src-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_cow_reuse = av_buffer_pool_get(pool_cow);
    fail_if(!pool_cow_reuse, "av_buffer_pool_get cow reuse failed");
    print_buffer("pool-cow:reuse", pool_cow_reuse);
    PoolOpaque *pool_cow_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_cow_reuse);
    fail_if(!pool_cow_reuse_opaque, "pool cow reuse opaque missing");
    printf("pool-cow:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_cow_reuse_opaque->id, pool_cow_reuse_opaque->size);
    av_buffer_unref(&pool_cow_reuse);
    av_buffer_pool_uninit(&pool_cow);
    printf("pool-cow:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_realloc_opaque = { 57, 3 };
    AVBufferPool *pool_realloc_pool =
        av_buffer_pool_init2(3, &pool_realloc_opaque, test_pool_alloc,
                             test_pool_owner_free);
    fail_if(!pool_realloc_pool, "av_buffer_pool_init2 realloc failed");
    AVBufferRef *pool_realloc = av_buffer_pool_get(pool_realloc_pool);
    fail_if(!pool_realloc, "av_buffer_pool_get realloc failed");
    static const uint8_t pool_realloc_bytes[] = { 0x10, 0x11, 0x12 };
    fill_bytes(pool_realloc, pool_realloc_bytes, sizeof(pool_realloc_bytes));
    ret = av_buffer_realloc(&pool_realloc, 5);
    printf("pool-realloc:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_realloc pool ref failed");
    print_buffer_prefix("pool-realloc:dst", pool_realloc, 3);
    printf("pool-realloc:dst-opaque-null|%d\n",
           av_buffer_get_opaque(pool_realloc) == NULL);
    printf("pool-realloc:after-realloc|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_realloc_reuse =
        av_buffer_pool_get(pool_realloc_pool);
    fail_if(!pool_realloc_reuse, "av_buffer_pool_get realloc reuse failed");
    print_buffer("pool-realloc:reuse", pool_realloc_reuse);
    PoolOpaque *pool_realloc_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_realloc_reuse);
    fail_if(!pool_realloc_reuse_opaque, "pool realloc reuse opaque missing");
    printf("pool-realloc:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_realloc_reuse_opaque->id, pool_realloc_reuse_opaque->size);
    pool_realloc->data[0] = 0xee;
    print_buffer_prefix("pool-realloc:dst-mutated", pool_realloc, 3);
    av_buffer_unref(&pool_realloc);
    printf("pool-realloc:after-dst-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_realloc_reuse);
    av_buffer_pool_uninit(&pool_realloc_pool);
    printf("pool-realloc:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_replace_opaque = { 58, 3 };
    AVBufferPool *pool_replace_pool =
        av_buffer_pool_init2(3, &pool_replace_opaque, test_pool_alloc,
                             test_pool_owner_free);
    fail_if(!pool_replace_pool, "av_buffer_pool_init2 replace failed");
    AVBufferRef *pool_replace_source = av_buffer_allocz(2);
    AVBufferRef *pool_replace_dst = av_buffer_pool_get(pool_replace_pool);
    fail_if(!pool_replace_source || !pool_replace_dst,
            "av_buffer_pool_get replace failed");
    static const uint8_t pool_replace_source_bytes[] = { 0x91, 0x92 };
    static const uint8_t pool_replace_dst_bytes[] = { 0x20, 0x21, 0x22 };
    fill_bytes(pool_replace_source, pool_replace_source_bytes,
               sizeof(pool_replace_source_bytes));
    fill_bytes(pool_replace_dst, pool_replace_dst_bytes,
               sizeof(pool_replace_dst_bytes));
    ret = av_buffer_replace(&pool_replace_dst, pool_replace_source);
    printf("pool-replace:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_replace pool destination failed");
    print_buffer("pool-replace:dst", pool_replace_dst);
    printf("pool-replace:dst-opaque-null|%d\n",
           av_buffer_get_opaque(pool_replace_dst) == NULL);
    printf("pool-replace:shares|%d\n",
           pool_replace_dst->data == pool_replace_source->data);
    printf("pool-replace:after-replace|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_replace_reuse =
        av_buffer_pool_get(pool_replace_pool);
    fail_if(!pool_replace_reuse, "av_buffer_pool_get replace reuse failed");
    print_buffer("pool-replace:reuse", pool_replace_reuse);
    PoolOpaque *pool_replace_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_replace_reuse);
    fail_if(!pool_replace_reuse_opaque, "pool replace reuse opaque missing");
    printf("pool-replace:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_replace_reuse_opaque->id, pool_replace_reuse_opaque->size);
    av_buffer_unref(&pool_replace_dst);
    printf("pool-replace:after-dst-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_replace_source);
    av_buffer_unref(&pool_replace_reuse);
    av_buffer_pool_uninit(&pool_replace_pool);
    printf("pool-replace:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_null_replace_opaque = { 63, 3 };
    AVBufferPool *pool_null_replace_pool =
        av_buffer_pool_init2(3, &pool_null_replace_opaque,
                             test_pool_alloc, test_pool_owner_free);
    fail_if(!pool_null_replace_pool,
            "av_buffer_pool_init2 null replace failed");
    AVBufferRef *pool_null_replace_ref =
        av_buffer_pool_get(pool_null_replace_pool);
    fail_if(!pool_null_replace_ref,
            "av_buffer_pool_get null replace failed");
    static const uint8_t pool_null_replace_bytes[] = { 0x63, 0x64, 0x65 };
    fill_bytes(pool_null_replace_ref, pool_null_replace_bytes,
               sizeof(pool_null_replace_bytes));
    ret = av_buffer_replace(&pool_null_replace_ref, NULL);
    printf("pool-null-replace:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_replace pool null failed");
    printf("pool-null-replace:dst-null|%d\n",
           pool_null_replace_ref == NULL);
    printf("pool-null-replace:after-replace|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_null_replace_reuse =
        av_buffer_pool_get(pool_null_replace_pool);
    fail_if(!pool_null_replace_reuse,
            "av_buffer_pool_get null replace reuse failed");
    print_buffer("pool-null-replace:reuse", pool_null_replace_reuse);
    PoolOpaque *pool_null_replace_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_null_replace_reuse);
    fail_if(!pool_null_replace_reuse_opaque,
            "pool null replace reuse opaque missing");
    printf("pool-null-replace:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_null_replace_reuse_opaque->id,
           pool_null_replace_reuse_opaque->size);
    av_buffer_unref(&pool_null_replace_reuse);
    av_buffer_pool_uninit(&pool_null_replace_pool);
    printf("pool-null-replace:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_source_replace_opaque = { 59, 3 };
    AVBufferPool *pool_source_replace_pool =
        av_buffer_pool_init2(3, &pool_source_replace_opaque,
                             test_pool_alloc, test_pool_owner_free);
    fail_if(!pool_source_replace_pool,
            "av_buffer_pool_init2 source replace failed");
    AVBufferRef *pool_source_replace_source =
        av_buffer_pool_get(pool_source_replace_pool);
    AVBufferRef *pool_source_replace_dst = av_buffer_allocz(2);
    fail_if(!pool_source_replace_source || !pool_source_replace_dst,
            "av_buffer_pool_get source replace failed");
    static const uint8_t pool_source_replace_bytes[] = { 0x41, 0x42, 0x43 };
    static const uint8_t pool_source_replace_dst_bytes[] = { 0x66, 0x67 };
    fill_bytes(pool_source_replace_source, pool_source_replace_bytes,
               sizeof(pool_source_replace_bytes));
    fill_bytes(pool_source_replace_dst, pool_source_replace_dst_bytes,
               sizeof(pool_source_replace_dst_bytes));
    ret = av_buffer_replace(&pool_source_replace_dst,
                            pool_source_replace_source);
    printf("pool-source-replace:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_replace pool source failed");
    print_buffer("pool-source-replace:dst", pool_source_replace_dst);
    PoolOpaque *pool_source_replace_dst_opaque =
        av_buffer_pool_buffer_get_opaque(pool_source_replace_dst);
    fail_if(!pool_source_replace_dst_opaque,
            "pool source replace destination opaque missing");
    printf("pool-source-replace:dst-opaque|%" PRIuPTR "|%zu\n",
           pool_source_replace_dst_opaque->id,
           pool_source_replace_dst_opaque->size);
    printf("pool-source-replace:shares|%d\n",
           pool_source_replace_dst->data == pool_source_replace_source->data);
    printf("pool-source-replace:after-replace|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_source_replace_source);
    printf("pool-source-replace:after-src-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_source_replace_dst);
    printf("pool-source-replace:after-dst-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_source_replace_reuse =
        av_buffer_pool_get(pool_source_replace_pool);
    fail_if(!pool_source_replace_reuse,
            "av_buffer_pool_get source replace reuse failed");
    print_buffer("pool-source-replace:reuse", pool_source_replace_reuse);
    PoolOpaque *pool_source_replace_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_source_replace_reuse);
    fail_if(!pool_source_replace_reuse_opaque,
            "pool source replace reuse opaque missing");
    printf("pool-source-replace:opaque-reuse|%" PRIuPTR "|%zu\n",
           pool_source_replace_reuse_opaque->id,
           pool_source_replace_reuse_opaque->size);
    av_buffer_unref(&pool_source_replace_reuse);
    av_buffer_pool_uninit(&pool_source_replace_pool);
    printf("pool-source-replace:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_pair_destination_opaque = { 60, 3 };
    PoolOpaque pool_pair_source_opaque = { 61, 3 };
    AVBufferPool *pool_pair_destination_pool =
        av_buffer_pool_init2(3, &pool_pair_destination_opaque,
                             test_pool_alloc, test_pool_owner_free);
    AVBufferPool *pool_pair_source_pool =
        av_buffer_pool_init2(3, &pool_pair_source_opaque,
                             test_pool_alloc, test_pool_owner_free);
    fail_if(!pool_pair_destination_pool || !pool_pair_source_pool,
            "av_buffer_pool_init2 pair replace failed");
    AVBufferRef *pool_pair_source =
        av_buffer_pool_get(pool_pair_source_pool);
    AVBufferRef *pool_pair_destination =
        av_buffer_pool_get(pool_pair_destination_pool);
    fail_if(!pool_pair_source || !pool_pair_destination,
            "av_buffer_pool_get pair replace failed");
    static const uint8_t pool_pair_source_bytes[] = { 0x51, 0x52, 0x53 };
    static const uint8_t pool_pair_destination_bytes[] = { 0x31, 0x32, 0x33 };
    fill_bytes(pool_pair_source, pool_pair_source_bytes,
               sizeof(pool_pair_source_bytes));
    fill_bytes(pool_pair_destination, pool_pair_destination_bytes,
               sizeof(pool_pair_destination_bytes));
    ret = av_buffer_replace(&pool_pair_destination, pool_pair_source);
    printf("pool-pair-replace:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_replace pool pair failed");
    print_buffer("pool-pair-replace:dst", pool_pair_destination);
    PoolOpaque *pool_pair_destination_opaque_after =
        av_buffer_pool_buffer_get_opaque(pool_pair_destination);
    fail_if(!pool_pair_destination_opaque_after,
            "pool pair replacement destination opaque missing");
    printf("pool-pair-replace:dst-opaque|%" PRIuPTR "|%zu\n",
           pool_pair_destination_opaque_after->id,
           pool_pair_destination_opaque_after->size);
    printf("pool-pair-replace:shares|%d\n",
           pool_pair_destination->data == pool_pair_source->data);
    printf("pool-pair-replace:after-replace|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_pair_destination_reuse =
        av_buffer_pool_get(pool_pair_destination_pool);
    fail_if(!pool_pair_destination_reuse,
            "av_buffer_pool_get pair destination reuse failed");
    print_buffer("pool-pair-replace:dst-reuse",
                 pool_pair_destination_reuse);
    PoolOpaque *pool_pair_destination_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_pair_destination_reuse);
    fail_if(!pool_pair_destination_reuse_opaque,
            "pool pair destination reuse opaque missing");
    printf("pool-pair-replace:opaque-reuse-dst|%" PRIuPTR "|%zu\n",
           pool_pair_destination_reuse_opaque->id,
           pool_pair_destination_reuse_opaque->size);
    av_buffer_unref(&pool_pair_source);
    printf("pool-pair-replace:after-src-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&pool_pair_destination);
    printf("pool-pair-replace:after-dst-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_pair_source_reuse =
        av_buffer_pool_get(pool_pair_source_pool);
    fail_if(!pool_pair_source_reuse,
            "av_buffer_pool_get pair source reuse failed");
    print_buffer("pool-pair-replace:src-reuse", pool_pair_source_reuse);
    PoolOpaque *pool_pair_source_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(pool_pair_source_reuse);
    fail_if(!pool_pair_source_reuse_opaque,
            "pool pair source reuse opaque missing");
    printf("pool-pair-replace:opaque-reuse-src|%" PRIuPTR "|%zu\n",
           pool_pair_source_reuse_opaque->id,
           pool_pair_source_reuse_opaque->size);
    av_buffer_unref(&pool_pair_destination_reuse);
    av_buffer_pool_uninit(&pool_pair_destination_pool);
    printf("pool-pair-replace:dst-uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);
    av_buffer_unref(&pool_pair_source_reuse);
    av_buffer_pool_uninit(&pool_pair_source_pool);
    printf("pool-pair-replace:src-uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque pool_same_replace_opaque = { 64, 3 };
    AVBufferPool *pool_same_replace_pool =
        av_buffer_pool_init2(3, &pool_same_replace_opaque,
                             test_pool_alloc, test_pool_owner_free);
    fail_if(!pool_same_replace_pool, "av_buffer_pool_init2 same replace failed");
    AVBufferRef *pool_same_replace_source =
        av_buffer_pool_get(pool_same_replace_pool);
    AVBufferRef *pool_same_replace_dst =
        av_buffer_pool_get(pool_same_replace_pool);
    fail_if(!pool_same_replace_source || !pool_same_replace_dst,
            "av_buffer_pool_get same replace failed");
    static const uint8_t pool_same_replace_source_bytes[] = {
        0x51, 0x52, 0x53
    };
    static const uint8_t pool_same_replace_dst_bytes[] = {
        0x31, 0x32, 0x33
    };
    fill_bytes(pool_same_replace_source, pool_same_replace_source_bytes,
               sizeof(pool_same_replace_source_bytes));
    fill_bytes(pool_same_replace_dst, pool_same_replace_dst_bytes,
               sizeof(pool_same_replace_dst_bytes));
    ret = av_buffer_replace(&pool_same_replace_dst, pool_same_replace_source);
    printf("pool-same-replace:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_replace pool same failed");
    print_buffer("pool-same-replace:dst", pool_same_replace_dst);
    PoolOpaque *pool_same_replace_dst_opaque =
        av_buffer_pool_buffer_get_opaque(pool_same_replace_dst);
    fail_if(!pool_same_replace_dst_opaque,
            "pool same replacement destination opaque missing");
    printf("pool-same-replace:dst-opaque|%" PRIuPTR "|%zu\n",
           pool_same_replace_dst_opaque->id,
           pool_same_replace_dst_opaque->size);
    printf("pool-same-replace:shares|%d\n",
           pool_same_replace_dst->data == pool_same_replace_source->data);
    printf("pool-same-replace:after-replace|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *pool_same_replace_reuse_dst =
        av_buffer_pool_get(pool_same_replace_pool);
    fail_if(!pool_same_replace_reuse_dst,
            "av_buffer_pool_get same replace reuse failed");
    print_buffer("pool-same-replace:reuse-dst",
                 pool_same_replace_reuse_dst);
    PoolOpaque *pool_same_replace_reuse_dst_opaque =
        av_buffer_pool_buffer_get_opaque(pool_same_replace_reuse_dst);
    fail_if(!pool_same_replace_reuse_dst_opaque,
            "pool same replacement reuse opaque missing");
    printf("pool-same-replace:opaque-reuse-dst|%" PRIuPTR "|%zu\n",
           pool_same_replace_reuse_dst_opaque->id,
           pool_same_replace_reuse_dst_opaque->size);
    av_buffer_unref(&pool_same_replace_reuse_dst);
    av_buffer_unref(&pool_same_replace_source);
    av_buffer_unref(&pool_same_replace_dst);
    AVBufferRef *pool_same_replace_reuse_first =
        av_buffer_pool_get(pool_same_replace_pool);
    AVBufferRef *pool_same_replace_reuse_second =
        av_buffer_pool_get(pool_same_replace_pool);
    fail_if(!pool_same_replace_reuse_first ||
            !pool_same_replace_reuse_second,
            "av_buffer_pool_get same replace final reuse failed");
    print_buffer("pool-same-replace:reuse-first",
                 pool_same_replace_reuse_first);
    print_buffer("pool-same-replace:reuse-second",
                 pool_same_replace_reuse_second);
    PoolOpaque *pool_same_replace_reuse_first_opaque =
        av_buffer_pool_buffer_get_opaque(pool_same_replace_reuse_first);
    fail_if(!pool_same_replace_reuse_first_opaque,
            "pool same replacement reuse-first opaque missing");
    printf("pool-same-replace:opaque-reuse-first|%" PRIuPTR "|%zu\n",
           pool_same_replace_reuse_first_opaque->id,
           pool_same_replace_reuse_first_opaque->size);
    PoolOpaque *pool_same_replace_reuse_second_opaque =
        av_buffer_pool_buffer_get_opaque(pool_same_replace_reuse_second);
    fail_if(!pool_same_replace_reuse_second_opaque,
            "pool same replacement reuse-second opaque missing");
    printf("pool-same-replace:opaque-reuse-second|%" PRIuPTR "|%zu\n",
           pool_same_replace_reuse_second_opaque->id,
           pool_same_replace_reuse_second_opaque->size);
    av_buffer_unref(&pool_same_replace_reuse_second);
    av_buffer_unref(&pool_same_replace_reuse_first);
    av_buffer_pool_uninit(&pool_same_replace_pool);
    printf("pool-same-replace:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, pool_release_ids[0]);
    print_hex(pool_release_data[0], pool_release_sizes[0]);
    printf("|%" PRIuPTR "|", pool_release_ids[1]);
    print_hex(pool_release_data[1], pool_release_sizes[1]);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque offset_pool_opaque = { 88, 4 };
    AVBufferPool *offset_pool =
        av_buffer_pool_init2(3, &offset_pool_opaque,
                             test_pool_alloc_offset, NULL);
    fail_if(!offset_pool, "av_buffer_pool_init2 offset failed");
    AVBufferRef *offset_first = av_buffer_pool_get(offset_pool);
    fail_if(!offset_first, "av_buffer_pool_get offset first failed");
    print_buffer("pool-offset:first", offset_first);
    PoolOpaque *offset_first_opaque =
        av_buffer_pool_buffer_get_opaque(offset_first);
    fail_if(!offset_first_opaque, "pool offset first opaque missing");
    printf("pool-offset:opaque-first|%" PRIuPTR "|%zu\n",
           offset_first_opaque->id, offset_first_opaque->size);
    av_buffer_unref(&offset_first);
    printf("pool-offset:after-first-unref|%d\n", pool_release_count);
    AVBufferRef *offset_reuse = av_buffer_pool_get(offset_pool);
    fail_if(!offset_reuse, "av_buffer_pool_get offset reuse failed");
    print_buffer("pool-offset:reuse", offset_reuse);
    PoolOpaque *offset_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(offset_reuse);
    fail_if(!offset_reuse_opaque, "pool offset reuse opaque missing");
    printf("pool-offset:opaque-reuse|%" PRIuPTR "|%zu\n",
           offset_reuse_opaque->id, offset_reuse_opaque->size);
    av_buffer_unref(&offset_reuse);
    av_buffer_pool_uninit(&offset_pool);
    printf("pool-offset:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("\n");

    reset_pool_counters();
    PoolOpaque unique_offset_pool_opaque = { 90, 4 };
    AVBufferPool *unique_offset_pool =
        av_buffer_pool_init2(3, &unique_offset_pool_opaque,
                             test_pool_alloc_offset, NULL);
    fail_if(!unique_offset_pool, "av_buffer_pool_init2 unique offset failed");
    AVBufferRef *unique_offset = av_buffer_pool_get(unique_offset_pool);
    fail_if(!unique_offset, "av_buffer_pool_get unique offset failed");
    ret = av_buffer_make_writable(&unique_offset);
    printf("pool-offset-unique:ret|%d\n", ret);
    fail_if(ret < 0, "av_buffer_make_writable unique offset failed");
    unique_offset->data[0] = 0xaa;
    print_buffer("pool-offset-unique:after", unique_offset);
    av_buffer_unref(&unique_offset);
    printf("pool-offset-unique:after-first-unref|%d\n",
           pool_release_count);
    AVBufferRef *unique_offset_reuse = av_buffer_pool_get(unique_offset_pool);
    fail_if(!unique_offset_reuse, "av_buffer_pool_get unique offset reuse failed");
    print_buffer("pool-offset-unique:reuse", unique_offset_reuse);
    av_buffer_unref(&unique_offset_reuse);
    av_buffer_pool_uninit(&unique_offset_pool);
    printf("pool-offset-unique:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("\n");

    reset_pool_counters();
    PoolOpaque readonly_offset_pool_opaque = { 89, 4 };
    AVBufferPool *readonly_offset_pool =
        av_buffer_pool_init2(3, &readonly_offset_pool_opaque,
                             test_pool_alloc_readonly_offset,
                             test_pool_owner_free);
    fail_if(!readonly_offset_pool, "av_buffer_pool_init2 readonly offset failed");
    AVBufferRef *readonly_offset_first =
        av_buffer_pool_get(readonly_offset_pool);
    fail_if(!readonly_offset_first,
            "av_buffer_pool_get readonly offset first failed");
    print_buffer("pool-readonly-offset:first", readonly_offset_first);
    PoolOpaque *readonly_offset_first_opaque =
        av_buffer_pool_buffer_get_opaque(readonly_offset_first);
    fail_if(!readonly_offset_first_opaque,
            "pool readonly offset first opaque missing");
    printf("pool-readonly-offset:opaque-first|%" PRIuPTR "|%zu\n",
           readonly_offset_first_opaque->id, readonly_offset_first_opaque->size);
    av_buffer_unref(&readonly_offset_first);
    printf("pool-readonly-offset:after-first-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *readonly_offset_reuse =
        av_buffer_pool_get(readonly_offset_pool);
    fail_if(!readonly_offset_reuse,
            "av_buffer_pool_get readonly offset reuse failed");
    print_buffer("pool-readonly-offset:reuse", readonly_offset_reuse);
    PoolOpaque *readonly_offset_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(readonly_offset_reuse);
    fail_if(!readonly_offset_reuse_opaque,
            "pool readonly offset reuse opaque missing");
    printf("pool-readonly-offset:opaque-reuse|%" PRIuPTR "|%zu\n",
           readonly_offset_reuse_opaque->id, readonly_offset_reuse_opaque->size);
    readonly_offset_reuse->data[0] = 0xaa;
    av_buffer_unref(&readonly_offset_reuse);
    av_buffer_pool_uninit(&readonly_offset_pool);
    printf("pool-readonly-offset:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque readonly_pool_opaque = { 77, 3 };
    AVBufferPool *readonly_pool =
        av_buffer_pool_init2(3, &readonly_pool_opaque,
                             test_pool_alloc_readonly,
                             test_pool_owner_free);
    fail_if(!readonly_pool, "av_buffer_pool_init2 readonly failed");
    AVBufferRef *readonly_first = av_buffer_pool_get(readonly_pool);
    fail_if(!readonly_first, "av_buffer_pool_get readonly first failed");
    print_buffer("pool-readonly:first", readonly_first);
    PoolOpaque *readonly_first_opaque =
        av_buffer_pool_buffer_get_opaque(readonly_first);
    fail_if(!readonly_first_opaque, "pool readonly first opaque missing");
    printf("pool-readonly:opaque-first|%" PRIuPTR "|%zu\n",
           readonly_first_opaque->id, readonly_first_opaque->size);
    av_buffer_unref(&readonly_first);
    printf("pool-readonly:after-first-unref|%d|%d\n",
           pool_release_count, pool_free_count);
    AVBufferRef *readonly_reuse = av_buffer_pool_get(readonly_pool);
    fail_if(!readonly_reuse, "av_buffer_pool_get readonly reuse failed");
    print_buffer("pool-readonly:reuse", readonly_reuse);
    PoolOpaque *readonly_reuse_opaque =
        av_buffer_pool_buffer_get_opaque(readonly_reuse);
    fail_if(!readonly_reuse_opaque, "pool readonly reuse opaque missing");
    printf("pool-readonly:opaque-reuse|%" PRIuPTR "|%zu\n",
           readonly_reuse_opaque->id, readonly_reuse_opaque->size);
    readonly_reuse->data[0] = 0xaa;
    av_buffer_unref(&readonly_reuse);
    av_buffer_pool_uninit(&readonly_pool);
    printf("pool-readonly:uninit-release|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d|%" PRIuPTR "\n", pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque huge_default_opaque = { 99, SIZE_MAX };
    AVBufferPool *huge_default_pool =
        av_buffer_pool_init2(SIZE_MAX, &huge_default_opaque, NULL,
                             test_pool_owner_free);
    fail_if(!huge_default_pool, "av_buffer_pool_init2 huge default failed");
    AVBufferRef *huge_default_get = av_buffer_pool_get(huge_default_pool);
    printf("pool-default-huge:get|%d|%d\n",
           huge_default_get == NULL, pool_free_count);
    av_buffer_unref(&huge_default_get);
    av_buffer_pool_uninit(&huge_default_pool);
    printf("pool-default-huge:uninit|%d|%" PRIuPTR "\n",
           pool_free_count, last_pool_free_id);

    reset_pool_counters();
    PoolOpaque outstanding_opaque = { 66, 2 };
    pool = av_buffer_pool_init2(2, &outstanding_opaque, test_pool_alloc,
                                test_pool_owner_free);
    fail_if(!pool, "av_buffer_pool_init2 outstanding failed");
    AVBufferRef *outstanding = av_buffer_pool_get(pool);
    fail_if(!outstanding, "av_buffer_pool_get outstanding failed");
    static const uint8_t outstanding_mutated[] = { 0x11, 0x22 };
    fill_bytes(outstanding, outstanding_mutated, sizeof(outstanding_mutated));
    av_buffer_pool_uninit(&pool);
    printf("pool:outstanding-after-uninit|%d|%d\n",
           pool_release_count, pool_free_count);
    av_buffer_unref(&outstanding);
    printf("pool:outstanding-after-drop|%d|%" PRIuPTR "|",
           pool_release_count, last_pool_release_id);
    print_hex(last_pool_release, last_pool_release_size);
    printf("|%d\n", pool_free_count);

    reset_pool_counters();
    PoolOpaque fail_opaque = { 77, 4 };
    pool = av_buffer_pool_init2(4, &fail_opaque, test_pool_alloc_fail,
                                test_pool_owner_free);
    fail_if(!pool, "av_buffer_pool_init2 alloc fail failed");
    AVBufferRef *pool_fail = av_buffer_pool_get(pool);
    printf("pool:alloc-fail|%d|%d|%d|%d\n",
           pool_fail == NULL, pool_alloc_count, pool_release_count,
           pool_free_count);
    av_buffer_unref(&pool_fail);
    av_buffer_pool_uninit(&pool);
    printf("pool:alloc-fail-uninit|%d|%" PRIuPTR "\n",
           pool_free_count, last_pool_free_id);

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
