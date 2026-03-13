---
name: "rust-workspace-scaffold"
description: "Scaffold and verify the Azurite Rust workspace under rust/"
domain: "rust-port"
confidence: "high"
source: "Aragorn 2026-03-13"
---

## Context

Use this pattern when the Rust port needs an initial workspace scaffold or when the workspace layout needs to be re-rooted under `rust/` while preserving the TypeScript-to-Rust correspondence.

## Patterns

### Workspace Root

- Put the workspace manifest at `rust/Cargo.toml`.
- Use `resolver = "2"`.
- Keep shared package metadata and dependency versions in `[workspace.package]` and `[workspace.dependencies]`.
- Register five members: `azurite`, `azurite-common`, `azurite-blob`, `azurite-queue`, and `azurite-table`.

### Crate Layout

- `rust/crates/azurite` is the combined binary crate mirroring `src/azurite.ts`.
- `rust/crates/azurite-common` is the shared library crate mirroring `src/common/`.
- `rust/crates/azurite-blob`, `rust/crates/azurite-queue`, and `rust/crates/azurite-table` should each include `src/lib.rs` plus `src/main.rs` so the service entry points stay explicit.
- Keep top-level module filenames close to their TypeScript counterparts (`blob_server.rs`, `queue_server.rs`, `table_server.rs`, etc.).

### Porting Database Placement

- Keep strategy docs and per-file records in `rust/porting-db/`.
- Update examples and record targets to point to `rust/porting-db/src/...yaml` and `rust/crates/...`.
- Remove stale root-level `porting-db/` files after the move is complete.

### Verification

- Run `cargo check` from `rust/` after scaffolding or moving files.
- Mark Phase 0 scaffolding tasks complete in `rust/porting-db/PORTING-ORDER.md` when the workspace is in place.

## Examples

```text
rust/
├── Cargo.toml
├── crates/
│   ├── azurite/
│   ├── azurite-common/
│   ├── azurite-blob/
│   ├── azurite-queue/
│   └── azurite-table/
└── porting-db/
```

## Anti-Patterns

- **Root-level Rust artifacts** — Do not leave Rust manifests or porting-db files at repository root.
- **Per-crate version drift** — Do not repeat shared dependency versions in every crate when the workspace root can own them.
- **Idiomatic restructuring during scaffolding** — Do not invent a more clever crate layout that breaks 1:1 correspondence with the TypeScript services.
