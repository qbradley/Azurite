# Porting Record — `src/blob/generated/handlers/IPageBlobHandler.ts`

## File info
- Source path: `src/blob/generated/handlers/IPageBlobHandler.ts`
- Source lines: `25`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/i_page_blob_handler.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::i_page_blob_handler`
- Phase: `5.14`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface IPageBlobHandler {
  create(contentLength: number, blobContentLength: number, options: Models.PageBlobCreateOptionalParams, context: Context): Promise<Models.PageBlobCreateResponse>;
  uploadPages(body: NodeJS.ReadableStream, contentLength: number, options: Models.PageBlobUploadPagesOptionalParams, context: Context): Promise<Models.PageBlobUploadPagesResponse>;
  clearPages(contentLength: number, options: Models.PageBlobClearPagesOptionalParams, context: Context): Promise<Models.PageBlobClearPagesResponse>;
  uploadPagesFromURL(sourceUrl: string, sourceRange: string, contentLength: number, range: string, options: Models.PageBlobUploadPagesFromURLOptionalParams, context: Context): Promise<Models.PageBlobUploadPagesFromURLResponse>;
  getPageRanges(options: Models.PageBlobGetPageRangesOptionalParams, context: Context): Promise<Models.PageBlobGetPageRangesResponse>;
  getPageRangesDiff(options: Models.PageBlobGetPageRangesDiffOptionalParams, context: Context): Promise<Models.PageBlobGetPageRangesDiffResponse>;
  resize(blobContentLength: number, options: Models.PageBlobResizeOptionalParams, context: Context): Promise<Models.PageBlobResizeResponse>;
  updateSequenceNumber(sequenceNumberAction: Models.SequenceNumberActionType, options: Models.PageBlobUpdateSequenceNumberOptionalParams, context: Context): Promise<Models.PageBlobUpdateSequenceNumberResponse>;
  copyIncremental(copySource: string, options: Models.PageBlobCopyIncrementalOptionalParams, context: Context): Promise<Models.PageBlobCopyIncrementalResponse>;
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
- Declares 9 operation methods for one generated handler family. The method order and argument order match `handlerMappers.ts` exactly.
- All methods return generated `Models.*Response` wrappers that encode headers/body/statusCode as the public contract.

## Middleware chain ordering
- Stage 3: Handler middleware uses `handlerMappers.ts` to pick one of these methods, extracts positional arguments from `context.handlerParameters`, then appends `context` as the last argument.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If any method signature changes, update `handlerMappers.ts`, `IHandlers.ts`, and the concrete Rust handler implementation traits together.
