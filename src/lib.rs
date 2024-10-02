#![deny(missing_docs)]
//! Parsing self-describing command line arguments
//!
//! The crate revolves around the [`ArgumentBag`] data structure from which options, flags and
//! operands may be extracted by name.
//!
//! You get an instance of the bag by callind [`parse`] or [`parse_env`].

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

/// A bag of parsed command line arguments
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ArgumentBag {
    /// The name of the program being run
    pub program_name: String,
    _command: Option<String>,
    args: Vec<Arg>,
    ignored: Vec<String>,
}

impl ArgumentBag {
    /// Returns the name of the parsed sub-command, if any
    pub fn command(&self) -> Option<&str> {
        self._command.as_deref()
    }

    /// Removes the first flag with the given name from the bag if it exists.
    ///
    /// # Example
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "--flag1", "--flag2"]).unwrap();
    /// assert_eq!(bag.remove_flag("flag2"), true);
    /// assert_eq!(bag.remove_flag("flag2"), false);
    /// ```
    pub fn remove_flag(&mut self, name: &str) -> bool {
        for i in 0..self.args.len() {
            let Arg::Flag(flag) = &self.args[i] else {
                continue;
            };

            if flag.name != name {
                continue;
            }

            std::mem::take(&mut self.args[i]);
            return true;
        }
        false
    }

    /// Removes the first option with the given `name` if it exists.
    ///
    /// # Example
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "--opt1=value", "--opt2=value2"]).unwrap();
    /// assert_eq!(bag.remove_option("opt2").as_deref(), Some("value2"));
    /// assert_eq!(bag.remove_option("opt2").as_deref(), None);
    /// ```
    pub fn remove_option(&mut self, name: &str) -> Option<String> {
        for i in 0..self.args.len() {
            let Arg::Option(option) = &self.args[i] else {
                continue;
            };

            if option.name != name {
                continue;
            }

            let arg = std::mem::take(&mut self.args[i]);
            return Some(arg.option().value);
        }
        None
    }

    /// Removes the operand at the given position if it exists.
    ///
    /// This does not alter the position of other operands.
    ///
    /// # Example
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "a", "b", "c"]).unwrap();
    ///
    /// assert_eq!(bag.remove_operand(0).as_deref(), Some("a"));
    /// assert_eq!(bag.remove_operand(0).as_deref(), None);
    /// assert_eq!(bag.remove_operand(2).as_deref(), Some("c"));
    /// ```
    pub fn remove_operand(&mut self, position: usize) -> Option<String> {
        for i in 0..self.args.len() {
            let Arg::Operand(op) = &self.args[i] else {
                continue;
            };

            if op.position != position {
                continue;
            }

            let arg = std::mem::take(&mut self.args[i]);
            return Some(arg.operand().value);
        }
        None
    }

    /// Removes any leftover flags, options and operands that have not been `remove_*`d.
    ///
    /// Subsequent calls will return an empty `Vec`
    ///
    /// The returned `Vec` will not include any arguments that appeared after the end-of-optiosn
    /// marker (i.e. `--`).
    /// Use [`remove_ignored`](crate::ArgumentBag::remove_ignored) for those.
    ///
    /// # Example
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "arg", "--", "stuff"]).unwrap();
    /// assert_eq!(bag.remove_remaining(), vec![String::from("arg")]);
    /// ```
    pub fn remove_remaining(&mut self) -> Vec<String> {
        let mut leftover = vec![];

        for i in 0..self.args.len() {
            let current = &self.args[i];
            if !current.is_empty() {
                leftover.push(std::mem::take(&mut self.args[i]).to_string());
            }
        }
        leftover
    }

    /// Returns an owned `Vec` with all the arguments after the "end of options" marker (i.e. `--`)
    ///
    /// Subsequent calls to this function will return an empty `Vec`.
    pub fn remove_ignored(&mut self) -> Vec<String> {
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
pub fn parse_env() -> Result<ArgumentBag, ParseError> {
    parse(std::env::args())
}

/// Parses the given command line arguments into a [bag](crate::ArgumentBag)
///
/// The input is expected to have at least one element corresponding to the name of the executing
/// program.
///
/// # End of options marker
///
/// The end-of-options marker (i.e. `--`) is respected.
/// Arguments occuring after it are not parsed and are stored in the bag as-is.
///
/// # Example
///
/// ```
/// use bind_args::parse;
/// let parsed = parse(["git", "@remote-add"]).unwrap();
///
/// assert_eq!(parsed.program_name, "git");
/// assert_eq!(parsed.command(), Some("remote-add"));
/// ```
///
/// # Panics
///
/// Panics if any argument to the process is not valid Unicode.
pub fn parse<I, T>(arguments: I) -> Result<ArgumentBag, ParseError>
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

    Ok(ArgumentBag {
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
        let mut bag = parse(["program", "--", "@a", "@b"]).unwrap();
        let  ignored = bag.remove_ignored();

        assert_eq!(ignored, vec![String::from("@a"), String::from("@b")]);
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
    fn parse_command() {
        let result = parse(["program", "@cmd"]).unwrap();
        assert_eq!(result.command(), Some("cmd"));
    }

    #[test]
    fn remove_option() {
        let mut result = parse(["program", "--opt1=val1", "--opt2=val2"]).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.remove_option("opt1"), Some("val1".to_string()));
        assert_eq!(result.remove_option("opt2"), Some("val2".to_string()));
        assert_eq!(result.remove_option("opt3"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_flag() {
        let mut result = parse(["prgoram", "--flag1", "--flag2"]).unwrap();
        assert!(!result.is_empty());
        assert!(result.remove_flag("flag1"));
        assert!(result.remove_flag("flag2"));
        assert!(!result.remove_flag("flag2"));
        assert!(result.is_empty());
    }

    #[test]
    fn remove_operand() {
        let mut result = parse(["program", "a", "b"]).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.remove_operand(0), Some("a".to_string()));
        assert_eq!(result.remove_operand(0), None);
        assert_eq!(result.remove_operand(1), Some("b".to_string()));
        assert_eq!(result.remove_operand(1), None);
        assert!(result.is_empty());
    }
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDocTest;
