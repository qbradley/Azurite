# Porting Record — `src/blob/generated/artifacts/operation.ts`

## File info
- Source path: `src/blob/generated/artifacts/operation.ts`
- Source lines: `83`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/artifacts/operation.rs`
- Crate: `azurite-blob`
- Module: `generated::artifacts::operation`
- Phase: `5.11`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export enum Operation {
  Service_SetProperties,
  Service_GetProperties,
  Service_GetStatistics,
  Service_ListContainersSegment,
  Service_GetUserDelegationKey,
  Service_GetAccountInfo,
  Service_GetAccountInfoWithHead,
  Service_SubmitBatch,
  Service_FilterBlobs,
  Container_Create,
  Container_GetProperties,
  Container_GetPropertiesWithHead,
  Container_Delete,
  Container_SetMetadata,
  Container_GetAccessPolicy,
  Container_SetAccessPolicy,
  Container_Restore,
  Container_SubmitBatch,
  Container_FilterBlobs,
  Container_AcquireLease,
  Container_ReleaseLease,
  Container_RenewLease,
  Container_BreakLease,
  Container_ChangeLease,
  Container_ListBlobFlatSegment,
  Container_ListBlobHierarchySegment,
  Container_GetAccountInfo,
  Container_GetAccountInfoWithHead,
  Blob_Download,
  Blob_GetProperties,
  Blob_Delete,
  Blob_Undelete,
  Blob_SetExpiry,
  Blob_SetHTTPHeaders,
  Blob_SetImmutabilityPolicy,
  Blob_DeleteImmutabilityPolicy,
  Blob_SetLegalHold,
  Blob_SetMetadata,
  Blob_AcquireLease,
  Blob_ReleaseLease,
  Blob_RenewLease,
  Blob_ChangeLease,
  Blob_BreakLease,
  Blob_CreateSnapshot,
  Blob_StartCopyFromURL,
  Blob_CopyFromURL,
  Blob_AbortCopyFromURL,
  Blob_SetTier,
  Blob_GetAccountInfo,
  Blob_GetAccountInfoWithHead,
  Blob_Query,
  Blob_GetTags,
  Blob_SetTags,
  PageBlob_Create,
  PageBlob_UploadPages,
  PageBlob_ClearPages,
  PageBlob_UploadPagesFromURL,
  PageBlob_GetPageRanges,
  PageBlob_GetPageRangesDiff,
  PageBlob_Resize,
  PageBlob_UpdateSequenceNumber,
  PageBlob_CopyIncremental,
  AppendBlob_Create,
  AppendBlob_AppendBlock,
  AppendBlob_AppendBlockFromUrl,
  AppendBlob_Seal,
  BlockBlob_Upload,
  BlockBlob_PutBlobFromUrl,
  BlockBlob_StageBlock,
  BlockBlob_StageBlockFromURL,
  BlockBlob_CommitBlockList,
  BlockBlob_GetBlockList,
}

export default Operation;
```

## Dependencies
- None.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `enum Operation` numeric auto-increment | `#[repr(u16)] enum Operation` or equivalent stable numeric enum | Numeric order is the identity used by `Specifications[]` and `operationHandlerMapping[]`. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Contains 72 enum members in declaration order. TypeScript auto-assigns zero-based numeric values; Rust must preserve that order exactly.
- `dispatch.middleware.ts`, `specifications.ts`, `handlerMappers.ts`, and telemetry later all treat these enum values as stable lookup keys.

## Middleware chain ordering
- Dispatch writes an `Operation` into `Context`; all later middleware stages depend on that exact numeric selection.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- Never reorder or insert enum members in Rust without updating every spec/handler lookup table in lockstep.
