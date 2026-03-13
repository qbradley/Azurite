# Porting Record — `src/blob/generated/utils/xml.ts`

## File info
- Source path: `src/blob/generated/utils/xml.ts`
- Source lines: `41`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/utils/xml.rs`
- Crate: `azurite-blob`
- Module: `generated::utils::xml`
- Phase: `5.6`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export function stringifyXML(obj: any, opts?: { rootName?: string }) {
  const builder = new xml2js.Builder({
    explicitArray: false,
    explicitCharkey: false,
    renderOpts: {
      pretty: false
    },
    rootName: (opts || {}).rootName
  });
  return builder.buildObject(obj);
}

export function parseXML(
  str: string,
  explicitChildrenWithOrder: boolean = false
): Promise<any> {
  const xmlParser = new xml2js.Parser({
    explicitArray: false,
    explicitCharkey: false,
    explicitRoot: false,
    preserveChildrenOrder: explicitChildrenWithOrder,
    explicitChildren: explicitChildrenWithOrder,
    emptyTag: undefined
  });
  return new Promise((resolve, reject) => {
    xmlParser.parseString(str, (err?: Error, res?: any) => {
      if (err) {
        reject(err);
      } else {
        resolve(res);
      }
    });
  });
}

export function jsonToXML(json: any): string {
  const build = new xml2js.Builder();
  return build.buildObject(json);
}
```

## Dependencies
- External packages:
  - `xml2js` — imported as `* as xml2js`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `xml2js.Builder` / `xml2js.Parser` | `quick_xml`/custom XML adapter layer | Preserve parser/builder option values exactly. |
| `any` XML payloads | `serde_json::Value` or explicit XML DOM helper | Generated serializer/deserializer treat XML bodies as shape-flexible data. |
| `Promise<any>` | `async fn -> serde_json::Value` | XML parse remains asynchronous because middleware awaits it. |
## `any` hotspots
- `3: export function stringifyXML(obj: any, opts?: { rootName?: string }) {`
- `18: ): Promise<any> {`
- `28: xmlParser.parseString(str, (err?: Error, res?: any) => {`
- `38: export function jsonToXML(json: any): string {`

## Generated-code notes
- `stringifyXML()` uses `explicitArray: false`, `explicitCharkey: false`, `renderOpts.pretty: false`, and optional `rootName`.
- `parseXML()` uses `explicitArray: false`, `explicitCharkey: false`, `explicitRoot: false`, `preserveChildrenOrder`/`explicitChildren` toggled together, and `emptyTag: undefined`; these options shape every XML request body.

## Middleware chain ordering
- Called by generated serializer/deserializer; not itself a middleware stage.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If parser options or XML library behavior changes, audit `serializer.ts`, `mappers.ts`, and XML response tests together because body shape is contract-sensitive.
