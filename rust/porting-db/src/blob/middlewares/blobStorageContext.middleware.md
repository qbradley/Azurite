# Porting Record — `src/blob/middlewares/blobStorageContext.middleware.ts`

## File info
- Source path: `src/blob/middlewares/blobStorageContext.middleware.ts`
- Source lines: `294`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/middlewares/blob_storage_context.rs`
- Crate: `azurite-blob`
- Module: `middlewares::blob_storage_context`
- Phase: `12.3`
- Status: `not_started`

## Exported API
### Middleware helpers
- Default export `createStorageBlobContextMiddleware()`
- `internalBlobStorageContextMiddleware()` for batch/subrequest execution
- `blobStorageContextMiddleware()` for Express requests
- `extractStoragePartsFromPath()`

## Dependencies
- Express `RequestHandler` / `NextFunction`
- `uuid/v4`
- Common constants `IP_REGEX`, `NO_ACCOUNT_HOST_NAMES`
- `BlobStorageContext`, `StorageErrorFactory`, generated `IRequest` / `IResponse`
- Blob constants (`DEFAULT_CONTEXT_PATH`, `HeaderConstants`, `SECONDARY_SUFFIX`, `ValidAPIVersions`, `VERSION`)
- Utility helpers `checkApiVersion()` and `validateContainerName()`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| Express middleware mutating `res.locals`-backed context | axum/tower extractor or per-request state initializer | Preserve the exact fields written before dispatch/authentication. |
| shared parsing helper for live and batch requests | pure helper fn over `(host, path, flags)` | Both middleware entrypoints depend on identical parsing. |
| `uuid()` request IDs | UUID v4 generator | Request ID assignment happens before any handler logic. |

## Special handling
- `blobStorageContext.middleware.ts:55-70` and `153-169` always set the `Server` header to `Azurite-Blob/${VERSION}` before anything else.
- `blobStorageContext.middleware.ts:59-64`, `157-162` optionally enforce API-version allowlisting based on `x-ms-version`.
- `blobStorageContext.middleware.ts:72-77` logs `ClientIP=${req.getEndpoint()}` and the literal text `HTTPVersion=version` in the internal batch/subrequest variant; preserve that logging quirk.
- `blobStorageContext.middleware.ts:94-99`, `197-201` set `dispatchPattern` to `/`, `/container`, or `/container/blob` based only on parsed container/blob presence.
- `blobStorageContext.middleware.ts:100-109`, `203-212` build `authenticationPath` from the incoming path and strip `-secondary` when present.
- `blobStorageContext.middleware.ts:111-121`, `214-224` treat a missing account as `InvalidQueryParameterValue`.
- `blobStorageContext.middleware.ts:124-128`, `227-230` validate container names only for non-system containers (names not starting with `$`).
- `blobStorageContext.middleware.ts:261-286` decodes the full path, chooses the account from the host when product-style URLs are enabled and the host is not an IP/known local host, then joins remaining path segments into the blob name while replacing backslashes with forward slashes.
- `blobStorageContext.middleware.ts:288-290` strips `SECONDARY_SUFFIX` from the parsed account name and separately sets `isSecondary = true`.

## Change propagation notes
- This file is the root of blob request parsing for both live requests and batch subrequests. Any change here cascades into authentication, dispatch, and handler routing.
- Preserve the current hostname-vs-path account parsing asymmetry; future TS changes may refine it further, and Aragorn needs a mechanically similar structure to diff against.
