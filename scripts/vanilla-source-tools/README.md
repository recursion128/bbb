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

The stable developer commands are `scripts/dev lint python` and
`scripts/dev test python --suite unit|integration|vanilla|all`. Tests are
collected by pytest; Ruff owns formatting-independent Python lint. Vanilla
inputs are resolved only through `scripts/vanilla path 26.1` and are prepared
separately, never by a test or generator.

The shared Java frontend derives source structure only. Wire codecs, world
state transitions, client tick/physics/animation/rendering, persistent
Codec/SNBT semantics, and runtime fallback remain handwritten in their owning
Rust modules.
