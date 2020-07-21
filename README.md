Transpiles XDR into Rust code

Builds an AST using the [Pest] crate, resolves type aliases and constants, and
marks applicable nodes as requiring a generic `AsRef<[u8]>` bound to support
opaque byte types

https://tools.ietf.org/html/rfc1832

[Pest]: https://github.com/pest-parser/pest
[PEG]: https://en.wikipedia.org/wiki/Parsing_expression_grammar

// TODO: fix visibility



----
Requires minor edits. Does not affect wire protocol.

Arrays of primitive types cannot be decoded directly.

```
typedef uint32_t        bitmap4<>;
```

As the orphan rule prevents a `TryFrom` implementation being added to the `u32`
target.

```
typedef uint32_t        bitmap_inner;
typedef bitmap_inner    bitmap4<>;
```