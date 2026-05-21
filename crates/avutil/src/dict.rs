use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    CaseInsensitive,
    CaseSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetMode {
    Overwrite,
    KeepExisting,
    Append,
    AllowMultiple,
    AllowMultipleDedup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionarySet {
    Inserted,
    Replaced,
    Kept,
    Appended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryEntry {
    key: String,
    value: String,
}

impl DictionaryEntry {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dictionary {
    entries: Vec<DictionaryEntry>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[DictionaryEntry] {
        &self.entries
    }

    pub fn set(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> AvResult<DictionarySet> {
        self.set_with_mode(key, value, MatchMode::CaseInsensitive, SetMode::Overwrite)
    }

    pub fn set_with_mode(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        match_mode: MatchMode,
        set_mode: SetMode,
    ) -> AvResult<DictionarySet> {
        let key = validate_key(key.into())?;
        let value = validate_value(value.into())?;

        if set_mode == SetMode::AllowMultipleDedup
            && self
                .entries
                .iter()
                .any(|entry| key_matches(entry.key(), &key, match_mode) && entry.value() == value)
        {
            return Ok(DictionarySet::Kept);
        }

        if matches!(
            set_mode,
            SetMode::AllowMultiple | SetMode::AllowMultipleDedup
        ) {
            self.entries.push(DictionaryEntry { key, value });
            return Ok(DictionarySet::Inserted);
        }

        if let Some(index) = self.find_index(&key, match_mode) {
            let entry = &mut self.entries[index];
            return match set_mode {
                SetMode::Overwrite => {
                    entry.key = key;
                    entry.value = value;
                    Ok(DictionarySet::Replaced)
                }
                SetMode::KeepExisting => Ok(DictionarySet::Kept),
                SetMode::Append => {
                    entry.key = key;
                    entry.value.push_str(&value);
                    Ok(DictionarySet::Appended)
                }
                SetMode::AllowMultiple | SetMode::AllowMultipleDedup => unreachable!(),
            };
        }

        self.entries.push(DictionaryEntry { key, value });
        Ok(DictionarySet::Inserted)
    }

    pub fn set_int(&mut self, key: impl Into<String>, value: i64) -> AvResult<DictionarySet> {
        self.set_int_with_mode(key, value, MatchMode::CaseInsensitive, SetMode::Overwrite)
    }

    pub fn set_int_with_mode(
        &mut self,
        key: impl Into<String>,
        value: i64,
        match_mode: MatchMode,
        set_mode: SetMode,
    ) -> AvResult<DictionarySet> {
        self.set_with_mode(key, value.to_string(), match_mode, set_mode)
    }

    pub fn copy_from(
        &mut self,
        source: &Dictionary,
        match_mode: MatchMode,
        set_mode: SetMode,
    ) -> AvResult<Vec<DictionarySet>> {
        let mut results = Vec::with_capacity(source.len());
        for entry in source.entries() {
            let result = self.set_with_mode(entry.key(), entry.value(), match_mode, set_mode)?;
            results.push(result);
        }
        Ok(results)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.get_entry(key, MatchMode::CaseInsensitive)
            .map(DictionaryEntry::value)
    }

    pub fn get_entry(&self, key: &str, match_mode: MatchMode) -> Option<&DictionaryEntry> {
        self.entries
            .iter()
            .find(|entry| key_matches(entry.key(), key, match_mode))
    }

    pub fn matching_entries<'a>(
        &'a self,
        key: &'a str,
        match_mode: MatchMode,
    ) -> impl Iterator<Item = &'a DictionaryEntry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| key_matches(entry.key(), key, match_mode))
    }

    pub fn get_prefixed(&self, prefix: &str, match_mode: MatchMode) -> Option<&DictionaryEntry> {
        self.entries
            .iter()
            .find(|entry| key_has_prefix(entry.key(), prefix, match_mode))
    }

    pub fn prefixed_entries<'a>(
        &'a self,
        prefix: &'a str,
        match_mode: MatchMode,
    ) -> impl Iterator<Item = &'a DictionaryEntry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| key_has_prefix(entry.key(), prefix, match_mode))
    }

    pub fn remove(&mut self, key: &str, match_mode: MatchMode) -> Option<DictionaryEntry> {
        self.find_index(key, match_mode)
            .map(|index| self.entries.remove(index))
    }

    pub fn remove_all(&mut self, key: &str, match_mode: MatchMode) -> Vec<DictionaryEntry> {
        let mut removed = Vec::new();
        let mut index = 0;

        while index < self.entries.len() {
            if key_matches(self.entries[index].key(), key, match_mode) {
                removed.push(self.entries.remove(index));
            } else {
                index += 1;
            }
        }

        removed
    }

    pub fn parse_pairs(
        raw: &str,
        key_value_separators: &str,
        pair_separators: &str,
        match_mode: MatchMode,
        set_mode: SetMode,
    ) -> AvResult<Self> {
        let mut dict = Self::new();
        dict.parse_pairs_into(
            raw,
            key_value_separators,
            pair_separators,
            match_mode,
            set_mode,
        )?;
        Ok(dict)
    }

    pub fn parse_pairs_into(
        &mut self,
        raw: &str,
        key_value_separators: &str,
        pair_separators: &str,
        match_mode: MatchMode,
        set_mode: SetMode,
    ) -> AvResult<Vec<DictionarySet>> {
        validate_separator_set(key_value_separators, "key/value separators")?;
        validate_separator_set(pair_separators, "pair separators")?;
        validate_disjoint_separator_sets(key_value_separators, pair_separators)?;

        let mut chars = raw.chars().peekable();
        let mut results = Vec::new();

        while chars.peek().is_some() {
            let (key, key_separator) = parse_escaped_token(&mut chars, key_value_separators)?;
            if key_separator.is_none() {
                return Err(AvError::invalid_argument(
                    "dictionary pair is missing a key/value separator",
                ));
            }

            let (value, _) = parse_escaped_token(&mut chars, pair_separators)?;
            let result = self.set_with_mode(key, value, match_mode, set_mode)?;
            results.push(result);
        }

        Ok(results)
    }

    pub fn to_pairs_string(
        &self,
        key_value_separator: char,
        pair_separator: char,
    ) -> AvResult<String> {
        validate_output_separator(key_value_separator, "key/value separator")?;
        validate_output_separator(pair_separator, "pair separator")?;
        if key_value_separator == pair_separator {
            return Err(AvError::invalid_argument(
                "dictionary separators must be distinct",
            ));
        }

        let mut output = String::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if index > 0 {
                output.push(pair_separator);
            }
            push_escaped(
                &mut output,
                entry.key(),
                key_value_separator,
                pair_separator,
            );
            output.push(key_value_separator);
            push_escaped(
                &mut output,
                entry.value(),
                key_value_separator,
                pair_separator,
            );
        }
        Ok(output)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn find_index(&self, key: &str, match_mode: MatchMode) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| key_matches(entry.key(), key, match_mode))
    }
}

