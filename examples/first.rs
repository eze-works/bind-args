use bind_args::{Command, Flag, Prop};
use std::env::args;

fn main() {
    let args = args();

    let app = Command::new("myexample", "my help is here")
        .add_prop(Prop::new("log-level", "The log leve to use"))
        .add_flag(Flag::new("verbose", "If to be loud"));

    let args = app.parse_from(args);

    println!("{:?}", args);
}
