use anyhow::bail;
use bind_args::{parse_env, ArgumentBag};

const ROOT_HELP: &str = r#"GIT
Documentation for root command

More info

# Examples

And such
"#;

const REMOTE_HELP: &str = r#"GIT REMOTE
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

fn handle_remote(mut args: ArgumentBag) -> anyhow::Result<()> {
    let mut remote = Remote::default();

    if args.remove_flag("help") || args.remove_flag("h") {
        println!("{REMOTE_HELP}");
        std::process::exit(0);
    }

    let Some(level) = args.remove_option("level") else {
        bail!("missing required option 'level'");
    };

    remote.verbose = args.remove_flag("verbose");
    remote.level = level;

    if !args.is_empty() {
        let remaining = args.remove_remaining().join(",");
        bail!("unexpected args: {remaining}");
    }

    Ok(())
}

fn handle_root(mut args: ArgumentBag) -> anyhow::Result<()> {
    let mut root = Root::default();
    if args.remove_flag("help") || args.remove_flag("h") {
        println!("{ROOT_HELP}");
        std::process::exit(0);
    }
    root.verbose = args.remove_flag("verbose");

    if !args.is_empty() {
        let remaining = args.remove_remaining().join(",");
        bail!("unexpected args: {remaining}");
    }

    Ok(())
}
pub fn main() -> anyhow::Result<()> {
    let mut cmdline = parse_env()?;

    match cmdline.remove_operand().as_deref() {
        Some("remote") => handle_remote(cmdline),
        Some(cmd) => bail!("Argument '{cmd}' is not a valid command"),
        None => handle_root(cmdline),
    }
}
