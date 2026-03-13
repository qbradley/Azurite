# Porting Record — `src/blob/generated/handlers/IBlobHandler.ts`

## File info
- Source path: `src/blob/generated/handlers/IBlobHandler.ts`
- Source lines: `40`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/i_blob_handler.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::i_blob_handler`
- Phase: `5.14`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface IBlobHandler {
  download(options: Models.BlobDownloadOptionalParams, context: Context): Promise<Models.BlobDownloadResponse>;
  getProperties(options: Models.BlobGetPropertiesOptionalParams, context: Context): Promise<Models.BlobGetPropertiesResponse>;
  delete(options: Models.BlobDeleteMethodOptionalParams, context: Context): Promise<Models.BlobDeleteResponse>;
  undelete(options: Models.BlobUndeleteOptionalParams, context: Context): Promise<Models.BlobUndeleteResponse>;
  setExpiry(expiryOptions: Models.BlobExpiryOptions, options: Models.BlobSetExpiryOptionalParams, context: Context): Promise<Models.BlobSetExpiryResponse>;
  setHTTPHeaders(options: Models.BlobSetHTTPHeadersOptionalParams, context: Context): Promise<Models.BlobSetHTTPHeadersResponse>;
  setImmutabilityPolicy(options: Models.BlobSetImmutabilityPolicyOptionalParams, context: Context): Promise<Models.BlobSetImmutabilityPolicyResponse>;
  deleteImmutabilityPolicy(options: Models.BlobDeleteImmutabilityPolicyOptionalParams, context: Context): Promise<Models.BlobDeleteImmutabilityPolicyResponse>;
  setLegalHold(legalHold: boolean, options: Models.BlobSetLegalHoldOptionalParams, context: Context): Promise<Models.BlobSetLegalHoldResponse>;
  setMetadata(options: Models.BlobSetMetadataOptionalParams, context: Context): Promise<Models.BlobSetMetadataResponse>;
  acquireLease(options: Models.BlobAcquireLeaseOptionalParams, context: Context): Promise<Models.BlobAcquireLeaseResponse>;
  releaseLease(leaseId: string, options: Models.BlobReleaseLeaseOptionalParams, context: Context): Promise<Models.BlobReleaseLeaseResponse>;
  renewLease(leaseId: string, options: Models.BlobRenewLeaseOptionalParams, context: Context): Promise<Models.BlobRenewLeaseResponse>;
  changeLease(leaseId: string, proposedLeaseId: string, options: Models.BlobChangeLeaseOptionalParams, context: Context): Promise<Models.BlobChangeLeaseResponse>;
  breakLease(options: Models.BlobBreakLeaseOptionalParams, context: Context): Promise<Models.BlobBreakLeaseResponse>;
  createSnapshot(options: Models.BlobCreateSnapshotOptionalParams, context: Context): Promise<Models.BlobCreateSnapshotResponse>;
  startCopyFromURL(copySource: string, options: Models.BlobStartCopyFromURLOptionalParams, context: Context): Promise<Models.BlobStartCopyFromURLResponse>;
  copyFromURL(copySource: string, options: Models.BlobCopyFromURLOptionalParams, context: Context): Promise<Models.BlobCopyFromURLResponse>;
  abortCopyFromURL(copyId: string, options: Models.BlobAbortCopyFromURLOptionalParams, context: Context): Promise<Models.BlobAbortCopyFromURLResponse>;
  setTier(tier: Models.AccessTier, options: Models.BlobSetTierOptionalParams, context: Context): Promise<Models.BlobSetTierResponse>;
  getAccountInfo(context: Context): Promise<Models.BlobGetAccountInfoResponse>;
  query(options: Models.BlobQueryOptionalParams, context: Context): Promise<Models.BlobQueryResponse>;
  getTags(options: Models.BlobGetTagsOptionalParams, context: Context): Promise<Models.BlobGetTagsResponse>;
  setTags(options: Models.BlobSetTagsOptionalParams, context: Context): Promise<Models.BlobSetTagsResponse>;
}
```

## Dependencies
- Internal imports:
  - `../artifacts/models` → `src/blob/generated/artifacts/models.ts` — Phase 5 — analyzed in this pass
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `interface ...Handler` | `async trait ...Handler` | One Rust trait per handler group preserves autorest’s service/container/blob split. |
| `Promise<Models.*Response>` | `async fn -> Result<Models*Response, StorageError>` or direct response type | Keep the generated response wrapper types visible. |
| `NodeJS.ReadableStream` parameters | `dyn AsyncRead + Send` | Upload/download style methods must keep streaming inputs. |
| `Context` final argument | `&mut Context` / owned context wrapper | Generated middleware always appends context as the final argument. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Declares 24 operation methods for one generated handler family. The method order and argument order match `handlerMappers.ts` exactly.
- All methods return generated `Models.*Response` wrappers that encode headers/body/statusCode as the public contract.

## Middleware chain ordering
- Stage 3: Handler middleware uses `handlerMappers.ts` to pick one of these methods, extracts positional arguments from `context.handlerParameters`, then appends `context` as the last argument.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If any method signature changes, update `handlerMappers.ts`, `IHandlers.ts`, and the concrete Rust handler implementation traits together.
