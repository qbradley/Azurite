# Porting Record — `src/blob/generated/ExpressResponseAdapter.ts`

## File info
- Source path: `src/blob/generated/ExpressResponseAdapter.ts`
- Source lines: `66`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/express_response_adapter.rs`
- Crate: `azurite-blob`
- Module: `generated::express_response_adapter`
- Phase: `5.20`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class ExpressResponseAdapter implements IResponse {
  public constructor(private readonly res: Response) {}

  public setStatusCode(code: number): IResponse {
    this.res.status(code);
    return this;
  }

  public getStatusCode(): number {
    return this.res.statusCode;
  }

  public setStatusMessage(message: string): IResponse {
    this.res.statusMessage = message;
    return this;
  }

  public getStatusMessage(): string {
    return this.res.statusMessage;
  }

  public setHeader(
    field: string,
    value?: string | string[] | undefined | number | boolean
  ): IResponse {
    if (typeof value === "number") {
      value = `${value}`;
    }

    if (typeof value === "boolean") {
      value = `${value}`;
    }

    // Cannot remove if block because of a potential TypeScript bug
    if (typeof value === "string" || value instanceof Array) {
      this.res.setHeader(field, value);
    }
    return this;
  }

  public getHeader(field: string): number | string | string[] | undefined {
    return this.res.getHeader(field);
  }

  public getHeaders(): OutgoingHttpHeaders {
    return this.res.getHeaders();
  }

  public headersSent(): boolean {
    return this.res.headersSent;
  }

  public setContentType(value: string): IResponse {
    this.res.setHeader("content-type", value);
    return this;
  }

  public getBodyStream(): NodeJS.WritableStream {
    return this.res;
  }
}
```

## Dependencies
- External packages:
  - `express` — imported as `{ Response }`
- Node built-ins:
  - `http` — imported as `{ OutgoingHttpHeaders }`
- Internal imports:
  - `./IResponse` → `src/blob/generated/IResponse.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `express.Response` | `axum response writer wrapper` | Generated middleware writes headers and body through this abstraction. |
| `string | string[] | number | boolean` header setter | helper that coerces `number`/`boolean` to strings before insertion | Type coercion is observable today. |
| `NodeJS.WritableStream` | `dyn AsyncWrite` | Serializer/error/end write directly into the response body stream. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- `setHeader()` only calls the underlying framework when the value is a string or array after explicit boolean/number stringification. This guard exists because of a comment about a TypeScript bug; keep the behavior explicit.
- `setContentType()` hardcodes header name `content-type` in lowercase.

## Middleware chain ordering
- Constructed afresh in stages 1, 2, 4, 5, and 6 of the generated Express pipeline.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If header coercion rules or body-stream behavior change, audit serializer/error/end middleware together because they all assume this adapter contract.
