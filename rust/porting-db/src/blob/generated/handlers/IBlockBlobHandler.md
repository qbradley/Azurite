# Porting Record — `src/blob/generated/handlers/IBlockBlobHandler.ts`

## File info
- Source path: `src/blob/generated/handlers/IBlockBlobHandler.ts`
- Source lines: `22`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/i_block_blob_handler.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::i_block_blob_handler`
- Phase: `5.14`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface IBlockBlobHandler {
  upload(body: NodeJS.ReadableStream, contentLength: number, options: Models.BlockBlobUploadOptionalParams, context: Context): Promise<Models.BlockBlobUploadResponse>;
  putBlobFromUrl(contentLength: number, copySource: string, options: Models.BlockBlobPutBlobFromUrlOptionalParams, context: Context): Promise<Models.BlockBlobPutBlobFromUrlResponse>;
  stageBlock(blockId: string, contentLength: number, body: NodeJS.ReadableStream, options: Models.BlockBlobStageBlockOptionalParams, context: Context): Promise<Models.BlockBlobStageBlockResponse>;
  stageBlockFromURL(blockId: string, contentLength: number, sourceUrl: string, options: Models.BlockBlobStageBlockFromURLOptionalParams, context: Context): Promise<Models.BlockBlobStageBlockFromURLResponse>;
  commitBlockList(blocks: Models.BlockLookupList, options: Models.BlockBlobCommitBlockListOptionalParams, context: Context): Promise<Models.BlockBlobCommitBlockListResponse>;
  getBlockList(options: Models.BlockBlobGetBlockListOptionalParams, context: Context): Promise<Models.BlockBlobGetBlockListResponse>;
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
- Declares 6 operation methods for one generated handler family. The method order and argument order match `handlerMappers.ts` exactly.
- All methods return generated `Models.*Response` wrappers that encode headers/body/statusCode as the public contract.

## Middleware chain ordering
- Stage 3: Handler middleware uses `handlerMappers.ts` to pick one of these methods, extracts positional arguments from `context.handlerParameters`, then appends `context` as the last argument.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If any method signature changes, update `handlerMappers.ts`, `IHandlers.ts`, and the concrete Rust handler implementation traits together.
