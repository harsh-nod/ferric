//! Explicit width selection; all inherited model/profile options remain closed.
pub(super) fn parse_width(
    arguments: impl IntoIterator<Item = String>,
) -> Result<(u32, Vec<String>), String> {
    let mut arguments = arguments.into_iter();
    let mut rest = Vec::new();
    let mut rows = None;
    while let Some(argument) = arguments.next() {
        if argument == "--native-prefill-rows" {
            if rows.is_some() {
                return Err("duplicate native prefill width".into());
            }
            rows = Some(match arguments.next().as_deref() {
                Some("16") => 16,
                Some("32") => 32,
                _ => return Err("native prefill rows requires exactly 16 or 32".into()),
            });
        } else {
            rest.push(argument);
        }
    }
    Ok((
        rows.ok_or("explicit --native-prefill-rows is required")?,
        rest,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_is_explicit_and_preserves_other_arguments() {
        for rows in ["16", "32"] {
            let (actual, rest) = parse_width(
                [
                    "--model",
                    "m",
                    "--native-prefill-rows",
                    rows,
                    "--worker",
                    "w",
                ]
                .map(String::from),
            )
            .unwrap();
            assert_eq!(actual.to_string(), rows);
            assert_eq!(rest, ["--model", "m", "--worker", "w"]);
        }
    }
    #[test]
    fn missing_duplicate_and_noncanonical_widths_refuse() {
        for arguments in [
            vec![],
            vec!["--native-prefill-rows"],
            vec!["--native-prefill-rows", "016"],
            vec!["--native-prefill-rows", "64"],
            vec!["--native-prefill-rows", "16", "--native-prefill-rows", "32"],
        ] {
            assert!(parse_width(arguments.into_iter().map(String::from)).is_err());
        }
    }
}
