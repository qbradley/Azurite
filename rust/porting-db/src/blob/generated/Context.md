# Porting Record — `src/blob/generated/Context.ts`

## File info
- Source path: `src/blob/generated/Context.ts`
- Source lines: `143`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/context.rs`
- Crate: `azurite-blob`
- Module: `generated::context`
- Phase: `5.7`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export interface IHandlerParameters {
  [key: string]: any;
}

export default class Context {
  public readonly context: any;
  public readonly path: string;

  /**
   * Creates an instance of Context.
   * Context holds generated server context information.
   * Every incoming HTTP request will initialize a new context.
   *
   * @param {Context} context An existing Context
   * @memberof Context
   */
  public constructor(context: Context);

  /**
   * Creates an instance of Context.
   * Context holds generated server context information.
   * Every incoming HTTP request will initialize a new context.
   *
   * @param {Object} holder Holder is an Object which used to keep context information
   * @param {string} [path="context"] holder[path] is used as context object by default
   * @param {IRequest} [req]
   * @param {IResponse} [res]
   * @memberof Context
   */
  public constructor(
    holder: object,
    path: string,
    req?: IRequest,
    res?: IResponse
  );

  public constructor(
    holderOrContext: object | Context,
    path: string = "context",
    req?: IRequest,
    res?: IResponse
  ) {
    if (holderOrContext instanceof Context) {
      this.context = holderOrContext.context;
      this.path = holderOrContext.path;
    } else {
      const context = holderOrContext as any;
      this.path = path;

      if (context[this.path] === undefined) {
        context[this.path] = {};
      }

      if (typeof context[this.path] !== "object") {
        throw new TypeError(
          `Initialize Context error because holder.${this.path} is not an object.`
        );
      }

      this.context = context[this.path];

      this.request = req;
      this.response = res;
    }
  }

  public get operation(): Operation | undefined {
    return this.context.operation;
  }

  public set operation(operation: Operation | undefined) {
    this.context.operation = operation;
  }

  public set request(request: IRequest | undefined) {
    this.context.request = request;
  }

  public get request(): IRequest | undefined {
    return this.context.request;
  }

  public get dispatchPattern(): string | undefined {
    return this.context.dispatchPattern;
  }

  public set dispatchPattern(path: string | undefined) {
    this.context.dispatchPattern = path;
  }

  public set response(response: IResponse | undefined) {
    this.context.response = response;
  }

  public get response(): IResponse | undefined {
    return this.context.response;
  }

  public get handlerParameters(): IHandlerParameters | undefined {
    return this.context.handlerParameters;
  }

  public set handlerParameters(
    handlerParameters: IHandlerParameters | undefined
  ) {
    this.context.handlerParameters = handlerParameters;
  }

  public get handlerResponses(): any {
    return this.context.handlerResponses;
  }

  public set handlerResponses(handlerResponses: any) {
    this.context.handlerResponses = handlerResponses;
  }

  public get contextId(): string | undefined {
    return this.context.contextID;
  }

  public set contextId(contextID: string | undefined) {
    this.context.contextID = contextID;
  }

  public set startTime(startTime: Date | undefined) {
    this.context.startTime = startTime;
  }

  public get startTime(): Date | undefined {
    return this.context.startTime;
  }
}
```

## Dependencies
- Internal imports:
  - `./artifacts/operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass
  - `./IRequest` → `src/blob/generated/IRequest.ts` — Phase 5 — analyzed in this pass
  - `./IResponse` → `src/blob/generated/IResponse.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `context: any` bag | `ContextState` struct plus overflow map for dynamic fields | Need typed core fields but must still tolerate autorest injecting arbitrary extras. |
| `IHandlerParameters = { [key: string]: any }` | `BTreeMap<String, serde_json::Value>` or dedicated enum-backed map | Handler middleware indexes parameters dynamically by name strings. |
| `handlerResponses: any` | `serde_json::Value` / response enum / boxed trait object | Returned handler payload shape varies by operation and may include streams. |
| `holder: object` + `path` | `shared extension map keyed by path string` | Express implementation stores context in `res.locals[contextPath]`. |
## `any` hotspots
- `6: [key: string]: any;`
- `17: public readonly context: any;`
- `58: const context = holderOrContext as any;`
- `120: public get handlerResponses(): any {`
- `124: public set handlerResponses(handlerResponses: any) {`

## Generated-code notes
- Constructor overloads support either cloning from an existing `Context` or projecting onto `holder[path]`; Express uses the holder-path route every time.
- `contextId` getter/setter actually read and write `context.contextID`, preserving the historical `contextID` casing mismatch while exposing the public property as `contextId`.
- Core fields carried in the bag: `operation`, `request`, `response`, `dispatchPattern`, `handlerParameters`, `handlerResponses`, `contextID`, and `startTime`.

## Middleware chain ordering
- Shared state object across all six generated middleware stages.
- `dispatch` writes `operation`; `deserializer` writes `handlerParameters`; handler middleware writes `handlerResponses`; `end` reads `startTime`.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest adds new context fields, surface them explicitly here rather than hiding them in an untracked Rust side map.
