# Porting Record — `src/blob/authentication/IAuthenticationContext.ts`

## File info
- Source path: `src/blob/authentication/IAuthenticationContext.ts`
- Source lines: `3`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/i_authentication_context.rs`
- Crate: `azurite-blob`
- Module: `authentication::i_authentication_context`
- Phase: `7.2`
- Status: `ported`

## Exported API
### Default interface `IAuthenticationContext`
- Optional property:
  - `account?: string`

## Dependencies
- Imports: none.
- Key consumers:
  - `BlobStorageContext.ts` implements it and expands it with blob-specific accessors.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| one-field interface | small struct or trait with `account()` accessor | Keep it minimal; TS uses it as a marker-level contract. |
| optional string | `Option<String>` | `undefined` is a valid state. |

## Special handling
- This file is intentionally tiny; most real authentication context state lives in `BlobStorageContext` and generated `Context`.
- Avoid over-engineering the Rust translation here. The porting fidelity value is simply that `account` is optional at the interface boundary.

## Change propagation notes
- If TS adds more fields to the auth context, update this record first and then audit `BlobStorageContext` plus any code that currently reaches through `context.context` directly.
