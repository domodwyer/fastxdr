use crate::ast::{indexes::*, Ast, BasicType};
use crate::impls::SafeName;
use crate::Result;

pub fn print_serializers<W: std::fmt::Write>(w: &mut W, ast: &Ast) -> Result<()> {
    for primitive in [
        "u8", "i8", "u16", "i16", "f32", "u32", "i32", "f64", "u64", "i64", "u128", "i128",
    ] {
        writeln!(w,"impl Serialisable for {p} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_{p}(*self) }}\n}}", p=primitive)?;
    }
    // bool
    writeln!(w,"impl Serialisable for bool {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(*self as u32) }}\n}}")?;
    // String
    writeln!(w,"impl Serialisable for String {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.len() as u32); dst.put_slice(self.as_bytes()) }}\n}}")?;
    // serialize variable-length vectors
    writeln!(w,"impl<T> Serialisable for Vec<T> where T: Serialisable {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.len() as u32); let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} }}\n}}")?;
    // serialize fixed-length vectors
    writeln!(w,"impl<T> Serialisable for [T] where T: Serialisable {{\nfn serialize(&self, dst: &mut BytesMut) {{ let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} }}\n}}")?;
    // serialize Option<T>
    writeln!(w,"impl<T> Serialisable for Option<Box<T>> where T: Serialisable  {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.is_some() as u32); if self.is_some() {{ self.serialize(dst) }} }}\n}}")?;
    writeln!(w,"impl Serialisable for Bytes  {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_slice(self.as_ref()) }}\n}}")?;

    for item in ast.types().iter() {
        match item {
            AstType::Typedef(v) => {
                // map Typedefs to their target type
                match v.target {
                    BasicType::Ident(_) => {
                        if ast.generics().contains(v.alias.unwrap_array().as_str()) {
                            writeln!(w,"impl<T> Serialisable for {}<T> where T: Serialisable + AsRef<[u8]> + Debug {{\nfn serialize(&self, dst: &mut BytesMut) {{ self.0.serialize(dst) }}\n}}", v.alias.unwrap_array().as_str())?;
                        }
                        else {
                            writeln!(w,"impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{ self.0.serialize(dst) }}\n}}", v.alias.unwrap_array().as_str())?;
                        }
                    }
                    BasicType::Opaque => {
                        writeln!(w,"impl<T> Serialisable for {}<T> where T: AsRef<[u8]> + Debug {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_slice(self.0.as_ref()) }}\n}}", v.alias.unwrap_array().as_str())?;
                    }
                    _ => {
                        if v.target.as_str() == "bool" {
                            writeln!(w,"impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.0 as u32) }}\n}}", v.alias.unwrap_array().as_str())?;
                        }
                        else if v.target.as_str() == "String" {
                            writeln!(w,"impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.0.len() as u32); dst.put_slice(self.0.as_bytes()) }}\n}}", v.alias.unwrap_array().as_str())?;
                        }
                        else {
                            writeln!(w,"impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_{type}(self.0) }}\n}}", v.alias.unwrap_array().as_str(), type=v.target.as_str())?;
                        }
                        
                    }
                }
            }
            AstType::Enum(v) => {
                if ast.generics().contains(v.name.as_str()) {
                    writeln!(
                        w,
                        "impl<T> Serialisable for {}<T> where T: Serialisable + AsRef<[u8]> + Debug {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                        v.name
                    )?;
                } else {
                    writeln!(
                        w,
                        "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                        v.name
                    )?;
                }
                // enum serializer body
                writeln!(w, "dst.put_u32(self.clone() as u32);")?;
                writeln!(w, "}}\n}}")?;
            }
            AstType::Union(v) => {
                if ast.generics().contains(v.name.as_str()) {
                    writeln!(
                        w,
                        "impl<T> Serialisable for {}<T> where T: Serialisable + AsRef<[u8]> + Debug {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                        v.name
                    )?;
                } else {
                    writeln!(
                        w,
                        "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                        v.name
                    )?;
                }

                // enum/union serializer body
                // https://datatracker.ietf.org/doc/html/rfc1014#section-3.14

                writeln!(w, "let n_start = dst.len();")?;
                //serialize discriminant (4 bytes)
                writeln!(w, "match self {{",)?;
                for c in v.cases.iter() {
                    for c_value in c.case_values.iter() {
                        let matcher = ast.types().get(v.name.as_str());
                        
                        println!("{:?}", matcher);
                        writeln!(
                            w,
                            "Self::{}(var) => {{dst.put_u32(Self::{} as u32); var.serialize(dst);}},",
                            c_value,
                            c_value,
                        )?;
                    }
                }
                writeln!(w, "_ => dst.put_i32(0)")?;
                writeln!(w, "}}")?;

                writeln!(
                    w,
                    "for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} \n}}"
                )?;

                writeln!(w, "}}")?;
            }
            AstType::Struct(v) => {
                if ast.generics().contains(v.name.as_str()) {
                    writeln!(
                        w,
                        "impl<T> Serialisable for {}<T> where T: Serialisable + AsRef<[u8]> + Debug {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                        v.name
                    )?;
                } else {
                    writeln!(
                        w,
                        "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                        v.name
                    )?;
                }

                // struct serializer body
                writeln!(w, "let n_start = dst.len();")?;
                for field in v.fields.iter() {
                    match field.field_value.unwrap_array() {
                        BasicType::Ident(f) => {
                            writeln!(
                                w,
                                "self.{}.serialize(dst);",
                                SafeName(field.field_name.as_str())
                            )?;
                        }
                        _ => {
                            writeln!(
                                w,
                                "self.{}.serialize(dst);",
                                SafeName(field.field_name.as_str())
                            )?;
                        }
                    }
                }
                writeln!(
                    w,
                    "for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} "
                )?;
                writeln!(w, "}}\n}}")?;
            }
        }
    }
    Ok(())
}
