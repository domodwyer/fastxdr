use super::SafeName;
use crate::ast::{BasicType, Node};
use crate::indexes::{ConcreteType, GenericIndex, TypeIndex};
use crate::Result;
use std::collections::{BTreeMap, HashMap};

pub fn print_impl_from<'a, W: std::fmt::Write>(
    w: &mut W,
    item: &Node,
    generic_index: &GenericIndex<'a>,
    case_index: &BTreeMap<&str, String>,
    type_index: &TypeIndex<'a>,
) -> Result<()> {
    match item {
        Node::EOF => {}
        Node::Root(v) => {
            for field in v.iter() {
                print_impl_from(w, field, generic_index, case_index, type_index)?;
            }
        }
        Node::Struct(v) => print_try_from(w, v.name.as_str(), generic_index, |w| {
            writeln!(w, "Ok({} {{", v.name)?;
            for f in v.fields.iter() {
                write!(w, "{}: ", SafeName(&f.field_name))?;
                print_decode_field(w, f.field_value.unwrap_array(), type_index)?;
                writeln!(w, "?,")?;
            }
            writeln!(w, "}})")?;
            Ok(())
        })?,
        Node::Union(v) => print_try_from(w, v.name.as_str(), generic_index, |w| {
            write!(w, "let {} = ", v.switch.var_name)?;
            print_decode_field(w, &v.switch.var_type, type_index)?;
            writeln!(w, "?;")?;

            writeln!(w, "Ok(match {} {{", v.switch.var_name)?;
            for c in v.cases.iter() {
                // A single case statement may have many case values tied to it
                // if fallthrough values are used:
                //
                // 	case 1:
                // 	case 2:
                // 		// statement
                //
                for c_value in c.case_values.iter() {
                    // The case value may be a declaried constant or enum value.
                    //
                    // Lookup the value in the `case_index`.
                    let matcher = case_index.get(c_value.as_str()).unwrap_or(c_value);

                    write!(w, "{} => Self::{}(", SafeName(matcher), c.field_name)?;
                    print_decode_field(w, c.field_value.unwrap_array(), type_index)?;
                    writeln!(w, "?),")?;
                }
            }

            // There may also be several "void" cases
            for c in v.void_cases.iter() {
                let name = match c.as_str() {
                    "default" => "_",
                    v => &v,
                };
                writeln!(w, "{} => Self::Void,", name)?;
            }

            // Write a default case if present, else a catch all case that
            // returns an error.
            if let Some(ref d) = v.default {
                write!(w, "_ => Self::{}(", SafeName(&d.field_name))?;
                print_decode_field(w, d.field_value.unwrap_array(), type_index)?;
                writeln!(w, "?),")?;
            } else {
                writeln!(w, "d => return Err(Error::UnknownVariant(d)),")?;
            }

            writeln!(w, "}})")?;

            Ok(())
        })?,

        Node::Typedef(_) | Node::Constant(_) | Node::Enum(_) => return Ok(()),

        Node::Ident(_)
        | Node::Type(_)
        | Node::Option(_)
        | Node::UnionDefault(_)
        | Node::UnionCase(_)
        | Node::UnionDataField(_)
        | Node::UnionVoid
        | Node::StructDataField(_)
        | Node::Array(_)
        | Node::EnumVariant(_)
        | Node::ArrayVariable(_)
        | Node::ArrayFixed(_) => unreachable!(),
    };

    Ok(())
}

