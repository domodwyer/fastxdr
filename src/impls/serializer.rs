use crate::ast::{indexes::*, ArrayType, Ast, BasicType};
use crate::impls::SafeName;
use crate::Result;
use std::collections::HashMap;

pub fn print_serializers<W: std::fmt::Write>(w: &mut W, ast: &Ast) -> Result<()> {
    for primitive in [
        "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "u128", "i128",
    ] {
        writeln!(w,"impl Serialisable for {p} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_{p}(*self) }}\n}}", p=primitive)?;
    }
    for item in ast.types().iter() {
        match item {
            AstType::Typedef(v) => {
                // map Typedefs to their target type
                writeln!(w,"impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_{type}(self.0) }}\n}}", v.alias.unwrap_array().as_str(), type=v.target.as_str())?;
            }
            AstType::Enum(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // enum serializer body
                writeln!(w, "dst.put_u32(self.clone() as u32);")?;
                writeln!(w, "}}\n}}")?;
            }
            AstType::Union(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // enum/union serializer body
                // https://datatracker.ietf.org/doc/html/rfc1014#section-3.14

                //serialize discriminant (4 bytes)
                writeln!(w, "match self {{",)?;
                for c in v.cases.iter() {
                    for c_value in c.case_values.iter() {
                        let matcher = ast
                            .constants()
                            .get(c_value.as_str())
                            .map(|c| match *c {
                                ConstantType::ConstValue(ref v) => SafeName(v).to_string(),
                                ConstantType::EnumValue {
                                    ref enum_name,
                                    ref variant,
                                } => format!(
                                    "c if c == {}::{} as {}",
                                    enum_name, variant, v.switch.var_type,
                                ),
                            })
                            .unwrap_or_else(|| SafeName(c_value).to_string());

                        writeln!(
                            w,
                            "Self::{}(var) => {{dst.put_{}({} as {}); var.serialize(dst);}},",
                            SafeName(c_value),
                            SafeName(v.switch.var_type.as_str()),
                            matcher,
                            SafeName(v.switch.var_type.as_str())
                        )?;
                    }
                }
                writeln!(w, "_ => dst.put_i32(0)")?;
                writeln!(w, "}}\n}}")?;

                //serialize implied arm
                //writeln!(w, "if let Self::{}(var) = Self {{")?;
                //writeln!(w, "}}\n}}")?;

                writeln!(w, "}}")?;
            }
            AstType::Struct(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // struct serializer body
                for field in v.fields.iter() {
                    match field.field_value.unwrap_array() {
                        BasicType::Ident(f) => {
                            //if typedef.contains_key(f.as_str()) {
                            //    writeln!(
                            //        w,
                            //        "dst.put_{}(&self.{});",
                            //        typedef.get(f.as_str()).unwrap(),
                            //        field.field_name
                            //    )?;
                            //} else {
                            writeln!(w, "self.{}.serialize(dst);", field.field_name)?;
                            // }
                        }
                        _ => {
                            writeln!(w, "self.{}.serialize(dst);", field.field_name)?;
                        }
                    }
                }

                writeln!(w, "}}\n}}")?;
            }
        }
    }
    Ok(())
}
