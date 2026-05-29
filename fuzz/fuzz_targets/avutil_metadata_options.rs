#![no_main]

use avutil::{
    AvErrorCode, AvOptionRanges, ChannelLayout, ChannelLayoutSpec, Dictionary, DictionarySet,
    MatchMode, OptionChild, OptionConstant, OptionDefinition, OptionEntryMatch, OptionFlags,
    OptionKind, OptionQuery, OptionSearchFlags, OptionSerializeFlags, OptionSet, OptionValue,
    PixelFormat, Rational, RgbaColor, SampleFormat, SetMode,
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
        match cursor.next().unwrap_or_default() % 11 {
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
            5 => {
                let key = literal_from(cursor);
                let match_mode = match_mode_from(cursor.next());
                let matches: Vec<_> = dict.matching_entries(&key, match_mode).collect();
                for entry in &matches {
                    assert!(dictionary_key_matches(entry.key(), &key, match_mode));
                }
                if let Some(first) = matches.first() {
                    assert_eq!(dict.get_entry(&key, match_mode), Some(*first));
                } else {
                    assert!(dict.get_entry(&key, match_mode).is_none());
                }
            }
            6 => {
                let prefix = literal_from(cursor);
                let match_mode = match_mode_from(cursor.next());
                let matches: Vec<_> = dict.prefixed_entries(&prefix, match_mode).collect();
                for entry in &matches {
                    assert!(dictionary_key_has_prefix(entry.key(), &prefix, match_mode));
                }
                if let Some(first) = matches.first() {
                    assert_eq!(dict.get_prefixed(&prefix, match_mode), Some(*first));
                } else {
                    assert!(dict.get_prefixed(&prefix, match_mode).is_none());
                }
            }
            7 => {
                let key_value_separator = separator_char_from(cursor.next());
                let pair_separator = separator_char_from(cursor.next());
                if let Ok(encoded) = dict.to_pairs_string(key_value_separator, pair_separator) {
                    let key_value_separator = key_value_separator.to_string();
                    let pair_separator = pair_separator.to_string();
                    let decoded = Dictionary::parse_pairs(
                        &encoded,
                        &key_value_separator,
                        &pair_separator,
                        MatchMode::CaseInsensitive,
                        SetMode::AllowMultiple,
                    )
                    .unwrap();
                    assert_eq!(decoded, dict);
                }
            }
            8 => {
                let raw = dictionary_pairs_string_from(cursor);
                let key_value_separators = separator_set_from(cursor);
                let pair_separators = separator_set_from(cursor);
                let match_mode = match_mode_from(cursor.next());
                let set_mode = set_mode_from(cursor.next());
                let result = dict.parse_pairs_into(
                    &raw,
                    &key_value_separators,
                    &pair_separators,
                    match_mode,
                    set_mode,
                );
                assert_valid_dictionary(&dict);
                if let Ok(results) = result {
                    assert!(results.len() <= dict.len().saturating_add(MAX_OPS));
                }
            }
            9 => {
                let key = literal_from(cursor);
                let match_mode = match_mode_from(cursor.next());
                let before_len = dict.len();
                let removed = dict.remove_all(&key, match_mode);
                assert_eq!(dict.len() + removed.len(), before_len);
                for entry in &removed {
                    assert!(dictionary_key_matches(entry.key(), &key, match_mode));
                    assert!(!entry.key().is_empty());
                    assert!(!entry.key().as_bytes().contains(&0));
                    assert!(!entry.value().as_bytes().contains(&0));
                }
                assert!(dict.matching_entries(&key, match_mode).next().is_none());
                assert_valid_dictionary(&dict);
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
        match cursor.next().unwrap_or_default() % 23 {
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
                let before = options.clone();
                let constant = generated_constant(cursor);
                let result = constant.and_then(|constant| options.define_constant(constant));
                match result {
                    Ok(()) => {
                        assert_eq!(options.constants().len(), before.constants().len() + 1);
                        assert_option_set_invariants(&options);
                    }
                    Err(_) => {
                        assert_eq!(options, before);
                    }
                }
            }
            2 => {
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
            3 => {
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
            4 => {
                let before = options.clone();
                let child = generated_child(cursor);
                let result = child.and_then(|child| options.define_child(child));
                match result {
                    Ok(()) => {
                        assert_eq!(options.children().len(), before.children().len() + 1);
                        assert_option_set_invariants(&options);
                    }
                    Err(_) => {
                        assert_eq!(options, before);
                    }
                }
            }
            5 => {
                let before = options.clone();
                let query = generated_query(cursor);
                if let Ok(query) = query {
                    let matches = options.definitions_matching(&query);
                    for found in matches {
                        assert_option_match_satisfies_query(&query, found);
                    }
                }
                assert_eq!(options, before);
            }
            6 => {
                let _ = options.get(&option_name_from(cursor));
            }
            7 => {
                let _ = options.definition(&option_name_from(cursor));
            }
            8 => {
                let child_name = option_child_name_from(cursor);
                let option_name = option_name_from(cursor);
                let value = option_value_from(cursor);
                let before = options.clone();
                let result = options.set_child(&child_name, &option_name, value);
                if result.is_ok() {
                    assert_child_option_value_is_valid(&options, &child_name, &option_name);
                } else {
                    assert_eq!(options, before);
                }
            }
            9 => {
                let child_name = option_child_name_from(cursor);
                let option_name = option_name_from(cursor);
                let raw = option_value_string_from(cursor);
                let before = options.clone();
                let result = options.set_child_from_str(&child_name, &option_name, &raw);
                if result.is_ok() {
                    assert_child_option_value_is_valid(&options, &child_name, &option_name);
                } else {
                    assert_eq!(options, before);
                }
            }
            10 => {
                let name = option_name_from(cursor);
                let raw = option_value_string_from(cursor);
                let before = options.clone();
                let result = options.set_avoption_from_str(&name, &raw);
                if result.is_ok() {
                    assert_option_value_is_valid(&options, &name);
                } else {
                    assert_eq!(options, before);
                }
            }
            11 => {
                let name = option_name_from(cursor);
                let before = options.clone();
                let result = options.get_avoption_string(&name);
                if let Ok(value) = result {
                    assert!(!value.as_bytes().contains(&0));
                }
                assert_eq!(options, before);
            }
            12 => {
                let name = option_name_from(cursor);
                let before = options.clone();
                if let Ok(ranges) = options.query_avoption_ranges(&name) {
                    assert_avoption_ranges_are_valid(&ranges);
                }
                assert_eq!(options, before);
            }
            13 => {
                let name = option_name_from(cursor);
                let raw = option_value_string_from(cursor);
                let flags = option_search_flags_from(cursor.next());
                let before = options.clone();
                let result = options.set_avoption_from_str_with_flags(&name, &raw, flags);
                if result.is_ok() {
                    assert_option_set_invariants(&options);
                } else {
                    assert_eq!(options, before);
                }
            }
            14 => {
                let name = option_name_from(cursor);
                let flags = option_search_flags_from(cursor.next());
                let before = options.clone();
                if let Ok(value) = options.get_avoption_string_with_flags(&name, flags) {
                    assert!(!value.as_bytes().contains(&0));
                }
                assert_eq!(options, before);
            }
            15 => {
                let mut dict = generated_options_dictionary(cursor);
                let before_dict = dict.clone();
                let flags = option_search_flags_from(cursor.next());
                let result = options.set_avoptions_from_dict(&mut dict, flags);
                if result.is_ok() {
                    assert_valid_dictionary(&dict);
                    assert_option_set_invariants(&options);
                } else {
                    assert_eq!(dict, before_dict);
                    assert_option_set_invariants(&options);
                }
            }
            16 => {
                let mut source = sample_options();
                let known_name = option_name_from(cursor);
                let raw = option_value_string_from(cursor);
                let _ = source.set_avoption_from_str(&known_name, &raw);
                let before = options.clone();
                let result = options.copy_avoptions_from(&source);
                if result.is_ok() {
                    assert_root_option_values_match(&options, &source);
                    assert_option_set_invariants(&options);
                } else {
                    assert_eq!(options, before);
                }
            }
            17 => {
                let shorthand = ["threads", "bitexact"];
                let key_val_sep = match cursor.next().unwrap_or_default() % 11 {
                    0 => "=",
                    1 => "=",
                    2 => "=",
                    3 => "=",
                    4 => "=",
                    5 => "=",
                    6 => "=",
                    7 => "",
                    8 => "=",
                    9 => ":",
                    _ => ":=",
                };
                let pairs_sep = match cursor.next().unwrap_or_default() % 7 {
                    0 => ":",
                    1 => ":",
                    2 => ":",
                    3 => ":",
                    4 => ":",
                    5 => "",
                    _ => ":",
                };
                let opts = match cursor.next().unwrap_or_default() % 8 {
                    0 => "threads=7:quality=0.25:metadata=from-string",
                    1 => " 9 : yes : metadata = shorthand ",
                    2 => "10:quality=0.75:no",
                    3 => "threads=11:bitexact=maybe",
                    4 => "threads=12:unknown=1",
                    5 => "metadata=title\\:clip\\=one\\\\two:threads=14:preset_level=slow",
                    6 => "metadata=' title : clip = one ':threads=15",
                    _ => "12",
                };
                let before = options.clone();
                let result = options.set_avoptions_from_string(opts, &shorthand, key_val_sep, pairs_sep);
                let empty_key_result =
                    options.set_avoptions_from_string("=7", &shorthand, "=", ":");
                let separators_invalid = key_val_sep.is_empty();
                if separators_invalid
                {
                    assert_eq!(result.err().and_then(|err| err.code()), Some(AvErrorCode::EINVAL));
                    assert_eq!(options, before);
                } else if let Ok(count) = result {
                    assert!(count <= 3);
                    assert_option_set_invariants(&options);
                }
                assert!(empty_key_result.is_err());
                let mut unclosed_quote_options = sample_options();
                assert_eq!(
                    unclosed_quote_options
                        .set_avoptions_from_string("metadata='title", &[], "=", ":")
                        .unwrap(),
                    1
                );
                assert_eq!(
                    unclosed_quote_options.get("metadata"),
                    Some(&OptionValue::String("title".to_owned()))
                );
                let mut escaped_quote_options = sample_options();
                assert_eq!(
                    escaped_quote_options
                        .set_avoptions_from_string("metadata='\\''x'", &[], "=", ":")
                        .unwrap(),
                    1
                );
                assert_eq!(
                    escaped_quote_options.get("metadata"),
                    Some(&OptionValue::String("\\x".to_owned()))
                );
                assert_option_set_invariants(&options);
            }
            18 => {
                let opt_flags = option_flags_from(cursor.next());
                let serialize_flags = option_serialize_flags_from(cursor.next());
                let key_val_sep = separator_char_from(cursor.next());
                let pairs_sep = separator_char_from(cursor.next());
                let before = options.clone();
                let result =
                    options.serialize_avoptions(opt_flags, serialize_flags, key_val_sep, pairs_sep);
                if let Ok(serialized) = result {
                    assert!(!serialized.as_bytes().contains(&0));
                }
                assert_eq!(options, before);
            }
            19 => {
                let before = options.clone();
                let entries = options.avoption_entries();
                assert_eq!(
                    entries.len(),
                    options.definitions().len() + options.constants().len()
                );
                for entry in &entries {
                    assert!(entry.child_name().is_none());
                }

                let name = option_name_from(cursor);
                let unit = if cursor.next().unwrap_or_default().is_multiple_of(2) {
                    Some(option_unit_from(cursor))
                } else {
                    None
                };
                let flags = option_flags_from(cursor.next());
                let search_flags = option_search_flags_from(cursor.next());

                if let Some(found) =
                    options.find_avoption(&name, unit.as_deref(), flags, search_flags)
                {
                    assert_avoption_match_satisfies_query(
                        found,
                        &name,
                        unit.as_deref(),
                        flags,
                        search_flags,
                    );
                }
                assert_eq!(options, before);
            }
            20 => {
                let name = option_name_from(cursor);
                let before = options.clone();
                let result = options.remove_definition(&name);
                match result {
                    Ok((definition, value)) => {
                        assert_eq!(options.len() + 1, before.len());
                        definition.validate_value(&value).unwrap();
                        assert!(options.definition(definition.name()).is_none());
                        assert!(options.get(definition.name()).is_none());
                        assert_option_set_invariants(&options);
                    }
                    Err(_) => {
                        assert_eq!(options, before);
                    }
                }
            }
            21 => {
                let name = option_name_from(cursor);
                let flags = option_search_flags_from(cursor.next());
                let before = options.clone();
                let result = match cursor.next().unwrap_or_default() % 8 {
                    0 => options
                        .set_avoption_int(&name, i64::from(cursor.next().unwrap_or_default()) - 64),
                    1 => options.set_avoption_int_with_flags(
                        &name,
                        i64::from(cursor.next().unwrap_or_default()) - 64,
                        flags,
                    ),
                    2 => options.set_avoption_double(
                        &name,
                        f64::from(cursor.next().unwrap_or_default()) / 32.0,
                    ),
                    3 => options.set_avoption_double_with_flags(
                        &name,
                        f64::from(cursor.next().unwrap_or_default()) / 32.0,
                        flags,
                    ),
                    4 => options.set_avoption_q(
                        &name,
                        Rational::new(
                            i32::from(cursor.next().unwrap_or_default()) + 1,
                            i32::from(cursor.next().unwrap_or_default() % 16) + 1,
                        )
                        .unwrap(),
                    ),
                    5 => options.set_avoption_q_with_flags(
                        &name,
                        Rational::new(
                            i32::from(cursor.next().unwrap_or_default()) + 1,
                            i32::from(cursor.next().unwrap_or_default() % 16) + 1,
                        )
                        .unwrap(),
                        flags,
                    ),
                    6 => options.set_avoption_image_size(
                        &name,
                        i32::from(cursor.next().unwrap_or_default()),
                        i32::from(cursor.next().unwrap_or_default()),
                    ),
                    _ => options.set_avoption_image_size_with_flags(
                        &name,
                        i32::from(cursor.next().unwrap_or_default()),
                        i32::from(cursor.next().unwrap_or_default()),
                        flags,
                    ),
                };
                if result.is_ok() {
                    assert_option_set_invariants(&options);
                } else {
                    assert_eq!(options, before);
                }

                let before_get = options.clone();
                let _ = options.get_avoption_int_with_flags(&name, flags);
                let _ = options.get_avoption_double_with_flags(&name, flags);
                let _ = options.get_avoption_q_with_flags(&name, flags);
                let _ = options.get_avoption_image_size_with_flags(&name, flags);
                assert_eq!(options, before_get);
            }
            _ => {
                let before = options.clone();
                if cursor.next().unwrap_or_default().is_multiple_of(2) {
                    let unit = option_unit_from(cursor);
                    let name = option_constant_name_from(cursor);
                    let result = options.remove_constant(&unit, &name);
                    match result {
                        Ok(constant) => {
                            assert_eq!(options.constants().len() + 1, before.constants().len());
                            assert!(options
                                .constants_for_unit(constant.unit())
                                .all(|remaining| {
                                    !ascii_eq_ignore_case(remaining.name(), constant.name())
                                }));
                            assert_option_set_invariants(&options);
                        }
                        Err(_) => {
                            assert_eq!(options, before);
                        }
                    }
                } else {
                    let name = option_child_name_from(cursor);
                    let result = options.remove_child(&name);
                    match result {
                        Ok(child) => {
                            assert_eq!(options.children().len() + 1, before.children().len());
                            assert!(options.child(child.name()).is_none());
                            assert_option_set_invariants(&options);
                        }
                        Err(_) => {
                            assert_eq!(options, before);
                        }
                    }
                }
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
    assert_eq!(
        dict.set_with_mode(
            "TITLE",
            "Third",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap(),
        DictionarySet::Inserted
    );
    assert_eq!(dict.len(), 2);
    assert_eq!(dict.get("title"), Some("Second"));
    let duplicate_values: Vec<_> = dict
        .matching_entries("TITLE", MatchMode::CaseInsensitive)
        .map(|entry| entry.value())
        .collect();
    assert_eq!(duplicate_values, vec!["Second", "Third"]);
    let all_keys: Vec<_> = dict
        .prefixed_entries("", MatchMode::CaseInsensitive)
        .map(|entry| entry.key())
        .collect();
    assert_eq!(all_keys, vec!["title", "TITLE"]);
    let encoded = dict.to_pairs_string('=', ';').unwrap();
    assert_eq!(encoded, "title=Second;TITLE=Third");
    let decoded = Dictionary::parse_pairs(
        &encoded,
        "=",
        ";",
        MatchMode::CaseInsensitive,
        SetMode::AllowMultiple,
    )
    .unwrap();
    assert_eq!(decoded, dict);
    assert!(Dictionary::parse_pairs(
        "ok=value;bad",
        "=",
        ";",
        MatchMode::CaseInsensitive,
        SetMode::Overwrite,
    )
    .is_err());
    assert!(dict.set("", "value").is_err());
    assert!(dict.set("bad\0key", "value").is_err());
    assert!(dict.set("key", "bad\0value").is_err());
    let removed = dict.remove_all("TITLE", MatchMode::CaseInsensitive);
    assert_eq!(
        removed
            .iter()
            .map(|entry| entry.value())
            .collect::<Vec<_>>(),
        vec!["Second", "Third"]
    );
    assert!(dict
        .matching_entries("title", MatchMode::CaseInsensitive)
        .next()
        .is_none());

    let mut options = sample_options();
    options.set_from_str("threads", "8").unwrap();
    options.set_from_str("bitexact", "yes").unwrap();
    options.set_from_str("quality", "0.75").unwrap();
    options.set_from_str("aspect_ratio", "4/3").unwrap();
    options.set_from_str("metadata", "title=clip").unwrap();
    options.set_from_str("preset_level", "FAST").unwrap();
    assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
    assert_eq!(options.get_avoption_string("threads").unwrap(), "8");
    assert_eq!(options.get("BITEXACT"), Some(&OptionValue::Bool(true)));
    assert_eq!(options.get_avoption_string("bitexact").unwrap(), "true");
    assert_eq!(
        options.get("aspect_ratio"),
        Some(&OptionValue::Rational(Rational::new(4, 3).unwrap()))
    );
    assert_eq!(options.get_avoption_string("aspect_ratio").unwrap(), "4/3");
    assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(2)));
    assert_eq!(options.get_avoption_string("preset_level").unwrap(), "2");
    let threads_ranges = options.query_avoption_ranges("threads").unwrap();
    assert_eq!(threads_ranges.nb_ranges(), 1);
    assert_eq!(threads_ranges.nb_components(), 1);
    assert_eq!(threads_ranges.ranges()[0].value_min(), 1.0);
    assert_eq!(threads_ranges.ranges()[0].value_max(), 64.0);
    let bitexact_ranges = options.query_avoption_ranges("bitexact").unwrap();
    assert_eq!(bitexact_ranges.ranges()[0].value_min(), 0.0);
    assert_eq!(bitexact_ranges.ranges()[0].value_max(), 1.0);
    let metadata_ranges = options.query_avoption_ranges("metadata").unwrap();
    assert_eq!(metadata_ranges.ranges()[0].value_min(), -1.0);
    assert_eq!(metadata_ranges.ranges()[0].component_max(), 0x10ffff as f64);
    let mut nullable_options = OptionSet::new();
    nullable_options
        .define(
            OptionDefinition::new(
                "nullable",
                OptionKind::String { allow_empty: true },
                OptionValue::NullString,
                "nullable string",
            )
            .unwrap(),
        )
        .unwrap();
    nullable_options
        .define(
            OptionDefinition::new(
                "nullable_blob",
                OptionKind::Binary,
                OptionValue::NullBinary,
                "nullable binary",
            )
            .unwrap(),
        )
        .unwrap();
    nullable_options
        .define(
            OptionDefinition::new(
                "nullable_dict",
                OptionKind::Dictionary,
                OptionValue::NullDictionary,
                "nullable dictionary",
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        nullable_options.get_avoption_string("nullable").unwrap(),
        ""
    );
    assert_eq!(
        nullable_options
            .get_avoption_string_nullable_with_flags("nullable", OptionSearchFlags::ALLOW_NULL)
            .unwrap(),
        None
    );
    assert_eq!(
        nullable_options
            .get_avoption_string_nullable_with_flags(
                "nullable_blob",
                OptionSearchFlags::ALLOW_NULL,
            )
            .unwrap(),
        None
    );
    assert_eq!(
        nullable_options
            .get_avoption_string_nullable_with_flags(
                "nullable_dict",
                OptionSearchFlags::ALLOW_NULL,
            )
            .unwrap(),
        None
    );
    assert_eq!(
        nullable_options
            .get_avoption_binary("nullable_blob")
            .unwrap(),
        Vec::<u8>::new()
    );
    assert!(nullable_options
        .get_avoption_dictionary("nullable_dict")
        .unwrap()
        .is_empty());
    nullable_options
        .set_avoption_from_str("nullable", "owned")
        .unwrap();
    assert_eq!(
        nullable_options
            .get_avoption_string_nullable_with_flags("nullable", OptionSearchFlags::ALLOW_NULL)
            .unwrap(),
        Some("owned".to_owned())
    );
    let aspect_ranges = options.query_avoption_ranges("aspect_ratio").unwrap();
    assert_eq!(aspect_ranges.ranges()[0].component_min(), i32::MIN as f64);
    assert_eq!(aspect_ranges.ranges()[0].component_max(), i32::MAX as f64);
    let missing_range = options.query_avoption_ranges("THREADS").unwrap_err();
    assert_eq!(missing_range.code(), Some(AvErrorCode::ENOMEM));

    let mut typed_options = sample_options();
    typed_options.set_avoption_int("threads", 21).unwrap();
    typed_options.set_avoption_int("bitexact", 1).unwrap();
    typed_options.set_avoption_double("quality", 0.625).unwrap();
    typed_options
        .set_avoption_q("aspect_ratio", Rational::new(3, 2).unwrap())
        .unwrap();
    typed_options.set_avoption_int("preset_level", 6).unwrap();
    assert_eq!(typed_options.get("threads"), Some(&OptionValue::Int(21)));
    assert_eq!(
        typed_options.get("bitexact"),
        Some(&OptionValue::Bool(true))
    );
    assert_eq!(
        typed_options.get("quality"),
        Some(&OptionValue::Float(0.625))
    );
    assert_eq!(typed_options.get_avoption_int("threads").unwrap(), 21);
    assert_eq!(typed_options.get_avoption_double("threads").unwrap(), 21.0);
    assert_eq!(
        typed_options.get_avoption_q("aspect_ratio").unwrap(),
        Rational::new(3, 2).unwrap()
    );
    assert_eq!(typed_options.get_avoption_int("quality").unwrap(), 0);
    let typed_before_errors = typed_options.clone();
    assert_eq!(
        typed_options
            .set_avoption_int("metadata", 1)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        typed_options
            .set_avoption_int("threads", 128)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        typed_options
            .set_avoption_int("readonly", 1)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        typed_options
            .get_avoption_int("metadata")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        typed_options
            .get_avoption_int("missing")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::OPTION_NOT_FOUND)
    );
    assert_eq!(typed_options, typed_before_errors);

    let missing_exact = options.set_avoption_from_str("THREADS", "9").unwrap_err();
    assert_eq!(missing_exact.code(), Some(AvErrorCode::OPTION_NOT_FOUND));
    let missing_get = options.get_avoption_string("THREADS").unwrap_err();
    assert_eq!(missing_get.code(), Some(AvErrorCode::OPTION_NOT_FOUND));
    assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
    let before_exact_error = options.clone();
    assert!(options
        .set_avoption_from_str("preset_level", "FAST")
        .is_err());
    assert_eq!(options, before_exact_error);
    options
        .set_avoption_from_str("preset_level", "fast")
        .unwrap();
    assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(2)));

    let mut expression_options = sample_options();
    expression_options
        .set_avoption_from_str("threads", "2*3")
        .unwrap();
    expression_options
        .set_avoption_from_str("quality", "500m")
        .unwrap();
    expression_options
        .set_avoption_from_str("aspect_ratio", "1+1/2")
        .unwrap();
    expression_options
        .set_avoption_from_str("preset_level", "slow+2")
        .unwrap();
    assert_eq!(
        expression_options.get("threads"),
        Some(&OptionValue::Int(6))
    );
    assert_eq!(
        expression_options.get("quality"),
        Some(&OptionValue::Float(0.5))
    );
    assert_eq!(
        expression_options.get("aspect_ratio"),
        Some(&OptionValue::Rational(Rational::new(3, 2).unwrap()))
    );
    assert_eq!(
        expression_options.get("preset_level"),
        Some(&OptionValue::Int(10))
    );
    let before_expression_errors = expression_options.clone();
    assert_eq!(
        expression_options
            .set_avoption_from_str("threads", "1K")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(expression_options, before_expression_errors);

    let mut duration_options = OptionSet::new();
    duration_options
        .define(
            OptionDefinition::new(
                "timeout",
                OptionKind::Duration {
                    min: 0,
                    max: 7_200_000_000,
                },
                OptionValue::Duration(0),
                "timeout",
            )
            .unwrap(),
        )
        .unwrap();
    duration_options
        .set_avoption_from_str("timeout", "00:01:02.250")
        .unwrap();
    assert_eq!(
        duration_options.get("timeout"),
        Some(&OptionValue::Duration(62_250_000))
    );
    assert_eq!(
        duration_options.get_avoption_string("timeout").unwrap(),
        "1:02.25"
    );
    duration_options
        .set_avoption_from_str("timeout", "1500ms")
        .unwrap();
    assert_eq!(
        duration_options.get("timeout"),
        Some(&OptionValue::Duration(1_500_000))
    );
    duration_options
        .set_avoption_from_str("timeout", "42us")
        .unwrap();
    assert_eq!(
        duration_options.get("timeout"),
        Some(&OptionValue::Duration(42))
    );
    duration_options
        .set_avoption_int("timeout", 90_500_000)
        .unwrap();
    assert_eq!(
        duration_options.get_avoption_int("timeout").unwrap(),
        90_500_000
    );
    assert_eq!(
        duration_options.get_avoption_q("timeout").unwrap(),
        Rational::new(90_500_000, 1).unwrap()
    );
    let before_duration_errors = duration_options.clone();
    assert_eq!(
        duration_options
            .set_avoption_from_str("timeout", "bad_duration")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(duration_options, before_duration_errors);
    assert_eq!(
        duration_options
            .set_avoption_from_str("timeout", "-1")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(duration_options, before_duration_errors);

    let mut image_size_options = OptionSet::new();
    image_size_options
        .define(
            OptionDefinition::new(
                "size",
                OptionKind::ImageSize,
                OptionValue::ImageSize {
                    width: 320,
                    height: 240,
                },
                "image size",
            )
            .unwrap(),
        )
        .unwrap();
    image_size_options
        .set_avoption_from_str("size", "640x480")
        .unwrap();
    assert_eq!(
        image_size_options.get("size"),
        Some(&OptionValue::ImageSize {
            width: 640,
            height: 480
        })
    );
    image_size_options
        .set_avoption_from_str("size", "hd720")
        .unwrap();
    assert_eq!(
        image_size_options.get_avoption_image_size("size").unwrap(),
        (1280, 720)
    );
    image_size_options
        .set_avoption_from_str("size", "none")
        .unwrap();
    assert_eq!(
        image_size_options.get_avoption_string("size").unwrap(),
        "0x0"
    );
    image_size_options
        .set_avoption_image_size("size", 800, 600)
        .unwrap();
    assert_eq!(
        image_size_options.get_avoption_image_size("size").unwrap(),
        (800, 600)
    );
    let before_image_size_errors = image_size_options.clone();
    assert_eq!(
        image_size_options
            .set_avoption_from_str("size", "bad_size")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(image_size_options, before_image_size_errors);
    assert_eq!(
        image_size_options
            .set_avoption_image_size("size", -1, 480)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(image_size_options, before_image_size_errors);

    let mut pixel_format_options = OptionSet::new();
    pixel_format_options
        .define(
            OptionDefinition::new(
                "pix_fmt",
                OptionKind::PixelFormat { min: -1, max: 266 },
                OptionValue::PixelFormat(Some(PixelFormat::Yuv420p)),
                "pixel format",
            )
            .unwrap(),
        )
        .unwrap();
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "rgb24")
        .unwrap();
    assert_eq!(
        pixel_format_options.get("pix_fmt"),
        Some(&OptionValue::PixelFormat(Some(PixelFormat::Rgb24)))
    );
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "gray")
        .unwrap();
    assert_eq!(
        pixel_format_options
            .get_avoption_pixel_format("pix_fmt")
            .unwrap(),
        Some(PixelFormat::Gray8)
    );
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "none")
        .unwrap();
    assert_eq!(
        pixel_format_options.get_avoption_string("pix_fmt").unwrap(),
        "none"
    );
    pixel_format_options
        .set_avoption_pixel_format("pix_fmt", Some(PixelFormat::Bgr24))
        .unwrap();
    assert_eq!(pixel_format_options.get_avoption_int("pix_fmt").unwrap(), 3);
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "gbrap32le")
        .unwrap();
    assert_eq!(
        pixel_format_options.get_avoption_int("pix_fmt").unwrap(),
        257
    );
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "259")
        .unwrap();
    assert_eq!(
        pixel_format_options
            .get_avoption_pixel_format("pix_fmt")
            .unwrap(),
        Some(PixelFormat::Yuv444p10MsbLe)
    );
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "vaapi")
        .unwrap();
    assert_eq!(
        pixel_format_options
            .get_avoption_pixel_format("pix_fmt")
            .unwrap(),
        Some(PixelFormat::Vaapi)
    );
    pixel_format_options
        .set_avoption_from_str("pix_fmt", "227")
        .unwrap();
    assert_eq!(
        pixel_format_options
            .get_avoption_pixel_format("pix_fmt")
            .unwrap(),
        Some(PixelFormat::D3d12)
    );
    pixel_format_options
        .set_avoption_int("pix_fmt", 266)
        .unwrap();
    assert_eq!(
        pixel_format_options.get_avoption_string("pix_fmt").unwrap(),
        "ohcodec"
    );
    let before_pixel_format_errors = pixel_format_options.clone();
    assert_eq!(
        pixel_format_options
            .set_avoption_from_str("pix_fmt", "bad_pix_fmt")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(pixel_format_options, before_pixel_format_errors);
    assert_eq!(
        pixel_format_options
            .set_avoption_from_str("pix_fmt", "267")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(22))
    );
    assert_eq!(pixel_format_options, before_pixel_format_errors);

    let mut sample_format_options = OptionSet::new();
    sample_format_options
        .define(
            OptionDefinition::new(
                "sample_fmt",
                OptionKind::SampleFormat { min: -1, max: 11 },
                OptionValue::SampleFormat(Some(SampleFormat::S16)),
                "sample format",
            )
            .unwrap(),
        )
        .unwrap();
    sample_format_options
        .set_avoption_from_str("sample_fmt", "fltp")
        .unwrap();
    assert_eq!(
        sample_format_options.get("sample_fmt"),
        Some(&OptionValue::SampleFormat(Some(SampleFormat::FltP)))
    );
    sample_format_options
        .set_avoption_from_str("sample_fmt", "none")
        .unwrap();
    assert_eq!(
        sample_format_options
            .get_avoption_sample_format("sample_fmt")
            .unwrap(),
        None
    );
    sample_format_options
        .set_avoption_sample_format("sample_fmt", Some(SampleFormat::Dbl))
        .unwrap();
    assert_eq!(
        sample_format_options
            .get_avoption_int("sample_fmt")
            .unwrap(),
        4
    );
    let before_sample_format_errors = sample_format_options.clone();
    assert_eq!(
        sample_format_options
            .set_avoption_from_str("sample_fmt", "bad_sample_fmt")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(sample_format_options, before_sample_format_errors);
    assert_eq!(
        sample_format_options
            .set_avoption_from_str("sample_fmt", "12")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(sample_format_options, before_sample_format_errors);
    assert_eq!(
        sample_format_options
            .set_avoption_int("sample_fmt", 12)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(sample_format_options, before_sample_format_errors);

    let mut channel_layout_options = OptionSet::new();
    channel_layout_options
        .define(
            OptionDefinition::new(
                "layout",
                OptionKind::ChannelLayout,
                OptionValue::ChannelLayout(ChannelLayoutSpec::native(ChannelLayout::stereo())),
                "channel layout",
            )
            .unwrap(),
        )
        .unwrap();
    channel_layout_options
        .set_avoption_from_str("layout", "mono")
        .unwrap();
    assert_eq!(
        channel_layout_options.get("layout"),
        Some(&OptionValue::ChannelLayout(ChannelLayoutSpec::native(
            ChannelLayout::mono()
        )))
    );
    channel_layout_options
        .set_avoption_from_str("layout", "5.1")
        .unwrap();
    assert_eq!(
        channel_layout_options
            .get_avoption_channel_layout("layout")
            .unwrap()
            .describe(),
        "5.1"
    );
    channel_layout_options
        .set_avoption_from_str("layout", "2C")
        .unwrap();
    assert_eq!(
        channel_layout_options
            .get_avoption_string("layout")
            .unwrap(),
        "2 channels"
    );
    assert_eq!(
        channel_layout_options
            .set_avoption_from_str("layout", "bad_layout")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        channel_layout_options
            .get_avoption_string("layout")
            .unwrap(),
        "0 channels"
    );
    channel_layout_options
        .set_avoption_channel_layout("layout", ChannelLayoutSpec::native(ChannelLayout::stereo()))
        .unwrap();
    let before_channel_layout_errors = channel_layout_options.clone();
    assert_eq!(
        channel_layout_options
            .set_avoption_int("layout", 2)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        channel_layout_options
            .set_avoption_int("layout", 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        channel_layout_options
            .get_avoption_int("layout")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        channel_layout_options
            .query_avoption_ranges("layout")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::ENOSYS)
    );
    assert_eq!(channel_layout_options, before_channel_layout_errors);

    let mut binary_options = OptionSet::new();
    binary_options
        .define(
            OptionDefinition::new(
                "blob",
                OptionKind::Binary,
                OptionValue::Binary(vec![0x00, 0x01, 0xAA, 0xFF]),
                "binary data",
            )
            .unwrap(),
        )
        .unwrap();
    binary_options
        .define(
            OptionDefinition::new(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
            )
            .unwrap(),
        )
        .unwrap();
    binary_options
        .set_avoption_from_str("blob", "0f10Aa")
        .unwrap();
    assert_eq!(
        binary_options.get("blob"),
        Some(&OptionValue::Binary(vec![0x0F, 0x10, 0xAA]))
    );
    assert_eq!(
        binary_options.get_avoption_string("blob").unwrap(),
        "0F10AA"
    );
    binary_options.set_avoption_from_str("blob", "").unwrap();
    assert_eq!(
        binary_options.get("blob"),
        Some(&OptionValue::Binary(Vec::new()))
    );
    binary_options
        .set_avoption_from_str("blob", "deAd")
        .unwrap();
    assert_eq!(binary_options.get_avoption_string("blob").unwrap(), "DEAD");
    assert_eq!(
        binary_options
            .set_avoption_from_str("blob", "abc")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        binary_options.get("blob"),
        Some(&OptionValue::Binary(Vec::new()))
    );
    binary_options
        .set_avoption_binary("blob", &[0xBE, 0xEF])
        .unwrap();
    assert_eq!(
        binary_options.get_avoption_binary("blob").unwrap(),
        vec![0xBE, 0xEF]
    );
    let before_binary_errors = binary_options.clone();
    assert_eq!(
        binary_options
            .set_avoption_int("blob", 2)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        binary_options
            .set_avoption_int("blob", 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        binary_options
            .query_avoption_ranges("blob")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::ENOSYS)
    );
    assert_eq!(binary_options, before_binary_errors);

    let mut default_dict = Dictionary::new();
    default_dict.set("title", "clip").unwrap();
    default_dict.set("note", "hello:world").unwrap();
    let mut dictionary_options = OptionSet::new();
    dictionary_options
        .define(
            OptionDefinition::new(
                "dict",
                OptionKind::Dictionary,
                OptionValue::Dictionary(default_dict),
                "dictionary data",
            )
            .unwrap(),
        )
        .unwrap();
    dictionary_options
        .define(
            OptionDefinition::new(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        dictionary_options.get_avoption_string("dict").unwrap(),
        "title=clip:note=hello\\:world"
    );
    dictionary_options
        .set_avoption_from_str("dict", "artist=rust:comment='a:b'")
        .unwrap();
    assert_eq!(
        dictionary_options
            .get_avoption_dictionary("dict")
            .unwrap()
            .get("comment"),
        Some("a:b")
    );
    assert_eq!(
        dictionary_options.get_avoption_string("dict").unwrap(),
        "artist=rust:comment=a\\:b"
    );
    dictionary_options
        .set_avoption_from_str("dict", "")
        .unwrap();
    assert!(dictionary_options
        .get_avoption_dictionary("dict")
        .unwrap()
        .is_empty());
    dictionary_options
        .set_avoption_from_str("dict", "key=value")
        .unwrap();
    let before_dictionary_errors = dictionary_options.clone();
    assert_eq!(
        dictionary_options
            .set_avoption_from_str("dict", "missing")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        dictionary_options
            .set_avoption_from_str("dict", "key=")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        dictionary_options
            .set_avoption_int("dict", 2)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        dictionary_options
            .set_avoption_int("dict", 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        dictionary_options
            .query_avoption_ranges("dict")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::ENOSYS)
    );
    assert_eq!(dictionary_options, before_dictionary_errors);

    let mut array_options = OptionSet::new();
    array_options
        .define(
            OptionDefinition::new(
                "ints",
                OptionKind::array(OptionKind::Int { min: 0, max: 10 }, 0, Some(4), ',').unwrap(),
                OptionValue::Array(vec![OptionValue::Int(1), OptionValue::Int(2)]),
                "integer array",
            )
            .unwrap(),
        )
        .unwrap();
    array_options
        .define(
            OptionDefinition::new(
                "words",
                OptionKind::array(OptionKind::String { allow_empty: true }, 0, Some(3), ',')
                    .unwrap(),
                OptionValue::Array(vec![
                    OptionValue::String("alpha".to_owned()),
                    OptionValue::String("beta,gamma".to_owned()),
                ]),
                "string array",
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(array_options.get_avoption_string("ints").unwrap(), "1,2");
    assert_eq!(
        array_options.get_avoption_string("words").unwrap(),
        "alpha,beta\\,gamma"
    );
    array_options.set_avoption_from_str("ints", "3,4").unwrap();
    array_options
        .set_avoption_from_str("words", "left,right\\,inner")
        .unwrap();
    assert_eq!(array_options.get_avoption_array_size("ints").unwrap(), 2);
    assert_eq!(
        array_options.get_avoption_array("ints", 0, 2).unwrap(),
        vec![OptionValue::Int(3), OptionValue::Int(4)]
    );
    array_options
        .set_avoption_array(
            "ints",
            1,
            &[OptionValue::String("6".to_owned())],
            OptionSearchFlags::empty(),
        )
        .unwrap();
    array_options
        .set_avoption_array(
            "ints",
            2,
            &[OptionValue::String("9".to_owned())],
            OptionSearchFlags::ARRAY_REPLACE,
        )
        .unwrap();
    array_options
        .remove_avoption_array("ints", 0, 1, OptionSearchFlags::empty())
        .unwrap();
    assert_eq!(
        array_options.get_avoption_array("ints", 0, 2).unwrap(),
        vec![OptionValue::Int(6), OptionValue::Int(9)]
    );
    assert_eq!(
        array_options
            .get_avoption_array_strings("ints", 0, 2)
            .unwrap(),
        vec!["6".to_owned(), "9".to_owned()]
    );
    array_options.set_avoption_from_str("ints", "3,4").unwrap();
    array_options
        .set_avoption_array(
            "ints",
            1,
            &[OptionValue::Float(6.0)],
            OptionSearchFlags::empty(),
        )
        .unwrap();
    array_options
        .set_avoption_array(
            "ints",
            2,
            &[OptionValue::Rational(Rational::new(9, 1).unwrap())],
            OptionSearchFlags::ARRAY_REPLACE,
        )
        .unwrap();
    array_options
        .remove_avoption_array("ints", 0, 1, OptionSearchFlags::empty())
        .unwrap();
    assert_eq!(
        array_options
            .get_avoption_array_doubles("ints", 0, 2)
            .unwrap(),
        vec![6.0, 9.0]
    );
    assert_eq!(
        array_options
            .get_avoption_array_rationals("ints", 0, 2)
            .unwrap(),
        vec![Rational::new(6, 1).unwrap(), Rational::new(9, 1).unwrap()]
    );
    assert_eq!(
        array_options
            .get_avoption_array("ints", 2, 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        array_options
            .get_avoption_array_strings("ints", 2, 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        array_options
            .get_avoption_array_doubles("ints", 2, 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        array_options
            .get_avoption_array_rationals("ints", 2, 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    array_options
        .set_avoption_array("ints", 2, &[], OptionSearchFlags::empty())
        .unwrap();
    array_options
        .set_avoption_array("ints", 2, &[], OptionSearchFlags::ARRAY_REPLACE)
        .unwrap();
    array_options
        .remove_avoption_array("ints", 2, 0, OptionSearchFlags::empty())
        .unwrap();
    assert_eq!(array_options.get_avoption_string("ints").unwrap(), "6,9");
    assert_eq!(
        array_options
            .get_avoption_array("ints", 3, 0)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    array_options
        .set_avoption_array(
            "words",
            1,
            &[OptionValue::String("middle,comma".to_owned())],
            OptionSearchFlags::empty(),
        )
        .unwrap();
    array_options
        .set_avoption_array(
            "words",
            2,
            &[OptionValue::String("tail\\slash".to_owned())],
            OptionSearchFlags::ARRAY_REPLACE,
        )
        .unwrap();
    array_options
        .remove_avoption_array("words", 0, 1, OptionSearchFlags::empty())
        .unwrap();
    assert_eq!(
        array_options.get_avoption_array("words", 0, 2).unwrap(),
        vec![
            OptionValue::String("middle,comma".to_owned()),
            OptionValue::String("tail\\slash".to_owned()),
        ]
    );
    assert_eq!(
        array_options.get_avoption_string("words").unwrap(),
        "middle\\,comma,tail\\\\slash"
    );
    let before_array_errors = array_options.clone();
    assert_eq!(
        array_options
            .set_avoption_from_str("ints", "7,11")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        array_options
            .set_avoption_from_str("words", "a,b,c,d")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        array_options
            .set_avoption_array(
                "ints",
                0,
                &[OptionValue::Rational(Rational::new(11, 1).unwrap())],
                OptionSearchFlags::empty()
            )
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(
        array_options
            .query_avoption_ranges("ints")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::ENOSYS)
    );
    assert_eq!(array_options, before_array_errors);

    let mut video_rate_options = OptionSet::new();
    video_rate_options
        .define(
            OptionDefinition::new(
                "rate",
                OptionKind::VideoRate {
                    min: Rational::ONE,
                    max: Rational::new(120, 1).unwrap(),
                },
                OptionValue::VideoRate(Rational::new(25, 1).unwrap()),
                "video rate",
            )
            .unwrap(),
        )
        .unwrap();
    video_rate_options
        .set_avoption_from_str("rate", "ntsc")
        .unwrap();
    assert_eq!(
        video_rate_options.get("rate"),
        Some(&OptionValue::VideoRate(Rational::new(30000, 1001).unwrap()))
    );
    video_rate_options
        .set_avoption_from_str("rate", "film")
        .unwrap();
    assert_eq!(
        video_rate_options.get_avoption_string("rate").unwrap(),
        "24/1"
    );
    video_rate_options
        .set_avoption_video_rate("rate", Rational::new(50, 1).unwrap())
        .unwrap();
    assert_eq!(
        video_rate_options.get_avoption_string("rate").unwrap(),
        "50/1"
    );
    let before_video_rate_errors = video_rate_options.clone();
    assert_eq!(
        video_rate_options
            .set_avoption_from_str("rate", "bad_rate")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(video_rate_options, before_video_rate_errors);
    assert_eq!(
        video_rate_options
            .set_avoption_video_rate("rate", Rational::ZERO)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(video_rate_options, before_video_rate_errors);
    assert_eq!(
        video_rate_options
            .get_avoption_video_rate("rate")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );

    let mut color_options = OptionSet::new();
    color_options
        .define(
            OptionDefinition::new(
                "color",
                OptionKind::Color,
                OptionValue::Color(RgbaColor::from_rgba([0xFF, 0x00, 0x00, 0xFF])),
                "color",
            )
            .unwrap(),
        )
        .unwrap();
    color_options
        .set_avoption_from_str("color", "Blue@0.5")
        .unwrap();
    assert_eq!(
        color_options.get("color"),
        Some(&OptionValue::Color(RgbaColor::from_rgba([
            0x00, 0x00, 0xFF, 0x7F
        ])))
    );
    color_options
        .set_avoption_from_str("color", "#112233")
        .unwrap();
    assert_eq!(
        color_options.get_avoption_string("color").unwrap(),
        "0x112233ff"
    );
    color_options
        .set_avoption_from_str("color", "0x11223344")
        .unwrap();
    assert_eq!(
        color_options.get_avoption_string("color").unwrap(),
        "0x11223344"
    );
    let before_name_error = color_options.clone();
    assert_eq!(
        color_options
            .set_avoption_from_str("color", "not-a-color")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        color_options.get_avoption_string("color").unwrap(),
        "0x112233ff"
    );
    assert_ne!(color_options, before_name_error);
    assert_eq!(
        color_options
            .set_avoption_from_str("color", "red@2")
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(
        color_options.get_avoption_string("color").unwrap(),
        "0xff0000ff"
    );
    let before_numeric_errors = color_options.clone();
    assert_eq!(
        color_options
            .set_avoption_int("color", 10)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::from_posix_errno(34))
    );
    assert_eq!(color_options, before_numeric_errors);
    assert_eq!(
        color_options.get_avoption_int("color").unwrap_err().code(),
        Some(AvErrorCode::EINVAL)
    );

    options.set_from_str("preset_level", "slow").unwrap();
    assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(8)));
    assert!(options
        .define_constant(OptionConstant::new("PRESET", "FAST", OptionValue::Int(4), "").unwrap())
        .is_err());
    assert!(options.set_from_str("threads", "0").is_err());
    assert!(options.set_from_str("bitexact", "maybe").is_err());
    assert!(options.set_from_str("aspect_ratio", "1/0").is_err());
    assert!(options.set_from_str("aspect_ratio", "2/1").is_err());
    assert!(options.set_from_str("metadata", "bad\0value").is_err());
    assert!(options.set_from_str("readonly", "yes").is_err());
    assert_eq!(options.get("readonly"), Some(&OptionValue::Bool(false)));
    assert!(options
        .definition("readonly")
        .unwrap()
        .flags()
        .contains(OptionFlags::READONLY));
    assert_eq!(
        options
            .avoption_entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>(),
        vec![
            "threads",
            "bitexact",
            "quality",
            "metadata",
            "size",
            "pix_fmt",
            "sample_fmt",
            "layout",
            "blob",
            "dict",
            "aspect_ratio",
            "readonly",
            "preset_level",
            "fast",
            "slow",
        ]
    );
    assert_eq!(
        options
            .find_avoption(
                "threads",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::empty()
            )
            .unwrap()
            .name(),
        "threads"
    );
    assert!(options
        .find_avoption(
            "THREADS",
            None,
            OptionFlags::empty(),
            OptionSearchFlags::empty()
        )
        .is_none());
    assert!(options
        .find_avoption(
            "fast",
            None,
            OptionFlags::empty(),
            OptionSearchFlags::empty()
        )
        .is_none());
    let preset = options
        .find_avoption(
            "fast",
            Some("preset"),
            OptionFlags::empty(),
            OptionSearchFlags::empty(),
        )
        .unwrap();
    assert_eq!(preset.name(), "fast");
    assert!(preset.entry().is_constant());

    let mut child_options = OptionSet::new();
    child_options
        .define(
            OptionDefinition::new(
                "threads",
                OptionKind::Int { min: 1, max: 16 },
                OptionValue::Int(2),
                "child worker count",
            )
            .unwrap(),
        )
        .unwrap();
    child_options
        .define(
            OptionDefinition::new(
                "child_size",
                OptionKind::ImageSize,
                OptionValue::ImageSize {
                    width: 320,
                    height: 240,
                },
                "child image size",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_child(OptionChild::new("encoder", child_options, "encoder options").unwrap())
        .unwrap();
    assert_eq!(
        options.child("ENCODER").unwrap().options().get("THREADS"),
        Some(&OptionValue::Int(2))
    );
    assert_eq!(
        options
            .get_avoption_string_with_flags("threads", OptionSearchFlags::CHILDREN)
            .unwrap(),
        "2"
    );
    options
        .set_child_from_str("encoder", "threads", "8")
        .unwrap();
    assert_eq!(
        options.get_child_option("ENCODER", "THREADS").unwrap(),
        &OptionValue::Int(8)
    );
    options
        .set_avoption_from_str_with_flags("threads", "9", OptionSearchFlags::CHILDREN)
        .unwrap();
    assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
    assert_eq!(
        options.get_child_option("encoder", "threads").unwrap(),
        &OptionValue::Int(9)
    );
    options
        .set_avoption_int_with_flags("threads", 10, OptionSearchFlags::CHILDREN)
        .unwrap();
    assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
    assert_eq!(
        options
            .get_avoption_int_with_flags("threads", OptionSearchFlags::CHILDREN)
            .unwrap(),
        10
    );
    assert_eq!(
        options
            .get_avoption_string_with_flags("threads", OptionSearchFlags::FAKE_OBJ)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::OPTION_NOT_FOUND)
    );
    assert_eq!(
        options
            .get_avoption_image_size_with_flags("child_size", OptionSearchFlags::empty())
            .unwrap_err()
            .code(),
        Some(AvErrorCode::OPTION_NOT_FOUND)
    );
    assert_eq!(
        options
            .get_avoption_image_size_with_flags("child_size", OptionSearchFlags::CHILDREN)
            .unwrap(),
        (320, 240)
    );
    options
        .set_avoption_image_size_with_flags("child_size", 800, 600, OptionSearchFlags::CHILDREN)
        .unwrap();
    assert_eq!(
        options.get_child_option("encoder", "child_size").unwrap(),
        &OptionValue::ImageSize {
            width: 800,
            height: 600
        }
    );
    assert_eq!(
        options
            .set_avoption_image_size_with_flags(
                "child_size",
                1024,
                768,
                OptionSearchFlags::FAKE_OBJ,
            )
            .unwrap_err()
            .code(),
        Some(AvErrorCode::OPTION_NOT_FOUND)
    );
    assert!(options
        .set_child("encoder", "threads", OptionValue::Int(99))
        .is_err());
    assert_eq!(
        options.get_child_option("encoder", "threads").unwrap(),
        &OptionValue::Int(10)
    );
    assert!(options
        .define_child(OptionChild::new("ENCODER", OptionSet::new(), "").unwrap())
        .is_err());

    let mut dict_options = sample_options();
    let mut option_dict = Dictionary::new();
    for (key, value) in [
        ("threads", "11"),
        ("unknown", "first"),
        ("bitexact", "true"),
        ("unknown", "second"),
        ("metadata", "from-dict"),
    ] {
        option_dict
            .set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
            .unwrap();
    }
    dict_options
        .set_avoptions_from_dict(&mut option_dict, OptionSearchFlags::empty())
        .unwrap();
    assert_eq!(dict_options.get("threads"), Some(&OptionValue::Int(11)));
    assert_eq!(dict_options.get("bitexact"), Some(&OptionValue::Bool(true)));
    assert_eq!(
        option_dict
            .entries()
            .iter()
            .map(|entry| (entry.key(), entry.value()))
            .collect::<Vec<_>>(),
        vec![("unknown", "first"), ("unknown", "second")]
    );

    let mut error_options = sample_options();
    let mut error_dict = Dictionary::new();
    for (key, value) in [
        ("threads", "13"),
        ("bitexact", "maybe"),
        ("unknown", "later"),
    ] {
        error_dict
            .set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
            .unwrap();
    }
    let original_error_dict = error_dict.clone();
    assert!(error_options
        .set_avoptions_from_dict(&mut error_dict, OptionSearchFlags::empty())
        .is_err());
    assert_eq!(error_options.get("threads"), Some(&OptionValue::Int(13)));
    assert_eq!(
        error_options.get("bitexact"),
        Some(&OptionValue::Bool(false))
    );
    assert_eq!(error_dict, original_error_dict);

    let mut string_options = sample_options();
    assert_eq!(
        string_options
            .set_avoptions_from_string(
                "threads=7:quality=0.25:metadata=from-string",
                &[],
                "=",
                ":",
            )
            .unwrap(),
        3
    );
    assert_eq!(string_options.get("threads"), Some(&OptionValue::Int(7)));
    assert_eq!(
        string_options.get("metadata"),
        Some(&OptionValue::String("from-string".to_owned()))
    );
    let mut shorthand_options = sample_options();
    assert_eq!(
        shorthand_options
            .set_avoptions_from_string(
                " 9 : yes : metadata = shorthand ",
                &["threads", "bitexact"],
                "=",
                ":",
            )
            .unwrap(),
        3
    );
    assert_eq!(shorthand_options.get("threads"), Some(&OptionValue::Int(9)));
    assert_eq!(
        shorthand_options.get("bitexact"),
        Some(&OptionValue::Bool(true))
    );
    let mut partial_options = sample_options();
    assert_eq!(
        partial_options
            .set_avoptions_from_string("10:quality=0.75:no", &["threads", "bitexact"], "=", ":",)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(partial_options.get("threads"), Some(&OptionValue::Int(10)));
    assert_eq!(
        partial_options.get("quality"),
        Some(&OptionValue::Float(0.75))
    );
    assert_eq!(
        partial_options.get("bitexact"),
        Some(&OptionValue::Bool(false))
    );
    let mut escaped_options = sample_options();
    escaped_options
        .set_avoptions_from_string(
            "metadata=title\\:clip\\=one\\\\two:threads=14:preset_level=slow",
            &[],
            "=",
            ":",
        )
        .unwrap();
    assert_eq!(
        escaped_options.get("metadata"),
        Some(&OptionValue::String("title:clip=one\\two".to_owned()))
    );
    assert_eq!(
        escaped_options.get("preset_level"),
        Some(&OptionValue::Int(8))
    );
    let mut quoted_options = sample_options();
    quoted_options
        .set_avoptions_from_string("metadata=' title : clip = one ':threads=15", &[], "=", ":")
        .unwrap();
    assert_eq!(
        quoted_options.get("metadata"),
        Some(&OptionValue::String(" title : clip = one ".to_owned()))
    );
    let serialized = options
        .serialize_avoptions(
            OptionFlags::empty(),
            OptionSerializeFlags::empty(),
            '=',
            ',',
        )
        .unwrap();
    assert!(serialized.contains("threads=8"));
    assert!(serialized.contains("metadata=title\\=clip"));
    assert_eq!(
        options
            .serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '=',
                '=',
            )
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );

    let mut copy_source = sample_options();
    copy_source.set_avoption_from_str("threads", "12").unwrap();
    copy_source
        .set_avoption_from_str("bitexact", "true")
        .unwrap();
    copy_source
        .set_avoption_from_str("quality", "0.875")
        .unwrap();
    copy_source
        .set_avoption_from_str("aspect_ratio", "3/2")
        .unwrap();
    copy_source
        .set_avoption_from_str("metadata", "source")
        .unwrap();
    copy_source
        .set_avoption_from_str("preset_level", "slow")
        .unwrap();
    let mut copy_destination = sample_options();
    copy_destination
        .set_avoption_from_str("threads", "3")
        .unwrap();
    copy_destination
        .set_avoption_from_str("metadata", "destination")
        .unwrap();
    copy_destination.copy_avoptions_from(&copy_source).unwrap();
    assert_root_option_values_match(&copy_destination, &copy_source);
    copy_source
        .set_avoption_from_str("metadata", "mutated-source")
        .unwrap();
    assert_eq!(
        copy_destination.get("metadata"),
        Some(&OptionValue::String("source".to_owned()))
    );
    let mut mismatch_destination = OptionSet::new();
    mismatch_destination
        .define(
            OptionDefinition::new("other", OptionKind::Bool, OptionValue::Bool(false), "").unwrap(),
        )
        .unwrap();
    let before_mismatch = mismatch_destination.clone();
    assert_eq!(
        mismatch_destination
            .copy_avoptions_from(&copy_source)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::EINVAL)
    );
    assert_eq!(mismatch_destination, before_mismatch);

    let exported = options.definitions_matching(&OptionQuery::exported());
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].definition().name(), "readonly");
    assert_eq!(exported[0].child_name(), None);
    assert!(options
        .definitions_matching(
            &OptionQuery::writable()
                .with_name("readonly")
                .unwrap()
                .include_children(true),
        )
        .is_empty());
    assert_eq!(
        options
            .definitions_matching(
                &OptionQuery::new()
                    .with_name("THREADS")
                    .unwrap()
                    .include_children(true),
            )
            .len(),
        2
    );

    let (removed_definition, removed_value) = options.remove_definition("THREADS").unwrap();
    assert_eq!(removed_definition.name(), "threads");
    assert_eq!(removed_value, OptionValue::Int(8));
    assert!(options.get("threads").is_none());
    let removed_constant = options.remove_constant("PRESET", "FAST").unwrap();
    assert_eq!(removed_constant.name(), "fast");
    assert!(options.set_from_str("preset_level", "fast").is_err());
    options.set_from_str("preset_level", "slow").unwrap();
    let removed_child = options.remove_child("ENCODER").unwrap();
    assert_eq!(removed_child.name(), "encoder");
    assert!(options.child("encoder").is_none());
    assert_option_set_invariants(&options);
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
    assert_option_set_invariants_at_depth(options, 0);
}

fn assert_option_set_invariants_at_depth(options: &OptionSet, depth: usize) {
    assert_eq!(options.is_empty(), options.definitions().is_empty());
    assert_eq!(options.len(), options.definitions().len());
    for definition in options.definitions() {
        let value = options.get(definition.name()).unwrap();
        definition.validate_value(value).unwrap();
        assert_eq!(definition.flags().bits() & !OptionFlags::all().bits(), 0);
        if let Some(unit) = definition.unit() {
            assert!(!unit.is_empty());
            assert!(!unit.as_bytes().contains(&0));
        }
        if let Some(range) = definition.range() {
            definition.validate_value(range.min()).unwrap();
            definition.validate_value(range.max()).unwrap();
            match (range.min(), range.max()) {
                (OptionValue::Int(min), OptionValue::Int(max)) => assert!(min <= max),
                (OptionValue::Duration(min), OptionValue::Duration(max)) => assert!(min <= max),
                (OptionValue::Float(min), OptionValue::Float(max)) => {
                    assert!(min.is_finite());
                    assert!(max.is_finite());
                    assert!(min <= max);
                }
                (OptionValue::Rational(min), OptionValue::Rational(max)) => {
                    assert!(min.den() > 0);
                    assert!(max.den() > 0);
                    assert!(min <= max);
                }
                (OptionValue::VideoRate(min), OptionValue::VideoRate(max)) => {
                    assert!(min.num() > 0);
                    assert!(min.den() > 0);
                    assert!(max.num() > 0);
                    assert!(max.den() > 0);
                    assert!(min <= max);
                }
                _ => unreachable!("option ranges are numeric"),
            }
        }
    }
    for constant in options.constants() {
        assert!(!constant.name().is_empty());
        assert!(!constant.name().as_bytes().contains(&0));
        assert!(!constant.unit().is_empty());
        assert!(!constant.unit().as_bytes().contains(&0));
        assert!(!constant.help().as_bytes().contains(&0));
        assert_eq!(constant.flags().bits() & !OptionFlags::all().bits(), 0);
    }
    let entries = options.avoption_entries();
    assert_eq!(
        entries.len(),
        options.definitions().len() + options.constants().len()
    );
    for entry in entries {
        assert!(entry.child_name().is_none());
        assert!(!entry.name().is_empty());
        assert_eq!(entry.entry().flags().bits() & !OptionFlags::all().bits(), 0);
    }
    for (index, child) in options.children().iter().enumerate() {
        assert!(!child.name().is_empty());
        assert!(!child.name().as_bytes().contains(&0));
        assert!(!child.help().as_bytes().contains(&0));
        for previous in &options.children()[..index] {
            assert!(!ascii_eq_ignore_case(previous.name(), child.name()));
        }
        if depth < 2 {
            assert_option_set_invariants_at_depth(child.options(), depth + 1);
        }
    }
}

fn assert_root_option_values_match(actual: &OptionSet, expected: &OptionSet) {
    for definition in expected.definitions() {
        assert_eq!(
            actual.get(definition.name()),
            expected.get(definition.name()),
            "copied root option `{}` diverged",
            definition.name()
        );
    }
}

fn assert_avoption_ranges_are_valid(ranges: &AvOptionRanges) {
    assert!(ranges.nb_ranges() > 0);
    assert!(ranges.nb_components() > 0);
    assert_eq!(ranges.nb_ranges(), ranges.ranges().len());
    for range in ranges.ranges() {
        assert!(range.value_min().is_finite());
        assert!(range.value_max().is_finite());
        assert!(range.component_min().is_finite());
        assert!(range.component_max().is_finite());
        assert!(range.value_min() <= range.value_max());
        assert!(range.component_min() <= range.component_max());
        assert!(range.is_range());
    }
}

fn assert_option_value_is_valid(options: &OptionSet, name: &str) {
    let definition = options.definition(name).unwrap();
    let value = options.get(name).unwrap();
    definition.validate_value(value).unwrap();
}

fn assert_child_option_value_is_valid(options: &OptionSet, child_name: &str, option_name: &str) {
    let child = options.child(child_name).unwrap();
    let definition = child.options().definition(option_name).unwrap();
    let value = options.get_child_option(child_name, option_name).unwrap();
    definition.validate_value(value).unwrap();
}

fn assert_option_match_satisfies_query(query: &OptionQuery, found: avutil::OptionMatch<'_>) {
    let definition = found.definition();
    if let Some(name) = query.name() {
        assert!(ascii_eq_ignore_case(definition.name(), name));
    }
    if let Some(unit) = query.unit() {
        assert!(definition
            .unit()
            .is_some_and(|definition_unit| ascii_eq_ignore_case(definition_unit, unit)));
    }
    assert!(definition.flags().contains(query.required_flags()));
    if !query.rejected_flags().is_empty() {
        assert!(!definition.flags().intersects(query.rejected_flags()));
    }
    if found.child_name().is_some() {
        assert!(query.searches_children());
    }
}

fn assert_avoption_match_satisfies_query(
    found: OptionEntryMatch<'_>,
    name: &str,
    unit: Option<&str>,
    flags: OptionFlags,
    search_flags: OptionSearchFlags,
) {
    let entry = found.entry();
    assert_eq!(entry.name(), name);
    assert!(entry.flags().contains(flags));

    match unit {
        Some(unit) => {
            assert!(entry.is_constant());
            assert_eq!(entry.unit(), Some(unit));
        }
        None => {
            assert!(!entry.is_constant());
        }
    }

    if found.child_name().is_some() {
        assert!(search_flags.contains(OptionSearchFlags::CHILDREN));
    }
}

fn generated_definition(cursor: &mut Cursor<'_>) -> avutil::AvResult<OptionDefinition> {
    let name = option_name_from(cursor);
    let help = literal_from(cursor);
    let kind_tag = cursor.next().unwrap_or_default();
    let kind = match kind_tag % 24 {
        0 => OptionKind::Bool,
        1 => OptionKind::Int { min: 0, max: 64 },
        2 => OptionKind::Int { min: 8, max: 1 },
        3 => OptionKind::Float { min: 0.0, max: 1.0 },
        4 => OptionKind::Float { min: 1.0, max: 0.0 },
        5 => OptionKind::Float {
            min: f64::NAN,
            max: 1.0,
        },
        6 => OptionKind::Rational {
            min: Rational::ONE,
            max: Rational::new(16, 9).unwrap(),
        },
        7 => OptionKind::Rational {
            min: Rational::ONE,
            max: Rational::ZERO,
        },
        8 => OptionKind::Rational {
            min: Rational::from_raw(1, 0),
            max: Rational::ONE,
        },
        9 => OptionKind::Duration {
            min: 0,
            max: 7_200_000_000,
        },
        10 => OptionKind::Duration { min: 8, max: 1 },
        11 => OptionKind::ImageSize,
        12 => OptionKind::PixelFormat { min: -1, max: 266 },
        13 => OptionKind::PixelFormat { min: 24, max: -1 },
        14 => OptionKind::SampleFormat { min: -1, max: 11 },
        15 => OptionKind::SampleFormat { min: 11, max: -1 },
        16 => OptionKind::VideoRate {
            min: Rational::ONE,
            max: Rational::new(120, 1).unwrap(),
        },
        17 => OptionKind::VideoRate {
            min: Rational::new(120, 1).unwrap(),
            max: Rational::ONE,
        },
        18 => OptionKind::ChannelLayout,
        19 => OptionKind::Color,
        20 => OptionKind::Binary,
        21 => OptionKind::Dictionary,
        22 => OptionKind::array(OptionKind::Int { min: 0, max: 10 }, 0, Some(4), ',').unwrap(),
        23 => OptionKind::String { allow_empty: true },
        _ => OptionKind::String { allow_empty: false },
    };
    let default = default_value_for(&kind, cursor);
    let flags = option_flags_from(cursor.next());
    let unit = option_definition_unit_from(cursor);
    OptionDefinition::new_with_flags_and_unit(name, kind, default, help, flags, unit)
}

fn generated_constant(cursor: &mut Cursor<'_>) -> avutil::AvResult<OptionConstant> {
    let unit = option_unit_from(cursor);
    let name = option_constant_name_from(cursor);
    let value = option_value_from(cursor);
    let help = literal_from(cursor);
    let flags = option_flags_from(cursor.next());
    OptionConstant::new_with_flags(unit, name, value, help, flags)
}

fn generated_child(cursor: &mut Cursor<'_>) -> avutil::AvResult<OptionChild> {
    let name = option_child_name_from(cursor);
    let options = generated_child_options(cursor);
    let help = literal_from(cursor);
    OptionChild::new(name, options, help)
}

fn generated_options_dictionary(cursor: &mut Cursor<'_>) -> Dictionary {
    let mut dict = Dictionary::new();
    let entry_count = usize::from(cursor.next().unwrap_or_default()) % 5;

    for _ in 0..entry_count {
        let key = option_name_from(cursor);
        let value = option_value_string_from(cursor);
        let _ = dict.set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple);
    }

    dict
}

fn generated_query(cursor: &mut Cursor<'_>) -> avutil::AvResult<OptionQuery> {
    let mut query = match cursor.next().unwrap_or_default() % 3 {
        0 => OptionQuery::new(),
        1 => OptionQuery::exported(),
        _ => OptionQuery::writable(),
    };

    if cursor.next().unwrap_or_default().is_multiple_of(2) {
        query = query.with_name(option_name_from(cursor))?;
    }
    if cursor.next().unwrap_or_default().is_multiple_of(3) {
        query = query.with_unit(option_unit_from(cursor))?;
    }
    if cursor.next().unwrap_or_default().is_multiple_of(2) {
        query = query.require_flags(option_flags_from(cursor.next()));
    }
    if cursor.next().unwrap_or_default().is_multiple_of(2) {
        query = query.reject_flags(option_flags_from(cursor.next()));
    }

    Ok(query.include_children(cursor.next().unwrap_or_default().is_multiple_of(2)))
}

fn generated_child_options(cursor: &mut Cursor<'_>) -> OptionSet {
    let mut options = OptionSet::new();
    match cursor.next().unwrap_or_default() % 3 {
        0 => {
            options
                .define(
                    OptionDefinition::new(
                        "threads",
                        OptionKind::Int { min: 1, max: 16 },
                        OptionValue::Int(2),
                        "child worker count",
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        1 => {
            options
                .define(
                    OptionDefinition::new(
                        "enabled",
                        OptionKind::Bool,
                        OptionValue::Bool(false),
                        "child enable flag",
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        _ => {}
    }
    options
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
        OptionKind::Duration { min, max } => {
            if min <= max {
                OptionValue::Duration(*min)
            } else {
                OptionValue::Duration(0)
            }
        }
        OptionKind::ImageSize => OptionValue::ImageSize {
            width: i32::from(cursor.next().unwrap_or_default()),
            height: i32::from(cursor.next().unwrap_or_default()),
        },
        OptionKind::PixelFormat { min, max } => {
            if *min <= 0 && *max >= 0 {
                OptionValue::PixelFormat(Some(PixelFormat::Yuv420p))
            } else if *min <= -1 && *max >= -1 {
                OptionValue::PixelFormat(None)
            } else {
                OptionValue::PixelFormat(Some(PixelFormat::Rgb24))
            }
        }
        OptionKind::SampleFormat { min, max } => {
            if *min <= 1 && *max >= 1 {
                OptionValue::SampleFormat(Some(SampleFormat::S16))
            } else if *min <= -1 && *max >= -1 {
                OptionValue::SampleFormat(None)
            } else {
                OptionValue::SampleFormat(Some(SampleFormat::U8))
            }
        }
        OptionKind::ChannelLayout => {
            OptionValue::ChannelLayout(ChannelLayoutSpec::native(ChannelLayout::stereo()))
        }
        OptionKind::Float { min, max } => {
            if min.is_finite() && max.is_finite() && min <= max {
                OptionValue::Float(*min + (f64::from(cursor.next().unwrap_or_default()) / 255.0))
            } else {
                OptionValue::Float(0.0)
            }
        }
        OptionKind::Rational { min, max } => {
            if min.den() > 0 && max.den() > 0 && min <= max {
                OptionValue::Rational(*min)
            } else {
                OptionValue::Rational(Rational::ONE)
            }
        }
        OptionKind::VideoRate { min, max } => {
            if min.num() > 0 && min.den() > 0 && max.num() > 0 && max.den() > 0 && min <= max {
                OptionValue::VideoRate(*min)
            } else {
                OptionValue::VideoRate(Rational::ONE)
            }
        }
        OptionKind::Color => OptionValue::Color(RgbaColor::from_rgba([
            cursor.next().unwrap_or_default(),
            cursor.next().unwrap_or_default(),
            cursor.next().unwrap_or_default(),
            cursor.next().unwrap_or_default(),
        ])),
        OptionKind::Binary => {
            let len = usize::from(cursor.next().unwrap_or_default()) % 8;
            let mut value = Vec::with_capacity(len);
            for _ in 0..len {
                value.push(cursor.next().unwrap_or_default());
            }
            OptionValue::Binary(value)
        }
        OptionKind::Dictionary => OptionValue::Dictionary(generated_options_dictionary(cursor)),
        OptionKind::Array(array) => {
            let max_len = array.max_len().unwrap_or(4).min(4);
            let min_len = array.min_len().min(max_len);
            let extra = if max_len > min_len {
                usize::from(cursor.next().unwrap_or_default()) % (max_len - min_len + 1)
            } else {
                0
            };
            let len = min_len + extra;
            let values = (0..len)
                .map(|_| default_value_for(array.element(), cursor))
                .collect();
            OptionValue::Array(values)
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
        .define(
            OptionDefinition::new(
                "size",
                OptionKind::ImageSize,
                OptionValue::ImageSize {
                    width: 320,
                    height: 240,
                },
                "image size",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "pix_fmt",
                OptionKind::PixelFormat { min: -1, max: 266 },
                OptionValue::PixelFormat(Some(PixelFormat::Yuv420p)),
                "pixel format",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "sample_fmt",
                OptionKind::SampleFormat { min: -1, max: 11 },
                OptionValue::SampleFormat(Some(SampleFormat::S16)),
                "sample format",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "layout",
                OptionKind::ChannelLayout,
                OptionValue::ChannelLayout(ChannelLayoutSpec::native(ChannelLayout::stereo())),
                "channel layout",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "blob",
                OptionKind::Binary,
                OptionValue::Binary(vec![0x00, 0x01, 0xAA, 0xFF]),
                "binary data",
            )
            .unwrap(),
        )
        .unwrap();
    let mut metadata_dict = Dictionary::new();
    metadata_dict.set("title", "clip").unwrap();
    metadata_dict.set("note", "hello:world").unwrap();
    options
        .define(
            OptionDefinition::new(
                "dict",
                OptionKind::Dictionary,
                OptionValue::Dictionary(metadata_dict),
                "dictionary data",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new(
                "aspect_ratio",
                OptionKind::Rational {
                    min: Rational::ONE,
                    max: Rational::new(16, 9).unwrap(),
                },
                OptionValue::Rational(Rational::ONE),
                "sample aspect ratio",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "readonly",
                OptionKind::Bool,
                OptionValue::Bool(false),
                "exported read-only value",
                OptionFlags::from_bits_truncate(
                    OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
                ),
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_unit(
                "preset_level",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(0),
                "preset level",
                "preset",
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_constant(
            OptionConstant::new_with_flags(
                "preset",
                "fast",
                OptionValue::Int(2),
                "fast preset",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_constant(
            OptionConstant::new_with_flags(
                "preset",
                "slow",
                OptionValue::Int(8),
                "slow preset",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn option_name_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 22 {
        0 => "threads".to_owned(),
        1 => "THREADS".to_owned(),
        2 => "bitexact".to_owned(),
        3 => "quality".to_owned(),
        4 => "metadata".to_owned(),
        5 => "size".to_owned(),
        6 => "codec".to_owned(),
        7 => "CODEC".to_owned(),
        8 => String::new(),
        9 => "bad\0name".to_owned(),
        10 => literal_from(cursor),
        11 => "new-option".to_owned(),
        12 => "new_option".to_owned(),
        13 => "readonly".to_owned(),
        14 => "aspect_ratio".to_owned(),
        15 => "color".to_owned(),
        16 => "pix_fmt".to_owned(),
        17 => "sample_fmt".to_owned(),
        18 => "layout".to_owned(),
        19 => "blob".to_owned(),
        20 => "dict".to_owned(),
        _ => "preset_level".to_owned(),
    }
}

fn option_value_string_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 77 {
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
        16 => "fast".to_owned(),
        17 => "FAST".to_owned(),
        18 => "slow".to_owned(),
        19 => "not_an_int".to_owned(),
        20 => "4/3".to_owned(),
        21 => "3/2".to_owned(),
        22 => "1/0".to_owned(),
        23 => "1/".to_owned(),
        24 => "2/1".to_owned(),
        25 => "2*3".to_owned(),
        26 => "500m".to_owned(),
        27 => "slow+2".to_owned(),
        28 => "1+1/2".to_owned(),
        29 => "1K".to_owned(),
        30 => "2*".to_owned(),
        31 => "1.5".to_owned(),
        32 => "00:01:02.250".to_owned(),
        33 => "1500ms".to_owned(),
        34 => "42us".to_owned(),
        35 => "bad_duration".to_owned(),
        36 => "640x480".to_owned(),
        37 => "hd720".to_owned(),
        38 => "none".to_owned(),
        39 => "bad_size".to_owned(),
        40 => "0x480".to_owned(),
        41 => "ntsc".to_owned(),
        42 => "film".to_owned(),
        43 => "30000/1001".to_owned(),
        44 => "bad_rate".to_owned(),
        45 => "121".to_owned(),
        46 => "red".to_owned(),
        47 => "Blue@0.5".to_owned(),
        48 => "#112233".to_owned(),
        49 => "0x11223344".to_owned(),
        50 => "not-a-color".to_owned(),
        51 => "red@2".to_owned(),
        52 => "rgb24".to_owned(),
        53 => "gray".to_owned(),
        54 => "0x3".to_owned(),
        55 => "25".to_owned(),
        56 => "bad_pix_fmt".to_owned(),
        57 => "bgr24".to_owned(),
        58 => "fltp".to_owned(),
        59 => "s64".to_owned(),
        60 => "s64p".to_owned(),
        61 => "bad_sample_fmt".to_owned(),
        62 => "12".to_owned(),
        63 => "dbl".to_owned(),
        64 => "mono".to_owned(),
        65 => "5.1".to_owned(),
        66 => "2C".to_owned(),
        67 => "bad_layout".to_owned(),
        68 => "0x3".to_owned(),
        69 => "0001aaff".to_owned(),
        70 => "0f10Aa".to_owned(),
        71 => "abc".to_owned(),
        72 => "0g".to_owned(),
        73 => "title=clip:note=hello\\:world".to_owned(),
        74 => "artist=rust:comment='a:b'".to_owned(),
        75 => "missing-separator".to_owned(),
        76 => "key=".to_owned(),
        _ => literal_from(cursor),
    }
}

fn option_value_from(cursor: &mut Cursor<'_>) -> OptionValue {
    match cursor.next().unwrap_or_default() % 14 {
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
        3 => {
            let value = match cursor.next().unwrap_or_default() % 5 {
                0 => Rational::ONE,
                1 => Rational::new(4, 3).unwrap(),
                2 => Rational::new(2, 1).unwrap(),
                3 => Rational::from_raw(1, 0),
                _ => Rational::ZERO,
            };
            OptionValue::Rational(value)
        }
        4 => OptionValue::Duration(i64::from(cursor.next().unwrap_or_default()) * 1_000),
        5 => OptionValue::ImageSize {
            width: i32::from(cursor.next().unwrap_or_default()),
            height: i32::from(cursor.next().unwrap_or_default()),
        },
        6 => {
            let value = match cursor.next().unwrap_or_default() % 5 {
                0 => Rational::ONE,
                1 => Rational::new(25, 1).unwrap(),
                2 => Rational::new(30000, 1001).unwrap(),
                3 => Rational::from_raw(1, 0),
                _ => Rational::ZERO,
            };
            OptionValue::VideoRate(value)
        }
        7 => OptionValue::Color(RgbaColor::from_rgba([
            cursor.next().unwrap_or_default(),
            cursor.next().unwrap_or_default(),
            cursor.next().unwrap_or_default(),
            cursor.next().unwrap_or_default(),
        ])),
        8 => {
            let value = match cursor.next().unwrap_or_default() % 5 {
                0 => Some(PixelFormat::Yuv420p),
                1 => Some(PixelFormat::Rgb24),
                2 => Some(PixelFormat::Bgr24),
                3 => Some(PixelFormat::Nv21),
                _ => None,
            };
            OptionValue::PixelFormat(value)
        }
        9 => {
            let value = match cursor.next().unwrap_or_default() % 5 {
                0 => Some(SampleFormat::S16),
                1 => Some(SampleFormat::FltP),
                2 => Some(SampleFormat::Dbl),
                3 => Some(SampleFormat::S64P),
                _ => None,
            };
            OptionValue::SampleFormat(value)
        }
        10 => {
            let value = match cursor.next().unwrap_or_default() % 4 {
                0 => ChannelLayoutSpec::native(ChannelLayout::stereo()),
                1 => ChannelLayoutSpec::native(ChannelLayout::mono()),
                2 => ChannelLayoutSpec::native(ChannelLayout::five_one()),
                _ => ChannelLayoutSpec::unspecified(2).unwrap(),
            };
            OptionValue::ChannelLayout(value)
        }
        11 => {
            let len = usize::from(cursor.next().unwrap_or_default()) % 8;
            let mut value = Vec::with_capacity(len);
            for _ in 0..len {
                value.push(cursor.next().unwrap_or_default());
            }
            OptionValue::Binary(value)
        }
        12 => OptionValue::Dictionary(generated_options_dictionary(cursor)),
        _ => OptionValue::String(option_value_string_from(cursor)),
    }
}

fn option_flags_from(byte: Option<u8>) -> OptionFlags {
    let raw = u32::from(byte.unwrap_or_default());
    let mut bits = 0;

    if raw & 0x01 != 0 {
        bits |= OptionFlags::ENCODING_PARAM.bits();
    }
    if raw & 0x02 != 0 {
        bits |= OptionFlags::DECODING_PARAM.bits();
    }
    if raw & 0x04 != 0 {
        bits |= OptionFlags::READONLY.bits();
    }
    if raw & 0x08 != 0 {
        bits |= OptionFlags::VIDEO_PARAM.bits();
    }
    if raw & 0x10 != 0 {
        bits |= OptionFlags::AUDIO_PARAM.bits();
    }
    if raw & 0x20 != 0 {
        bits |= OptionFlags::FILTERING_PARAM.bits();
    }
    if raw & 0x40 != 0 {
        bits |= OptionFlags::EXPORT.bits();
    }
    if raw & 0x80 != 0 {
        bits |= OptionFlags::RUNTIME_PARAM.bits();
    }

    OptionFlags::from_bits_truncate(bits | 0x8000_0000)
}

fn option_search_flags_from(byte: Option<u8>) -> OptionSearchFlags {
    let raw = u32::from(byte.unwrap_or_default());
    let mut bits = 0;

    if raw & 0x01 != 0 {
        bits |= OptionSearchFlags::CHILDREN.bits();
    }
    if raw & 0x02 != 0 {
        bits |= OptionSearchFlags::FAKE_OBJ.bits();
    }

    OptionSearchFlags::from_bits_truncate(bits | 0x8000_0000)
}

fn option_serialize_flags_from(byte: Option<u8>) -> OptionSerializeFlags {
    let raw = u32::from(byte.unwrap_or_default());
    let mut bits = 0;

    if raw & 0x01 != 0 {
        bits |= OptionSerializeFlags::SKIP_DEFAULTS.bits();
    }
    if raw & 0x02 != 0 {
        bits |= OptionSerializeFlags::OPT_FLAGS_EXACT.bits();
    }
    if raw & 0x04 != 0 {
        bits |= OptionSerializeFlags::SEARCH_CHILDREN.bits();
    }

    OptionSerializeFlags::from_bits_truncate(bits | 0x8000_0000)
}

fn option_definition_unit_from(cursor: &mut Cursor<'_>) -> Option<String> {
    match cursor.next().unwrap_or_default() % 5 {
        0 => None,
        1 => Some("preset".to_owned()),
        2 => Some("mode".to_owned()),
        3 => Some("bad\0unit".to_owned()),
        _ => Some(literal_from(cursor)),
    }
}

fn option_unit_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 6 {
        0 => "preset".to_owned(),
        1 => "PRESET".to_owned(),
        2 => "mode".to_owned(),
        3 => String::new(),
        4 => "bad\0unit".to_owned(),
        _ => literal_from(cursor),
    }
}

fn option_constant_name_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 8 {
        0 => "fast".to_owned(),
        1 => "FAST".to_owned(),
        2 => "slow".to_owned(),
        3 => "not_an_int".to_owned(),
        4 => String::new(),
        5 => "bad\0name".to_owned(),
        _ => literal_from(cursor),
    }
}

fn option_child_name_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 8 {
        0 => "encoder".to_owned(),
        1 => "ENCODER".to_owned(),
        2 => "decoder".to_owned(),
        3 => "filter".to_owned(),
        4 => String::new(),
        5 => "bad\0child".to_owned(),
        _ => literal_from(cursor),
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
    match byte.unwrap_or_default() % 5 {
        0 => SetMode::Overwrite,
        1 => SetMode::KeepExisting,
        2 => SetMode::Append,
        3 => SetMode::AllowMultiple,
        _ => SetMode::AllowMultipleDedup,
    }
}

fn separator_char_from(byte: Option<u8>) -> char {
    match byte.unwrap_or_default() % 7 {
        0 => '=',
        1 => ';',
        2 => ':',
        3 => '|',
        4 => ',',
        5 => '\\',
        _ => '\0',
    }
}

fn separator_set_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 8 {
        0 => "=".to_owned(),
        1 => ";".to_owned(),
        2 => ":".to_owned(),
        3 => "|,".to_owned(),
        4 => String::new(),
        5 => "\\".to_owned(),
        6 => "=\0".to_owned(),
        _ => literal_from(cursor),
    }
}

fn dictionary_pairs_string_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 10 {
        0 => "artist=Alice;title=Clip".to_owned(),
        1 => "artist=old;ARTIST=new".to_owned(),
        2 => "a\\=b=v\\;x".to_owned(),
        3 => "ok=value;bad".to_owned(),
        4 => "dangling=escape\\".to_owned(),
        5 => "=empty-key".to_owned(),
        6 => "nul=bad\0value".to_owned(),
        7 => "a:b|c:d".to_owned(),
        8 => "key=value,".to_owned(),
        _ => literal_from(cursor),
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

fn ascii_eq_ignore_case(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .as_bytes()
            .iter()
            .zip(right.as_bytes())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn dictionary_key_matches(candidate: &str, key: &str, match_mode: MatchMode) -> bool {
    match match_mode {
        MatchMode::CaseInsensitive => ascii_eq_ignore_case(candidate, key),
        MatchMode::CaseSensitive => candidate == key,
    }
}

fn dictionary_key_has_prefix(candidate: &str, prefix: &str, match_mode: MatchMode) -> bool {
    match match_mode {
        MatchMode::CaseInsensitive => {
            candidate.len() >= prefix.len()
                && candidate.as_bytes()[..prefix.len()]
                    .iter()
                    .zip(prefix.as_bytes())
                    .all(|(left, right)| left.eq_ignore_ascii_case(right))
        }
        MatchMode::CaseSensitive => candidate.starts_with(prefix),
    }
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