fn validate_key(key: String) -> AvResult<String> {
    if key.is_empty() {
        return Err(AvError::invalid_argument(
            "dictionary key must not be empty",
        ));
    }

    if key.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "dictionary key must not contain NUL",
        ));
    }

    Ok(key)
}

fn validate_value(value: String) -> AvResult<String> {
    if value.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "dictionary value must not contain NUL",
        ));
    }

    Ok(value)
}

fn validate_separator_set(separators: &str, label: &str) -> AvResult<()> {
    if separators.is_empty() {
        return Err(AvError::invalid_argument(format!(
            "{label} must not be empty"
        )));
    }
    for separator in separators.chars() {
        validate_output_separator(separator, label)?;
    }
    Ok(())
}

fn validate_disjoint_separator_sets(left: &str, right: &str) -> AvResult<()> {
    if left.chars().any(|separator| right.contains(separator)) {
        return Err(AvError::invalid_argument(
            "dictionary separator sets must be distinct",
        ));
    }
    Ok(())
}

fn validate_output_separator(separator: char, label: &str) -> AvResult<()> {
    if separator == '\0' {
        return Err(AvError::invalid_argument(format!(
            "{label} must not be NUL"
        )));
    }
    if separator == '\\' {
        return Err(AvError::invalid_argument(
            "dictionary separators must not be backslash",
        ));
    }
    Ok(())
}

