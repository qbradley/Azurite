# Porting Record — `src/common/utils/BufferStream.ts`

## File info
- Source path: `src/common/utils/BufferStream.ts`
- Source lines: `33`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/utils/buffer_stream.rs`
- Crate: `azurite-common`
- Module: `utils::buffer_stream`
- Phase: `4.3`
- Status: `ported`

## Exported API
### Default class `BufferStream extends Readable`
- Constructor: `constructor(buffer: Buffer, options?: any)`
- Public method: `_read(): void`
- Private helper: `_readNextChunk(): Buffer | null`

## Dependencies
- Node built-in: `stream.Readable`.
- No internal imports.
- Typical downstream use: any caller that needs to expose an in-memory `Buffer` as a Node readable stream.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Readable` subclass | small struct implementing `std::io::Read`, `tokio::io::AsyncRead`, or `Stream<Item = Bytes>` | Pick the trait that best matches actual call sites in the Rust service layer. |
| `Buffer` | `bytes::Bytes` / `Vec<u8>` | `Bytes` is a natural fit if HTTP bodies consume it. |
| `options?: any` | optional constructor config or omitted entirely | TS forwards options to `Readable`; Rust likely does not need an exact equivalent. |
| `Buffer | null` | `Option<Bytes>` / `Option<Vec<u8>>` | `null` means EOF. |

## Recommended Rust translation
- Use a small wrapper over `Bytes`/`Vec<u8>` plus an offset cursor.
- Preserve the fixed 64 KiB chunk size unless profiling or future TS changes justify a different boundary.
- Prefer a direct async reader/body-stream adapter instead of introducing extra buffering layers.

## Function / method mapping notes
| TS member | Rust recommendation | Fidelity notes |
|---|---|---|
| constructor | `fn new(buffer: Bytes)` | Store the full buffer, offset `0`, chunk size `64 * 1024`, and cached length. |
| `_read()` | `poll_read` / `poll_next` implementation | Keep pulling chunks until backpressure/consumer limit is reached. |
| `_readNextChunk()` | private `next_chunk()` helper | EOF when offset reaches buffer length. |

## Special handling
- `_read()` keeps calling `push()` while it returns `true`; this means the class honors Node backpressure instead of dumping the whole buffer at once.
- `_readNextChunk()` uses `buffer.slice(this.offset, end)`, so chunks share the original buffer storage rather than copying when possible. Prefer `Bytes::slice()` or an equivalent zero-copy view in Rust.
- The chunk size is a literal `64 * 1024`, not configurable.

## Change propagation notes
- If TS later exposes the chunk size as configuration, the Rust wrapper should expose the same shape instead of baking in a hidden constant.
- Any future move from Node streams to web streams or async iterators in TS should trigger a Rust API review here.

## Fidelity risks and edge cases
- Backpressure semantics are the main fidelity concern: a Rust implementation that eagerly materializes all chunks would not match the TS `Readable` behavior.
- The TS class accepts `options?: any` only to feed the `Readable` base class. Omitting that hook in Rust is acceptable, but document it if upstream TS starts using custom stream options.

## Rust port notes
- Ported the stream wrapper to `rust/crates/azurite-common/src/utils/buffer_stream.rs` as a chunked in-memory reader with the same fixed 64 KiB chunk size.
- The Rust wrapper keeps the TS chunking boundary explicit so later request-body plumbing can stay mechanically comparable to `BufferStream.ts`.
