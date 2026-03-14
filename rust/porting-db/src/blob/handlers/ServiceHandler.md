# Porting Record — `src/blob/handlers/ServiceHandler.ts`

## File info
- Source path: `src/blob/handlers/ServiceHandler.ts`
- Source lines: `416`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/service_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::service_handler`
- Phase: `11.2`
- Status: `not_started`

## Exported API
### Default class `ServiceHandler extends BaseHandler implements IServiceHandler`
- Constructor injects `accountDataStore`, `oauth`, shared stores/logger/loose, and optional `disableProductStyle`.
- Methods:
  - `getUserDelegationKey()`
  - `submitBatch()`
  - `setProperties()` / `getProperties()`
  - `getStatistics()`
  - `listContainersSegment()`
  - `getAccountInfo()` / `getAccountInfoWithHead()`
  - `filterBlobs()`

## Dependencies
- `IAccountDataStore`, `OAuthLevel`, `jsonwebtoken.decode`, `getUserDelegationKeyValue()`.
- `BlobBatchHandler` for service-level batch requests.
- `parseXML()` plus generated blob models/handler interface.
- `IBlobMetadataStore` for service properties, container listing, and tag-query filtering.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| stateful handler class | concrete struct embedding `BaseHandler` state plus account/oauth fields | Keep direct field injection instead of over-abstracting service operations. |
| inline `defaultServiceProperties` object | constant/default builder function | Preserve exact default values and missing-vs-empty distinctions. |
| bearer token decode without verification | explicit JWT payload decode helper | This code only decodes claims; signature validation already happened in auth middleware. |

## Special handling
- `ServiceHandler.ts:57-86` hard-codes default service properties; Rust should preserve these literal defaults for missing persisted service settings.
- `ServiceHandler.ts:88-117` derives the user delegation key from decoded bearer-token `oid`/`tid` claims without re-validating the JWT.
- `ServiceHandler.ts:121-154` delegates service batch requests to `BlobBatchHandler` with an empty subrequest path prefix and returns `202` even though the generated swagger shape expected `200`.
- `ServiceHandler.ts:174-183` reparses raw XML to detect whether `cors` was omitted, because the deserializer turns missing CORS into `[]`; that asymmetry is observable.
- `ServiceHandler.ts:185-190` normalizes empty CORS `allowedHeaders` and `exposedHeaders` to empty strings, not `undefined`.
- `ServiceHandler.ts:229-235` checks `properties.cors === undefined` twice; preserve the duplicated TS shape in the record even if Rust implementation compresses it internally.
- `ServiceHandler.ts:272-289` only allows `getStatistics()` on secondary endpoints (`context.context.isSecondary`).
- `ServiceHandler.ts:322-349` exposes container metadata only when `include` explicitly contains `metadata`; everything else stays suppressed.
- `ServiceHandler.ts:332` still carries the TODO about stale lease properties when listing containers.

## Change propagation notes
- Service batch behavior spans this file and `BlobBatchHandler.ts`; update both together if TS adds more allowed batch operations.
- If service-properties XML generation/deserialization changes upstream, revisit both the raw-body workaround in `setProperties()` and the default-filling logic in `getProperties()`.
