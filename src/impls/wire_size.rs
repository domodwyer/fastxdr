use super::SafeName;
use crate::ast::BasicType;
use crate::impls::from::FromTemplate;
use crate::indexes::{AstType, GenericIndex, TypeIndex};
use crate::Result;

pub fn print_impl_wire_size<'a, W: std::fmt::Write, T: FromTemplate>(
    mut w: W,
    template: T,
    type_index: &TypeIndex<'a>,
    generic_index: &GenericIndex<'a>,
) -> Result<()> {
    for item in type_index.iter() {
        match item {
            AstType::Struct(v) => {
                print_impl(&mut w, template, v.name(), generic_index, |w| {
                    for f in v.fields.iter() {
                        writeln!(w, r#"self.{}.wire_size() +"#, SafeName(&f.field_name))?;

                        // In-line opaques require padding
                        if f.contains_opaque() {
                            writeln!(
                                w,
                                r#" pad_length(self.{}.wire_size()) +"#,
                                SafeName(&f.field_name)
                            )?;
                        }
                    }

                    writeln!(w, "0")?;
                    Ok(())
                })?;
            }

            AstType::Union(v) => {
                print_impl(&mut w, template, v.name(), generic_index, |w| {
                    writeln!(w, "4 + match self {{")?;
                    // Iterate over all the variants of v, including the
                    // default.
                    for case in v.cases.iter().chain(v.default.iter()) {
                        write!(
                            w,
                            r#"Self::{}(inner) => inner.wire_size()"#,
                            SafeName(&case.field_name)
                        )?;

                        // In-line opaques require padding
                        if case.contains_opaque() {
                            writeln!(w, r#" + pad_length(inner.wire_size()),"#)?;
                        } else {
                            writeln!(w, ",")?;
                        }
                    }

                    // And there may be one or more void cases.
                    if v.void_cases.len() > 0 {
                        writeln!(w, r#"Self::Void => 0,"#)?;
                    }

                    writeln!(w, "}}")?;
                    Ok(())
                })?;
            }

            AstType::Enum(v) => {
                print_impl(&mut w, template, &v.name, generic_index, |w| {
                    writeln!(w, "4")?;
                    Ok(())
                })?;
            }

            AstType::Typedef(v) => {
                print_impl(
                    &mut w,
                    template,
                    v.alias.unwrap_array().as_str(),
                    generic_index,
                    |w| {
                        writeln!(w, "self.0.wire_size()")?;

                        // If the target is opaque, it needs padding, and a
                        // length prefix adding.
                        if let BasicType::Opaque = v.target {
                            writeln!(w, "+ pad_length(self.0.wire_size()) + 4")?;
                        }

                        Ok(())
                    },
                )?;
            }
        }
    }

    Ok(())
}

fn print_impl<W: std::fmt::Write, T: FromTemplate, F: Fn(&mut W) -> Result<()>>(
    mut w: W,
    template: T,
    name: &str,
    generic_index: &GenericIndex,
    func: F,
) -> Result<()> {
    if generic_index.contains(name) {
        writeln!(w, "impl WireSize for {}<{}> {{", name, template.type_name(),)?;
    } else {
        writeln!(w, r#"impl WireSize for {} {{"#, name)?;
    }

    writeln!(w, r#"fn wire_size(&self) -> usize {{"#)?;
    func(&mut w)?;
    writeln!(w, "}}\n}}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impls::from::bytes::RefMutBytes;
    use crate::indexes::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    macro_rules! test_convert {
        ($name: ident, $input: expr, $want: expr) => {
            #[test]
            fn $name() {
                let mut ast = XDRParser::parse(Rule::item, $input).unwrap();
                let ast = walk(ast.next().unwrap()).unwrap();
                let generic_index = build_generic_index(&ast);
                let type_index = TypeIndex::new(&ast);

                let mut got = String::new();
                print_impl_wire_size(&mut got, RefMutBytes, &type_index, &generic_index).unwrap();

                assert_eq!(got, $want);
            }
        };
    }

    test_convert!(
        test_struct_with_variable_array,
        r#"
			struct small {
				unsigned int a;
				unsigned hyper b;
				int c;
				hyper d;
				float e;
				double f;
				string g;
				bool h;
			};
		"#,
        r#"impl WireSize for small {
fn wire_size(&self) -> usize {
self.a.wire_size() +
self.b.wire_size() +
self.c.wire_size() +
self.d.wire_size() +
self.e.wire_size() +
self.f.wire_size() +
self.g.wire_size() +
self.h.wire_size() +
0
}
}
"#
    );

    test_convert!(
        test_struct_with_fixed_array,
        r#"
			struct small {
				unsigned int a[10];
			};
		"#,
        r#"impl WireSize for small {
fn wire_size(&self) -> usize {
self.a.wire_size() +
0
}
}
"#
    );

    test_convert!(
        test_struct_basic_types,
        r#"
			struct small {
				unsigned int a;
				unsigned hyper b;
				int c;
				hyper d;
				float e;
				double f;
				string g;
				bool h;
			};
		"#,
        r#"impl WireSize for small {
fn wire_size(&self) -> usize {
self.a.wire_size() +
self.b.wire_size() +
self.c.wire_size() +
self.d.wire_size() +
self.e.wire_size() +
self.f.wire_size() +
self.g.wire_size() +
self.h.wire_size() +
0
}
}
"#
    );

    test_convert!(
        test_struct_reserved_name,
        r#"
			struct small {
				unsigned int type;
			};
		"#,
        r#"impl WireSize for small {
fn wire_size(&self) -> usize {
self.type_v.wire_size() +
0
}
}
"#
    );

    test_convert!(
        test_struct_basic_types_generic,
        r#"
			struct small {
				unsigned int a;
				unsigned hyper b;
				int c;
				hyper d;
				float e;
				double f;
				string g;
				bool h;
				opaque i;
			};
		"#,
        r#"impl WireSize for small<Bytes> {
fn wire_size(&self) -> usize {
self.a.wire_size() +
self.b.wire_size() +
self.c.wire_size() +
self.d.wire_size() +
self.e.wire_size() +
self.f.wire_size() +
self.g.wire_size() +
self.h.wire_size() +
self.i.wire_size() +
 pad_length(self.i.wire_size()) +
0
}
}
"#
    );

    test_convert!(
        test_union,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
				u64       name;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::resok4(inner) => inner.wire_size(),
Self::name(inner) => inner.wire_size(),
}
}
}
"#
    );

    test_convert!(
        test_union_switch_type,
        r#"
            enum choice {
                OP_A = 1,
                OP_B = 2
            };
			union CB_GETATTR4res switch (choice status) {
			case 1:
				u32       resok4;
			case 2:
				u64       name;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::resok4(inner) => inner.wire_size(),
Self::name(inner) => inner.wire_size(),
}
}
}
impl WireSize for choice {
fn wire_size(&self) -> usize {
4
}
}
"#
    );

    test_convert!(
        test_union_reserved_name,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
				u64       type;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::resok4(inner) => inner.wire_size(),
Self::type_v(inner) => inner.wire_size(),
}
}
}
"#
    );

    test_convert!(
        test_union_generic,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
				opaque       name;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res<Bytes> {
fn wire_size(&self) -> usize {
4 + match self {
Self::resok4(inner) => inner.wire_size(),
Self::name(inner) => inner.wire_size() + pad_length(inner.wire_size()),
}
}
}
"#
    );

    test_convert!(
        test_union_with_fallthrough,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
			case 3:
				u64       name;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::resok4(inner) => inner.wire_size(),
Self::name(inner) => inner.wire_size(),
}
}
}
"#
    );

    test_convert!(
        test_union_with_default,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			default:
				u64       name;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::resok4(inner) => inner.wire_size(),
Self::name(inner) => inner.wire_size(),
}
}
}
"#
    );

    test_convert!(
        test_enum,
        r#"
			enum Status{
				MODE4_SUID = 1,
				MODE4_OTHER = 2
			};
		"#,
        r#"impl WireSize for Status {
fn wire_size(&self) -> usize {
4
}
}
"#
    );

    test_convert!(
        test_typedef,
        r#"
			typedef my_union alias;
			union my_union switch (unsigned int status) {
			case 1:
				u32       var;
			};
		"#,
        r#"impl WireSize for alias {
fn wire_size(&self) -> usize {
self.0.wire_size()
}
}
impl WireSize for my_union {
fn wire_size(&self) -> usize {
4 + match self {
Self::var(inner) => inner.wire_size(),
}
}
}
"#
    );

    test_convert!(
        test_typedef_generic,
        r#"
			typedef opaque alias;
		"#,
        r#"impl WireSize for alias<Bytes> {
fn wire_size(&self) -> usize {
self.0.wire_size()
+ pad_length(self.0.wire_size()) + 4
}
}
"#
    );

    test_convert!(
        test_typedef_variable_array_generic,
        r#"
			typedef opaque  alias<NFS4_FHSIZE>;
		"#,
        r#"impl WireSize for alias<Bytes> {
fn wire_size(&self) -> usize {
self.0.wire_size()
+ pad_length(self.0.wire_size()) + 4
}
}
"#
    );
}
