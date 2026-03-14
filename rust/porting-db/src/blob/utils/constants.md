# Porting Record — `src/blob/utils/constants.ts`

## File info
- Source path: `src/blob/utils/constants.ts`
- Source lines: `178`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/utils/constants.rs`
- Crate: `azurite-blob`
- Module: `utils::constants`
- Phase: `12.1`
- Status: `not_started`

## Exported API
### Module constants
- Service identity/version: `VERSION`, `BLOB_API_VERSION`
- Defaults: listening host/port, keep-alive timeout, DB paths, persistence path, debug/access log paths, GC interval
- Emulator account constants and bearer-token audience regexes
- Header/method constants used throughout handlers and middleware
- Batch protocol constants: `HTTP_LINE_ENDING`, `HTTP_HEADER_DELIMITER`
- Append-blob limits and user-delegation-key seed

## Dependencies
- `StoreDestinationArray` from common persistence.
- Generated blob `Models` for `SkuName`, `AccountKind`, and related enums.
- Read across handlers, middleware, auth, server bootstrap, and tests.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| mutable exported object literals (`HeaderConstants`, `MethodConstants`) | static structs / const maps / enums | Preserve the exact string spellings used in request lookup. |
| mutable default persistence array | shared mutable configuration seed | `BlobServerFactory` mutates this value at runtime. |
| regex literal arrays | compiled regex set | Audience validation uses these exact patterns. |

## Special handling
- `constants.ts:4-5` pins blob API surface to `3.35.0` / `2025-11-05`; later phases read these values directly into responses.
- `constants.ts:37-85` mixes lowercase and title-case header names intentionally (`authorization` vs `Server` / `Vary` / `Range`). Preserve spellings exactly.
- `constants.ts:91` defines the secondary-account suffix `-secondary`, which middleware later strips from account names and auth paths.
- `constants.ts:93-99` exports `DEFAULT_BLOB_PERSISTENCE_ARRAY` as a mutable singleton array. `BlobServerFactory.ts:35-38` mutates `locationPath` in place.
- `constants.ts:101-147` hard-codes the full accepted API-version allowlist in reverse chronological order.
- `constants.ts:164-170` encodes accepted bearer-token audiences as regexes rather than normalized host parsing.
- `constants.ts:176` hard-codes the base HMAC key used for synthetic user delegation keys.

## Change propagation notes
- Treat this file as a contract surface, not just convenience constants. Header spelling, mutable defaults, and version lists all affect observable behavior.
- Any TS change here should trigger a sweep across handlers, middleware, auth, and bootstrap code because they import these literals directly.
