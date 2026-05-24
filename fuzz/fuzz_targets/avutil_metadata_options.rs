#![no_main]

use avutil::{
    AvErrorCode, AvOptionRanges, Dictionary, DictionarySet, MatchMode, OptionChild, OptionConstant,
    OptionDefinition, OptionEntryMatch, OptionFlags, OptionKind, OptionQuery, OptionSearchFlags,
    OptionSet, OptionValue, Rational, SetMode,
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
        match cursor.next().unwrap_or_default() % 19 {
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
            17 => {
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
    let aspect_ranges = options.query_avoption_ranges("aspect_ratio").unwrap();
    assert_eq!(aspect_ranges.ranges()[0].component_min(), i32::MIN as f64);
    assert_eq!(aspect_ranges.ranges()[0].component_max(), i32::MAX as f64);
    let missing_range = options.query_avoption_ranges("THREADS").unwrap_err();
    assert_eq!(missing_range.code(), Some(AvErrorCode::ENOMEM));

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
    assert_eq!(
        options
            .get_avoption_string_with_flags("threads", OptionSearchFlags::FAKE_OBJ)
            .unwrap_err()
            .code(),
        Some(AvErrorCode::OPTION_NOT_FOUND)
    );
    assert!(options
        .set_child("encoder", "threads", OptionValue::Int(99))
        .is_err());
    assert_eq!(
        options.get_child_option("encoder", "threads").unwrap(),
        &OptionValue::Int(9)
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
    let kind = match kind_tag % 11 {
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
        9 => OptionKind::String { allow_empty: true },
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
    match cursor.next().unwrap_or_default() % 15 {
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
        11 => "new_option".to_owned(),
        12 => "readonly".to_owned(),
        13 => "aspect_ratio".to_owned(),
        _ => "preset_level".to_owned(),
    }
}

fn option_value_string_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 26 {
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
        _ => literal_from(cursor),
    }
}

fn option_value_from(cursor: &mut Cursor<'_>) -> OptionValue {
    match cursor.next().unwrap_or_default() % 5 {
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
        1 => "artist=old;artist=new".to_owned(),
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
