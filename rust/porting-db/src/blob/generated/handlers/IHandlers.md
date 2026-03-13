# Porting Record — `src/blob/generated/handlers/IHandlers.ts`

## File info
- Source path: `src/blob/generated/handlers/IHandlers.ts`
- Source lines: `17`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/i_handlers.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::i_handlers`
- Phase: `5.15`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export interface IHandlers {
  serviceHandler: IServiceHandler;
  containerHandler: IContainerHandler;
  blobHandler: IBlobHandler;
  pageBlobHandler: IPageBlobHandler;
  appendBlobHandler: IAppendBlobHandler;
  blockBlobHandler: IBlockBlobHandler;
}

export default IHandlers;
```

## Dependencies
- Internal imports:
  - `./IServiceHandler` → `src/blob/generated/handlers/IServiceHandler.ts` — Phase 5 — analyzed in this pass
  - `./IContainerHandler` → `src/blob/generated/handlers/IContainerHandler.ts` — Phase 5 — analyzed in this pass
  - `./IBlobHandler` → `src/blob/generated/handlers/IBlobHandler.ts` — Phase 5 — analyzed in this pass
  - `./IPageBlobHandler` → `src/blob/generated/handlers/IPageBlobHandler.ts` — Phase 5 — analyzed in this pass
  - `./IAppendBlobHandler` → `src/blob/generated/handlers/IAppendBlobHandler.ts` — Phase 5 — analyzed in this pass
  - `./IBlockBlobHandler` → `src/blob/generated/handlers/IBlockBlobHandler.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `IHandlers` aggregate interface | `struct/trait bundle holding the six generated handler traits | Dynamic handler lookup indexes by these property names. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Aggregates the six service-specific generated handler interfaces under fixed property names: `serviceHandler`, `containerHandler`, `blobHandler`, `pageBlobHandler`, `appendBlobHandler`, and `blockBlobHandler`.
- `handlerMappers.ts` and `HandlerMiddlewareFactory.ts` index this object by those exact strings; preserve names verbatim.

## Middleware chain ordering
- Stage 3 handler middleware receives one aggregate `IHandlers` object and selects the concrete method dynamically.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest adds a new handler family, update this aggregate and the handler-dispatch string table together.
