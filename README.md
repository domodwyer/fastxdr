Transpiles XDR into Rust code

Builds an AST using the [Pest] crate, resolves type aliases and constants, and
marks applicable nodes as requiring a generic `AsRef<[u8]>` bound to support
opaque byte types

https://tools.ietf.org/html/rfc1832

[Pest]: https://github.com/pest-parser/pest
[PEG]: https://en.wikipedia.org/wiki/Parsing_expression_grammar

// TODO: array types
// TODO: fix visibility