fn parse_escaped_token<I>(
    chars: &mut std::iter::Peekable<I>,
    separators: &str,
) -> AvResult<(String, Option<char>)>
where
    I: Iterator<Item = char>,
{
    let mut token = String::new();

    while let Some(ch) = chars.next() {
        if separators.contains(ch) {
            return Ok((token, Some(ch)));
        }

        if ch == '\\' {
            let escaped = chars.next().ok_or_else(|| {
                AvError::invalid_argument("dictionary token ends with a dangling escape")
            })?;
            token.push(escaped);
            continue;
        }

        token.push(ch);
    }

    Ok((token, None))
}

fn push_escaped(output: &mut String, value: &str, key_value_separator: char, pair_separator: char) {
    for ch in value.chars() {
        if ch == '\\' || ch == key_value_separator || ch == pair_separator {
            output.push('\\');
        }
        output.push(ch);
    }
}

fn key_matches(candidate: &str, key: &str, match_mode: MatchMode) -> bool {
    match match_mode {
        MatchMode::CaseSensitive => candidate == key,
        MatchMode::CaseInsensitive => ascii_eq_ignore_case(candidate, key),
    }
}

fn key_has_prefix(candidate: &str, prefix: &str, match_mode: MatchMode) -> bool {
    match match_mode {
        MatchMode::CaseSensitive => candidate.starts_with(prefix),
        MatchMode::CaseInsensitive => {
            candidate.len() >= prefix.len()
                && ascii_eq_ignore_case_bytes(
                    &candidate.as_bytes()[..prefix.len()],
                    prefix.as_bytes(),
                )
        }
    }
}

fn ascii_eq_ignore_case(left: &str, right: &str) -> bool {
    left.len() == right.len() && ascii_eq_ignore_case_bytes(left.as_bytes(), right.as_bytes())
}

