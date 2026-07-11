# Vanilla source maintenance tools

The lockfile pins the Python Tree-sitter runtime and Java grammar used by
vanilla-source extractors. Prepare the ignored local environment once with:

```sh
uv sync --frozen --project scripts/vanilla-source-tools --python 3.12
```

Maintenance commands run through `scripts/vanilla-source-tools.sh`, which uses
`uv run --offline --frozen`. It never resolves or downloads dependencies and
fails explicitly when the locked environment/cache is unavailable. Ordinary
Cargo build/test commands do not invoke this environment or read the external
vanilla source tree.

The shared Java frontend derives source structure only. Wire codecs, world
state transitions, client tick/physics/animation/rendering, persistent
Codec/SNBT semantics, and runtime fallback remain handwritten in their owning
Rust modules.
