use super::{NonDigitName, SafeName};
use crate::ast::{Ast, BasicType};
use crate::impls::template::*;
use crate::indexes::AstType;
use crate::Result;

pub fn print_impl_wire_size<'a, W: std::fmt::Write, T: FromTemplate>(
    mut w: W,
    template: T,
    ast: &Ast,
) -> Result<()> {
    for item in ast.types().iter() {
        match item {
            AstType::Struct(v) => {
                print_impl(&mut w, template, v.name(), ast, |w| {
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
                print_impl(&mut w, template, v.name(), ast, |w| {
                    writeln!(w, "4 + match self {{")?;
                    // Iterate over all the variants of v, including the
                    // default.
                    for case in v.cases.iter().chain(v.default.iter()) {
                        // A single case statement may have many case values tied to it
                        // if fallthrough values are used:
                        //
                        // 	case 1:
                        // 	case 2:
                        // 		// statement
                        //
                        for c_value in case.case_values.iter() {
                            write!(
                                w,
                                r#"Self::{}(inner) => inner.wire_size()"#,
                                NonDigitName(SafeName(c_value))
                            )?;

                            // In-line opaques require padding
                            if case.contains_opaque() {
                                writeln!(w, r#" + pad_length(inner.wire_size()),"#)?;
                            } else {
                                writeln!(w, ",")?;
                            }
                        }
                    }

                    // There may also be several "void" cases
                    for c in v.void_cases.iter() {
                        writeln!(w, "Self::{} => 0,", NonDigitName(SafeName(c.as_str())))?;
                    }

                    if v.default.is_some() {
                        writeln!(w, "Self::default => 0,")?;
                    }

                    writeln!(w, "}}")?;
                    Ok(())
                })?;
            }

            AstType::Enum(v) => {
                print_impl(&mut w, template, &v.name, ast, |w| {
                    writeln!(w, "4")?;
                    Ok(())
                })?;
            }

            AstType::Typedef(v) => {
                print_impl(
                    &mut w,
                    template,
                    v.alias.unwrap_array().as_str(),
                    ast,
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
    ast: &Ast,
    func: F,
) -> Result<()> {
    if ast.generics().contains(name) {
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
    use crate::impls::template::bytes::RefMutBytes;

    macro_rules! test_convert {
        ($name: ident, $input: expr, $want: expr) => {
            #[test]
            fn $name() {
                let ast = Ast::new($input).unwrap();

                let mut got = String::new();
                print_impl_wire_size(&mut got, RefMutBytes, &ast).unwrap();

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
Self::v_1(inner) => inner.wire_size(),
Self::v_2(inner) => inner.wire_size(),
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
Self::v_1(inner) => inner.wire_size(),
Self::v_2(inner) => inner.wire_size(),
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
			case type:
				u64       async;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::v_1(inner) => inner.wire_size(),
Self::type(inner) => inner.wire_size(),
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
Self::v_1(inner) => inner.wire_size(),
Self::v_2(inner) => inner.wire_size() + pad_length(inner.wire_size()),
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
Self::v_1(inner) => inner.wire_size(),
Self::v_2(inner) => inner.wire_size(),
Self::v_3(inner) => inner.wire_size(),
}
}
}
"#
    );

    test_convert!(
        test_union_with_fallthrough_void,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
			case 3:
                void;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::v_1(inner) => inner.wire_size(),
Self::v_2 => 0,
Self::v_3 => 0,
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
				void;
			};
		"#,
        r#"impl WireSize for CB_GETATTR4res {
fn wire_size(&self) -> usize {
4 + match self {
Self::v_1(inner) => inner.wire_size(),
Self::default => 0,
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
Self::v_1(inner) => inner.wire_size(),
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
