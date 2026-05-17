#![no_main]

use avutil::{
    Dictionary, DictionarySet, MatchMode, OptionDefinition, OptionKind, OptionSet, OptionValue,
    SetMode,
};
use libfuzzer_sys::fuzz_target;

const MAX_OPS: usize = 64;
const MAX_LITERAL_LEN: usize = 24;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    exercise_dictionary(&mut cursor);
    exercise_options(&mut cursor);
    exercise_fixtures();
});

fn exercise_dictionary(cursor: &mut Cursor<'_>) {
    let mut dict = Dictionary::new();
    let op_count = usize::from(cursor.next().unwrap_or_default()) % (MAX_OPS + 1);

    for _ in 0..op_count {
        match cursor.next().unwrap_or_default() % 6 {
            0 => {
                let key = literal_from(cursor);
                let value = literal_from(cursor);
                let match_mode = match_mode_from(cursor.next());
                let set_mode = set_mode_from(cursor.next());
                let before = dict.clone();
                let result = dict.set_with_mode(key.clone(), value.clone(), match_mode, set_mode);
                match result {
                    Ok(DictionarySet::Inserted) => {
                        assert_eq!(dict.len(), before.len() + 1);
                        assert_valid_dictionary(&dict);
                        assert!(dict.get_entry(&key, match_mode).is_some());
                    }
                    Ok(DictionarySet::Replaced | DictionarySet::Kept | DictionarySet::Appended) => {
                        assert_eq!(dict.len(), before.len());
                        assert_valid_dictionary(&dict);
                        assert!(dict.get_entry(&key, match_mode).is_some());
                    }
                    Err(_) => {
                        assert_eq!(dict, before);
                    }
                }
            }
            1 => {
                let key = literal_from(cursor);
                let before_len = dict.len();
                let removed = dict.remove(&key, match_mode_from(cursor.next()));
                assert_eq!(dict.len() + usize::from(removed.is_some()), before_len);
                if let Some(entry) = removed {
                    assert!(!entry.key().is_empty());
                    assert!(!entry.key().as_bytes().contains(&0));
                    assert!(!entry.value().as_bytes().contains(&0));
                }
                assert_valid_dictionary(&dict);
            }
            2 => {
                let _ = dict.get(&literal_from(cursor));
            }
            3 => {
                let _ = dict.get_entry(&literal_from(cursor), match_mode_from(cursor.next()));
            }
            4 => {
                let _ = dict.get_prefixed(&literal_from(cursor), match_mode_from(cursor.next()));
            }
            _ => {
                dict.clear();
                assert!(dict.is_empty());
            }
        }
    }
}

fn exercise_options(cursor: &mut Cursor<'_>) {
    let mut options = sample_options();
    let op_count = usize::from(cursor.next().unwrap_or_default()) % (MAX_OPS + 1);

    for _ in 0..op_count {
        match cursor.next().unwrap_or_default() % 5 {
            0 => {
                let before = options.clone();
                let definition = generated_definition(cursor);
                let result = definition.and_then(|definition| options.define(definition));
                match result {
                    Ok(()) => {
                        assert_eq!(options.len(), before.len() + 1);
                        assert_option_set_invariants(&options);
                    }
                    Err(_) => {
                        assert_eq!(options, before);
                    }
                }
            }
            1 => {
                let name = option_name_from(cursor);
                let raw = option_value_string_from(cursor);
                let before = options.clone();
                let result = options.set_from_str(&name, &raw);
                if result.is_ok() {
                    assert_option_value_is_valid(&options, &name);
                } else {
                    assert_eq!(options, before);
                }
            }
            2 => {
                let name = option_name_from(cursor);
                let value = option_value_from(cursor);
                let before = options.clone();
                let result = options.set(&name, value);
                if result.is_ok() {
                    assert_option_value_is_valid(&options, &name);
                } else {
                    assert_eq!(options, before);
                }
            }
            3 => {
                let _ = options.get(&option_name_from(cursor));
            }
            _ => {
                let _ = options.definition(&option_name_from(cursor));
            }
        }
    }
}

