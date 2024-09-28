use crate::argument::Argument;
use itertools::Itertools;

pub struct Command {
    long: &'static str,
    short: char,
}

impl Command {
    fn parse<'i>(&self, args: &'i [&'i str]) -> Option<(Argument, &'i [&'i str])> {
        let (arg, rest) = args.split_first()?;

        let result = Some((Argument::Command(self.long), rest));
        if *arg == self.long {
            return result;
        }

        let single_char = arg.chars().exactly_one().ok()?;

        if single_char == self.short {
            return result;
        }

        None
    }
}

pub struct Switch {
    long: &'static str,
    short: char,
}

impl Switch {
    fn parse<'i>(&self, args: &'i [&'i str]) -> Option<(Argument, &'i [&'i str])> {
        let (arg, rest) = args.split_first()?;

        if let Some(long) = arg.strip_prefix("--") {
            // The value of the switch was given as a separate argument.
            // e.g. --message hello
            //        ^^^^^^^
            //         long
            if long == self.long {
                let (value, rest) = rest.split_first()?;
                return Some((Argument::Switch(self.long, value.to_string()), rest));
            }

            // The value of the switch is in the same argument.
            // e.g. --message=hello
            //        ^^^^^^^^^^^^^
            //          long
            let (key, value) = long.split_once("=")?;

            if key != self.long {
                return None;
            }

            return Some((Argument::Switch(self.long, value.to_string()), rest));
        }

        if let Some(short) = arg.strip_prefix("-") {
            if let Some(c) = short.chars().exactly_one().ok() {
                if c != self.short {
                    return None;
                }

                let (value, rest) = rest.split_first()?;
                return Some((Argument::Switch(self.long, value.to_string()), rest));
            }

            let (key, value) = short.split_once("=")?;

            let Some(c) = key.chars().exactly_one().ok() else {
                return None;
            };

            if c != self.short {
                return None;
            }

            return Some((Argument::Switch(self.long, value.to_string()), rest));
        }

        None
    }
}

pub struct Flag {
    long: &'static str,
    short: char,
}

impl Flag {
    fn parse<'i>(&self, args: &'i [&'i str]) -> Option<(Argument, &'i [&'i str])> {
        let (arg, rest) = args.split_first()?;

        if let Some(long) = arg.strip_prefix("--") {
            if long != self.long {
                return None;
            }
            return Some((Argument::Flag(self.long), rest));
        }

        if let Some(short) = arg.strip_prefix("-") {
            let single_char = short.chars().exactly_one().ok()?;

            if single_char != self.short {
                return None;
            }
            return Some((Argument::Flag(self.long), rest));
        }

        None
    }
}

pub struct Positional {
    name: &'static str,
}

impl Positional {
    fn parse<'i>(&self, args: &'i [&'i str]) -> Option<(Argument, &'i [&'i str])> {
        let (arg, rest) = args.split_first()?;

        if arg.starts_with('-') {
            return None;
        }

        Some((Argument::Positional(self.name, arg.to_string()), rest))
    }
}

pub enum Parser {
    Flag(Flag),
    Switch(Switch),
    Positional(Positional),
    Command(Command),
}

impl Parser {
    pub fn parse<'i>(&self, args: &'i [&'i str]) -> Option<(Argument, &'i [&'i str])> {
        match self {
            Parser::Command(c) => c.parse(args),
            Parser::Flag(f) => f.parse(args),
            Parser::Switch(s) => s.parse(args),
            Parser::Positional(p) => p.parse(args),
        }
    }
}
