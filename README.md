# Sätteri

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/bruits/satteri?utm_source=badge)

High-performance Markdown and MDX processing. Powered by a Rust arena backend with zero-copy binary transfer to JavaScript, and a plugin system for custom transformations.

Don't know where to start? Check out our npm package's [ documentation](./packages/satteri/README.md).

## Packages

Sätteri is a Rust + TypeScript monorepo, that contains the following crates (Rust packages):

| Name                    | Description                                                                            | Registry | README                                             |
| ----------------------- | -------------------------------------------------------------------------------------- | -------- | -------------------------------------------------- |
| `satteri-mdast`         | Arena-allocated MDAST with zero-copy string references and binary buffer format        | _WIP_    | [README](./crates/mdast/README.md)                 |
| `parser`                | Bridges pulldown-cmark events into an `MdastArena` (tables, footnotes, math, MDX, …)   | _WIP_    | [README](./crates/parser/README.md)                |
| `satteri-hast`          | MDAST → HAST conversion and HTML serialization, with binary HAST buffer support        | _WIP_    | [README](./crates/hast/README.md)                  |
| `satteri-plugin-api`    | Rust `Plugin` trait, typed visitors, `PluginRunner`, and command/patch mutation system | _WIP_    | [README](./crates/plugin-api/README.md)            |
| `satteri-napi`          | NAPI bindings exposing the Rust pipeline to Node.js                                    | _WIP_    | [README](./crates/napi-binding/README.md)          |
| `mdxjs`                 | MDX → JavaScript compiler — fork of [mdxjs-rs] adapted for pulldown-cmark and [OXC]    | _WIP_    | [README](./crates/mdxjs-rs/readme.md)              |
| `pulldown-cmark`        | Vendored CommonMark parser with MDX extension support                                  | _WIP_    | _—_                                                |
| `pulldown-cmark-escape` | Vendored HTML escape utilities from the pulldown-cmark project                         | _WIP_    | [README](./crates/pulldown-cmark-escape/README.md) |
| `satteri-bench`         | Benchmarks and profiling harnesses for the pipeline                                    | _WIP_    | [README](./crates/bench/README.md)                 |

Sätteri also includes the following npm package:

| Name             | Description                                                                                           | Registry | README                                 |
| ---------------- | ----------------------------------------------------------------------------------------------------- | -------- | -------------------------------------- |
| [`satteri`][npm] | TypeScript layer: binary buffer readers, visitor pattern, plugin API, and top-level compile functions | _WIP_    | [README](./packages/satteri/README.md) |

## Acknowledgements

Sätteri builds on the work of several open-source projects:

- [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) — CommonMark pull parser, vendored here with MDX extensions.
- [mdxjs-rs] — original MDX compiler by Titus Wormer, forked to use pulldown-cmark and OXC.
- [OXC] — fast JavaScript/TypeScript toolchain used for AST manipulation and code generation.
- [NAPI-RS] — Rust ↔ Node.js bridge that makes zero-copy binary transfer possible.

Sätteri is an open-source project born from [Bruits](https://bruits.org/), a Rust-focused collective 💛

[mdxjs-rs]: https://github.com/wooorm/mdxjs-rs
[OXC]: https://oxc.rs/
[NAPI-RS]: https://napi.rs/
[npm]: https://www.npmjs.com/package/satteri
