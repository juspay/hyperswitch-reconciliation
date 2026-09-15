use common_types::metadata_schema::datatypes::majorunit::ReplaceMode;

pub fn replace_char_in_value(
    value: &mut Option<String>,
    from: char,
    to: Option<char>,
    mode: &ReplaceMode,
) {
    let Some(inner) = value.as_deref() else {
        return;
    };

    let replaced = match mode {
        ReplaceMode::All => match to {
            Some(to_char) => inner.replace(from, &to_char.to_string()),
            None => inner.chars().filter(|&c| c != from).collect::<String>(),
        },
        ReplaceMode::Single {
            occurrence,
            from_end,
        } => {
            let target = if *from_end {
                inner
                    .char_indices()
                    .rev()
                    .filter(|(_, c)| *c == from)
                    .nth(*occurrence)
                    .map(|(index, _)| index)
            } else {
                inner
                    .char_indices()
                    .filter(|(_, c)| *c == from)
                    .nth(*occurrence)
                    .map(|(index, _)| index)
            };
            replace_single_occurrence(inner, to, target)
        }
    };

    *value = Some(replaced);
}

fn replace_single_occurrence(input: &str, to: Option<char>, target: Option<usize>) -> String {
    match target {
        None => input.to_string(),
        Some(target_index) => input.char_indices().fold(
            String::with_capacity(input.len()),
            |mut acc, (index, current)| {
                if index == target_index {
                    if let Some(to_char) = to {
                        acc.push(to_char);
                    }
                } else {
                    acc.push(current);
                }
                acc
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use common_types::metadata_schema::datatypes::majorunit::ReplaceMode;

    use super::replace_char_in_value;

    fn run(value: &str, from: char, to: Option<char>, mode: ReplaceMode) -> Option<String> {
        let mut value = Some(value.to_string());
        replace_char_in_value(&mut value, from, to, &mode);
        value
    }

    #[test]
    fn all_delete_strips_every_occurrence() {
        assert_eq!(
            run("1,000,000", ',', None, ReplaceMode::All),
            Some("1000000".to_string())
        );
    }

    #[test]
    fn all_replace_swaps_every_occurrence() {
        assert_eq!(
            run("1,000,000", ',', Some('.'), ReplaceMode::All),
            Some("1.000.000".to_string())
        );
    }

    #[test]
    fn single_from_start_targets_nth() {
        assert_eq!(
            run(
                "a,b,c",
                ',',
                Some(';'),
                ReplaceMode::Single {
                    occurrence: 1,
                    from_end: false
                }
            ),
            Some("a,b;c".to_string())
        );
    }

    #[test]
    fn single_from_end_targets_last() {
        assert_eq!(
            run(
                "1,234,56",
                ',',
                Some('.'),
                ReplaceMode::Single {
                    occurrence: 0,
                    from_end: true
                }
            ),
            Some("1,234.56".to_string())
        );
    }

    #[test]
    fn single_out_of_range_is_noop() {
        assert_eq!(
            run(
                "1,2",
                ',',
                Some('.'),
                ReplaceMode::Single {
                    occurrence: 5,
                    from_end: false
                }
            ),
            Some("1,2".to_string())
        );
    }

    #[test]
    fn char_absent_is_noop() {
        assert_eq!(
            run("1000", ',', None, ReplaceMode::All),
            Some("1000".to_string())
        );
    }

    #[test]
    fn none_value_is_noop() {
        let mut value: Option<String> = None;
        replace_char_in_value(&mut value, ',', None, &ReplaceMode::All);
        assert_eq!(value, None);
    }

    #[test]
    fn multibyte_prefix_does_not_shift_target() {
        assert_eq!(
            run("\u{20ac}1,00", ',', None, ReplaceMode::All),
            Some("\u{20ac}100".to_string())
        );
        assert_eq!(
            run(
                "\u{20ac}1,00",
                ',',
                Some('.'),
                ReplaceMode::Single {
                    occurrence: 0,
                    from_end: false
                }
            ),
            Some("\u{20ac}1.00".to_string())
        );
    }
}
