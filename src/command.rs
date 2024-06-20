use crate::args::Args;
use std::collections::{HashMap, HashSet};
use std::io::stdout;

pub mod help;

#[derive(Debug)]
pub enum ArgumentKind {
    Command,
    Flag,
    Prop,
}

impl std::fmt::Display for ArgumentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgumentKind::Command => write!(f, "command"),
            ArgumentKind::Prop => write!(f, "option"),
            ArgumentKind::Flag => write!(f, "flag"),
        }
    }
}

/// A variant of this enum is returned when the command line arguments don't match the command
/// definition
#[derive(Debug, thiserror::Error)]
pub enum InvalidArguments {
    #[error("{name} is not a valid {kind}")]
    UnrecognizedArgument { name: String, kind: ArgumentKind },
    #[error("missing required option '{0}'")]
    MissingRequiredOptions(String),
}

/// A blueprint for command line props (e.g. `prop=value`)
#[derive(Debug, Clone)]
pub struct Prop {
    help: &'static str,
    required: bool,
    names: Vec<&'static str>,
}

impl Prop {
    /// Defines a new property
    pub fn new(name: &'static str, help: &'static str) -> Self {
        Prop {
            help,
            names: vec![name],
            required: false,
        }
    }

    /// Returns the primary name for this prop
    pub fn name(&self) -> &'static str {
        self.names
            .first()
            .expect("props must have at least one name")
    }

    /// Makes this prop required
    pub fn make_required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Attaches an alias to this prop
    pub fn add_alias(mut self, alias: &'static str) -> Self {
        self.names.push(alias);
        self
    }
}

/// A blueprint for command line flags (e.g. `+flag`)
#[derive(Debug, Clone)]
pub struct Flag {
    help: &'static str,
    names: Vec<&'static str>,
}

impl Flag {
    /// Defines a new flag
    pub fn new(name: &'static str, help: &'static str) -> Self {
        Flag {
            help,
            names: vec![name],
        }
    }

    /// Returns the primary name for this flag
    pub fn name(&self) -> &'static str {
        self.names
            .first()
            .expect("flags must have at least one name")
    }

    /// Attaches an alias to this flag
    pub fn add_alias(mut self, alias: &'static str) -> Self {
        self.names.push(alias);
        self
    }
}

/// A blueprint for what valid command line should look like
#[derive(Debug, Clone)]
pub struct Command {
    names: Vec<&'static str>,
    help: &'static str,

    props: Vec<Prop>,
    flags: Vec<Flag>,
    commands: Vec<Command>,
}

impl Command {
    /// Creates a new instance.
    ///
    /// This structure can be used to represent both root and sub-commands.
    pub fn new(name: &'static str, help: &'static str) -> Self {
        Self {
            names: vec![name],
            help,
            props: vec![],
            flags: vec![],
            commands: vec![],
        }
    }