/// Prints the "impl TryFrom" block around the output of func.
///
/// `func` should write the body of the `try_from` implementation to `w`, using
/// `v` as the Bytes source.
fn print_try_from<'a, W: std::fmt::Write, F: FnOnce(&mut W) -> Result<()>>(
    w: &mut W,
    name: &str,
    generic_index: &GenericIndex,
    func: F,
) -> Result<()> {
    if generic_index.contains(name) {
        write!(w, "impl TryFrom<Bytes> for {}<Bytes>", name)?;
    } else {
        write!(w, r#"impl TryFrom<Bytes> for {}"#, name)?;
    }
    writeln!(
        w,
        r#" {{
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {{"#
    )?;

    func(w)?;

    writeln!(w, "}}\n}}")?;

    Ok(())
}

/// Generates the template required to decode `t` from a variable called `v`
/// that implements the reader trait.
///
/// If `t` is a typedef alias, the typedef chain is resolved to the underlying
/// type.
fn print_decode_field<'a, W: std::fmt::Write>(
    w: &mut W,
    t: &BasicType<'a>,
    type_index: &TypeIndex<'a>,
) -> Result<()> {
    match t {
        BasicType::U32 => write!(w, "v.try_u32()")?,
        BasicType::U64 => write!(w, "v.try_u64()")?,
        BasicType::I32 => write!(w, "v.try_i32()")?,
        BasicType::I64 => write!(w, "v.try_i64()")?,
        BasicType::F32 => write!(w, "v.try_f32()")?,
        BasicType::F64 => write!(w, "v.try_f64()")?,
        BasicType::String => write!(w, "v.try_string()")?,
        BasicType::Bool => write!(w, "v.try_bool()")?,
        BasicType::Opaque => write!(w, "v.try_bytes(None)")?,

        // An ident may refer to a typedef, or a compound type.
        BasicType::Ident(c) => match type_index.get_concrete(c) {
            Some(ConcreteType::Basic(ref b)) => return print_decode_field(w, b, type_index),
            Some(ConcreteType::Struct(s)) => write!(w, "{}::try_from(v)", s.name())?,
            Some(ConcreteType::Union(u)) => write!(w, "{}::try_from(v)", u.name())?,
            Some(ConcreteType::Enum(e)) => write!(w, "v.try_i32()")?,
            None => return Err(format!("unresolvable type {}", c.as_ref()).into()),
        },
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexes::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    macro_rules! test_convert {
        ($name: ident, $input: expr, $want: expr) => {
            #[test]
            fn $name() {
                let mut ast = XDRParser::parse(Rule::item, $input).unwrap();
                let ast = walk(ast.next().unwrap()).unwrap();
                let case_index = build_constant_index(&ast);
                let generic_index = build_generic_index(&ast);
                let type_index = TypeIndex::new(&ast);

                let mut got = String::new();
                print_impl_from(&mut got, &ast, &generic_index, &case_index, &type_index).unwrap();

                assert_eq!(got, $want);
            }
        };
    }

    // TODO: option struct

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
				opaque i;
			};
		"#,
        r#"impl TryFrom<Bytes> for small<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: v.try_u32()?,
b: v.try_u64()?,
c: v.try_i32()?,
d: v.try_i64()?,
e: v.try_f32()?,
f: v.try_f64()?,
g: v.try_string()?,
h: v.try_bool()?,
i: v.try_bytes(None)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_reserved_keyword,
        r#"
			struct small {
				unsigned int type;
			};
		"#,
        r#"impl TryFrom<Bytes> for small {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
type_v: v.try_u32()?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_struct,
        r#"
			struct other {
				u32 b;
			};
			struct small {
				other a;
			};
		"#,
        r#"impl TryFrom<Bytes> for other {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(other {
b: v.try_u32()?,
})
}
}
impl TryFrom<Bytes> for small {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: other::try_from(v)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_struct_generic,
        r#"
			struct other {
				opaque b;
			};
			struct small {
				other a;
			};
		"#,
        r#"impl TryFrom<Bytes> for other<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(other {
b: v.try_bytes(None)?,
})
}
}
impl TryFrom<Bytes> for small<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: other::try_from(v)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_typedef_to_struct,
        r#"
			typedef other alias;
			struct other {
				u32 b;
			};
			struct small {
				alias a;
			};
		"#,
        r#"impl TryFrom<Bytes> for other {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(other {
b: v.try_u32()?,
})
}
}
impl TryFrom<Bytes> for small {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: other::try_from(v)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_typedef_to_struct_generic,
        r#"
			typedef other alias;
			struct other {
				opaque b;
			};
			struct small {
				alias a;
			};
		"#,
        r#"impl TryFrom<Bytes> for other<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(other {
b: v.try_bytes(None)?,
})
}
}
impl TryFrom<Bytes> for small<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: other::try_from(v)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_typedef_to_basic_type,
        r#"
			typedef u32 alias;
			struct small {
				alias a;
			};
		"#,
        r#"impl TryFrom<Bytes> for small {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: v.try_u32()?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_typedef_to_basic_type_generic,
        r#"
			typedef opaque alias;
			struct small {
				alias a;
			};
		"#,
        r#"impl TryFrom<Bytes> for small<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: v.try_bytes(None)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_typedef_to_union,
        r#"
			typedef my_union alias;
			union my_union switch (unsigned int status) {
			case 1:
				u32       resok4;
			};
			struct small {
				alias a;
			};
		"#,
        r#"impl TryFrom<Bytes> for my_union {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
