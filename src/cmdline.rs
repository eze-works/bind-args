use std::collections::{HashMap, HashSet};

pub enum CmdLine {
    // This variant is emitted when we parse a command line without any help arguments
    Args(Args),
    // This variant is emitted when we parse a command line with help arguments
    Help(HelpRequest),
}

// The help flag may occur at any subcommand depth, so we need to keep track of the "path" to the command to
// show help for
#[derive(Debug)]
pub struct HelpRequest {
    pub path: Vec<String>,
}

/// A structured view of command line arguments    
#[derive(Debug)]
#[non_exhaustive]
pub struct Args {
    pub name: String,
    pub flags: HashSet<String>,
    pub props: HashMap<String, String>,
    pub subcommand: Option<Box<Args>>,
}

impl CmdLine {
    // Parses `args` as command line arguments. The input is expected to be in the same format
    // that [args()](std::env::args) returns (i.e. the name of the executable is first
    pub(crate) fn parse_from<I, T>(args: I) -> Self
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

        let mut path = vec![result.name.clone()];

        let mut current = &mut result;

        for arg in iter {
            let mut arg = arg.trim().to_string();
            if arg.is_empty() {
                continue;
            }

            let help_flags = ["-h", "+h", "--help", "+help"];

            if help_flags.contains(&arg.as_str()) {
                // If a help flag is detected, return early with the `HelpRequest` variant. This
                // allows us to bypass prop/flag validation when the user wants  help (i.e. we can
                // show the help even if a requireed argument was missing)
                return CmdLine::Help(HelpRequest { path });
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
                    name: arg.clone(),
                    flags: HashSet::new(),
                    props: HashMap::new(),
                    subcommand: None,
                };
                current.subcommand = Some(Box::new(command));
                current = current.subcommand.as_mut().unwrap();
                path.push(arg.clone());
                continue;
            }

            break;
        }

        CmdLine::Args(result)
    }

    #[cfg(test)]
    fn to_args(self) -> Args {
        match self {
            CmdLine::Args(a) => a,
            _ => panic!("enum had the wrong variant"),
        }
    }

    #[cfg(test)]
    fn to_help(self) -> HelpRequest {
        match self {
            CmdLine::Help(r) => r,
            _ => panic!("enum had the wrong variant"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args() {
        let cmdline: [&str; 0] = [];
        let args = CmdLine::parse_from(cmdline).to_args();
        assert_eq!(args.name, "");
        assert!(args.flags.is_empty());
        assert!(args.props.is_empty());
        assert!(args.subcommand.is_none());
    }

    #[test]
    fn args_with_whitespace() {
        let cmdline = ["", " ", "\t", "\n"];
        let args = CmdLine::parse_from(cmdline).to_args();

        assert_eq!(args.name, "");
        assert!(args.flags.is_empty());
        assert!(args.props.is_empty());
        assert!(args.subcommand.is_none());
    }

    #[test]
    fn arguments_starting_with_equals_sign_treated_as_a_command() {
        let cmdline = ["exe", "=value"];
        let cmd = CmdLine::parse_from(cmdline).to_args();
        assert!(cmd.props.is_empty());
        assert_eq!(cmd.subcommand.unwrap().name, "=value");
    }

    #[test]
    fn flags_props_and_subcommands() {
        let cmdline = [
            "exe", "+flag1", "prop=1", "command1", "+flag2", "prop=2", "command2", "+flag3",
        ];
        let cmd = CmdLine::parse_from(cmdline).to_args();
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

    #[test]
    fn help_short_circuits() {
        let cmdline = ["exe", "+flag1", "command1", "+h", "command2"];
        let help = CmdLine::parse_from(cmdline).to_help();
        assert_eq!(help.path, ["exe", "command1"]);
    }
}
