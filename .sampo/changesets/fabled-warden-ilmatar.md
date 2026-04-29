---
npm/satteri: patch
---

Added visitor support for `containerDirective`, `leafDirective`, and `textDirective` nodes. Plugin authors can now subscribe to directive nodes directly (with typed `name`/`attributes` and children).

Removed the `root` visitor key. Plugins should subscribe to specific node types instead; a dedicated API for prepending or appending content at the document level will land separately.
