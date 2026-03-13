# Porting Record — `src/blob/generated/handlers/IServiceHandler.ts`

## File info
- Source path: `src/blob/generated/handlers/IServiceHandler.ts`
- Source lines: `24`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/i_service_handler.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::i_service_handler`
- Phase: `5.14`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface IServiceHandler {
  setProperties(storageServiceProperties: Models.StorageServiceProperties, options: Models.ServiceSetPropertiesOptionalParams, context: Context): Promise<Models.ServiceSetPropertiesResponse>;
  getProperties(options: Models.ServiceGetPropertiesOptionalParams, context: Context): Promise<Models.ServiceGetPropertiesResponse>;
  getStatistics(options: Models.ServiceGetStatisticsOptionalParams, context: Context): Promise<Models.ServiceGetStatisticsResponse>;
  listContainersSegment(options: Models.ServiceListContainersSegmentOptionalParams, context: Context): Promise<Models.ServiceListContainersSegmentResponse>;
  getUserDelegationKey(keyInfo: Models.KeyInfo, options: Models.ServiceGetUserDelegationKeyOptionalParams, context: Context): Promise<Models.ServiceGetUserDelegationKeyResponse>;
  getAccountInfo(context: Context): Promise<Models.ServiceGetAccountInfoResponse>;
  submitBatch(body: NodeJS.ReadableStream, contentLength: number, multipartContentType: string, options: Models.ServiceSubmitBatchOptionalParams, context: Context): Promise<Models.ServiceSubmitBatchResponse>;
  filterBlobs(options: Models.ServiceFilterBlobsOptionalParams, context: Context): Promise<Models.ServiceFilterBlobsResponse>;
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
- Declares 8 operation methods for one generated handler family. The method order and argument order match `handlerMappers.ts` exactly.
- All methods return generated `Models.*Response` wrappers that encode headers/body/statusCode as the public contract.

## Middleware chain ordering
- Stage 3: Handler middleware uses `handlerMappers.ts` to pick one of these methods, extracts positional arguments from `context.handlerParameters`, then appends `context` as the last argument.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If any method signature changes, update `handlerMappers.ts`, `IHandlers.ts`, and the concrete Rust handler implementation traits together.
