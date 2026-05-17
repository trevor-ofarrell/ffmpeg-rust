#![no_main]

use fftools::{parse_ffmpeg_args, parse_log_level_value, CliOption, ParsedCommand};
use libfuzzer_sys::fuzz_target;

const MAX_ARGS: usize = 48;
const MAX_LITERAL_LEN: usize = 24;

fuzz_target!(|data: &[u8]| {
    exercise_args(&args_from_bytes(data));
    exercise_args(&fixture_args());
});

fn exercise_args(args: &[String]) {
    let Ok(parsed) = parse_ffmpeg_args(args) else {
        return;
    };

    assert!(parsed.inputs().len() + parsed.outputs().len() <= args.len());
    for option in parsed.global_options() {
        assert!(matches!(
            option.name(),
            "hide_banner" | "y" | "n" | "nostdin" | "version" | "loglevel" | "v"
        ));
        assert_option_arity(option);
        if matches!(option.name(), "loglevel" | "v") {
            assert!(option.value_ref().and_then(parse_log_level_value).is_some());
        }
    }
    for file in parsed.inputs().iter().chain(parsed.outputs()) {
        for option in file.options() {
            assert_option_arity(option);
        }
    }

    let rendered = render_args(&parsed);
    let reparsed = parse_ffmpeg_args(&rendered).unwrap();
    assert_eq!(reparsed, parsed);
}

fn args_from_bytes(data: &[u8]) -> Vec<String> {
    let mut args = Vec::new();
    let mut cursor = 0;
    while cursor < data.len() && args.len() < MAX_ARGS {
        let tag = data[cursor];
        cursor += 1;
        match tag % 32 {
            0 => args.push("-i".to_owned()),
            1 => args.push("-hide_banner".to_owned()),
            2 => args.push("-version".to_owned()),
            3 => args.push("-y".to_owned()),
            4 => args.push("-n".to_owned()),
            5 => args.push("-nostdin".to_owned()),
            6 => push_value_option(&mut args, "loglevel", data, &mut cursor),
            7 => push_value_option(&mut args, "v", data, &mut cursor),
            8 => args.push("-an".to_owned()),
            9 => args.push("-vn".to_owned()),
            10 => args.push("-sn".to_owned()),
            11 => args.push("-dn".to_owned()),
            12 => args.push("-shortest".to_owned()),
            13 => args.push("-bitexact".to_owned()),
            14 => push_value_option(&mut args, "f", data, &mut cursor),
            15 => push_value_option(&mut args, "c:v", data, &mut cursor),
            16 => push_value_option(&mut args, "codec:a", data, &mut cursor),
            17 => push_value_option(&mut args, "map", data, &mut cursor),
            18 => push_value_option(&mut args, "ar", data, &mut cursor),
            19 => push_value_option(&mut args, "ac", data, &mut cursor),
            20 => push_value_option(&mut args, "s", data, &mut cursor),
            21 => push_value_option(&mut args, "r", data, &mut cursor),
            22 => push_value_option(&mut args, "framerate", data, &mut cursor),
            23 => push_value_option(&mut args, "pix_fmt", data, &mut cursor),
            24 => push_value_option(&mut args, "start_number", data, &mut cursor),
            25 => push_value_option(&mut args, "vf", data, &mut cursor),
            26 => push_value_option(&mut args, "af", data, &mut cursor),
            27 => push_value_option(&mut args, "metadata", data, &mut cursor),
            28 => args.push("-definitely_not_ffmpeg".to_owned()),
            29 => args.push("-".to_owned()),
            30 => args.push(literal_arg(data, &mut cursor)),
            _ => args.push(prefixed_literal_arg(data, &mut cursor)),
        }
    }
    args
}

fn push_value_option(args: &mut Vec<String>, name: &str, data: &[u8], cursor: &mut usize) {
    args.push(format!("-{name}"));
    if args.len() < MAX_ARGS {
        args.push(literal_arg(data, cursor));
    }
}

fn literal_arg(data: &[u8], cursor: &mut usize) -> String {
    if *cursor >= data.len() {
        return String::new();
    }

    let len = usize::from(data[*cursor] % (MAX_LITERAL_LEN as u8 + 1));
    *cursor += 1;
    let end = (*cursor).saturating_add(len).min(data.len());
    let mut out = String::new();
    for byte in &data[*cursor..end] {
        out.push(match byte % 12 {
            0 => 'a',
            1 => '0',
            2 => '.',
            3 => '/',
            4 => '_',
            5 => ':',
            6 => '=',
            7 => '%',
            8 => '-',
            9 => ' ',
            10 => 'x',
            _ => 'Z',
        });
    }
    *cursor = end;
    out
}

fn prefixed_literal_arg(data: &[u8], cursor: &mut usize) -> String {
    let mut value = literal_arg(data, cursor);
    if value.is_empty() {
        value.push('x');
    }
    format!("file:{value}")
}

fn assert_option_arity(option: &CliOption) {
    if is_flag_option(option.name()) {
        assert!(option.value_ref().is_none());
    } else {
        assert!(is_value_option(option.name()));
        assert!(option.value_ref().is_some());
    }
}

fn is_flag_option(name: &str) -> bool {
    matches!(
        base_name(name),
        "hide_banner"
            | "y"
            | "n"
            | "nostdin"
            | "version"
            | "an"
            | "vn"
            | "sn"
            | "dn"
            | "shortest"
            | "bitexact"
    )
}

fn is_value_option(name: &str) -> bool {
    matches!(
        base_name(name),
        "loglevel"
            | "v"
            | "f"
            | "c"
            | "codec"
            | "map"
            | "ar"
            | "ac"
            | "s"
            | "r"
            | "framerate"
            | "pix_fmt"
            | "start_number"
            | "vf"
            | "af"
            | "filter"
            | "metadata"
    )
}

fn base_name(name: &str) -> &str {
    name.split_once(':').map_or(name, |(base, _)| base)
}

fn render_args(parsed: &ParsedCommand) -> Vec<String> {
    let mut out = Vec::new();
    for option in parsed.global_options() {
        render_option(&mut out, option);
    }
    for input in parsed.inputs() {
        for option in input.options() {
            render_option(&mut out, option);
        }
        out.push("-i".to_owned());
        out.push(input.url().to_owned());
    }
    for output in parsed.outputs() {
        for option in output.options() {
            render_option(&mut out, option);
        }
        out.push(output.url().to_owned());
    }
    out
}

fn render_option(out: &mut Vec<String>, option: &CliOption) {
    out.push(format!("-{}", option.name()));
    if let Some(value) = option.value_ref() {
        out.push(value.to_owned());
    }
}

fn fixture_args() -> Vec<String> {
    [
        "-hide_banner",
        "-loglevel",
        "warning",
        "-f",
        "image2",
        "-start_number",
        "5",
        "-framerate",
        "24",
        "-i",
        "in-%03d.png",
        "-map",
        "0:v",
        "-c:v",
        "rawvideo",
        "-f",
        "null",
        "-",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
