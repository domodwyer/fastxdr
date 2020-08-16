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
    pub value: i32,
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
            value: f[1]
                .ident_str()
                .parse()
                .expect("non-i32 enum variant value"),
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

            let root = walk(ast).unwrap();
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
        assert_eq!(got.variants[0].value, 0);

        assert_eq!(got.variants[1].name, "GUARDED4".to_string());
        assert_eq!(got.variants[1].value, 1);

        assert_eq!(got.variants[2].name, "EXCLUSIVE4".to_string());
        assert_eq!(got.variants[2].value, 2);
    }
}
