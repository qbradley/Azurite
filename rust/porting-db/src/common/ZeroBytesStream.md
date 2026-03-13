# Porting Record — `src/common/ZeroBytesStream.ts`

## File info
- Source path: `src/common/ZeroBytesStream.ts`
- Source lines: `30`
- Source type: `handwritten`
- Rust target: `azurite-common/src/zero_bytes_stream.rs`
- Crate: `azurite-common`
- Module: `zero_bytes_stream`
- Phase: `2.6`
- Status: `ported`

## Exported API
### Default class `ZeroBytesStream extends Readable`
- **Constructor**: `new ZeroBytesStream(length: number, options?: ReadableOptions)`
- **Method**:
  - `_read(size: number): void` (internal Node.js Readable callback; implements stream behavior)
- **Properties**:
  - `public readonly length: number` (total bytes to emit)
  - `private leftBytes: number` (bytes remaining to emit)

### Private constant `zeroBytesRangeUnit`
- `512` (chunk size for each push)

### Private constant `zeroBytesChunk`
- `Buffer.alloc(zeroBytesRangeUnit)` (pre-allocated zero buffer)

## Dependencies
- Imports: `stream.Readable`, `stream.ReadableOptions`
- Porting status:
  - `Readable` base class: Replace with `tokio::io::AsyncRead` impl or custom reader struct.
  - `ReadableOptions`: Constructor parameters or builder.
- Important consumers: FSExtentStore, MemoryExtentStore (return for empty/zero-filled extents)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `extends Readable` | `impl AsyncRead` or custom struct | No inheritance; use trait impl. |
| `_read(size): void` callback | `poll_read()` method (AsyncRead trait) | Rust uses async/polling model. |
| `this.push(Buffer)` | Write to caller's buffer in poll_read | Rust stream semantics. |
| `this.push(null)` | Return Poll::Ready(Ok(())) with 0 bytes | EOF signaling. |
| `ReadableOptions` | Constructor struct or params | Configure stream (e.g., highWaterMark). |

## Recommended Rust translation
```rust
use tokio::io::{AsyncRead, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ZeroBytesStream {
    pub length: u64,
    left_bytes: u64,
}

impl ZeroBytesStream {
    pub fn new(length: u64) -> Self {
        ZeroBytesStream {
            length,
            left_bytes: length,
        }
    }
}

impl AsyncRead for ZeroBytesStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        const ZERO_BYTES_RANGE_UNIT: usize = 512;

        if self.left_bytes == 0 {
            return Poll::Ready(Ok(())); // EOF
        }

        let to_write = std::cmp::min(
            self.left_bytes as usize,
            buf.remaining(),
        ).min(ZERO_BYTES_RANGE_UNIT);

        // Write zeros directly to caller's buffer
        for _ in 0..to_write {
            buf.put_u8(0);
        }
        self.left_bytes -= to_write as u64;

        Poll::Ready(Ok(()))
    }
}

// Alternative: implement futures::stream::Stream
impl futures::stream::Stream for ZeroBytesStream {
    type Item = std::io::Result<bytes::Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.left_bytes == 0 {
            return Poll::Ready(None);
        }
        let chunk_size = std::cmp::min(512, self.left_bytes as usize);
        self.left_bytes -= chunk_size as u64;
        Poll::Ready(Some(Ok(bytes::Bytes::from(vec![0u8; chunk_size]))))
    }
}
```

## Special handling
- **Readable vs AsyncRead**: Node.js `Readable` uses pull-based callbacks (_read is called when data is requested). Rust `AsyncRead` uses similar poll-based model via `poll_read()`. Both are demand-driven.
- **Zero-buffer optimization**: TS pre-allocates singleton `zeroBytesChunk` to avoid repeated allocations. Rust should write zeros directly to caller's buffer (no allocation) or use a static zero buffer if efficiency matters.
- **Chunk pacing**: TS emits max 512 bytes per `_read()` call. Rust's `poll_read()` respects caller's buffer size; emit min(remaining, bufSize, 512) bytes per poll.
- **process.nextTick() in TS**: Used for small remainder writes. Rust's tokio runtime naturally paces async operations; no explicit deferral needed.

## Control flow notes
1. **Construction**: Initialize `length` and `leftBytes = length`.
2. **_read(size) / poll_read(buf)**:
   - If `leftBytes === 0`: emit EOF (push(null) or return Ready with 0 bytes).
   - If `leftBytes >= 512`: write 512 zeros to buffer, decrement leftBytes, return Ready.
   - If `0 < leftBytes < 512`: write all remaining zeros to buffer, set leftBytes=0, return Ready.
   - Rust version: write min(leftBytes, buf.remaining(), 512) bytes to buf directly.

## Performance notes
- Avoid allocating new buffers for each read; write directly to caller's buffer.
- Use `bytes::Bytes::from_static()` with zero constant if benchmarks show allocation is bottleneck.
- Chunk size (512) can be tuned based on typical extent sizes and I/O buffer alignment.

## Change propagation notes
- If chunk size (512) becomes configurable, add as const parameter or struct field.
- If zero-byte streams are used for massive extents (TB+), reconsider allocation strategy.
- If ReadableOptions parameters are leveraged elsewhere, extend constructor.

## Rust port notes
- Ported to `azurite-common/src/zero_bytes_stream.rs` as an `AsyncRead` implementation that emits at most 512 zero bytes per poll, matching the TS chunk pacing.
- The Rust reader writes directly into `ReadBuf` instead of allocating a new remainder buffer, but keeps the same `length`/`leftBytes` state split.
- Validated with `cargo check` and `cargo test -p azurite-common`.

