use super::*;

#[derive(Debug, PartialEq)]
pub struct UnionSwitch<'a> {
    pub var_name: String,
    pub var_type: BasicType<'a>,
}

#[derive(Debug, PartialEq)]
pub struct Union<'a> {
    pub name: String,
    pub cases: Vec<UnionCase<'a>>,
    pub default: Option<UnionCase<'a>>,
    pub void_cases: Vec<String>,
    pub switch: UnionSwitch<'a>,
}

impl<'a> Union<'a> {
    pub fn new(vs: Vec<Node<'a>>) -> Self {
        let name = vs[0].ident_str().to_string();

        let mut cases = Vec::new();
        let mut void_cases = Vec::new();
        let mut default = None;

        let switch = UnionSwitch {
            var_name: vs[2].ident_str().to_string(),
            var_type: BasicType::from(vs[1].ident_str().to_string()),
        };

        // Collect the set of case values that "fallthrough" to the eventual
        // UnionCase
        let mut case_values = Vec::new();

        for v in vs.into_iter().skip(3) {
            let mut is_default_case = false;
            let stmt = match v {
                Node::UnionCase(nodes) => CaseStmt::parse(case_values, nodes),
                Node::UnionDefault(nodes) => {
                    is_default_case = true;
                    case_values.push("default".to_string());
                    CaseStmt::parse(case_values, nodes)
                }
                v => panic!("unexpected token type for union {:?}", v),
            };

            match stmt {
                CaseStmt::Defined(c) if is_default_case => default = Some(c),
                CaseStmt::Defined(c) => cases.push(c),
                CaseStmt::Fallthrough(values) => {
                    // The parsed fallthrough ident has been pushed to the
                    // returned case_values
                    case_values = values;
                    continue;
                }
                CaseStmt::Void(values) => void_cases.extend_from_slice(&values),
            }

            case_values = Vec::new()
        }

        Union {
            name,
            cases,
            default,
            void_cases,
            switch,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> CompoundType for Union<'a> {
    fn inner_types(&self) -> Vec<&ArrayType<BasicType>> {
        self.cases
            .iter()
            .chain(self.default.iter())
            .map(|f| &f.field_value)
            .collect()
    }

    fn contains_opaque(&self) -> bool {
        self.cases
            .iter()
            .chain(self.default.iter())
            .any(|f| f.contains_opaque())
    }
}

#[derive(Debug, PartialEq)]
pub struct UnionCase<'a> {
    /// The case values that map to this field name and type.
    ///
    /// This can be more than one value when the union contains fallthrough
    /// statements.
    pub case_values: Vec<String>,
    pub field_name: String,
    pub field_value: ArrayType<BasicType<'a>>,
}

impl<'a> UnionCase<'a> {
    pub fn new(case_values: Vec<String>, field: Vec<Node<'a>>) -> Self {
        match field.as_slice() {
            [Node::Type(t), Node::Type(BasicType::Ident(l))] => Self {
                case_values,
                field_name: l.to_string(),
                field_value: ArrayType::None(t.to_owned()),
            },
            [Node::Type(t), Node::Type(l)] => Self {
                case_values,
                field_name: l.as_str().to_string(),
                field_value: ArrayType::None(t.to_owned()),
            },
            _ => panic!("invalid number of union field tokens"),
        }
    }

    pub fn contains_opaque(&self) -> bool {
        match self.field_value.unwrap_array() {
            BasicType::Opaque => true,
            _ => false,
        }
    }
}

enum CaseStmt<'a> {
    /// A case statement with no fields defined, falling through to the next
    /// case statement.
    Fallthrough(Vec<String>),

    /// A fully-defined case statement, with a case value and fields.
    Defined(UnionCase<'a>),

    Void(Vec<String>),
}

impl<'a> CaseStmt<'a> {
    fn parse(mut case_values: Vec<String>, mut nodes: Vec<Node<'a>>) -> Self {
        match nodes.remove(0) {
            Node::Type(t) => case_values.push(t.as_str().to_string()),
            Node::UnionVoid => {
                // No ident, this is a default case
                return Self::Void(case_values);
            }
            Node::UnionDataField(nodes) => {
                // No ident, this is a default case
                return Self::Defined(UnionCase::new(case_values, nodes));
            }
            _ => unreachable!(),
        };

        if nodes.len() == 0 {
            return Self::Fallthrough(case_values);
        }

        match nodes.remove(0) {
            Node::UnionDataField(nodes) => Self::Defined(UnionCase::new(case_values, nodes)),
            Node::UnionVoid => return Self::Void(case_values),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;
    use std::borrow::Cow;

    macro_rules! parse {
        ($input: expr) => {{
            let ast = XDRParser::parse(Rule::item, $input)
                .unwrap()
                .next()
                .unwrap();

            let root = walk(ast).unwrap();
            let union = root.into_inner().remove(0);
            match union {
                Node::Union(u) => u,
                _ => panic!("not a union in ast root"),
            }
        }};
    }

    #[test]
    fn test_union() {
        let got = parse!(
            r#"
		union createhow4 switch (createmode4 mode) {
			case GUARDED4:
					fattr4         createattrs;
			case EXCLUSIVE4:
					verifier4      createverf;
		};"#
        );

        assert_eq!(got.name, "createhow4");
        assert_eq!(got.default, None);
        assert_eq!(got.void_cases.len(), 0);
        assert_eq!(got.cases.len(), 2);

        assert_eq!(&got.cases[0].case_values, &["GUARDED4"]);
        assert_eq!(got.cases[0].field_name, "createattrs");
        assert_eq!(
            got.cases[0].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("fattr4")))
        );

        assert_eq!(&got.cases[1].case_values, &["EXCLUSIVE4"]);
        assert_eq!(got.cases[1].field_name, "createverf");
        assert_eq!(
            got.cases[1].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("verifier4")))
        );

