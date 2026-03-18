# Porting Record — `src/queue/generated/IRequest.ts`

## File info
- Source path: `src/queue/generated/IRequest.ts`
- Source lines: `26`
- Source type: `generated interface`
- Rust target (per `PORTING-ORDER.md`): Part of `azurite-queue/src/generated/`
- Crate: `azurite-queue`
- Module: `generated::request`
- Phase: `14.1`
- Status: `ported`

## Exported API
### Type `HttpMethod`
- Union type: `"GET" | "HEAD" | "POST" | "PUT" | "DELETE" | "CONNECT" | "OPTIONS" | "TRACE" | "MERGE" | "PATCH"`

### Default interface `IRequest`
- 11 methods:
  - `getMethod(): HttpMethod`
  - `getUrl(): string`
  - `getEndpoint(): string`
  - `getPath(): string`
  - `getBodyStream(): NodeJS.ReadableStream`
  - `setBody(body: string | undefined): IRequest` (chainable)
  - `getBody(): string | undefined`
  - `getHeader(field: string): string | undefined`
  - `getHeaders(): { [header: string]: string | string[] | undefined }`
  - `getRawHeaders(): string[]`
  - `getQuery(key: string): string | undefined`
  - `getProtocol(): string`

## Dependencies
- None (pure interface definition)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `type HttpMethod` | `pub enum HttpMethod` or `pub const` strings | Use enum for better type safety |
| `NodeJS.ReadableStream` | `Box<dyn AsyncRead>` or `tokio::io::AsyncReadExt` | Async stream adaptor |
| `string \| undefined` | `Option<String>` | Body and header values nullable |
| `{ [header: string]: string \| string[] \| undefined }` | `HashMap<String, Vec<String>>` or custom headers type | Multi-value headers |
| `string[]` | `Vec<String>` | Raw header array |

## Special handling
1. **Fluent interface** (`setBody()` method)
   - `setBody()` returns `IRequest` for chaining
   - Allows pattern: `request.setBody(data).getBody()`
   - Preserve method chaining in Rust trait

2. **Stream vs String duality**
   - `getBodyStream()` returns NodeJS stream (raw)
   - `setBody()` and `getBody()` work with strings
   - Both must coexist — stream is primary, string is cached buffer
   - Implementation must handle both paths

3. **Header value variance**
   - `getHeader()` returns single value (string or undefined)
   - `getHeaders()` returns object with value arrays (union type)
   - Some headers may be multi-value (Set-Cookie, Accept, etc.)
   - getHeader() should return first value if multi-value header

4. **Query parameter extraction**
   - `getQuery(key: string)` extracts single query parameter by name
   - Returns string or undefined if not present
   - Does not handle multi-value query parameters (use getQuery() once per parameter)

5. **Protocol string**
   - `getProtocol()` returns "http" or "https" (as string, not typed)
   - May return other values in edge cases

6. **Method type consistency**
   - HttpMethod union must include all 10 HTTP verbs plus "MERGE" and "PATCH"
   - "MERGE" is Azure-specific (WebDAV), "PATCH" is RFC 5789
   - Do not abbreviate or normalize method names

## Change propagation notes
- If new HTTP verbs are added to spec, update HttpMethod union
- If request/response adapters change (ExpressRequestAdapter), IRequest contract must remain stable
- Maintain 1:1 mapping with Phase 5 blob-generated IRequest (should be identical)
- Header handling must support multi-value headers per HTTP spec

