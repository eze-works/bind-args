#![deny(missing_docs)]
//! Parsing command line arguments
//!
//! The crate revolves around the [`ArgumentBag`] data structure from which options, flags and
//! operands may be extracted.
//!
//! You get an instance of the bag by callind [`parse`] or [`parse_env`].

use std::error::Error;
use std::fmt::Display;

// e.g.: --blah
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct Switch {
    name: String,
}

// e.g.: --blah=hello
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct SwitchWithValue {
    name: String,
    value: String,
}

// e.g.: hello
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct Operand {
    position: usize,
    value: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum Arg {
    Switch(Switch),
    SwitchWithValue(SwitchWithValue),
    Operand(Operand),
    #[default]
    Empty,
}
impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Switch(flag) => {
                if flag.name.len() == 1 {
                    write!(f, "-{}", flag.name)
                } else {
                    write!(f, "--{}", flag.name)
                }
            }
            Self::SwitchWithValue(opt) => {
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
    fn into_operand(self) -> Operand {
        let Self::Operand(o) = self else {
            panic!("expected Arg::Operand variant");
        };
        o
    }

    fn into_switch_with_value(self) -> SwitchWithValue {
        let Self::SwitchWithValue(o) = self else {
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
    args: Vec<Arg>,
    ignored: Vec<String>,
}

impl ArgumentBag {
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
            let Arg::Switch(flag) = &self.args[i] else {
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

    /// Removes the first option with the given `name` and returns its value.
    ///
    /// This works with both space-separated and `=`-separated option forms (i.e. `--option=value`
    /// and `--option value`)
    ///
    /// # Example
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "--opt1=value", "--opt2", "value2"]).unwrap();
    /// assert_eq!(bag.remove_option("opt1").as_deref(), Some("value"));
    /// assert_eq!(bag.remove_option("opt1").as_deref(), None);
    /// assert_eq!(bag.remove_option("opt2").as_deref(), Some("value2"));
    /// assert_eq!(bag.remove_option("opt2").as_deref(), None);
    /// ```
    ///
    /// This will return `None` when a switch argument with the given name exists, but does not have a
    /// value.
    /// This is interpreted as a flag.
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "--opt"]).unwrap();
    /// assert_eq!(bag.remove_option("opt"), None);
    /// assert_eq!(bag.remove_flag("opt"), true);
    /// ```
    ///
    /// When a switch is followed by a value, it is ambiguous whether it should be treated as an option or a flag.
    /// The order in which functions are called resolves this ambiguity.
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// // Treated as a flag followed by an operand
    /// let mut bag = parse(["program", "--option", "value"]).unwrap();
    /// assert_eq!(bag.remove_flag("option"), true);
    /// assert_eq!(bag.remove_operand().as_deref(), Some("value"));
    /// assert!(bag.is_empty());
    ///
    /// // Treated as an option
    /// let mut bag = parse(["program", "--option", "value"]).unwrap();
    /// assert_eq!(bag.remove_option("option").as_deref(), Some("value"));
    /// assert!(bag.is_empty());
    /// ```
    pub fn remove_option(&mut self, name: &str) -> Option<String> {
        for i in 0..self.args.len() {
            match &self.args[i] {
                Arg::SwitchWithValue(s) => {
                    if s.name != name {
                        continue;
                    }

                    let arg = std::mem::take(&mut self.args[i]);
                    return Some(arg.into_switch_with_value().value);
                }
                Arg::Switch(s) => {
                    if s.name != name {
                        continue;
                    }

                    let Some(Arg::Operand(_)) = self.args.get(i + 1) else {
                        return None;
                    };

                    std::mem::take(&mut self.args[i]);
                    let value = std::mem::take(&mut self.args[i + 1]);

                    return Some(value.into_operand().value);
                }
                _ => continue,
            };
        }
        None
    }

    /// Removes the next operand from the argument bag, if any.
    ///
    /// Operands are removed in the order they were supplied.
    ///
    /// # Example
    ///
    /// ```
    /// use bind_args::parse;
    ///
    /// let mut bag = parse(["program", "a", "b", "c"]).unwrap();
    ///
    /// assert_eq!(bag.remove_operand().as_deref(), Some("a"));
    /// assert_eq!(bag.remove_operand().as_deref(), Some("b"));
    /// assert_eq!(bag.remove_operand().as_deref(), Some("c"));
    /// assert_eq!(bag.remove_operand().as_deref(), None);
    /// ```
    pub fn remove_operand(&mut self) -> Option<String> {
        for i in 0..self.args.len() {
            let Arg::Operand(_) = &self.args[i] else {
                continue;
            };

            let arg = std::mem::take(&mut self.args[i]);
            return Some(arg.into_operand().value);
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
    ///
    /// # Example
    ///
    /// ```
    /// let mut bag = bind_args::parse(["program", "--", "does-not-count"]).unwrap();
    /// assert!(bag.is_empty());
    /// assert_eq!(bag.remove_ignored(), vec![String::from("does-not-count")]);
    /// assert!(bag.remove_ignored().is_empty());
    /// ```
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
    /// let bag = bind_args::parse(["program", "--", "does-not-count"]).unwrap();
    /// assert!(bag.is_empty());
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
/// let bag = parse(["git"]).unwrap();
///
/// assert_eq!(bag.program_name, "git");
/// assert_eq!(bag.is_empty(), true);
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

    let mut parsed = Vec::new();
    let mut ignored = Vec::new();

    let mut operand_count = 0;
    let mut saw_end_of_options = false;

    for arg in args {
        if saw_end_of_options {
            ignored.push(arg);
            continue;
        }

        if arg == "--" {
            saw_end_of_options = true;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--") {
            if let Some((name, value)) = value.split_once('=') {
                if name.len() < 2 {
                    return Err(ParseError::MalformedOption(arg));
                }
                parsed.push(Arg::SwitchWithValue(SwitchWithValue {
                    name: name.to_string(),
                    value: value.to_string(),
                }));
            } else {
                if value.len() < 2 {
                    return Err(ParseError::MalformedFlag(arg));
                }

                parsed.push(Arg::Switch(Switch {
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

                parsed.push(Arg::SwitchWithValue(SwitchWithValue {
                    name: name.to_string(),
                    value: value.to_string(),
                }));
            } else {
                if value.len() != 1 {
                    return Err(ParseError::MalformedFlag(arg));
                }

                parsed.push(Arg::Switch(Switch {
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
        args: parsed,
        ignored,
    })
}

/// A command line parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
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
        let mut bag = parse(["program", "--", "a", "--b"]).unwrap();
        let ignored = bag.remove_ignored();

        assert_eq!(ignored, vec![String::from("a"), String::from("--b")]);
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
    fn remove_option() {
        // =-separated
        let mut result = parse(["program", "--opt1=val1", "--opt2=val2"]).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.remove_option("opt1"), Some("val1".to_string()));
        assert_eq!(result.remove_option("opt2"), Some("val2".to_string()));
        assert_eq!(result.remove_option("opt2"), None);
        assert!(result.is_empty());

        // space separated
        let mut bag = parse(["program", "--opt1", "val1", "--opt2", "val2"]).unwrap();
        assert_eq!(bag.remove_option("opt1"), Some("val1".to_string()));
        assert_eq!(bag.remove_option("opt2"), Some("val2".to_string()));
        assert_eq!(bag.remove_option("opt2"), None);
        assert!(bag.is_empty());

        // does not remove flags
        let mut bag = parse(["program", "--opt1"]).unwrap();
        assert_eq!(bag.remove_option("opt1"), None);
        assert!(bag.remove_flag("opt1"));
        assert!(!bag.remove_flag("opt1"));
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
        assert_eq!(result.remove_operand().as_deref(), Some("a"));
        assert_eq!(result.remove_operand().as_deref(), Some("b"));
        assert_eq!(result.remove_operand(), None);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_order_matters() {
        let mut bag = parse(["program", "--option", "value"]).unwrap();
        assert_eq!(bag.remove_option("option").as_deref(), Some("value"));
        assert!(!bag.remove_flag("option"));
        assert!(bag.is_empty());

        let mut bag = parse(["program", "--option", "value"]).unwrap();
        assert!(bag.remove_flag("option"));
        assert_eq!(bag.remove_option("option").as_deref(), None);
        assert_eq!(bag.remove_operand().as_deref(), Some("value"));
        assert!(bag.is_empty());
    }
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDocTest;