impl TryFrom<Bytes> for small {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: my_union::try_from(v)?,
})
}
}
"#
    );

    test_convert!(
        test_struct_nested_typedef_to_union_generic,
        r#"
			typedef my_union alias;
			union my_union switch (unsigned int status) {
			case 1:
				opaque       resok4;
			};
			struct small {
				alias a;
			};
		"#,
        r#"impl TryFrom<Bytes> for my_union<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_bytes(None)?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
impl TryFrom<Bytes> for small<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: my_union::try_from(v)?,
})
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
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::name(v.try_u64()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_constant_case_value,
        r#"
			const MODE4_SUID = 0x800;
			union CB_GETATTR4res switch (unsigned int status) {
			case MODE4_SUID:
				u32       resok4;
			case 2:
				u64       name;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
MODE4_SUID => Self::resok4(v.try_u32()?),
2 => Self::name(v.try_u64()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_enum_case_value,
        r#"
			enum Status{
				MODE4_SUID = 1,
				MODE4_OTHER = 2
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case MODE4_SUID:
				u32       resok4;
			case 2:
				u64       name;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
Status::MODE4_SUID => Self::resok4(v.try_u32()?),
2 => Self::name(v.try_u64()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_reserved_keyword_variant_ignored,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
				u64       type;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::type(v.try_u64()?),
d => return Err(Error::UnknownVariant(d)),
})
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
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::name(v.try_u64()?),
3 => Self::name(v.try_u64()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_default_case,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			default:
				u64       name;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
_ => Self::name(v.try_u64()?),
})
}
}
"#
    );

    test_convert!(
        test_union_default_case_with_fallthrough,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
			default:
				u64       name;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
_ => Self::name(v.try_u64()?),
})
}
}
"#
    );

    // This case isn't optimal - there's two wildcard branches but the first one
    // is the void, so this works fine and is simpler to generate.
    test_convert!(
        test_union_default_void,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			default:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
_ => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    // This case isn't optimal - there's two wildcard branches but the first one
    // is the void, so this works fine and is simpler to generate.
    test_convert!(
        test_union_default_void_with_fallthrough,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
			default:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::Void,
_ => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_void_case,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_void_case_with_fallthrough,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
			case 3:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::Void,
3 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_struct,
        r#"
			struct simple {
				u32 a;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				simple       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for simple {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(simple {
a: v.try_u32()?,
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(simple::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_union,
        r#"
			union my_union switch (unsigned int status) {
			case 1:
				u32       var;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				my_union       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for my_union {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::var(v.try_u32()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(my_union::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_union_generic,
        r#"
			union my_union switch (unsigned int status) {
			case 1:
				opaque       var;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				my_union       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for my_union<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::var(v.try_bytes(None)?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(my_union::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_typedef_to_union,
        r#"
			typedef my_union alias;
			union my_union switch (unsigned int status) {
			case 1:
				u32       var;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				alias       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for my_union {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::var(v.try_u32()?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(my_union::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_typedef_to_union_generic,
        r#"
			typedef my_union alias;
			union my_union switch (unsigned int status) {
			case 1:
				opaque       var;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				alias       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for my_union<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::var(v.try_bytes(None)?),
d => return Err(Error::UnknownVariant(d)),
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(my_union::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_typedef_to_struct,
        r#"
			typedef small alias;
			struct small {
				u32 a;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				alias       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for small {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: v.try_u32()?,
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(small::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_typedef_to_struct_generic,
        r#"
			typedef small alias;
			struct small {
				opaque a;
			};
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				alias       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for small<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
Ok(small {
a: v.try_bytes(None)?,
})
}
}
impl TryFrom<Bytes> for CB_GETATTR4res<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(small::try_from(v)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_typedef_to_basic_type,
        r#"
			typedef u32 alias;
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				alias       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_typedef_to_basic_type_generic,
        r#"
			typedef opaque alias;
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				alias       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_bytes(None)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_basic_type,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				u32       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_nested_basic_type_generic,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				opaque       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res<Bytes> {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_bytes(None)?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_switch_typedef,
        r#"
			typedef u32 alias;
			union CB_GETATTR4res switch (alias status) {
			case 1:
				u32       resok4;
			case 2:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for CB_GETATTR4res {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let status = v.try_u32()?;
Ok(match status {
1 => Self::resok4(v.try_u32()?),
2 => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    test_convert!(
        test_union_switch_enum,
        r#"
			enum time_how4 {
				SET_TO_SERVER_TIME4 = 0,
				SET_TO_CLIENT_TIME4 = 1
			};

			union settime4 switch (time_how4 set_it) {
			case SET_TO_CLIENT_TIME4:
				u32       time;
			default:
				void;
			};
		"#,
        r#"impl TryFrom<Bytes> for settime4 {
type Error = Error;

fn try_from(v: Bytes) -> Result<Self, Self::Error> {
let set_it = v.try_i32()?;
Ok(match set_it {
time_how4::SET_TO_CLIENT_TIME4 => Self::time(v.try_u32()?),
_ => Self::Void,
d => return Err(Error::UnknownVariant(d)),
})
}
}
"#
    );

    // TODO: fixed_array_const from enum
    // TODO: fixed_variable from enum
    // TODO: fixed_variable_max_known from enum
    // TODO: fixed_variable_max_const from enum
    //     test_convert!(
    //         test_fixed_array_known,
    //         r#"
    // 			struct small {
    // 				uint32_t a[3];
    // 			};
    // 		"#,
    //         r#"impl TryFrom<Bytes> for small {
    // 	type Error = Error;

    // 	fn try_from(v: Bytes) -> Result<Self, Self::Error> {
    // 		let a1 = v.try_u32();
    // 		let a2 = v.try_u32();
    // 		let a3 = v.try_u32();
    // 		small{a: [a1, a2, a3]}
    // 	}
    // }
    // "#
    //     );
}