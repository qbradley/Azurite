# Porting Record — `src/blob/generated/ExpressRequestAdapter.ts`

## File info
- Source path: `src/blob/generated/ExpressRequestAdapter.ts`
- Source lines: `57`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/express_request_adapter.rs`
- Crate: `azurite-blob`
- Module: `generated::express_request_adapter`
- Phase: `5.19`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class ExpressRequestAdapter implements IRequest {
  public constructor(private readonly req: Request) {}

  public getMethod(): HttpMethod {
    return this.req.method.toUpperCase() as HttpMethod;
  }

  public getUrl(): string {
    return this.req.url;
  }

  public getEndpoint(): string {
    return `${this.req.protocol}://${this.getHeader("host") ||
      this.req.hostname}`;
  }
```

## Dependencies
- External packages:
  - `express` — imported as `{ Request }`
- Internal imports:
  - `./IRequest` → `src/blob/generated/IRequest.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `express.Request` | `axum::Request` wrapper / custom adapter struct | Need exact helper methods required by generated middleware. |
| `req.query[key]` as `string | undefined` | query accessor returning first/single value | Current adapter narrows Express’s broader query type. |
| `req` itself as `ReadableStream` | body stream borrowed from request state | Streaming upload operations depend on this direct pass-through. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- `getEndpoint()` builds `${protocol}://${host-header-or-hostname}`; preserve header-first precedence.
- `setBody()` mutates the underlying Express request object and returns `this`, so the same wrapper instance can be fluently reused within middleware helpers.

## Middleware chain ordering
- Constructed afresh in stages 1, 2, 4, 5, and 6 of the generated Express pipeline.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If the request adapter surface changes, update both `IRequest.ts` and all middleware/helpers that call it, especially `serializer.ts` and `dispatch.middleware.ts`.
