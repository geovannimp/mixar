const INVALID_VERSION: &str =
    "invalid `version`: expected MAJOR.MINOR.PATCH with optional prerelease and build metadata";

// Only the version scalar is needed, not a general YAML parser. Validate the
// complete version before dropping Flutter build metadata for analytics.
pub fn app_version(pubspec: &str) -> Result<&str, &'static str> {
    // Ignore nested mappings while allowing a consistently indented document.
    let root_indent = pubspec
        .lines()
        .filter_map(|line| {
            let content = line.trim_start();
            if content.is_empty()
                || content.starts_with('#')
                || matches!(content.trim_end(), "---" | "...")
            {
                None
            } else {
                Some(line.len() - content.len())
            }
        })
        .min()
        .unwrap_or(0);
    let scalar = pubspec
        .lines()
        .find_map(|line| {
            if line.len() - line.trim_start().len() != root_indent {
                return None;
            }
            let (key, value) = line.trim().split_once(':')?;
            (key.trim() == "version").then_some(value.trim())
        })
        .ok_or("missing `version` scalar")?;
    if scalar.is_empty() || scalar.starts_with('#') {
        return Err("empty `version` scalar");
    }

    let (version, trailing) = match scalar.chars().next() {
        Some(quote @ ('\'' | '"')) => scalar[1..]
            .split_once(quote)
            .ok_or("unterminated quoted `version` scalar")?,
        _ => scalar.split_at(scalar.find(char::is_whitespace).unwrap_or(scalar.len())),
    };
    let trailing = trailing.trim_start();
    if !trailing.is_empty() && !trailing.starts_with('#') {
        return Err("unexpected text after `version` scalar");
    }
    if version.is_empty() {
        return Err("empty `version` scalar");
    }

    let release = match version.split_once('+') {
        Some((release, build)) => {
            if !valid_identifiers(build, false) {
                return Err(INVALID_VERSION);
            }
            release
        }
        None => version,
    };
    let core = match release.split_once('-') {
        Some((core, prerelease)) => {
            if !valid_identifiers(prerelease, true) {
                return Err(INVALID_VERSION);
            }
            core
        }
        None => release,
    };
    let mut components = core.split('.');
    if !(0..3).all(|_| components.next().is_some_and(valid_number)) || components.next().is_some() {
        return Err(INVALID_VERSION);
    }
    Ok(release)
}

fn valid_number(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0'))
}

fn valid_identifiers(value: &str, prerelease: bool) -> bool {
    value.split('.').all(|identifier| {
        !identifier.is_empty()
            && identifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && (!prerelease
                || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                || valid_number(identifier))
    })
}

#[cfg(test)]
mod tests {
    use super::{app_version, INVALID_VERSION};

    #[test]
    fn accepts_whitespace_quotes_and_comments() {
        for pubspec in [
            "version: 0.2.4",
            "version:0.2.4",
            "\tversion \t: \t0.2.4 \t\r\n",
            "name: mixar\nversion: '0.2.4' # release\n",
            "version: \"0.2.4\"\t# release",
            "version: 0.2.4 # release",
            "# version: ignored\nversion_name: ignored\nversion: 0.2.4\n",
        ] {
            assert_eq!(app_version(pubspec), Ok("0.2.4"), "{pubspec:?}");
        }
    }

    #[test]
    fn ignores_nested_versions_before_the_root_version() {
        for pubspec in [
            "dependencies:\n  example:\n    version: 9.8.7\nversion: 0.2.4\n",
            "---\n  dependencies:\n    example:\n      version: 9.8.7\n  version: '0.2.4+42'\n",
        ] {
            assert_eq!(app_version(pubspec), Ok("0.2.4"), "{pubspec:?}");
        }
    }

    #[test]
    fn nested_only_versions_report_missing() {
        for pubspec in [
            "dependencies:\n  example:\n    version: 9.8.7\n",
            "# manifest\n  dependencies:\n    example:\n      version: 9.8.7\n",
        ] {
            assert_eq!(
                app_version(pubspec),
                Err("missing `version` scalar"),
                "{pubspec:?}"
            );
        }
    }

    #[test]
    fn strips_build_metadata_but_preserves_prerelease() {
        for (pubspec, expected) in [
            ("version: 0.2.4+42", "0.2.4"),
            ("version: '0.2.4+0042'", "0.2.4"),
            ("version: \"0.2.4+build.42-sha\" # release", "0.2.4"),
            ("version: 0.2.4-rc.1+build.42", "0.2.4-rc.1"),
            ("version: '0.2.4-rc.1'", "0.2.4-rc.1"),
        ] {
            assert_eq!(app_version(pubspec), Ok(expected), "{pubspec:?}");
        }
    }

    #[test]
    fn rejects_missing_and_empty_versions_descriptively() {
        for pubspec in ["", "name: mixar", "# version: 0.2.4", "version_name: 0.2.4"] {
            assert_eq!(app_version(pubspec), Err("missing `version` scalar"));
        }
        for pubspec in [
            "version:",
            "version: \t",
            "version: # pending",
            "version: ''",
            "version: \"\"",
        ] {
            assert_eq!(app_version(pubspec), Err("empty `version` scalar"));
        }
    }

    #[test]
    fn rejects_malformed_scalars_descriptively() {
        for pubspec in ["version: '0.2.4", "version: \"0.2.4", "version: '0.2.4\""] {
            assert_eq!(
                app_version(pubspec),
                Err("unterminated quoted `version` scalar")
            );
        }
        for pubspec in [
            "version: '0.2.4' extra",
            "version: 0.2.4 extra",
            "version: 0.2.4 +42",
        ] {
            assert_eq!(
                app_version(pubspec),
                Err("unexpected text after `version` scalar")
            );
        }
    }

    #[test]
    fn rejects_invalid_versions_including_invalid_build_metadata() {
        for version in [
            "null",
            "true",
            "[]",
            "{}",
            "latest",
            "v0.2.4",
            "0.2",
            "0.2.4.1",
            "00.2.4",
            "0.02.4",
            "0.2.04",
            "-1.2.3",
            "0.2.4-",
            "0.2.4-01",
            "0.2.4-rc..1",
            "0.2.4-rc_1",
            "+42",
            "0.2.4+",
            "0.2.4+build..42",
            "0.2.4+build_42",
            "0.2.4+42+43",
            "0.2.4#not-a-comment",
            "' 0.2.4 '",
        ] {
            assert_eq!(
                app_version(&format!("version: {version}")),
                Err(INVALID_VERSION),
                "{version:?}"
            );
        }
    }
}