fn ascii_eq_ignore_case_bytes(left: &[u8], right: &[u8]) -> bool {
    left.iter()
        .zip(right)
        .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn default_set_replaces_case_insensitive_key() {
        let mut dict = Dictionary::new();

        assert_eq!(dict.set("Title", "First").unwrap(), DictionarySet::Inserted);
        assert_eq!(
            dict.set("title", "Second").unwrap(),
            DictionarySet::Replaced
        );

        assert_eq!(dict.len(), 1);
        assert_eq!(dict.get("TITLE"), Some("Second"));
        assert_eq!(dict.entries()[0].key(), "title");
    }

    #[test]
    fn case_sensitive_mode_allows_distinct_keys() {
        let mut dict = Dictionary::new();

        dict.set_with_mode(
            "TITLE",
            "upper",
            MatchMode::CaseSensitive,
            SetMode::Overwrite,
        )
        .unwrap();
        dict.set_with_mode(
            "title",
            "lower",
            MatchMode::CaseSensitive,
            SetMode::Overwrite,
        )
        .unwrap();

        assert_eq!(dict.len(), 2);
        assert_eq!(
            dict.get_entry("TITLE", MatchMode::CaseSensitive)
                .map(DictionaryEntry::value),
            Some("upper")
        );
        assert_eq!(
            dict.get_entry("title", MatchMode::CaseSensitive)
                .map(DictionaryEntry::value),
            Some("lower")
        );
    }

    #[test]
    fn keep_existing_preserves_original_entry() {
        let mut dict = Dictionary::new();

        dict.set("artist", "first").unwrap();
        let result = dict
            .set_with_mode(
                "ARTIST",
                "second",
                MatchMode::CaseInsensitive,
                SetMode::KeepExisting,
            )
            .unwrap();

        assert_eq!(result, DictionarySet::Kept);
        assert_eq!(dict.get("artist"), Some("first"));
        assert_eq!(dict.entries()[0].key(), "artist");
    }

    #[test]
    fn append_mode_concatenates_existing_value() {
        let mut dict = Dictionary::new();

        dict.set("comment", "part1").unwrap();
        let result = dict
            .set_with_mode(
                "COMMENT",
                "+part2",
                MatchMode::CaseInsensitive,
                SetMode::Append,
            )
            .unwrap();

        assert_eq!(result, DictionarySet::Appended);
        assert_eq!(dict.get("comment"), Some("part1+part2"));
        assert_eq!(dict.entries()[0].key(), "COMMENT");
    }

    #[test]
    fn multikey_mode_preserves_duplicate_entries_in_order() {
        let mut dict = Dictionary::new();

        dict.set("artist", "first").unwrap();
        let result = dict
            .set_with_mode(
                "ARTIST",
                "second",
                MatchMode::CaseInsensitive,
                SetMode::AllowMultiple,
            )
            .unwrap();

        assert_eq!(result, DictionarySet::Inserted);
        assert_eq!(dict.len(), 2);
        assert_eq!(dict.entries()[0].key(), "artist");
        assert_eq!(dict.entries()[0].value(), "first");
        assert_eq!(dict.entries()[1].key(), "ARTIST");
        assert_eq!(dict.entries()[1].value(), "second");
        assert_eq!(dict.get("artist"), Some("first"));
        assert_eq!(
            dict.get_entry("ARTIST", MatchMode::CaseSensitive)
                .map(DictionaryEntry::value),
            Some("second")
        );

        let removed = dict.remove("artist", MatchMode::CaseInsensitive).unwrap();
        assert_eq!(removed.value(), "first");
        assert_eq!(dict.len(), 1);
        assert_eq!(dict.get("artist"), Some("second"));
    }

    #[test]
    fn multikey_dedup_keeps_existing_matching_key_value_pair() {
        let mut dict = Dictionary::new();

        dict.set_with_mode(
            "artist",
            "first",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "ARTIST",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();

        let kept = dict
            .set_with_mode(
                "Artist",
                "first",
                MatchMode::CaseInsensitive,
                SetMode::AllowMultipleDedup,
            )
            .unwrap();
        let inserted = dict
            .set_with_mode(
                "Artist",
                "first",
                MatchMode::CaseSensitive,
                SetMode::AllowMultipleDedup,
            )
            .unwrap();

        assert_eq!(kept, DictionarySet::Kept);
        assert_eq!(inserted, DictionarySet::Inserted);
        assert_eq!(
            dict.entries()
                .iter()
                .map(|entry| (entry.key(), entry.value()))
                .collect::<Vec<_>>(),
            vec![
                ("artist", "first"),
                ("ARTIST", "second"),
                ("Artist", "first")
            ]
        );
    }

    #[test]
    fn set_int_formats_decimal_values() {
        let mut dict = Dictionary::new();

        assert_eq!(dict.set_int("count", -42).unwrap(), DictionarySet::Inserted);
        assert_eq!(
            dict.set_int_with_mode(
                "COUNT",
                i64::MAX,
                MatchMode::CaseInsensitive,
                SetMode::Overwrite,
            )
            .unwrap(),
            DictionarySet::Replaced
        );

        assert_eq!(dict.entries()[0].key(), "COUNT");
        assert_eq!(dict.get("count"), Some("9223372036854775807"));
    }

    #[test]
    fn copy_from_applies_requested_set_mode_in_order() {
        let mut source = Dictionary::new();
        source
            .set_with_mode(
                "artist",
                "first",
                MatchMode::CaseInsensitive,
                SetMode::AllowMultiple,
            )
            .unwrap();
        source
            .set_with_mode(
                "ARTIST",
                "second",
                MatchMode::CaseInsensitive,
                SetMode::AllowMultiple,
            )
            .unwrap();

        let mut destination = Dictionary::new();
        destination.set("artist", "old").unwrap();
        let results = destination
            .copy_from(
                &source,
                MatchMode::CaseInsensitive,
                SetMode::AllowMultipleDedup,
            )
            .unwrap();

        assert_eq!(
            results,
            vec![DictionarySet::Inserted, DictionarySet::Inserted]
        );
        assert_eq!(
            destination
                .entries()
                .iter()
                .map(|entry| (entry.key(), entry.value()))
                .collect::<Vec<_>>(),
            vec![("artist", "old"), ("artist", "first"), ("ARTIST", "second")]
        );
    }

    #[test]
    fn prefix_lookup_returns_first_matching_entry() {
        let mut dict = Dictionary::new();
        dict.set("artist-sort", "Lastname").unwrap();
        dict.set("album", "Record").unwrap();

        let entry = dict
            .get_prefixed("ARTIST", MatchMode::CaseInsensitive)
            .unwrap();

        assert_eq!(entry.key(), "artist-sort");
        assert_eq!(entry.value(), "Lastname");
        assert!(dict
            .get_prefixed("ARTIST", MatchMode::CaseSensitive)
            .is_none());
    }

    #[test]
    fn exact_match_iterator_preserves_duplicate_order() {
        let mut dict = Dictionary::new();
        dict.set("artist", "first").unwrap();
        dict.set_with_mode(
            "ARTIST",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "album",
            "record",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();

        let insensitive_matches: Vec<_> = dict
            .matching_entries("artist", MatchMode::CaseInsensitive)
            .map(|entry| (entry.key(), entry.value()))
            .collect();
        let sensitive_matches: Vec<_> = dict
            .matching_entries("artist", MatchMode::CaseSensitive)
            .map(DictionaryEntry::value)
            .collect();

        assert_eq!(
            insensitive_matches,
            vec![("artist", "first"), ("ARTIST", "second")]
        );
        assert_eq!(sensitive_matches, vec!["first"]);
        assert!(dict
            .matching_entries("missing", MatchMode::CaseInsensitive)
            .next()
            .is_none());
    }

    #[test]
    fn prefix_match_iterator_preserves_order_and_empty_prefix_matches_all() {
        let mut dict = Dictionary::new();
        dict.set_with_mode(
            "artist",
            "name",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "ARTIST-sort",
            "sort",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "album",
            "record",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();

        let insensitive: Vec<_> = dict
            .prefixed_entries("artist", MatchMode::CaseInsensitive)
            .map(DictionaryEntry::key)
            .collect();
        let sensitive: Vec<_> = dict
            .prefixed_entries("artist", MatchMode::CaseSensitive)
            .map(DictionaryEntry::key)
            .collect();
        let all_keys: Vec<_> = dict
            .prefixed_entries("", MatchMode::CaseInsensitive)
            .map(DictionaryEntry::key)
            .collect();

        assert_eq!(insensitive, vec!["artist", "ARTIST-sort"]);
        assert_eq!(sensitive, vec!["artist"]);
        assert_eq!(all_keys, vec!["artist", "ARTIST-sort", "album"]);
    }

    #[test]
    fn remove_defaults_to_requested_match_mode() {
        let mut dict = Dictionary::new();
        dict.set("language", "eng").unwrap();

        assert!(dict.remove("LANGUAGE", MatchMode::CaseSensitive).is_none());
        let removed = dict.remove("LANGUAGE", MatchMode::CaseInsensitive).unwrap();

        assert_eq!(removed.key(), "language");
        assert!(dict.is_empty());
    }

    #[test]
    fn remove_all_preserves_removed_order_and_remaining_entries() {
        let mut dict = Dictionary::new();
        dict.set_with_mode(
            "artist",
            "first",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "album",
            "record",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "ARTIST",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "artist-sort",
            "name",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();

        let sensitive_removed = dict.remove_all("artist", MatchMode::CaseSensitive);
        assert_eq!(
            sensitive_removed
                .iter()
                .map(|entry| (entry.key(), entry.value()))
                .collect::<Vec<_>>(),
            vec![("artist", "first")]
        );
        assert_eq!(
            dict.entries()
                .iter()
                .map(|entry| entry.key())
                .collect::<Vec<_>>(),
            vec!["album", "ARTIST", "artist-sort"]
        );

        let insensitive_removed = dict.remove_all("artist", MatchMode::CaseInsensitive);
        assert_eq!(
            insensitive_removed
                .iter()
                .map(|entry| (entry.key(), entry.value()))
                .collect::<Vec<_>>(),
            vec![("ARTIST", "second")]
        );
        assert_eq!(
            dict.entries()
                .iter()
                .map(|entry| (entry.key(), entry.value()))
                .collect::<Vec<_>>(),
            vec![("album", "record"), ("artist-sort", "name")]
        );

        assert!(dict
            .remove_all("missing", MatchMode::CaseInsensitive)
            .is_empty());
    }

    #[test]
    fn pairs_string_escapes_separators_and_round_trips_duplicate_entries() {
        let mut dict = Dictionary::new();
        dict.set_with_mode(
            "title=name",
            "one;two\\three",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "TITLE=NAME",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set("artist", "Alice").unwrap();

        let encoded = dict.to_pairs_string('=', ';').unwrap();
        assert_eq!(
            encoded,
            "title\\=name=one\\;two\\\\three;TITLE\\=NAME=second;artist=Alice"
        );

        let reparsed = Dictionary::parse_pairs(
            &encoded,
            "=",
            ";",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();

        assert_eq!(reparsed, dict);
    }

    #[test]
    fn parse_pairs_applies_modes_and_returns_per_entry_results() {
        let mut dict = Dictionary::new();
        dict.set("artist", "old").unwrap();

        let results = dict
            .parse_pairs_into(
                "ARTIST=new;comment=ok;comment=!",
                "=",
                ";",
                MatchMode::CaseInsensitive,
                SetMode::Append,
            )
            .unwrap();

        assert_eq!(
            results,
            vec![
                DictionarySet::Appended,
                DictionarySet::Inserted,
                DictionarySet::Appended
            ]
        );
        assert_eq!(dict.get("artist"), Some("oldnew"));
        assert_eq!(dict.get("comment"), Some("ok!"));
        assert_eq!(dict.entries()[0].key(), "ARTIST");
        assert_eq!(dict.entries()[1].key(), "comment");
    }

    #[test]
    fn parse_pairs_preserves_successful_entries_on_later_error() {
        let mut dict = Dictionary::new();

        let err = dict
            .parse_pairs_into(
                "ok=value;bad",
                "=",
                ";",
                MatchMode::CaseInsensitive,
                SetMode::Overwrite,
            )
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(dict.len(), 1);
        assert_eq!(dict.get("ok"), Some("value"));
    }

    #[test]
    fn pair_serialization_and_parsing_reject_invalid_separators_and_tokens() {
        let mut dict = Dictionary::new();
        dict.set("title", "value").unwrap();

        assert!(dict.to_pairs_string('\\', ';').is_err());
        assert!(dict.to_pairs_string('=', '=').is_err());
        assert!(Dictionary::parse_pairs(
            "a=b",
            "",
            ";",
            MatchMode::CaseInsensitive,
            SetMode::Overwrite
        )
        .is_err());
        assert!(Dictionary::parse_pairs(
            "a=b",
            "=",
            "=",
            MatchMode::CaseInsensitive,
            SetMode::Overwrite
        )
        .is_err());
        assert!(Dictionary::parse_pairs(
            "a=trailing\\",
            "=",
            ";",
            MatchMode::CaseInsensitive,
            SetMode::Overwrite
        )
        .is_err());
        assert!(Dictionary::parse_pairs(
            "=value",
            "=",
            ";",
            MatchMode::CaseInsensitive,
            SetMode::Overwrite
        )
        .is_err());
    }

    #[test]
    fn rejects_empty_or_nul_keys_and_nul_values() {
        let mut dict = Dictionary::new();

        let empty_key = dict.set("", "value").unwrap_err();
        let nul_key = dict.set("bad\0key", "value").unwrap_err();
        let nul_value = dict.set("key", "bad\0value").unwrap_err();

        assert_eq!(empty_key.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(nul_key.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(nul_value.kind(), AvErrorKind::InvalidArgument);
        assert!(dict.is_empty());
    }
}
