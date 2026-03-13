# Porting Record — `src/blob/generated/middleware/end.middleware.ts`

## File info
- Source path: `src/blob/generated/middleware/end.middleware.ts`
- Source lines: `32`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware/end.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware::end`
- Phase: `5.26`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default function endMiddleware(
  context: Context,
  res: IResponse,
  logger: ILogger,
): void {
  const totalTimeInMS = context.startTime
    ? new Date().getTime() - context.startTime.getTime()
    : undefined;

  logger.info(
    // tslint:disable-next-line:max-line-length
    `EndMiddleware: End response. TotalTimeInMS=${totalTimeInMS} StatusCode=${res.getStatusCode()} StatusMessage=${res.getStatusMessage()} Headers=${JSON.stringify(
      res.getHeaders()
    )}`,
    context.contextId
  );

  res.getBodyStream().end();
}
```

## Dependencies
- Internal imports:
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `../IResponse` → `src/blob/generated/IResponse.ts` — Phase 5 — analyzed in this pass
  - `../utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `new Date().getTime()` delta | `Instant`/`SystemTime` duration calc | Uses `context.startTime` if present, otherwise logs `undefined`. |
| `res.getBodyStream().end()` | response finalization call | This stage owns the final close after serializer/error middleware have written bytes. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Logs total request duration, status code/message, and serialized headers immediately before finalizing the response stream.
- Does not inspect `headersSent()` or body shape; it always calls `.end()`.

## Middleware chain ordering
- Stage 6 of 6: terminal generated middleware; must run after serializer and error middleware.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If serializer later takes ownership of stream finalization, record that as an explicit divergence because current TS keeps `.end()` here.
