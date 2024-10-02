# bind_args

A simple command-line argument parser.

- [Documentation](https://docs.rs/bind-args/)
- [Library on crates.io](https://crates.io/crates/bind-args)

# Example

```rust
use bind_args::parse;

struct AppArgs {
    verbose: bool,
    log_level: Option<String>,
    path: String,
}

let mut bag = parse(["program", "--log-level", "INFO", "--verbose", "/etc/config"]).unwrap();

let args = AppArgs {
    verbose: bag.remove_flag("verbose"),
    log_level: bag.remove_option("log-level"),
    path: bag.remove_operand().unwrap_or(String::from("/"))
};

assert_eq!(args.verbose, true);
assert_eq!(args.log_level.as_deref(), Some("INFO"));
assert_eq!(args.path, "/etc/config");

// It is important to make sure there are not unexpected arguments.
if !bag.is_empty() {
    let unexpected = bag.remove_remaining().join(", ");
    eprintln!("Unexpected argument(s): {}", unexpected);
    std::process::exit(1);
}
```
