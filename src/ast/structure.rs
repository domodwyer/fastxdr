use super::*;

#[derive(Debug, PartialEq)]
pub struct Struct<'a> {
    pub name: String,
    pub fields: Vec<StructField<'a>>,
}

impl<'a> Struct<'a> {
    pub fn new(vs: Vec<Node<'a>>) -> Self {
        let name = vs[0].ident_str().to_string();

        let mut fields = Vec::new();
        for v in vs.into_iter().skip(1) {
            fields.push(StructField::new(v));
        }

        Struct { name, fields }
    }

    pub fn name(&'a self) -> &'a str {
        &self.name
    }
}

impl<'a> CompoundType for Struct<'a> {
    fn inner_types(&self) -> Vec<&ArrayType<BasicType>> {
        self.fields.iter().map(|f| &f.field_value).collect()
    }

    fn contains_opaque(&self) -> bool {
        self.fields.iter().any(|v| v.contains_opaque())
    }
}

#[derive(Debug, PartialEq)]
pub struct StructField<'a> {
    pub field_name: String,
    pub field_value: ArrayType<BasicType<'a>>,
    pub is_optional: bool,
}

impl<'a> StructField<'a> {
    pub fn new(v: Node<'a>) -> Self {
        let f = match v {
            Node::StructDataField(f) => f,
            e => panic!("not a struct field: {:?}", e),
        };

        match f.as_slice() {
            [Node::Type(rhs), Node::Type(BasicType::Ident(lhs))] => Self {
                field_name: lhs.to_string(),
                field_value: ArrayType::None(rhs.to_owned()),
                is_optional: false,
            },
            [Node::Type(rhs), Node::Type(BasicType::Ident(lhs)), Node::ArrayVariable(size)] => {
                let size = match size.trim() {
                    "" => None,
                    s => Some(ArraySize::from(s)),
                };
                Self {
                    field_name: lhs.to_string(),
                    field_value: ArrayType::VariableSize(rhs.to_owned(), size),
                    is_optional: false,
                }
            }
            [Node::Type(rhs), Node::Type(BasicType::Ident(lhs)), Node::ArrayFixed(size)] => Self {
                field_name: lhs.to_string(),
                field_value: ArrayType::FixedSize(rhs.to_owned(), ArraySize::from(size)),
                is_optional: false,
            },
            [Node::Type(rhs), Node::Option(opt)] => {
                let lhs = match &opt[0] {
                    Node::Type(BasicType::Ident(lhs)) => lhs,
                    _ => panic!("unexpected struct field option layout"),
                };

                Self {
                    field_name: lhs.to_string(),
                    field_value: ArrayType::None(rhs.to_owned()),
                    is_optional: true,
                }
            }
            _ => panic!("invalid number of struct field tokens"),
        }
    }

    pub fn contains_opaque(&self) -> bool {
        match self.field_value.unwrap_array() {
            BasicType::Opaque => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    macro_rules! parse {
        ($input: expr) => {{
            let ast = XDRParser::parse(Rule::item, $input)
                .unwrap()
                .next()
                .unwrap();

            let root = walk(ast).unwrap();
            let union = root.into_inner().remove(0);
            match union {
                Node::Struct(u) => u,
                _ => panic!("not a struct in ast root"),
            }
        }};
    }

    #[test]
    fn test_struct() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie;
            component4      name;
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 2);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::None(BasicType::from("nfs_cookie4"))
        );
        assert_eq!(got.fields[0].is_optional, false);

        assert_eq!(got.fields[1].field_name, "name");
        assert_eq!(
            got.fields[1].field_value,
            ArrayType::None(BasicType::from("component4"))
        );
        assert_eq!(got.fields[1].is_optional, false);
    }

    #[test]
    fn test_struct_array_variable_size_with_max_constant() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie<SOME_CONST>;
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 1);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::VariableSize(
                BasicType::from("nfs_cookie4"),
                Some(ArraySize::Constant("SOME_CONST".to_string()))
            )
        );
        assert_eq!(got.fields[0].is_optional, false);
    }

    #[test]
    fn test_struct_array_variable_size_with_max_known() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie<42>;
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 1);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::VariableSize(BasicType::from("nfs_cookie4"), Some(ArraySize::Known(42)))
        );
        assert_eq!(got.fields[0].is_optional, false);
    }

    #[test]
    fn test_struct_array_variable_size_without_max() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie<>;
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 1);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::VariableSize(BasicType::from("nfs_cookie4"), None)
        );
        assert_eq!(got.fields[0].is_optional, false);
    }

    #[test]
    fn test_struct_array_fixed_constant() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie[SOME_CONST];
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 1);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::FixedSize(
                BasicType::from("nfs_cookie4"),
                ArraySize::Constant("SOME_CONST".to_string())
            )
        );
        assert_eq!(got.fields[0].is_optional, false);
    }

    #[test]
    fn test_struct_array_fixed_known() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie[42];
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 1);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::FixedSize(BasicType::from("nfs_cookie4"), ArraySize::Known(42))
        );
        assert_eq!(got.fields[0].is_optional, false);
    }

    #[test]
    fn test_struct_linked_list() {
        let got = parse!(
            r#"
		struct entry4 {
            nfs_cookie4     cookie;
            component4      name;
            fattr4          attrs;
            entry4          *nextentry;
        };"#
        );

        assert_eq!(got.name(), "entry4");
        assert_eq!(got.fields.len(), 4);

        assert_eq!(got.fields[0].field_name, "cookie");
        assert_eq!(
            got.fields[0].field_value,
            ArrayType::None(BasicType::from("nfs_cookie4"))
        );
        assert_eq!(got.fields[0].is_optional, false);

        assert_eq!(got.fields[1].field_name, "name");
        assert_eq!(
            got.fields[1].field_value,
            ArrayType::None(BasicType::from("component4"))
        );
        assert_eq!(got.fields[1].is_optional, false);

        assert_eq!(got.fields[2].field_name, "attrs");
        assert_eq!(
            got.fields[2].field_value,
            ArrayType::None(BasicType::from("fattr4"))
        );
        assert_eq!(got.fields[2].is_optional, false);

        assert_eq!(got.fields[3].field_name, "nextentry");
        assert_eq!(
            got.fields[3].field_value,
            ArrayType::None(BasicType::from("entry4"))
        );
        assert_eq!(got.fields[3].is_optional, true);
    }
}
