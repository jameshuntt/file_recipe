# file_recipe

Recipe-driven materialization of named file objects from raw bytes and
structured fields. No dependencies.

A `FileRecipe` says what a file object is made of: its inputs, its storage
class, the projection rules that turn fields into a readable view, and the
validation rules the result has to pass. Feed it bytes and fields and you get
a `MaterializedFileObject` you can read through a `ReadableView`, or a
`FileRecipeError` saying exactly which rule failed.

## Types

`FileRecipe`, `FileRecipeInput`, `StorageClass`, `ProjectionRule`,
`ValidationRuleSet`, `MaterializedFileObject`, `ReadableView`, `ReadMode`,
`ReadResult`, `FileRecipeError`.

## Tests

```bash
cargo test
```

## License

MIT or Apache-2.0, at your option.
