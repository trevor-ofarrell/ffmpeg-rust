use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    Packet, PacketFlags, PacketOpaque, PacketSideDataKind, Rational, SideData,
    AV_INPUT_BUFFER_PADDING_SIZE, AV_NOPTS_VALUE, AV_PACKET_POS_UNKNOWN,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavcodec/libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavcodec_packet_core_lifecycle_matches_packet_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavcodec = oracle_root.join("wsl/lib/libavcodec.a");
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

    let work_dir = repo_root.join("target/oracle/avutil-packet");
    fs::create_dir_all(&work_dir).expect("create avutil-packet oracle work dir");
    let source = work_dir.join("packet_oracle.c");
    let executable = work_dir.join("packet_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-packet oracle C source");

    let stdout =
        compile_and_run_oracle(&include_dir, &libavcodec, &libavutil, &source, &executable);
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

    let mut move_src = packet_with_common_props();
    let mut move_dst = Packet::new(vec![0x44], 9);
    move_dst.move_ref_from(&mut move_src);
    rows.insert("packet:move-dst".to_string(), packet_fields(&move_dst));
    rows.insert("packet:move-src".to_string(), packet_fields(&move_src));

    let mut unref = packet_with_common_props();
    unref.unref();
    rows.insert("packet:unref".to_string(), packet_fields(&unref));

    insert_side_data_api_rows(&mut rows);
    insert_payload_api_rows(&mut rows);

    rows
}

fn insert_payload_api_rows(rows: &mut BTreeMap<String, Vec<String>>) {
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
        format!("{}/{}", packet.time_base().num(), packet.time_base().den()),
    ]
}

fn first_side_data_fields(packet: &Packet) -> (String, String, String) {
    let Some(side_data) = packet.side_data().first() else {
        return ("-".to_string(), "0".to_string(), "-".to_string());
    };

    (
        packet_side_data_type(side_data.kind_id()).to_string(),
        side_data.len().to_string(),
        hex_or_dash(side_data.data()),
    )
}

fn side_data_summary_fields(packet: &Packet) -> Vec<String> {
    let mut fields = vec![packet.side_data().len().to_string()];
    for side_data in packet.side_data() {
        fields.push(packet_side_data_type(side_data.kind_id()).to_string());
        fields.push(side_data.len().to_string());
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

fn packet_side_data_type(kind: &PacketSideDataKind) -> &'static str {
    match kind {
        PacketSideDataKind::Palette => "0",
        PacketSideDataKind::NewExtradata => "1",
        _ => panic!("unexpected packet side data kind in oracle test: {kind:?}"),
    }
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
    libavutil: &Path,
    source: &Path,
    executable: &Path,
) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "gcc -I {} {} {} {} -lm -pthread -ldl -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavcodec)),
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
                "gcc -I {} {} {} {} -lm -pthread -ldl -o {} && {}",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavcodec.display().to_string()),
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
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "libavcodec/defs.h"
#include "libavcodec/packet.h"
#include "libavutil/buffer.h"
#include "libavutil/mem.h"

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
    printf("|%" PRIuPTR "|%d/%d\n", (uintptr_t)pkt->opaque,
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

static void print_payload(const char *name, const AVPacket *pkt) {
    printf("%s|%d|", name, pkt->size);
    print_hex_or_dash(pkt->data, pkt->size);
    printf("|");
    print_hex_or_dash(pkt->data ? pkt->data + pkt->size : NULL,
                      AV_INPUT_BUFFER_PADDING_SIZE);
    printf("|%d\n", pkt->buf ? av_buffer_is_writable(pkt->buf) : 0);
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

static void exercise_payload_api(void) {
    AVPacket *pkt = new_packet();
    uint8_t *owned = av_mallocz(3 + AV_INPUT_BUFFER_PADDING_SIZE);
    fail_if(!owned, "av_mallocz payload from-data failed");
    owned[0] = 0xaa;
    owned[1] = 0xbb;
    owned[2] = 0xcc;
    int ret = av_packet_from_data(pkt, owned, 3);
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

int main(void) {
    AVPacket *pkt = new_packet();
    print_packet("packet:default", pkt);
    av_packet_free(&pkt);

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
    exercise_payload_api();

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