        assert_eq!(got.switch.var_name, "mode");
        assert_eq!(
            got.switch.var_type,
            BasicType::Ident(Cow::from("createmode4"))
        );
    }

    #[test]
    fn test_union_fallthrough() {
        let got = parse!(
            r#"
		union createhow4 switch (createmode4 mode) {
			case UNCHECKED4:
			case GUARDED4:
					fattr4         createattrs;
			case EXCLUSIVE4:
					verifier4      createverf;
		};"#
        );

        assert_eq!(got.name, "createhow4");
        assert_eq!(got.default, None);
        assert_eq!(got.void_cases.len(), 0);
        assert_eq!(got.cases.len(), 2);

        assert_eq!(&got.cases[0].case_values, &["UNCHECKED4", "GUARDED4"]);
        assert_eq!(got.cases[0].field_name, "createattrs");
        assert_eq!(
            got.cases[0].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("fattr4")))
        );

        assert_eq!(&got.cases[1].case_values, &["EXCLUSIVE4"]);
        assert_eq!(got.cases[1].field_name, "createverf");
        assert_eq!(
            got.cases[1].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("verifier4")))
        );

        assert_eq!(got.switch.var_name, "mode");
        assert_eq!(
            got.switch.var_type,
            BasicType::Ident(Cow::from("createmode4"))
        );
    }

    #[test]
    fn test_union_void_default() {
        let got = parse!(
            r#"
		union LOCKU4res switch (nfsstat4 status) {
			case NFS4_OK:
					stateid4       lock_stateid;
			default:
					void;
		};"#
        );

        assert_eq!(got.name, "LOCKU4res");
        assert_eq!(got.default, None);

        assert_eq!(got.cases.len(), 1);
        assert_eq!(&got.cases[0].case_values, &["NFS4_OK"]);
        assert_eq!(got.cases[0].field_name, "lock_stateid");
        assert_eq!(
            got.cases[0].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("stateid4")))
        );

        assert_eq!(got.void_cases.len(), 1);
        assert_eq!(&got.void_cases, &["default"]);

        assert_eq!(got.switch.var_name, "status");
        assert_eq!(got.switch.var_type, BasicType::Ident(Cow::from("nfsstat4")));
    }

    #[test]
    fn test_union_default() {
        let got = parse!(
            r#"
		union LOCKU4res switch (nfsstat4 status) {
			case NFS4_OK:
					stateid4       lock_stateid;
			default:
					type_name field_name;
		};"#
        );

        assert_eq!(got.name, "LOCKU4res");
        assert_eq!(got.cases.len(), 1);
        assert_eq!(&got.cases[0].case_values, &["NFS4_OK"]);
        assert_eq!(got.cases[0].field_name, "lock_stateid");
        assert_eq!(
            got.cases[0].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("stateid4")))
        );

        assert_eq!(got.void_cases.len(), 0);

        let default = &got.default.unwrap();
        assert_eq!(default.case_values, &["default"]);
        assert_eq!(default.field_name, "field_name");
        assert_eq!(
            default.field_value,
            ArrayType::None(BasicType::Ident(Cow::from("type_name")))
        );

        assert_eq!(got.switch.var_name, "status");
        assert_eq!(got.switch.var_type, BasicType::Ident(Cow::from("nfsstat4")));
    }

    #[test]
    fn test_union_case_void() {
        let got = parse!(
            r#"
		union LOCKU4res switch (nfsstat4 status) {
			case NFS4_OK:
					stateid4       lock_stateid;
			case something:
				void;
			default:
					type_name field_name;
		};"#
        );

        assert_eq!(got.name, "LOCKU4res");
        assert_eq!(got.cases.len(), 1);
        assert_eq!(&got.cases[0].case_values, &["NFS4_OK"]);
        assert_eq!(got.cases[0].field_name, "lock_stateid");
        assert_eq!(
            got.cases[0].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("stateid4")))
        );

        assert_eq!(got.void_cases, &["something"]);

        let default = &got.default.unwrap();
        assert_eq!(default.case_values, &["default"]);
        assert_eq!(default.field_name, "field_name");
        assert_eq!(
            default.field_value,
            ArrayType::None(BasicType::Ident(Cow::from("type_name")))
        );

        assert_eq!(got.switch.var_name, "status");
        assert_eq!(got.switch.var_type, BasicType::Ident(Cow::from("nfsstat4")));
    }

    #[test]
    fn test_union_case_void_fallthrough() {
        let got = parse!(
            r#"
		union LOCKU4res switch (nfsstat4 status) {
			case NFS4_OK:
					stateid4       lock_stateid;
			case another:
			case something:
				void;
			default:
					type_name field_name;
		};"#
        );

        assert_eq!(got.name, "LOCKU4res");
        assert_eq!(got.cases.len(), 1);
        assert_eq!(&got.cases[0].case_values, &["NFS4_OK"]);
        assert_eq!(got.cases[0].field_name, "lock_stateid");
        assert_eq!(
            got.cases[0].field_value,
            ArrayType::None(BasicType::Ident(Cow::from("stateid4")))
        );

        assert_eq!(got.void_cases, &["another", "something",]);

        let default = &got.default.unwrap();
        assert_eq!(default.case_values, &["default"]);
        assert_eq!(default.field_name, "field_name");
        assert_eq!(
            default.field_value,
            ArrayType::None(BasicType::Ident(Cow::from("type_name")))
        );

        assert_eq!(got.switch.var_name, "status");
        assert_eq!(got.switch.var_type, BasicType::Ident(Cow::from("nfsstat4")));
    }
}
