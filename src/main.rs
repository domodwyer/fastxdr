use fastxdr::Generator;
use fastxdr::Result;
use std::env;

fn main() -> Result<()> {
    if env::args().len() < 2 {
        println!("usage: {} ./path/to/spec.x", env::args().nth(0).unwrap());
        std::process::exit(1);
    }

    for e in env::args().skip(1) {
        let xdr = std::fs::read_to_string(e)?;
        let code = Generator::default().generate(&xdr)?;
        println!("{}", code);
    }

    Ok(())
}
