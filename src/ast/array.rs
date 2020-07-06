#[derive(Debug, PartialEq)]
pub enum ArraySize {
    Known(u32),
    Constant(String),
}

impl<T> From<T> for ArraySize
where
    T: AsRef<str>,
{
    fn from(v: T) -> Self {
        v.as_ref()
            .parse::<u32>()
            .map(|v| Self::Known(v))
            .unwrap_or(Self::Constant(v.as_ref().to_string()))
    }
}

impl std::fmt::Display for ArraySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known(s) => write!(f, "{}", s),
            Self::Constant(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ArrayType<T>
where
    T: std::fmt::Display,
{
    None(T),
    FixedSize(T, ArraySize),
    VariableSize(T, Option<ArraySize>),
}

impl<T> ArrayType<T>
where
    T: std::fmt::Display,
{
    pub fn unwrap_array(&self) -> &T {
        match self {
            Self::None(t) => t,
            Self::FixedSize(t, _) => t,
            Self::VariableSize(t, _) => t,
        }
    }

    pub fn write_with_bounds<S, W>(&self, f: &mut W, b: Option<&[S]>) -> std::fmt::Result
    where
        S: AsRef<str>,
        W: std::fmt::Write,
    {
        let bounds = b
            .map(|bounds| {
                format!(
                    "<{}>",
                    bounds
                        .iter()
                        .map(|b| b.as_ref())
                        .collect::<Vec<&str>>()
                        .join(", ")
                )
            })
            .unwrap_or("".to_string());

        match self {
            Self::None(t) => write!(f, "{}{}", t, bounds),
            Self::FixedSize(t, s) => write!(f, "[{}{}; {}]", t, bounds, s),
            Self::VariableSize(t, _) => write!(f, "Vec<{}{}>", t, bounds),
        }
    }
}

impl<T> std::fmt::Display for ArrayType<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_with_bounds::<&str, _>(f, None)
    }
}
