use std::collections::{HashMap, HashSet};

/// A structured view of command line arguments    
#[derive(Debug)]
#[non_exhaustive]
pub struct Args {
    pub name: String,
    pub flags: HashSet<String>,
    pub props: HashMap<String, String>,
    pub subcommand: Option<Box<Args>>,
}

impl Args {
    /// Parses `args` as command line arguments. The input is expected to be in the same format
    /// that [args()](std::env::args) returns (i.e. the name of the executable is first)
    pub fn parse_from<I, T>(args: I) -> Args
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut iter = args.into_iter().map(|s| s.into());

        let mut result = Args {
            name: iter.next().unwrap_or_default(),
            flags: HashSet::new(),
            props: HashMap::new(),
            subcommand: None,
        };

        let mut current = &mut result;

        for arg in iter {
            let mut arg = arg.trim().to_string();
            if arg.is_empty() {
                continue;
            }

            // -h and --help are basically expected to be present on every cli.
            if arg == "-h" || arg == "--help" {
                current.flags.insert("help".to_string());
                continue;
            }

            if arg.starts_with('+') {
                current.flags.insert(arg.split_off(1));
                continue;
            }

            if let Some(idx) = arg.find('=') {
                if idx != 0 {
                    let value = arg.split_off(idx + 1);
                    // pop the trailing `=`
                    arg.pop();
                    current.props.insert(arg, value);
                    continue;
                }
            }

            if current.subcommand.is_none() {
                let command = Args {
                    name: arg,
                    flags: HashSet::new(),
                    props: HashMap::new(),
                    subcommand: None,
                };
                current.subcommand = Some(Box::new(command));
                current = current.subcommand.as_mut().unwrap();
                continue;
            }

            break;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args() {
        let cmdline: [&str; 0] = [];
        let args = Args::parse_from(cmdline);
        assert_eq!(args.name, "");
        assert!(args.flags.is_empty());
        assert!(args.props.is_empty());
        assert!(args.subcommand.is_none());
    }

    #[test]
    fn args_with_whitespace() {
        let cmdline = ["", " ", "\t", "\n"];
        let args = Args::parse_from(cmdline);

        assert_eq!(args.name, "");
        assert!(args.flags.is_empty());
        assert!(args.props.is_empty());
        assert!(args.subcommand.is_none());
    }

    #[test]
    fn arguments_starting_with_equals_sign_treated_as_a_command() {
        let cmdline = ["exe", "=value"];
        let cmd = Args::parse_from(cmdline);
        assert!(cmd.props.is_empty());
        assert_eq!(cmd.subcommand.unwrap().name, "=value");
    }

    #[test]
    fn flags_props_and_subcommands() {
        let cmdline = [
            "exe", "+flag1", "prop=1", "command1", "+flag2", "prop=2", "command2", "+flag3",
        ];
        let cmd = Args::parse_from(cmdline);

        assert_eq!(cmd.name, "exe");
        assert_eq!(cmd.flags, HashSet::from([String::from("flag1")]));
        assert_eq!(
            cmd.props,
            HashMap::from([(String::from("prop"), String::from("1"))])
        );

        assert!(cmd.subcommand.is_some());
        let sub = cmd.subcommand.unwrap();
        assert_eq!(sub.name, "command1");
        assert_eq!(sub.flags, HashSet::from([String::from("flag2")]));
        assert_eq!(
            sub.props,
            HashMap::from([(String::from("prop"), String::from("2"))])
        );

        assert!(sub.subcommand.is_some());
        let subsub = sub.subcommand.unwrap();
        assert_eq!(subsub.name, "command2");
        assert_eq!(subsub.flags, HashSet::from([String::from("flag3")]));
    }
}
