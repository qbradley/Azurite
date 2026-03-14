# Porting Record — `src/blob/handlers/SubResponseTextBodyStream.ts`

## File info
- Source path: `src/blob/handlers/SubResponseTextBodyStream.ts`
- Source lines: `29`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/sub_response_text_body_stream.rs`
- Crate: `azurite-blob`
- Module: `handlers::sub_response_text_body_stream`
- Phase: `11.13`
- Status: `not_started`

## Exported API
### Class `SubResponseTextBodyStream extends Writable`
- Constructor injects its owning `BlobBatchSubResponse`.
- Methods: `_write()`, overloads of `end()`, `getBodyContent()`.

## Dependencies
- Node `Writable` stream.
- `BlobBatchSubResponse` back-reference.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| writable stream that just accumulates UTF-8-ish text | simple string buffer writer | Batch responses only need text capture, not streaming side effects. |
| overloaded `end()` signatures | single helper that optionally appends final chunk then finalizes response | Keep callback/encoding omissions visible. |

## Special handling
- `SubResponseTextBodyStream.ts:13-16` appends `chunk.toString()` for every write and immediately calls the callback.
- `SubResponseTextBodyStream.ts:21-24` ignores the usual writable-stream `encoding` / callback semantics and simply appends a final chunk (if any) before calling `subResponse.end()`.
- The stream never emits the buffered body outward; callers must ask `getBodyContent()`.

## Change propagation notes
- If TypeScript switches batch subresponses to binary or streamed bodies, this file will need a real transport abstraction. Today it is intentionally string-only.
