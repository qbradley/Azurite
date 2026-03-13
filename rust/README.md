# Azurite Rust Workspace

This directory is the root of the Rust port workspace for Azurite. All Rust crates, build metadata, and porting records now live under `rust/`.

## Workspace Layout

```text
rust/
├── Cargo.toml
├── README.md
├── crates/
│   ├── azurite/
│   ├── azurite-common/
│   ├── azurite-blob/
│   ├── azurite-queue/
│   └── azurite-table/
└── porting-db/
    ├── README.md
    ├── STRATEGY.md
    └── PORTING-ORDER.md
```

## Crates

- `crates/azurite`: combined binary entry point mirroring `src/azurite.ts`
- `crates/azurite-common`: shared infrastructure mirroring `src/common/`
- `crates/azurite-blob`: blob service crate mirroring `src/blob/`
- `crates/azurite-queue`: queue service crate mirroring `src/queue/`
- `crates/azurite-table`: table service crate mirroring `src/table/`

## Building

From this directory:

```bash
cargo check
cargo test
```

Run a specific binary with Cargo package selection, for example:

```bash
cargo run -p azurite
cargo run -p azurite-blob --bin azurite-blob
```

## Porting Records

Porting strategy documents and per-file records live in `rust/porting-db/`. Future YAML records should mirror the TypeScript source tree under `rust/porting-db/src/`.
