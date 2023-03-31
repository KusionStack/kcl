use crate::util::*;
use anyhow::Result;
use clap::ArgMatches;
use kclvm_config::settings::{build_settings_pathbuf, Config, SettingsFile, SettingsPathBuf};
use kclvm_driver::arguments::parse_key_value_pair;
use kclvm_error::Handler;
use kclvm_runtime::PanicInfo;
use std::collections::HashMap;

/// Build settings from arg matches.
pub(crate) fn must_build_settings(matches: &ArgMatches) -> SettingsPathBuf {
    match build_settings(matches) {
        Ok(settings) => settings,
        Err(err) => {
            // New an error handler.
            let mut handler = Handler::default();
            handler
                .add_panic_info(&PanicInfo {
                    message: err.to_string(),
                    ..Default::default()
                })
                .abort_if_any_errors();
            SettingsPathBuf::default()
        }
    }
}

/// Build settings from arg matches.
pub(crate) fn build_settings(matches: &ArgMatches) -> Result<SettingsPathBuf> {
    let files: Vec<&str> = match matches.values_of("input") {
        Some(files) => files.into_iter().collect::<Vec<&str>>(),
        None => vec![],
    };

    let setting_files = matches
        .values_of("setting")
        .map(|files| files.into_iter().collect::<Vec<&str>>());
    let arguments = strings_from_matches(matches, "arguments");

    let package_maps = hashmaps_from_matches(matches, "package_map");

    build_settings_pathbuf(
        files.as_slice(),
        setting_files,
        Some(SettingsFile {
            kcl_cli_configs: Some(Config {
                output: matches.value_of("output").map(|v| v.to_string()),
                overrides: strings_from_matches(matches, "overrides"),
                path_selector: strings_from_matches(matches, "path_selector"),
                strict_range_check: bool_from_matches(matches, "strict_range_check"),
                disable_none: bool_from_matches(matches, "disable_none"),
                verbose: u32_from_matches(matches, "verbose"),
                debug: bool_from_matches(matches, "debug"),
                package_maps,
                ..Default::default()
            }),
            kcl_options: if arguments.is_some() {
                let mut key_value_pairs = vec![];
                if let Some(arguments) = arguments {
                    for arg in arguments {
                        key_value_pairs.push(parse_key_value_pair(&arg)?);
                    }
                }
                Some(key_value_pairs)
            } else {
                None
            },
        }),
    )
}

#[inline]
fn hashmaps_from_matches(matches: &ArgMatches, key: &str) -> Option<HashMap<String, String>> {
    matches.values_of(key).map(|files| {
        files
            .into_iter()
            .map(|s| {
                let parts: Vec<&str> = s.split('=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim();
                    (key.to_string(), value.to_string())
                } else {
                    (String::default(), String::default())
                }
            })
            .collect::<HashMap<String, String>>()
    })
}

#[inline]
fn strings_from_matches(matches: &ArgMatches, key: &str) -> Option<Vec<String>> {
    matches.values_of(key).map(|files| {
        files
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
    })
}

#[inline]
fn bool_from_matches(matches: &ArgMatches, key: &str) -> Option<bool> {
    let occurrences = matches.occurrences_of(key);
    if occurrences > 0 {
        Some(true)
    } else {
        None
    }
}

#[inline]
fn u32_from_matches(matches: &ArgMatches, key: &str) -> Option<u32> {
    let occurrences = matches.occurrences_of(key);
    if occurrences > 0 {
        Some(occurrences as u32)
    } else {
        None
    }
}
