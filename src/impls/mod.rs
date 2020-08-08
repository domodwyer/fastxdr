pub mod from;
mod types;

pub use types::*;

pub(crate) struct SafeName<T>(T)
where
    T: AsRef<str>;

impl<T> std::fmt::Display for SafeName<T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            "as" | "async" | "await" | "break" | "const" | "continue" | "crate" | "dyn"
            | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if" | "impl" | "in"
            | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return"
            | "Self" | "self" | "static" | "struct" | "super" | "trait" | "true" | "type"
            | "union" | "unsafe" | "use" | "where" | "while" => write!(f, "{}_v", self.0.as_ref()),
            "TRUE" | "FALSE" => write!(f, "{}", self.0.as_ref().to_lowercase()),
            _ => write!(f, "{}", self.0.as_ref()),
        }
    }
}
