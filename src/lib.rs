//! A parser for an alternative command line style
//!
//! This crate parses command lines of the form:
//! ```text
//! git commit message="message" +all
//! git commit m="message" +a
//! ```
//!
//! GNU/POSIX Style | bind_args style
//! -|-
//! Boolean flags (e.g. `--all`)|Flags (`+all`)
//! Options (e.g. `--key=value` or `--key value`)|Props (`key=value`)
//! Commands & subcommands (e.g. `git commit`| Same
//!
//!
//! # Syntax
//!
//! - An argument prefixed with `+` sets a [`Flag`] of the same name
//! - An argument containing a `=` sets a [`Prop`] as long as the `=` is not the prefix. Whatever is to the left of the
//! equals sign is the property name. Whatever is to the right of the equals sign is the value.
//! - Everything else is a sub-[`Command`]. Sub-commands can have their own props, flags and
//! sub-commands.

mod args;
mod command;

pub use args::Args;
pub use command::{Command, Flag, Prop};
