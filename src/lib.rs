#![deny(missing_docs)]
//! Parsing self-describing command line arguments
//!
//! # Introduction
//!
//! The self-describing command line syntax is similar to the prevalent GNU and POSIX syntaxes, but
//! deviates in a few ways for clarity and ease of implementation:
//!
//! - Options and their values are always written as one shell "word" separated by `=`.
//!   (e.g. `--level=info` or `-f=archilve.tar`)
//! - Fused-style arguments are not allowed
//! - Sub-commands are prefixed with the at-sign (i.e. `@`)
//! - There may only be one sub-command.
//!
//! It looks like this:
//!
//! ```text
//! ./program @command --option=value --flag operand
//! ```
//!
//! # Example
//!
//! ```
//! use bind_args::parse;
//!
//! struct AppArgs {
//!     verbose: bool,
//!     log_level: Option<String>,
//!     path: String,
//! }
//!
//! let mut cmdline = parse(["program", "--log-level=INFO", "--verbose", "/etc/config"]).unwrap();
//!
//! let args = AppArgs {
//!     verbose: cmdline.take_flag("verbose"),
//!     log_level: cmdline.take_option("log-level"),
//!     path: cmdline.take_operand(0).unwrap_or(String::from("/"))
//! };
//!
//! assert_eq!(args.verbose, true);
//! assert_eq!(args.log_level.as_deref(), Some("INFO"));
//! assert_eq!(args.path, "/etc/config");
//!
//! // It is important to make sure there are not unexpected arguments.
//! if !cmdline.is_empty() {
//!     let unexpected = cmdline.take_remaining().join(", ");
//!     eprintln!("Unexpected argument(s): {}", unexpected);
//!     std::process::exit(1);
//! }
//! ```

use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct Flag {
    name: String,
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct OptionArg {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct Operand {
    position: usize,
    value: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum Arg {
    Flag(Flag),
    Option(OptionArg),
    Operand(Operand),
    #[default]
    Empty,
}
impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Flag(flag) => {
                if flag.name.len() == 1 {
                    write!(f, "-{}", flag.name)
                } else {
                    write!(f, "--{}", flag.name)
                }
            }
            Self::Option(opt) => {
                if opt.name.len() == 1 {
                    write!(f, "-{}={}", opt.name, opt.value)
                } else {
                    write!(f, "--{}={}", opt.name, opt.value)
                }
            }
            Self::Operand(op) => write!(f, "{}", op.value),
        }
    }
}

impl Arg {
    fn as_flag(&self) -> Option<&Flag> {
        let Self::Flag(f) = self else {
            return None;
        };
        Some(f)
    }

    fn as_option(&self) -> Option<&OptionArg> {
        let Self::Option(o) = self else {
            return None;
        };
        Some(o)
    }

    fn as_operand(&self) -> Option<&Operand> {
        let Self::Operand(o) = self else {
            return None;
        };
        Some(o)
    }

    fn operand(self) -> Operand {
        let Self::Operand(o) = self else {
            panic!("expected Arg::Operand variant");
        };
        o
    }

    fn option(self) -> OptionArg {
        let Self::Option(o) = self else {
            panic!("expected Arg::Option variant");
        };
        o
    }

    fn is_empty(&self) -> bool {
        let Self::Empty = self else { return false };
        true
    }
}

/// Parsed command line arguments
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cmdline {
    /// The name of the program being run
    pub program_name: String,
    _command: Option<String>,
    args: Vec<Arg>,
    ignored: Vec<String>,
}

impl Cmdline {
    fn flag_iter(&self) -> impl Iterator<Item = (usize, &Flag)> {
        self.args
            .iter()
            .enumerate()
            .filter_map(|(idx, arg)| Some((idx, arg.as_flag()?)))
    }

    fn option_iter(&self) -> impl Iterator<Item = (usize, &OptionArg)> {
        self.args
            .iter()
            .enumerate()
            .filter_map(|(idx, arg)| Some((idx, arg.as_option()?)))
    }

    fn operand_iter(&self) -> impl Iterator<Item = (usize, &Operand)> {
        self.args
            .iter()
            .enumerate()
            .filter_map(|(idx, arg)| Some((idx, arg.as_operand()?)))
    }

    /// Returns the name of the parsed sub-command, if any
    pub fn command(&self) -> Option<&str> {
        self._command.as_deref()
    }

    /// Returns `true` if the command line invocation contained a flag `name`.
    pub fn flag(&self, name: &str) -> bool {
        self.flag_iter().any(|(_, f)| name == f.name)
    }

    /// Returns the value of option `name` if it exists
    pub fn option(&self, name: &str) -> Option<&str> {
        let (_, res) = self.option_iter().find(|(_, o)| name == o.name)?;
        Some(res.value.as_str())
    }

