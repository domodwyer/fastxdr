# fastxdr

Transpiles XDR specifications into Rust code.

* Generates Rust types with fast, zero-copy deserialisation
* Customisable derives for generated types
* No panicking - returns generated `Error` variants for malformed data
* Use as part of a [`build.rs`] or generate with a standalone binary
* XDR unions mapped to Rust enums 1-to-1 for convince
* XDR typedefs produce distinct Rust types (not type aliases)
* Complies with [`rfc1014`] / [`rfc1832`] / [`rfc4506`] 

Types containing `opaque` bytes are generic over `AsRef<[u8]>` implementations,
and all types have `TryFrom<Bytes>` implemented for idiomatic, zero-copy
deserialisation (see [`Bytes`]). 

## Speed

Deserialising the wire protocol is very fast, usually under 1 microsecond. 

The examples below are NFS messages captured from a production network, of
typical size and complexity for the protocol:

```
setclientid/decode      time:   [520.50 ns 523.42 ns 526.61 ns]
                        thrpt:  [289.75 MiB/s 291.52 MiB/s 293.15 MiB/s]

mount/decode            time:   [448.00 ns 449.95 ns 451.89 ns]
                        thrpt:  [126.62 MiB/s 127.17 MiB/s 127.72 MiB/s]

lookup/decode           time:   [597.41 ns 599.71 ns 602.19 ns]
                        thrpt:  [228.05 MiB/s 228.99 MiB/s 229.88 MiB/s]
```

By avoiding the need to copy opaque bytes entirely, even XDR messages containing
large amounts of data typically deserialise in 1us or less on a modern CPU in
O(n) time and space.

## Library functionality

This crate can be used as a library to implement custom code generation, or
build XDR linters, etc.

The library tokenises the XDR specs using the [Pest] crate, constructs an
abstract syntax tree, indexes and resolves type aliases and constants and
generates code to calculate the on-wire size of the XDR serialised types - this
information is exposed to users through the `index` and `ast` modules.

## Usage

Then either generate the code as part of a build script (preferred), or manually
using the CLI.

To view the generated types, either export the generated types in your
application and use `cargo doc`, or use the CLI to produce the generated code
directly for reading.

### Build Script

To use it as part of a `build.rs` build script, first add `fastxdr` to the build
dependencies:

```toml
[build-dependencies]
fastxdr = "1.0"
```

Then create a `build.rs` file at the crate root (not in `src`):

```rust
fn main() {
    // Tell Cargo to regenerate the types if the XDR spec changes
    println!("cargo:rerun-if-changed=src/xdr_spec.x");

    // Read from xdr_spec.x, writing the generated code to out.rs
    std::fs::write(
        std::path::Path::new(std::env::var("OUT_DIR").unwrap().as_str()).join("out.rs"),
        fastxdr::Generator::default()
            .generate(include_str!("src/xdr_spec.x"))
            .unwrap(),
    )
    .unwrap();
}
```

And include the generated content somewhere in your application:

```rust 
// Where out.rs is the filename from above
include!(concat!(env!("OUT_DIR"), "/out.rs"));
use xdr::*;
```

The generated content is within a module named `xdr` which you may choose to
re-export if needed.

### CLI

You can also generate the code with the CLI:

```bash
cargo install fastxdr
fastxdr ./path/to/spec.x > generated.rs
```

You'll also have to depend on `fastxdr` in the project that consumes the
generated file in order to access `fastxdr::Bytes` and friends.

This can get confusing if the spec is modified and the code is not regenerated,
or the spec is not checked into source control and typically a `build.rs` script
is the best way to go.

## Orphan Rule

Because of the orphan rule it is not possible to implement `TryFrom` for types
defined outside of generated code such as `u32`, etc. This is normally fine,
except for relatively uncommon typedefs of variable length arrays containing
primitive types.

Therefore any typedefs to arrays of primitive types must be modified slightly -
this is not a breaking change and does not affect the serialised on-wire format:

```text
typedef uint32_t        bitmap4<>;
```

As the orphan rule prevents a `TryFrom` implementation being added to the `u32`
typedef target, wrap it in a typedef to generate a new type:

```text
typedef uint32_t        bitmap_inner;
typedef bitmap_inner    bitmap4<>;
```

The array now contains the `bitmap_inner` type that can have `TryFrom`
implemented for it.


[Pest]: https://github.com/pest-parser/pest
[PEG]: https://en.wikipedia.org/wiki/Parsing_expression_grammar
[`Bytes`]: https://docs.rs/bytes/0.5.6/bytes/struct.Bytes.html
[`build.rs`]: https://doc.rust-lang.org/cargo/reference/build-scripts.html
[`rfc1014`]: https://tools.ietf.org/html/rfc1014
[`rfc1832`]: https://tools.ietf.org/html/rfc1832
[`rfc4506`]: https://tools.ietf.org/html/rfc4506