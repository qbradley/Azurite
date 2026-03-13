# Porting Record — `src/blob/generated/IRequest.ts`

## File info
- Source path: `src/blob/generated/IRequest.ts`
- Source lines: `26`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/i_request.rs`
- Crate: `azurite-blob`
- Module: `generated::i_request`
- Phase: `5.1`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export type HttpMethod =
  | "GET"
  | "HEAD"
  | "POST"
  | "PUT"
  | "DELETE"
  | "CONNECT"
  | "OPTIONS"
  | "TRACE"
  | "MERGE"
  | "PATCH";

export default interface IRequest {
  getMethod(): HttpMethod;
  getUrl(): string;
  getEndpoint(): string;
  getPath(): string;
  getBodyStream(): NodeJS.ReadableStream;
  setBody(body: string | undefined): IRequest;
  getBody(): string | undefined;
  getHeader(field: string): string | undefined;
  getHeaders(): { [header: string]: string | string[] | undefined };
  getRawHeaders(): string[];
  getQuery(key: string): string | undefined;
  getProtocol(): string;
}
```

## Dependencies
- None.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `HttpMethod` string union | `enum HttpMethod` or string-backed enum | Preserve exact verb set, including `MERGE` and `PATCH`. |
| `NodeJS.ReadableStream` | `dyn AsyncRead + Send` | Blob bodies remain streaming all the way into handlers. |
| `getHeaders(): { [header: string]: string | string[] | undefined }` | `HeaderMap` plus helper that can surface repeated values | Do not collapse multi-value headers to a single string. |
| `setBody(body)` / `getBody()` | `Option<String>` cached on request wrapper | Deserializer stores buffered body text for later readers. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## API contract notes
- `getUrl()`, `getEndpoint()`, and `getPath()` are distinct surfaces; later middleware uses all three forms.
- `getQuery()` returns a single string even though the underlying framework may offer arrays; keep that lossy contract visible.

## Middleware chain ordering
- Used by dispatch/deserializer middleware and by `Context`; not itself a middleware stage.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest adds verbs or new request wrapper methods, update every adapter (`ExpressRequestAdapter` now, queue/table adapters later) in lockstep.
