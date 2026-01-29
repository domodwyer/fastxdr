use crate::ast::{indexes::*, ArrayType, Ast, BasicType};
use crate::Result;
use std::collections::HashMap;

pub fn print_serializers<W: std::fmt::Write>(w: &mut W, ast: &Ast) -> Result<()> {
    let mut typedef = HashMap::new();
    for item in ast.types().iter() {
        println!("{:?}", item);
        match item {
            AstType::Typedef(v) => {
                // map Typedefs to their target type
                typedef.insert(v.alias.unwrap_array().as_str(), v.target.as_str());
            }
            AstType::Enum(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // enum serializer body
                writeln!(w, "dst.put_u32(*self as u32);")?;
                writeln!(w, "}}\n}}")?;
            }
            AstType::Union(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // enum/union serializer body
                writeln!(w, "}}\n}}")?;
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
                            if typedef.contains_key(f.as_str()) {
                                writeln!(
                                    w,
                                    "dst.put_{}(&self.{});",
                                    typedef.get(f.as_str()).unwrap(),
                                    field.field_name
                                )?;
                            } else {
                                writeln!(w, "self.{}.serialize(dst);", field.field_name)?;
                            }
                        }
                        BasicType::U32 => {
                            writeln!(w, "dst.put_u32(&self.{});", field.field_name)?;
                        }
                        BasicType::I32 => {
                            writeln!(w, "dst.put_i32(&self.{});", field.field_name)?;
                        }
                        BasicType::U64 => {
                            writeln!(w, "dst.put_u64(&self.{});", field.field_name)?;
                        }
                        BasicType::I64 => {
                            writeln!(w, "dst.put_i64(&self.{});", field.field_name)?;
                        }
                        _ => {}
                    }
                }

                writeln!(w, "}}\n}}")?;
            }
            _ => {}
        }
    }
    Ok(())
}
