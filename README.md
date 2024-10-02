# bind_args

A parser for [self-describing command line arguments](https://eze.works/post/self-describing-command-line-arguments).

- [Documentation](https://docs.rs/bind-args/0.3.0/bind_args/)
- [Library on crates.io](https://crates.io/crates/bind-args)

The syntax is similar to the prevalent GNU and POSIX syntaxes, but deviates in a few ways for clarity and ease of implementation:

- Options and their values are always written as one shell "word" separated by `=`.
  (e.g. `--level=info` or `-f=archilve.tar`)
- Fused-style arguments are not allowed
- Sub-commands are prefixed with the at-sign (i.e. `@`)
- There may only be one sub-command.

It looks like this:

```text
./program @command --option=value --flag operand
```

# Example

```rust
use bind_args::parse;

struct AppArgs {
    verbose: bool,
    log_level: Option<String>,
    path: String,
}

let mut bag = parse(["program", "--log-level=INFO", "--verbose", "/etc/config"]).unwrap();

let args = AppArgs {
    verbose: bag.remove_flag("verbose"),
    log_level: bag.remove_option("log-level"),
    path: bag.remove_operand(0).unwrap_or(String::from("/"))
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
