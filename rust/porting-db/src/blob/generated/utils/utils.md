# Porting Record — `src/blob/generated/utils/utils.ts`

## File info
- Source path: `src/blob/generated/utils/utils.ts`
- Source lines: `20`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/utils/utils.rs`
- Crate: `azurite-blob`
- Module: `generated::utils::utils`
- Phase: `5.4`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export function isURITemplateMatch(url: string, template: string): boolean {
  const uriTemplate = URITemplate(template);
  // TODO: Fixing $ parsing issue such as $logs container cannot work in strict mode issue
  const result = (uriTemplate.fromUri as any)(url, { strict: true });
  if (result === undefined) {
    return false;
  }

  for (const key in result) {
    if (result.hasOwnProperty(key)) {
      const element = result[key];
      if (element === "") {
        return false;
      }
    }
  }
  return true;
}
```

## Dependencies
- External packages:
  - `uri-templates` — imported as `URITemplate`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `uri-templates` matcher result cast via `as any` | `HashMap<String, String>` or typed URI-template match object | Need a compatibility layer because the TS library exposes an untyped `fromUri()` surface. |
| `boolean` return | `bool` | False means either no URI match or at least one empty captured segment. |
## `any` hotspots
- `6: const result = (uriTemplate.fromUri as any)(url, { strict: true });`

## Generated-code notes
- `isURITemplateMatch()` is the dispatch helper shared by all generated operations. It calls `uriTemplate.fromUri(url, { strict: true })` through an `any` cast and rejects matches that contain empty-string captures.
- The TODO about `$` parsing (`$logs`) is observable generated behavior today; do not silently "fix" it in Rust without a recorded divergence.

## Middleware chain ordering
- Called inside dispatch middleware before any handler/auth logic.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest or the URI-template library changes strict-matching semantics, revisit dispatch scoring because route selection depends on this exact helper.