    /// Returns the value of the operand (i.e. positional argument) at the given position, if any.
    pub fn operand(&self, position: usize) -> Option<&str> {
        let (_, res) = self.operand_iter().find(|(_, o)| position == o.position)?;
        Some(res.value.as_str())
    }

    /// Returns `true` if the command line invocation contained a flag `name`.
    /// The first flag found is removed, so unless the flag was provided multiple times, subsequent
    /// calls to this function will return `false`.
    pub fn take_flag(&mut self, name: &str) -> bool {
        let Some((idx, _)) = self.flag_iter().find(|(_, f)| name == f.name) else {
            return false;
        };
        std::mem::take(&mut self.args[idx]);
        true
    }

    /// Returns the value of the option `name` if it exists.
    /// The first option found is removed, so unless the option was provided multiple times,
    /// subsequent calls to this function will return `None`.
    pub fn take_option(&mut self, name: &str) -> Option<String> {
        let (idx, _) = self.option_iter().find(|(_, o)| name == o.name)?;
        let arg = std::mem::take(&mut self.args[idx]);
        Some(arg.option().value)
    }

    /// Returns the value of the operand (i.e. positional argument) at the given position, if any.
    /// The operand is removed.
    pub fn take_operand(&mut self, position: usize) -> Option<String> {
        let (idx, _) = self.operand_iter().find(|(_, o)| position == o.position)?;
        let arg = std::mem::take(&mut self.args[idx]);
        Some(arg.operand().value)
    }

    /// Returns any leftover arguments that have not been `take_`en.
    ///
    /// Subsequent calls will return an empty `Vec`
    ///
    /// The returned `Vec` will not include [`ignored`](crate::Cmdline::ignored) arguments.
    pub fn take_remaining(&mut self) -> Vec<String> {
        let mut leftover = vec![];

        for i in 0..self.args.len() {
            let current = &self.args[i];
            if !current.is_empty() {
                leftover.push(std::mem::take(&mut self.args[i]).to_string());
            }
        }
        leftover
    }

    /// Returns an iterator over arguments occuring after the "end of options" marker (i.e. `--`)
    pub fn ignored(&self) -> impl Iterator<Item = &str> {
        self.ignored.iter().map(String::as_str)
    }

    /// Returns an owned `Vec` with all the arguments after the "end of options" marker (i.e. `--`)
    ///
    /// Subsequent calls to this function will return an empty `Vec`.
    pub fn take_ignored(&mut self) -> Vec<String> {
        self.ignored.split_off(0)
    }

    /// Returns `true` when there are no more flags, options or operands left.
    ///
    /// "Ignored" arguments are not counted.
    ///
    /// # Example
    ///
    /// ```
    /// let cmdline = bind_args::parse(["program", "--", "does-not-count"]).unwrap();
    /// assert!(cmdline.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.args.iter().all(Arg::is_empty)
    }
}

/// Parses command line arguments from `std::env::args()`
///
/// See [`parse`]
pub fn parse_from_env() -> Result<Cmdline, ParseError> {
    parse(std::env::args())
}

/// Parses the given command line arguments
///
/// The input is expected to have at least one element corresponding to the name of the executing
/// program.
///
/// # Example
///
/// ```
/// let args = ["git", "@remote-add", "--ref=origin", "--verbose"];
/// let parsed = bind_args::parse(args).unwrap();
///
/// assert_eq!(parsed.program_name, "git");
/// assert_eq!(parsed.command(), Some("remote-add"));
/// assert_eq!(parsed.option("ref"), Some("origin"));
/// assert_eq!(parsed.flag("verbose"), true);
///
/// ```
///
/// # Panics
///
/// Panics if any argument to the process is not valid Unicode.
pub fn parse<I, T>(arguments: I) -> Result<Cmdline, ParseError>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let mut args = arguments
        .into_iter()
        .map(|i| i.into())
        .filter(|s| !s.is_empty())
        .peekable();

    let program_name = args.next().expect("missing program name");

    let mut command = None;

    let mut parsed = Vec::new();
    let mut ignored = Vec::new();

    let mut operand_count = 0;
    let mut saw_end_of_options = false;

    // A command may only appear as the first token
    if let Some(maybe_command) = args.peek() {
        if maybe_command.starts_with('@') {
            let name = args.next().unwrap().split_off(1);
            command = Some(name);
        }
    }

    for arg in args {
        if saw_end_of_options {
            ignored.push(arg);
            continue;
        }

        if arg == "--" {
            saw_end_of_options = true;
            continue;
        }

        if arg.starts_with('@') {
            return Err(ParseError::TooManyCommands(arg));
        }

        if let Some(value) = arg.strip_prefix("--") {
            if let Some((name, value)) = value.split_once('=') {
                if name.len() < 2 {
                    return Err(ParseError::MalformedOption(arg));
                }
                parsed.push(Arg::Option(OptionArg {
                    name: name.to_string(),
                    value: value.to_string(),
                }));
            } else {
                if value.len() < 2 {
                    return Err(ParseError::MalformedFlag(arg));
                }

                parsed.push(Arg::Flag(Flag {
                    name: value.to_string(),
                }));
            }
            continue;
        }

        if let Some(value) = arg.strip_prefix("-") {
            if let Some((name, value)) = value.split_once('=') {
                if name.len() != 1 {
                    return Err(ParseError::MalformedOption(arg));
                }

                parsed.push(Arg::Option(OptionArg {
                    name: name.to_string(),
                    value: value.to_string(),
                }));
            } else {
                if value.len() != 1 {
                    return Err(ParseError::MalformedFlag(arg));
                }

                parsed.push(Arg::Flag(Flag {
                    name: value.to_string(),
                }));
            }

            continue;
        }

        parsed.push(Arg::Operand(Operand {
            position: operand_count,
            value: arg,
        }));
        operand_count += 1;
    }

    Ok(Cmdline {
        program_name,
        _command: command,
        args: parsed,
        ignored,
    })
}

