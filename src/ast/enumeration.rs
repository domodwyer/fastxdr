use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<Variant>,
}

impl<'a> Enum {
    pub(crate) fn new(vs: Vec<Node<'a>>) -> Self {
        let name = vs[0].ident_str().to_string();

        let mut vars = Vec::new();
        for v in vs.into_iter().skip(1) {
            vars.push(Variant::new(v))
        }

        Self {
            name,
            variants: vars,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub value: VariantValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantValue {
    String(String),
    Numeric(i32),
}

impl std::fmt::Display for VariantValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Numeric(n) => write!(f, "{}", n),
        }
    }
}

impl<T> From<T> for VariantValue
where
    T: AsRef<str>,
{
    fn from(v: T) -> Self {
        let v = v.as_ref();
        if v.starts_with("0x") {
            let clean = v.trim_start_matches("0x");
            return Self::Numeric(i32::from_str_radix(clean, 16).unwrap());
        }

        v.parse::<i32>()
            .map(Self::Numeric)
            .unwrap_or_else(|_| Self::String(v.to_string()))
    }
}

impl<'a> Variant {
    fn new(v: Node<'a>) -> Self {
        let f = match v {
            Node::EnumVariant(f) => f,
            e => panic!("not a struct field: {:?}", e),
        };

        if f.len() != 2 {
            panic!("unexpected number of tokens in enum")
        }

        Self {
            name: f[0].ident_str().to_string(),
            value: f[1].ident_str().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! parse {
        ($input: expr) => {{
            let ast = XDRParser::parse(Rule::item, $input)
                .unwrap()
                .next()
                .unwrap();

            let root = walk(ast);
            let union = root.into_inner().remove(0);
            match union {
                Node::Enum(u) => u,
                _ => panic!("not an enum in ast root"),
            }
        }};
    }

    #[test]
    fn test_enum() {
        let got = parse!(
            r#"
        enum createmode4 {
                UNCHECKED4      = 0,
                GUARDED4        = 1,
                EXCLUSIVE4      = 2
        };"#
        );

        assert_eq!(got.name, "createmode4");
        assert_eq!(got.variants.len(), 3);

        assert_eq!(got.variants[0].name, "UNCHECKED4".to_string());
        assert_eq!(got.variants[0].value, VariantValue::Numeric(0));

        assert_eq!(got.variants[1].name, "GUARDED4".to_string());
        assert_eq!(got.variants[1].value, VariantValue::Numeric(1));

        assert_eq!(got.variants[2].name, "EXCLUSIVE4".to_string());
        assert_eq!(got.variants[2].value, VariantValue::Numeric(2));
    }

    #[test]
    fn test_enum_hex_values() {
        let got = parse!(
            r#"
        enum createmode4 {
                UNCHECKED4      = 0x01,
                GUARDED4        = 0x02,
                EXCLUSIVE4      = 0xFF
        };"#
        );

        assert_eq!(got.name, "createmode4");
        assert_eq!(got.variants.len(), 3);

        assert_eq!(got.variants[0].name, "UNCHECKED4".to_string());
        assert_eq!(got.variants[0].value, VariantValue::Numeric(1));

        assert_eq!(got.variants[1].name, "GUARDED4".to_string());
        assert_eq!(got.variants[1].value, VariantValue::Numeric(2));

        assert_eq!(got.variants[2].name, "EXCLUSIVE4".to_string());
        assert_eq!(got.variants[2].value, VariantValue::Numeric(255));
    }

    #[test]
    fn test_enum_const_strings() {
        let got = parse!(
            r#"
        enum createmode4 {
                UNCHECKED4      = CONST_ONE,
                GUARDED4        = CONST_TWO,
                EXCLUSIVE4      = CONST_THREE
        };"#
        );

        assert_eq!(got.name, "createmode4");
        assert_eq!(got.variants.len(), 3);

        assert_eq!(got.variants[0].name, "UNCHECKED4".to_string());
        assert_eq!(
            got.variants[0].value,
            VariantValue::String("CONST_ONE".to_string())
        );

        assert_eq!(got.variants[1].name, "GUARDED4".to_string());
        assert_eq!(
            got.variants[1].value,
            VariantValue::String("CONST_TWO".to_string())
        );

        assert_eq!(got.variants[2].name, "EXCLUSIVE4".to_string());
        assert_eq!(
            got.variants[2].value,
            VariantValue::String("CONST_THREE".to_string())
        );
    }
}
