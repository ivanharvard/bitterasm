# The `.em` format

`bitterasm compile` writes a program's emitted values to a `.em` file, and
an evaluator such as `bitter` reads it. This is the contract between them;
anything that reads `.em` should follow it.

A `.em` file is one JSON object:

```json
{
  "version": 1,
  "requires": ["sections", "extern-labels"],
  "module": "spec",
  "exports": { "start": 0 },
  "entries": [
    { "kind": "Struct", "id": "std.binary.bits",
      "args": [{ "kind": "Const", "value": "8" }],
      "fields": [["value", { "kind": "Int", "value": "7" }]],
      "section": ".text" },
    { "kind": "Enum", "id": "spec.Mode", "args": [], "variant": "Slow",
      "payload": { "kind": "Int", "value": "3" }, "section": ".text" },
    { "kind": "Deferred", "module": "spec_dep", "symbol": "far", "section": ".text" }
  ]
}
```

- **`version`** is `1`. It changes only when the file's structure changes
  incompatibly. A reader must refuse any version it doesn't know, and must
  refuse the unversioned plain-list files older compilers wrote.
- **`requires`** lists the language features the program actually uses. A
  reader must refuse a file that requires a feature it doesn't know,
  because ignoring one produces wrong output with no error:
  - `sections`: some entry has a `section`. Lay entries out grouped by
    section name, sections in order of first appearance, entries within a
    section in file order.
  - `extern-labels`: some value is a `Deferred` (see below), which only a
    linker with the other file's `.em` can resolve.
- **`module`** is the compiled file's module path: its path relative to the
  deepest search root containing it, with dots (`examples.x86_64.hello`).
  A file under no search root is named relative to the working directory,
  with one leading `.` per level up plus one (`..shared.util`).
- **`exports`** maps each top-level `pub` label to its position: how many
  entries precede it.
- **`entries`** is the emitted values, in emission order. Each is one of
  these, tagged by `kind`, plus an optional `section`:
  - `Int`: `value` is a decimal string, since integers are unbounded.
  - `Struct`: `id`, generic `args`, and `fields` as `[name, value]` pairs
    in declaration order.
  - `Enum`: `id`, generic `args`, `variant`, and an optional `payload`.
  - `Deferred`: the value of `pub` label `symbol` in the file whose
    `module` is given, not known until link time.

A generic argument is `{"kind": "Const", "value": "8"}` or
`{"kind": "Type", "type_kind": ..., ...}`, where the type is
`{"type_kind": "Builtin", "name": "int"}`, or `Struct`/`Enum` with an `id`
and its own `args`.

**Ids.** A struct or enum's `id` is its declaring module's path plus its
name: `std.binary.bits`, `spec.Mode`. Ids are unique within one program, and
they're what an evaluator matches on. Which ids an evaluator gives meaning
to is up to that evaluator. `bitter` understands `std.binary.bits`,
`std.bitter.byte_order.LittleEndian`, and `std.bitter.deferred`'s
`Positioned`, `Deferred`, `BinOp` and `Op`, and packs any other struct as
the concatenation of its fields.

A later version-1 file may add top-level fields that a reader can safely
ignore. Anything a reader must understand to produce correct output is
either a new `requires` feature or a new `version`.