fn exercise_fixtures() {
    let mut dict = Dictionary::new();
    assert_eq!(dict.set("Title", "First").unwrap(), DictionarySet::Inserted);
    assert_eq!(
        dict.set_with_mode(
            "title",
            "Second",
            MatchMode::CaseInsensitive,
            SetMode::Overwrite
        )
        .unwrap(),
        DictionarySet::Replaced
    );
    assert_eq!(dict.get("TITLE"), Some("Second"));
    assert!(dict.set("", "value").is_err());
    assert!(dict.set("bad\0key", "value").is_err());
    assert!(dict.set("key", "bad\0value").is_err());

    let mut options = sample_options();
    options.set_from_str("threads", "8").unwrap();
    options.set_from_str("bitexact", "yes").unwrap();
    options.set_from_str("quality", "0.75").unwrap();
    options.set_from_str("metadata", "title=clip").unwrap();
    assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
    assert_eq!(options.get("BITEXACT"), Some(&OptionValue::Bool(true)));
    assert!(options.set_from_str("threads", "0").is_err());
    assert!(options.set_from_str("bitexact", "maybe").is_err());
    assert!(options.set_from_str("metadata", "bad\0value").is_err());
}

fn assert_valid_dictionary(dict: &Dictionary) {
    assert_eq!(dict.is_empty(), dict.entries().is_empty());
    assert_eq!(dict.len(), dict.entries().len());
    for entry in dict.entries() {
        assert!(!entry.key().is_empty());
        assert!(!entry.key().as_bytes().contains(&0));
        assert!(!entry.value().as_bytes().contains(&0));
    }
}

fn assert_option_set_invariants(options: &OptionSet) {
    assert_eq!(options.is_empty(), options.definitions().is_empty());
    assert_eq!(options.len(), options.definitions().len());
    for definition in options.definitions() {
        let value = options.get(definition.name()).unwrap();
        definition.validate_value(value).unwrap();
    }
}

fn assert_option_value_is_valid(options: &OptionSet, name: &str) {
    let definition = options.definition(name).unwrap();
    let value = options.get(name).unwrap();
    definition.validate_value(value).unwrap();
}

fn generated_definition(cursor: &mut Cursor<'_>) -> avutil::AvResult<OptionDefinition> {
    let name = option_name_from(cursor);
    let help = literal_from(cursor);
    let kind_tag = cursor.next().unwrap_or_default();
    let kind = match kind_tag % 8 {
        0 => OptionKind::Bool,
        1 => OptionKind::Int { min: 0, max: 64 },
        2 => OptionKind::Int { min: 8, max: 1 },
        3 => OptionKind::Float { min: 0.0, max: 1.0 },
        4 => OptionKind::Float { min: 1.0, max: 0.0 },
        5 => OptionKind::Float {
            min: f64::NAN,
            max: 1.0,
        },
        6 => OptionKind::String { allow_empty: true },
        _ => OptionKind::String { allow_empty: false },
    };
    let default = default_value_for(&kind, cursor);
    OptionDefinition::new(name, kind, default, help)
}

fn default_value_for(kind: &OptionKind, cursor: &mut Cursor<'_>) -> OptionValue {
    match kind {
        OptionKind::Bool => OptionValue::Bool(cursor.next().unwrap_or_default().is_multiple_of(2)),
        OptionKind::Int { min, max } => {
            if min <= max {
                OptionValue::Int(
                    (*min).saturating_add(i64::from(cursor.next().unwrap_or_default())),
                )
            } else {
                OptionValue::Int(0)
            }
        }
        OptionKind::Float { min, max } => {
            if min.is_finite() && max.is_finite() && min <= max {
                OptionValue::Float(*min + (f64::from(cursor.next().unwrap_or_default()) / 255.0))
            } else {
                OptionValue::Float(0.0)
            }
        }
        OptionKind::String { allow_empty } => {
            let value = literal_from(cursor);
            if *allow_empty || !value.is_empty() {
                OptionValue::String(value)
            } else {
                OptionValue::String("default".to_owned())
            }
        }
    }
}

