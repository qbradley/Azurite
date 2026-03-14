# Porting Record — `src/blob/context/BlobStorageContext.ts`

## File info
- Source path: `src/blob/context/BlobStorageContext.ts`
- Source lines: `73`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/context/blob_storage_context.rs`
- Crate: `azurite-blob`
- Module: `context::blob_storage_context`
- Phase: `6.5`
- Status: `ported`

## Exported API
### Default class `BlobStorageContext`
- Extends generated `Context`.
- Implements `IAuthenticationContext`.
- Methods / accessors:
  - `getContainer(): string | undefined`
  - `account` getter/setter
  - `isSecondary` getter/setter
  - `container` getter/setter
  - `blob` getter/setter
  - `authenticationPath` getter/setter
  - `xMsRequestID` getter/setter
  - `disableProductStyleUrl` getter/setter
  - `loose` getter/setter

## Dependencies
- Imports:
  - `../authentication/IAuthenticationContext` — Phase `7.2` minimal auth context interface.
  - `../generated/Context` — generated request context from Phase `5.7`.
- Key consumers:
  - `BlobSharedKeyAuthenticator.ts` and `BlobTokenAuthenticator.ts` wrap incoming `Context` values with this type immediately.
  - Blob middleware and request-listener code can use it as the blob-specific view over generated context state.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| subclass wrapper around generated `Context` | wrapper struct containing generated/base context | This file mostly exposes blob-specific convenience accessors. |
| delegated optional fields | `Option<String>` / `Option<bool>` | TS forwards `undefined` through unchanged. |
| `xMsRequestID` alias | accessor around `context_id` field | Preserve the alias instead of renaming every caller. |

## Special handling
- This class does **not** own separate storage fields. Every accessor reads and writes through `this.context.*` or `this.contextId` on the base generated context.
- `xMsRequestID` is an alias over `contextId`, not a second stored field.
- `getContainer()` coexists with the `container` property; both surface the same underlying `this.context.container`.
- `isSecondary`, `disableProductStyleUrl`, and `loose` are important for authentication behavior later in Phase 7 and Phase 12. Keep them visible in the Rust wrapper.
- There is no validation when setting any property. Rust should stay permissive at this layer and leave enforcement to authenticators and middlewares.

## Change propagation notes
- If generated `Context` changes its internal `context` shape or `contextId` field, this wrapper needs a coordinated update.
- If future TS adds more blob-only convenience accessors, prefer mirroring them here rather than teaching every authenticator to reach into raw generated context.
