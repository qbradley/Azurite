# Porting Record — `src/queue/generated/IResponse.ts`

## File info
- Source path: `src/queue/generated/IResponse.ts`
- Source lines: `17`
- Source type: `generated interface`
- Rust target (per `PORTING-ORDER.md`): Part of `azurite-queue/src/generated/`
- Crate: `azurite-queue`
- Module: `generated::response`
- Phase: `14.1`
- Status: `ported`

## Exported API
### Default interface `IResponse`
- 10 methods:
  - `setStatusCode(code: number): IResponse` (chainable)
  - `getStatusCode(): number`
  - `setStatusMessage(message: string): IResponse` (chainable)
  - `getStatusMessage(): string`
  - `setHeader(field: string, value?: string | string[] | undefined | number | boolean): IResponse` (chainable)
  - `getHeader(field: string): number | string | string[] | undefined`
  - `getHeaders(): OutgoingHttpHeaders`
  - `headersSent(): boolean`
  - `setContentType(value: string | undefined): IResponse` (chainable)
  - `getBodyStream(): NodeJS.WritableStream`

## Dependencies
- `OutgoingHttpHeaders` from Node.js `http` module: Type for response headers

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `setStatusCode(code: number): IResponse` | Method returning `self` | Builder pattern for chainability |
| `getStatusCode(): number` | Method returning `u16` | HTTP status codes 100-599 |
| `setHeader(field, value?: ...)` | Method with flexible value types | Value stringifies numbers/booleans |
| `value?: string \| string[] \| undefined \| number \| boolean` | Multiple overloads or `enum HeaderValue` | Polymorphic header value type |
| `getHeaders(): OutgoingHttpHeaders` | Custom headers type or `HashMap` | Returns all set headers |
| `headersSent(): boolean` | `bool` | Tracks if headers already sent to client |
| `NodeJS.WritableStream` | `Box<dyn AsyncWrite>` or `tokio::io::AsyncWriteExt` | Body output stream |
| `OutgoingHttpHeaders` | Custom struct or `HashMap<String, Vec<String>>` | Multi-value header container |

## Special handling
1. **Fluent interface (chainable setters)**
   - `setStatusCode()`, `setStatusMessage()`, `setHeader()`, `setContentType()` all return `IResponse`
   - Allows pattern: `response.setStatusCode(200).setContentType("application/json").setHeader(...)`
   - Preserve method chaining in Rust trait

2. **Polymorphic header values** (`setHeader()` signature)
   - Accepts `string | string[] | number | boolean | undefined`
   - Numbers and booleans are stringified (e.g., `setHeader("x-custom", 123)` → `"123"`)
   - Arrays represent multi-value headers (comma-separated in HTTP, separate values in some protocols)
   - `undefined` value removes header

3. **Header value get variance**
   - `getHeader(field)` returns single value: `number | string | string[] | undefined`
   - `getHeaders()` returns all headers with full type fidelity
   - If multi-value header set, getHeader() may return array (implementation-dependent)

4. **Status code immutability after send**
   - `headersSent(): boolean` indicates if headers already sent to HTTP client
   - After `headersSent() === true`, status code/headers cannot be changed
   - Future calls to setters should either error or no-op (TS semantics TBD)

5. **Content-Type convenience setter**
   - `setContentType()` is syntactic sugar for `setHeader("content-type", value)`
   - `undefined` value removes content-type header
   - Shortcut for common header operation

6. **Body stream semantics**
   - `getBodyStream()` returns writable stream for response body
   - Must support both chunked and fixed-length writes
   - Multiple writes allowed (data accumulates)

## Change propagation notes
- Keep setters chainable even if implementation changes
- If HTTP/2 or HTTP/3 support added, header semantics may shift (pseudo-headers, etc.) — preserve IResponse interface stability
- Multi-value header handling must match phase 5 blob-generated IResponse exactly
- Header field names should be case-insensitive per HTTP spec, but preserve case as provided

