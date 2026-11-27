use std::ffi::OsString;

use clap::{Arg, ArgAction, Command, value_parser};

/// The command `units::dispatch` gets when argv names no subcommand.
pub const ROOT: &str = "aria2c";

fn rest() -> Arg {
    Arg::new("args")
        .num_args(0..)
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .value_parser(value_parser!(OsString))
}

/// Routes argv to a command. Anything this parser rejects runs the original, so it never
/// has to match every way the original accepts its arguments.
pub fn command() -> Command {
    Command::new(ROOT)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("checksum")
                .long("checksum")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(value_parser!(OsString)),
        )
        .arg(Arg::new("dry-run").long("dry-run").action(ArgAction::Count))
        .arg(
            Arg::new("ftp-passwd")
                .long("ftp-passwd")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(value_parser!(OsString)),
        )
        .arg(
            Arg::new("ftp-user")
                .long("ftp-user")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(value_parser!(OsString)),
        )
        .arg(
            Arg::new("load-cookies")
                .long("load-cookies")
                .action(ArgAction::Count),
        )
        .arg(Arg::new("short-P").short('P').action(ArgAction::Count))
        .arg(Arg::new("short-Z").short('Z').action(ArgAction::Count))
        .arg(Arg::new("short-c").short('c').action(ArgAction::Count))
        .arg(Arg::new("short-i").short('i').action(ArgAction::Count))
        .arg(rest())
}
