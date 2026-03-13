# Porting Record — `src/blob/generated/handlers/IContainerHandler.ts`

## File info
- Source path: `src/blob/generated/handlers/IContainerHandler.ts`
- Source lines: `34`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/i_container_handler.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::i_container_handler`
- Phase: `5.14`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface IContainerHandler {
  create(options: Models.ContainerCreateOptionalParams, context: Context): Promise<Models.ContainerCreateResponse>;
  getProperties(options: Models.ContainerGetPropertiesOptionalParams, context: Context): Promise<Models.ContainerGetPropertiesResponse>;
  getPropertiesWithHead(options: Models.ContainerGetPropertiesWithHeadOptionalParams, context: Context): Promise<Models.ContainerGetPropertiesWithHeadResponse>;
  delete(options: Models.ContainerDeleteMethodOptionalParams, context: Context): Promise<Models.ContainerDeleteResponse>;
  setMetadata(options: Models.ContainerSetMetadataOptionalParams, context: Context): Promise<Models.ContainerSetMetadataResponse>;
  getAccessPolicy(options: Models.ContainerGetAccessPolicyOptionalParams, context: Context): Promise<Models.ContainerGetAccessPolicyResponse>;
  setAccessPolicy(options: Models.ContainerSetAccessPolicyOptionalParams, context: Context): Promise<Models.ContainerSetAccessPolicyResponse>;
  restore(options: Models.ContainerRestoreOptionalParams, context: Context): Promise<Models.ContainerRestoreResponse>;
  submitBatch(body: NodeJS.ReadableStream, contentLength: number, multipartContentType: string, options: Models.ContainerSubmitBatchOptionalParams, context: Context): Promise<Models.ContainerSubmitBatchResponse>;
  filterBlobs(options: Models.ContainerFilterBlobsOptionalParams, context: Context): Promise<Models.ContainerFilterBlobsResponse>;
  acquireLease(options: Models.ContainerAcquireLeaseOptionalParams, context: Context): Promise<Models.ContainerAcquireLeaseResponse>;
  releaseLease(leaseId: string, options: Models.ContainerReleaseLeaseOptionalParams, context: Context): Promise<Models.ContainerReleaseLeaseResponse>;
  renewLease(leaseId: string, options: Models.ContainerRenewLeaseOptionalParams, context: Context): Promise<Models.ContainerRenewLeaseResponse>;
  breakLease(options: Models.ContainerBreakLeaseOptionalParams, context: Context): Promise<Models.ContainerBreakLeaseResponse>;
  changeLease(leaseId: string, proposedLeaseId: string, options: Models.ContainerChangeLeaseOptionalParams, context: Context): Promise<Models.ContainerChangeLeaseResponse>;
  listBlobFlatSegment(options: Models.ContainerListBlobFlatSegmentOptionalParams, context: Context): Promise<Models.ContainerListBlobFlatSegmentResponse>;
  listBlobHierarchySegment(delimiter: string, options: Models.ContainerListBlobHierarchySegmentOptionalParams, context: Context): Promise<Models.ContainerListBlobHierarchySegmentResponse>;
  getAccountInfo(context: Context): Promise<Models.ContainerGetAccountInfoResponse>;
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
- Declares 18 operation methods for one generated handler family. The method order and argument order match `handlerMappers.ts` exactly.
- All methods return generated `Models.*Response` wrappers that encode headers/body/statusCode as the public contract.

## Middleware chain ordering
- Stage 3: Handler middleware uses `handlerMappers.ts` to pick one of these methods, extracts positional arguments from `context.handlerParameters`, then appends `context` as the last argument.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If any method signature changes, update `handlerMappers.ts`, `IHandlers.ts`, and the concrete Rust handler implementation traits together.
