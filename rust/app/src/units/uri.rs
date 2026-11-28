//! Front-door URI validation, backed by the ported `uri-parse` unit.
//!
//! aria2@1.37.0 `src/ProtocolDetector.cc:56` calls `uri_split()` to decide
//! whether an argument is a stream URI; `src/download_helper.cc:451` throws
//! `MSG_UNRECOGNIZED_URI` (`src/message.h:108`) when it is not a stream URI,
//! magnet link or local torrent/metalink file. This module reproduces only
//! that rejection path; every argument that looks like a URI is still handed
//! to the original.

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use clap::ArgMatches;

/// Returns an exit code when an argument is definitely an unsupported URI,
/// otherwise `None` so the caller falls back to the original.
pub fn dispatch(matches: &ArgMatches) -> Option<ExitCode> {
    // `-i` / `--load-cookies` take file names; `-Z` expands parameterized URIs
    // (including the empty expansion, which prints "No files to download.").
    if matches.get_count("short-i") > 0
        || matches.get_count("load-cookies") > 0
        || matches.get_count("short-Z") > 0
    {
        return None;
    }
    let args = matches.get_many::<OsString>("args")?;
    for arg in args {
        let Some(uri) = arg.to_str() else {
            return None;
        };
        // Option-looking tokens (including options this front door does not
        // know) must be diagnosed by the original option parser.
        if uri.starts_with('-') {
            return None;
        }
        // Locally detectable non-URI inputs are handled by the original.
        // (magnet links, torrent and metalink files).
        if uri.to_ascii_lowercase().starts_with("magnet:") || Path::new(uri).is_file() {
            return None;
        }
        if uri_parse_rs::summary(uri.as_bytes()) != -1 {
            continue;
        }
        // aria2@1.37.0 src/message.h:108 MSG_UNRECOGNIZED_URI; error code 1
        // (src/error_code.h: UNKNOWN_ERROR) with the top-level "Exception caught"
        // banner from src/main.cc.
        eprintln!("Exception caught");
        eprintln!(
            "Exception: [download_helper.cc:451] errorCode=1 Unrecognized URI or unsupported protocol: {uri}\n"
        );
        return Some(ExitCode::from(1));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli;

    fn run(args: &[&str]) -> Option<ExitCode> {
        let argv: Vec<OsString> = std::iter::once(OsString::from("aria2c"))
            .chain(args.iter().map(OsString::from))
            .collect();
        let matches = cli::command().try_get_matches_from(&argv).ok()?;
        let (command, sub) = matches.subcommand().unwrap_or((cli::ROOT, &matches));
        assert_eq!(command, cli::ROOT);
        dispatch(sub)
    }

    #[test]
    fn rejects_unsupported_uri() {
        assert_eq!(run(&[":not-a-uri"]), Some(ExitCode::from(1)));
        assert_eq!(run(&["héllo wörld"]), Some(ExitCode::from(1)));
    }

    #[test]
    fn leaves_valid_uris_and_non_uri_inputs_to_the_original() {
        assert_eq!(run(&["http://127.0.0.1:18080/1mib.bin"]), None);
        assert_eq!(run(&["magnet:?xt=urn:btih:abc"]), None);
        assert_eq!(run(&["-i", "files.txt"]), None);
    }
}
