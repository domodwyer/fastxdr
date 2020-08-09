pub mod template;

mod from;
pub use from::*;

mod types;
pub use types::*;

mod wire_size;
pub use wire_size::*;

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

impl<T> std::convert::AsRef<str> for SafeName<T>
where
    T: std::fmt::Display + AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub(crate) struct NonDigitName<T>(T)
where
    T: AsRef<str>;

impl<T> std::fmt::Display for NonDigitName<T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.0.as_ref().chars().next() {
            if c.is_numeric() {
                write!(f, "v_")?;
            }
        }

        write!(f, "{}", self.0.as_ref())
    }
}

impl<T> std::convert::AsRef<str> for NonDigitName<T>
where
    T: std::fmt::Display + AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
