use anyhow::bail;
use bind_args::{parse_from_env, Cmdline};

const ROOT_HELP: &'static str = r#"GIT
Documentation for root command

More info

# Examples

And such
"#;

const REMOTE_HELP: &'static str = r#"GIT REMOTE
Documentation for git remote command

More info

"#;

#[derive(Default)]
struct Root {
    verbose: bool,
}

#[derive(Default)]
struct Remote {
    level: String,
    verbose: bool,
}

fn handle_remote(mut args: Cmdline) -> anyhow::Result<()> {
    let mut remote = Remote::default();

    if args.take_flag("help") || args.take_flag("h") {
        println!("{REMOTE_HELP}");
        std::process::exit(0);
    }

    let Some(level) = args.take_option("level") else {
        bail!("missing required option 'level'");
    };

    remote.verbose = args.take_flag("verbose");
    remote.level = level;

    if !args.is_empty() {
        let remaining = args.take_remaining().join(",");
        bail!("unexpected args: {remaining}");
    }

    Ok(())
}

fn handle_root(mut args: Cmdline) -> anyhow::Result<()> {
    let mut root = Root::default();
    if args.take_flag("help") || args.take_flag("h") {
        println!("{ROOT_HELP}");
        std::process::exit(0);
    }
    root.verbose = args.take_flag("verbose");

    if !args.is_empty() {
        let remaining = args.take_remaining().join(",");
        bail!("unexpected args: {remaining}");
    }

    Ok(())
}
pub fn main() -> anyhow::Result<()> {
    let cmdline = parse_from_env()?;

    match cmdline.command() {
        Some("remote") => handle_remote(cmdline),
        Some(cmd) => bail!("Argument '{cmd}' is not a valid command"),
        None => handle_root(cmdline),
    }
}
