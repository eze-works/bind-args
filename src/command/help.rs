use super::Command;
use crate::args::Args;
use std::io::{self, Write};

/// Returns the name of the command help was requested for, if any
pub fn requested_help(args: &Args) -> Option<&str> {
    let mut next = Some(args);

    while let Some(a) = next {
        if a.flags.contains("h") || a.flags.contains("help") {
            return Some(&a.name);
        }

        next = a.subcommand.as_deref();
    }
    None
}

pub fn write_help(mut w: impl Write, command: &Command) -> io::Result<()> {
    write!(&mut w, "{}\n\n", command.help)?;
    write_usage(&mut w, command)?;
    write!(&mut w, "\n\n")?;

    if !command.props.is_empty() {
        writeln!(&mut w, "Props:")?;

        let prop_labels = command
            .props
            .iter()
            .map(|p| format!("{}=<{}>", p.names.join("/"), p.name().to_uppercase()))
            .collect::<Vec<_>>();

        let col_width = calculate_col_width(&prop_labels);

        for (prop, label) in command.props.iter().zip(prop_labels) {
            writeln!(&mut w, "    {label:col_width$}{}", prop.help)?;
        }

        writeln!(&mut w)?;
    }

    if !command.flags.is_empty() {
        writeln!(&mut w, "Flags:")?;
        let flag_labels = command
            .flags
            .iter()
            .map(|f| format!("+{}", f.names.join("/")))
            .collect::<Vec<_>>();

        let col_width = calculate_col_width(&flag_labels);

        for (flag, label) in command.flags.iter().zip(flag_labels) {
            writeln!(&mut w, "    {label:col_width$}{}", flag.help)?;
        }

        writeln!(&mut w)?;
    }

    if !command.commands.is_empty() {
        writeln!(&mut w, "Commands:")?;
        let command_labels = command
            .commands
            .iter()
            .map(|c| c.names.join("/"))
            .collect::<Vec<_>>();

        let col_width = calculate_col_width(&command_labels);

        for (cmd, label) in command.commands.iter().zip(command_labels) {
            writeln!(&mut w, "    {label:col_width$}{}", cmd.help)?;
        }

        writeln!(&mut w)?;
    }
    w.flush()
}

fn write_usage(mut w: impl Write, command: &Command) -> io::Result<()> {
    write!(&mut w, "Usage: {}", command.name())?;

    for prop in command.props.iter().filter(|p| p.required) {
        write!(&mut w, " {}=<value>", prop.name())?;
    }

    for prop in command.props.iter().filter(|p| !p.required) {
        write!(&mut w, " [{}=<value>]", prop.name())?;
    }

    for flag in &command.flags {
        write!(&mut w, " [+{}]", flag.name())?;
    }

    if !command.commands.is_empty() {
        write!(&mut w, " [COMMAND] [COMMAND ARGUMENTS]")?;
    }

    Ok(())
}

fn calculate_col_width(list: &[String]) -> usize {
    list.iter().map(|s| s.len()).max().unwrap_or(0) + 5
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieve_help_command() {
        let cmdline = ["exe", "command", "subcommand", "+help"];
        let args = Args::parse_from(cmdline);
        assert_eq!(requested_help(&args), Some("subcommand"));

        let cmdline = ["exe", "+help"];
        let args = Args::parse_from(cmdline);
        assert_eq!(requested_help(&args), Some("exe"));

        let cmdline = ["exe", "command"];
        let args = Args::parse_from(cmdline);
        assert_eq!(requested_help(&args), None);

        let cmdline: Vec<String> = vec![];
        let args = Args::parse_from(cmdline);
        assert_eq!(requested_help(&args), None);
    }
}