    /// Returns the primary name for this command
    pub fn name(&self) -> &'static str {
        self.names
            .first()
            .expect("commands should have at least one name")
    }

    /// Attaches an alias to this command.
    ///
    /// Attaching an alias to the root command has no visible effect
    pub fn add_alias(mut self, alias: &'static str) -> Self {
        self.names.push(alias);
        self
    }

    /// Defines a flag parameter for this command
    pub fn add_flag(mut self, flag: Flag) -> Self {
        self.flags.push(flag);
        self
    }

    /// Defines a prop parameter for this command
    pub fn add_prop(mut self, prop: Prop) -> Self {
        self.props.push(prop);
        self
    }

    /// Defines a subcommand
    pub fn add_command(mut self, subcommand: Command) -> Self {
        self.commands.push(subcommand);
        self
    }

    /// Returns a reference to a subcommand with the given name or alias
    pub fn get_subcommand(&self, name: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.names.contains(&name))
    }

    /// Returns a reference to a flag with the given name or alias
    pub fn get_flag(&self, name: &str) -> Option<&Flag> {
        self.flags.iter().find(|f| f.names.contains(&name))
    }

    /// Returns a reference to a prop with the given name or alias
    pub fn get_prop(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.names.contains(&name))
    }

    /// Prints the help and exits if the user requested it
    pub fn intercept_help(&self, args: &Args) {
        if let Some(cmd_arg) = help::requested_help(args).and_then(|s| self.get_subcommand(s)) {
            let mut stdout = stdout();
            let mut code = 0;
            match help::write_help(&mut stdout, cmd_arg) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("could not write to stdout\n{e}");
                    code = 1;
                }
            };
            std::process::exit(code);
        }
    }

    /// Parses the command line arguments `items`, and validates that the parsed structure adheres
    /// to this command definition
    pub fn parse_from<I, T>(&self, items: I) -> Result<Args, InvalidArguments>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut args = Args::parse_from(items);
        Self::_validate(self, &args)?;
        Self::_resolve_aliases(self, &mut args);
        Ok(args)
    }

    fn _resolve_aliases(cmd_def: &Command, cmd_args: &mut Args) {
        // Assumes that _validate has been called, so we unwrap() without fear

        let mut canonical_flags = HashSet::new();
        for flag in &cmd_args.flags {
            canonical_flags.insert(cmd_def.get_flag(flag).unwrap().name().to_string());
        }

        let mut canonical_props = HashMap::new();
        for (prop, value) in &cmd_args.props {
            canonical_props.insert(
                cmd_def.get_prop(&prop).unwrap().name().to_string(),
                value.to_string(),
            );
        }

        cmd_args.flags = canonical_flags;
        cmd_args.props = canonical_props;

        if let Some(ref mut subcmd_arg) = cmd_args.subcommand {
            let subcmd_def = cmd_def.get_subcommand(&subcmd_arg.name).unwrap();
            subcmd_arg.name = subcmd_def.name().to_string();
            Self::_resolve_aliases(subcmd_def, subcmd_arg.as_mut());
        }
    }

    fn _validate(cmd_def: &Command, cmd_args: &Args) -> Result<(), InvalidArguments> {
        // Every flag argument must have a corresponding flag definition
        for flag in &cmd_args.flags {
            if cmd_def.get_flag(flag).is_none() {
                return Err(InvalidArguments::UnrecognizedArgument {
                    name: flag.clone(),
                    kind: ArgumentKind::Flag,
                });
            }
        }

        let required_props = cmd_def
            .props
            .iter()
            .filter_map(|p| if p.required { Some(p.name()) } else { None })
            .collect::<HashSet<_>>();
        let mut seen: HashSet<&str> = HashSet::new();

        for (prop, _) in &cmd_args.props {
            // Every prop argument must have a corresponding prop definition
            let Some(prop_def) = cmd_def.get_prop(prop) else {
                return Err(InvalidArguments::UnrecognizedArgument {
                    name: prop.clone(),
                    kind: ArgumentKind::Prop,
                });
            };

            // All required options must be observed
            if prop_def.required {
                seen.insert(prop_def.name());
            }
        }

        let missing = required_props
            .difference(&seen)
            .copied()
            .collect::<Vec<&str>>();
        if let Some(first) = missing.first() {
            return Err(InvalidArguments::MissingRequiredOptions(first.to_string()));
        }

        let Some(ref subcommand_arg) = cmd_args.subcommand else {
            return Ok(());
        };

        // If a command was provided, it must be defined as a command parameter
        let Some(subcommand_def) = cmd_def.get_subcommand(&subcommand_arg.name) else {
            return Err(InvalidArguments::UnrecognizedArgument {
                name: subcommand_arg.name.clone(),
                kind: ArgumentKind::Command,
            });
        };

        return Self::_validate(subcommand_def, subcommand_arg.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_command_tree() {
        let app = Command::new("my-command", "my-command-help")
            .add_flag(Flag::new("flag-1", "flag-1-help").add_alias("f1"))
            .add_prop(
                Prop::new("prop-1", "prop-1-help")
                    .add_alias("p1")
                    .make_required(),
            )
            .add_command(
                Command::new("my-subcommand", "my-subcommand-help")
                    .add_alias("c1")
                    .add_prop(Prop::new("prop-2", "prop-2-help").add_alias("p2")),
            );

        assert_eq!(app.get_flag("f1").unwrap().name(), "flag-1");
        assert_eq!(app.get_prop("p1").unwrap().name(), "prop-1");

        let subcommand = app.get_subcommand("c1").unwrap();
        assert_eq!(subcommand.name(), "my-subcommand");
        assert_eq!(subcommand.get_prop("p2").unwrap().name(), "prop-2");
    }

    #[test]
    fn help_text() {
        let app = Command::new("root", "root help")
            .add_flag(Flag::new("flag1", "flag1 help"))
            .add_prop(Prop::new("prop1", "prop1 help").make_required())
            .add_prop(Prop::new("prop2", "prop2 help"))
            .add_command(Command::new("sub", "sub help"));

        let mut buf = vec![];
        let _ = help::write_help(&mut buf, &app);

        let result = String::from_utf8(buf).unwrap();

        assert_eq!(result, concat!(
                "root help\n\n",
                "Usage: root prop1=<value> [prop2=<value>] [+flag1] [COMMAND] [COMMAND ARGUMENTS]\n",
                "\n",
                "Props:\n",
                "    prop1=<PROP1>     prop1 help\n",
                "    prop2=<PROP2>     prop2 help\n",
                "\n",
                "Flags:\n",
                "    +flag1     flag1 help\n",
                "\n",
                "Commands:\n",
                "    sub     sub help\n",
                "\n",
        ));
    }

    #[test]
    fn retrieve_arguments_using_primary_name() {
        let app = Command::new("my-command", "").add_flag(Flag::new("flag", "").add_alias("f"));
        let args = app.parse_from(["exe", "+f"]).unwrap();
        assert!(args.flags.contains("flag"));
    }
}
