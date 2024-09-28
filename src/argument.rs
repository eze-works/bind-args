pub enum Argument {
    Positional(&'static str, String),
    Flag(&'static str),
    Switch(&'static str, String),
    Command(&'static str),
}