fn sample_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new(
                "threads",
                OptionKind::Int { min: 1, max: 64 },
                OptionValue::Int(1),
                "worker count",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "bitexact",
                OptionKind::Bool,
                OptionValue::Bool(false),
                "bit-exact output",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "quality",
                OptionKind::Float { min: 0.0, max: 1.0 },
                OptionValue::Float(0.5),
                "quality",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "metadata",
                OptionKind::String { allow_empty: false },
                OptionValue::String("default".to_owned()),
                "metadata",
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn option_name_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 12 {
        0 => "threads".to_owned(),
        1 => "THREADS".to_owned(),
        2 => "bitexact".to_owned(),
        3 => "quality".to_owned(),
        4 => "metadata".to_owned(),
        5 => "codec".to_owned(),
        6 => "CODEC".to_owned(),
        7 => String::new(),
        8 => "bad\0name".to_owned(),
        9 => literal_from(cursor),
        10 => "new-option".to_owned(),
        _ => "new_option".to_owned(),
    }
}

fn option_value_string_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 18 {
        0 => "1".to_owned(),
        1 => "0".to_owned(),
        2 => "yes".to_owned(),
        3 => "no".to_owned(),
        4 => "maybe".to_owned(),
        5 => "8".to_owned(),
        6 => "0".to_owned(),
        7 => "64".to_owned(),
        8 => "65".to_owned(),
        9 => "-1".to_owned(),
        10 => "0.75".to_owned(),
        11 => "inf".to_owned(),
        12 => "NaN".to_owned(),
        13 => "title=clip".to_owned(),
        14 => String::new(),
        15 => "bad\0value".to_owned(),
        _ => literal_from(cursor),
    }
}

fn option_value_from(cursor: &mut Cursor<'_>) -> OptionValue {
    match cursor.next().unwrap_or_default() % 4 {
        0 => OptionValue::Bool(cursor.next().unwrap_or_default().is_multiple_of(2)),
        1 => OptionValue::Int(i64::from(cursor.next().unwrap_or_default()) - 32),
        2 => {
            let value = match cursor.next().unwrap_or_default() % 6 {
                0 => f64::NAN,
                1 => f64::INFINITY,
                2 => -1.0,
                3 => 0.0,
                4 => 0.5,
                _ => 1.5,
            };
            OptionValue::Float(value)
        }
        _ => OptionValue::String(option_value_string_from(cursor)),
    }
}

fn match_mode_from(byte: Option<u8>) -> MatchMode {
    if byte.unwrap_or_default().is_multiple_of(2) {
        MatchMode::CaseInsensitive
    } else {
        MatchMode::CaseSensitive
    }
}

fn set_mode_from(byte: Option<u8>) -> SetMode {
    match byte.unwrap_or_default() % 3 {
        0 => SetMode::Overwrite,
        1 => SetMode::KeepExisting,
        _ => SetMode::Append,
    }
}

fn literal_from(cursor: &mut Cursor<'_>) -> String {
    let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_LITERAL_LEN + 1);
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        output.push(match cursor.next().unwrap_or_default() % 18 {
            0 => 'a',
            1 => 'A',
            2 => '0',
            3 => '1',
            4 => '_',
            5 => '-',
            6 => '=',
            7 => ':',
            8 => '/',
            9 => '.',
            10 => ' ',
            11 => '\0',
            12 => 't',
            13 => 'i',
            14 => 'l',
            15 => 'e',
            16 => 'x',
            _ => 'Z',
        });
    }
    output
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.data.get(self.offset).copied();
        self.offset = self.offset.saturating_add(usize::from(byte.is_some()));
        byte
    }
}
