pub(super) fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let mut search_from = 0;
    while let Some(found) = tag[search_from..].find(&needle) {
        let start = search_from + found;
        let boundary = tag[..start]
            .chars()
            .next_back()
            .is_none_or(|ch| ch.is_ascii_whitespace());
        let value_start = start + needle.len();
        if boundary {
            let value_end = tag[value_start..].find('"')?;
            return Some(&tag[value_start..value_start + value_end]);
        }
        search_from = value_start;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_quoted_value() {
        assert_eq!(attribute(r#" id="a" d="M0,0""#, "d"), Some("M0,0"));
    }

    #[test]
    fn ignores_names_that_only_end_with_the_requested_one() {
        assert_eq!(attribute(r#" kvg:d="x" d="y""#, "d"), Some("y"));
        assert_eq!(attribute(r#" kvg:d="x""#, "d"), None);
    }

    #[test]
    fn a_missing_attribute_is_none() {
        assert_eq!(attribute(r#" id="a""#, "d"), None);
    }

    #[test]
    fn an_unterminated_value_is_none() {
        assert_eq!(attribute(r#" d="M0,0"#, "d"), None);
    }
}
