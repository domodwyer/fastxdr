#[derive(Debug, Clone, PartialEq)]
pub enum BasicType {
    U32,
    U64,
    I32,
    I64,
    F32,
    F64,
    String,
    Bool,
    Opaque,
    Ident(String),
}

impl<'a> BasicType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::U32 => "u32",
            Self::I32 => "i32",
            Self::U64 => "u64",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
            Self::String => "String",
            Self::Opaque => "T",
            Self::Ident(s) => s,
        }
    }

    /// Returns the same value as `as_str` except for idents that have reserved
    /// rust names, which are mapped to `<ident>_v`.
    pub fn as_safe_string(&self) -> String {
        let name = match self {
            Self::Ident(v) => v.as_ref(),
            _ => return self.as_str().to_string(),
        };

        let name = match name {
            "TRUE" | "FALSE" => name.to_lowercase(),
            v => v.to_string(),
        };

        match name.as_str() {
            "as" | "async" | "await" | "break" | "const" | "continue" | "crate" | "dyn"
            | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if" | "impl" | "in"
            | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return"
            | "Self" | "self" | "static" | "struct" | "super" | "trait" | "true" | "type"
            | "union" | "unsafe" | "use" | "where" | "while" => format!("{}_v", name),
            _ => name,
        }
    }

    pub fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque)
    }
}

impl<'a> std::fmt::Display for BasicType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_safe_string())
    }
}

impl<'a> From<&'a str> for BasicType {
    fn from(v: &'a str) -> Self {
        match v.trim() {
            "unsigned int" | "uint32_t" | "u32" | "unsigned" => Self::U32,
            "int" | "int32_t" | "i32" => Self::I32,
            "unsigned hyper" | "uint64_t" | "u64" => Self::U64,
            "hyper" | "int64_t" | "i64" => Self::I64,
            "float" => Self::F32,
            "double" => Self::F64,
            "string" => Self::String,
            "opaque" => Self::Opaque,
            "bool" => Self::Bool,
            s => Self::Ident(s.to_string()),
        }
    }
}

impl<'a> From<String> for BasicType {
    fn from(v: String) -> Self {
        match v.trim() {
            "unsigned int" | "uint32_t" | "u32" => Self::U32,
            "int" | "int32_t" | "i32" => Self::I32,
            "unsigned hyper" | "uint64_t" | "u64" => Self::U64,
            "hyper" | "int64_t" | "i64" => Self::I64,
            "float" => Self::F32,
            "double" => Self::F64,
            "string" => Self::String,
            "opaque" => Self::Opaque,
            "bool" => Self::Bool,
            s => Self::Ident(s.to_string()),
        }
    }
}
