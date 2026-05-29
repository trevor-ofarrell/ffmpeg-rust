use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use avutil::{
    BufferPool, BufferPoolAllocation, BufferPoolCallbacks, BufferRef, AV_BUFFER_FLAG_READONLY,
    AV_BUFFER_REF_ABI_LAYOUT,
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

    let alloc_zero = BufferRef::from_vec(Vec::new());
    rows.insert("buffer:alloc-zero".to_string(), buffer_fields(&alloc_zero));

    let allocz = BufferRef::zeroed(4).unwrap();
    rows.insert("buffer:allocz".to_string(), buffer_fields(&allocz));

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
}

static void test_pool_free(void *opaque, uint8_t *data) {
    PoolOpaque *pool_opaque = opaque;
    pool_release_count++;
    last_pool_release_id = pool_opaque->id;
    last_pool_release_size = pool_opaque->size;
    fail_if(last_pool_release_size > sizeof(last_pool_release),
            "pool release fixture too large");
    for (size_t i = 0; i < last_pool_release_size; i++)
        last_pool_release[i] = data[i];
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

    buf = av_buffer_alloc(0);
    fail_if(!buf, "av_buffer_alloc zero failed");
    print_buffer("buffer:alloc-zero", buf);
    av_buffer_unref(&buf);

    buf = av_buffer_allocz(4);
    fail_if(!buf, "av_buffer_allocz failed");
    print_buffer("buffer:allocz", buf);
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
