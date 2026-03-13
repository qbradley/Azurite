# Porting Database — Azurite TS→Rust

This directory maintains per-file records of all decisions, transformations, substitutions, and notes for every source file in the TypeScript Azurite codebase.

## Purpose

When the TypeScript codebase receives bug fixes or new features, this database provides everything needed to propagate those changes to the Rust port with high fidelity.

## Structure

Each TS source file gets a corresponding `.md` file in this directory, mirroring the `src/` path structure.

Example: `src/blob/BlobServer.ts` → `porting-db/src/blob/BlobServer.md`

## Per-File Record Format

Each record should include:

### File Info
- **TS Source:** `src/path/to/file.ts`
- **Rust Target:** `rust/src/path/to/file.rs`
- **Status:** `not-started` | `analyzed` | `in-progress` | `ported` | `reviewed` | `approved`
- **Last TS Commit:** (hash of the TS commit this port is based on)

### Type Mappings
Table of TS types → Rust types used in this file.

### Function/Method Mappings
Table of TS functions → Rust functions, noting any structural changes.

### Decisions
Numbered list of decisions made during porting, with rationale.

### Transformations
Any non-trivial transformations required (e.g., callback → future, class hierarchy → trait).

### Dependencies
What this file imports/depends on, and the porting status of those dependencies.

### Notes for Future Changes
Anything a future porter needs to know when updating this file from TS changes.
