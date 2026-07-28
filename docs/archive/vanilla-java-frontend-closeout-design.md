# Vanilla Java Generator Frontend Closeout

## Goal And Completion

Close the remaining generator-frontend migration inventory after a mechanical
audit of every production Python path that reads Vanilla Java source.

Completion requires:

- every production script that declares a `.java` input and reads source bytes
  to import and call `vanilla_java_source.parse_source` or `parse_sources`;
- repo-owned Java runtime probes to remain a distinct compile-and-execute
  boundary rather than being mistaken for source parsers;
- a source-ownership test to derive these consumers from the checked-in scripts
  instead of maintaining a second generator list;
- all Java-derived checked-in catalogs to match local Vanilla 26.1 sources; and
- the generated semantic-owner reverse guards to keep rejecting restored
  cross-crate numeric ids, ordinal tables, renderer memberships, and forwarding
  lookup bridges.

## Non-goals

- No catalog schema, semantic extraction, generated Rust, packet, registry, or
  runtime behavior changes.
- No migration of `scripts/vanilla-runtime/*.java`: those programs execute
  against the official client jar and emit bounded reports; they do not parse
  Vanilla Java source.
- No build-time dependency on `~/Work/mc-code`. Generated outputs remain
  checked in, and ordinary workspace builds continue to use only repo files.
- No replacement for domain recognition over the frontend's typed CST/IR.

## Ownership And Audit Contract

- `scripts/vanilla_java_source.py` remains the only Java lexer, CST parser,
  delimiter recovery, declaration/call IR, source-span, and hierarchy owner.
- Domain extractors may inspect typed nodes and canonical expressions. They may
  not read raw Java and reconstruct comments, tokens, braces, calls, fields, or
  declarations with regexes or private scanners.
- The meta-test scans production `scripts/*.py` with Python's AST. A module
  declaring Java input and reading source bytes must both import and call the
  shared parser. Generators that consume a domain extractor's typed result do
  not become a second source reader merely because they read their checked-in
  output for `--check`.
- The three checked-in runtime-report generators are classified by their
  `scripts/vanilla-runtime` source and must invoke `subprocess`; this is the
  only separate Java boundary found by the audit.
- Generator-emitted Rust reverse guards remain with each semantic catalog
  owner. The closeout does not centralize or duplicate those domain rules.

## Audit Evidence

The pre-start audit found 18 direct production Java-source consumers, covering packet,
core/gameplay ids, block/item registries and memberships, entity metadata and
renderers, bootstrap registries, command arguments, creative tabs,
environment attributes, furnace fuels, biomes, jukebox songs, and sound
catalogs. Every one already calls the shared frontend.

The only production scripts with Java input but without that frontend are the
three runtime-report generators for block placement defaults, block-state
fluid replaceability, and block-state support. Each compiles a repo-owned Java
probe and parses its report, so migration would cross rather than strengthen
the ownership boundary.

## Ordered Stages And Verification

1. Add the derived source-ownership regression test and run its focused unit
   test.
2. Run every Java-derived catalog generator with `--check` against local
   Vanilla 26.1 sources; any mismatch localizes to its semantic owner.
3. Remove the closed TODO inventory, run all script tests and per-commit
   checks, review the complete diff, and commit.
4. Because this changes generator enforcement, run the full high-risk gate at
   the committed checkpoint. A failure is fixed forward before another slice.