/// A command line parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Encountered two or more sub-commands (e.g. `program @one @two`)
    TooManyCommands(String),
    /// Encountered an option without a value  (e.g. `--invalid`)
    OptionMissingValue(String),
    /// Encountered an option withuot a name (e.g. `--=value`)
    MalformedOption(String),
    /// Encountered a flag without a name (e.g. `-`)
    MalformedFlag(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyCommands(s) => {
                write!(
                    f,
                    "Only one command may be given. Saw a second command '{s}'",
                )
            }
            Self::OptionMissingValue(s) => {
                write!(f, "Option '{s}' is missing a value")
            }
            Self::MalformedOption(s) => {
                write!(f, "'{s}' is not a valid option")
            }
            Self::MalformedFlag(s) => {
                write!(f, "'{s}' is not a valid flag")
            }
        }
    }
}

impl Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_of_options() {
        let result = parse(["program", "--", "@a", "@b"]).unwrap();
        let mut ignored = result.ignored();

        assert_eq!(ignored.next(), Some("@a"));
        assert_eq!(ignored.next(), Some("@b"));
        assert_eq!(ignored.next(), None);
    }

    #[test]
    fn at_most_one_command() {
        let result = parse(["program", "@cmd", "@another"]);

        assert_eq!(
            result,
            Err(ParseError::TooManyCommands("@another".to_string()))
        );
    }

    #[test]
    fn malformed_option() {
        let result = parse(["program", "--=value"]);
        assert_eq!(
            result,
            Err(ParseError::MalformedOption("--=value".to_string()))
        );

        let result = parse(["program", "--s=value"]);
        assert_eq!(
            result,
            Err(ParseError::MalformedOption("--s=value".to_string()))
        );

        let result = parse(["program", "-long=value"]);
        assert_eq!(
            result,
            Err(ParseError::MalformedOption("-long=value".to_string()))
        );
    }

    #[test]
    fn malformed_flag() {
        let result = parse(["program", "--s"]);
        assert_eq!(result, Err(ParseError::MalformedFlag("--s".to_string())));

        let result = parse(["program", "-long"]);
        assert_eq!(result, Err(ParseError::MalformedFlag("-long".to_string())));
    }

    #[test]
    fn get_option() {
        let result = parse(["program", "--name==value"]).unwrap();
        assert_eq!(result.option("name"), Some("=value"));
    }

    #[test]
    fn take_option() {
        let mut result = parse(["program", "--opt1=val1", "--opt2=val2"]).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.take_option("opt1"), Some("val1".to_string()));
        assert_eq!(result.take_option("opt2"), Some("val2".to_string()));
        assert_eq!(result.take_option("opt3"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn get_flag() {
        let result = parse(["program", "--flag"]).unwrap();
        assert_eq!(result.flag("flag"), true);
    }

    #[test]
    fn take_flag() {
        let mut result = parse(["prgoram", "--flag1", "--flag2"]).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.take_flag("flag1"), true);
        assert_eq!(result.take_flag("flag2"), true);
        assert_eq!(result.take_flag("flag2"), false);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_command() {
        let result = parse(["program", "@cmd"]).unwrap();
        assert_eq!(result.command(), Some("cmd"));
    }

    #[test]
    fn get_operand() {
        let result = parse(["program", "=a", "b"]).unwrap();
        assert_eq!(result.operand(0), Some("=a"));
        assert_eq!(result.operand(1), Some("b"));
    }

    #[test]
    fn take_operand() {
        let mut result = parse(["program", "a", "b"]).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.take_operand(0), Some("a".to_string()));
        assert_eq!(result.take_operand(0), None);
        assert_eq!(result.take_operand(1), Some("b".to_string()));
        assert_eq!(result.take_operand(1), None);
        assert!(result.is_empty());
    }
}
