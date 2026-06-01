---
cargo/satteri-pulldown-cmark: minor
cargo/satteri-ast: minor
cargo/satteri-plugin-api: minor
cargo/satteri-napi: minor
cargo/satteri: minor
---

Adds an `mdx` cargo feature (enabled by default) across the Rust crates. Disabling it compiles out all MDX support. In the future, this will be used to ship a "lite" version of Sätteri for environments where MDX is not needed and bundle size is a concern.

On Linux the native addon drops from ~2.99 MB to ~1.36 MB when disabling MDX.
