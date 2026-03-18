# Porting Record — `src/queue/context/QueueStorageContext.ts`

## File info
- Source path: `src/queue/context/QueueStorageContext.ts`
- Source lines: `61`
- Source type: `handwritten wrapper class`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/context/queue_storage_context.rs`
- Crate: `azurite-queue`
- Module: `context::queue_storage_context`
- Phase: `14.5`
- Status: `ported`

## Exported API
### Default class `QueueStorageContext`
- Extends `Context` (Phase 5 generated)
- Implements `IAuthenticationContext` (Phase 14 authentication)
- Constructor: Inherited from Context (takes generated context fields)
- Public property accessors (getter/setter pairs):
  - `account: string | undefined`
  - `isSecondary: boolean | undefined`
  - `queue: string | undefined`
  - `message: string | undefined`
  - `messageId: string | undefined`
  - `authenticationPath: string | undefined`
  - `xMsRequestID: string | undefined` (maps to internal `contextID` field)

## Dependencies
- `IAuthenticationContext` (Phase 14.6 authentication): Interface contract
- `Context` (Phase 5 generated): Base class providing internal context object
- Inherits from queue-generated context layer

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `extends Context` | Struct composition or trait implementation | QueueStorageContext wraps Context |
| `implements IAuthenticationContext` | Trait implementation | Authentication-specific accessors |
| `this.context.X` | Delegation to inner context field | Thin wrapper pattern |
| `this.contextID` | Delegation to inherited field | Parent class field access |
| `get X(): Type / set X(value: Type)` | Rust getter/setter methods or struct fields | Property accessor pattern |

## Special handling
1. **Wrapper pattern** (`QueueStorageContext.ts:4-5`)
   - Extends `Context` class (not standalone)
   - Adds queue-specific context properties on top of base
   - Constructor inherited — no custom initialization

2. **Context field delegation** (`QueueStorageContext.ts:throughout`)
   - All properties are getters/setters that directly delegate to `this.context.*`
   - No computation, validation, or caching
   - Thin adapter layer between interface and storage

3. **xMsRequestID special case** (`QueueStorageContext.ts:54-60`)
   - Property named `xMsRequestID` (HTTP header convention)
   - Actually maps to inherited `this.contextID` field from parent Context
   - Pattern: `context.xMsRequestID` → `context.contextID` (parent field)
   - This bridges a naming asymmetry (HTTP vs internal names)

4. **Queue-specific context properties**
   - `queue`: Queue name being accessed
   - `message`: Message content or ID under operation
   - `messageId`: Specific message ID (distinct from `message`)
   - `authenticationPath`: Request path used for SAS authentication
   - All optional (undefined allowed)

5. **Authentication context contract**
   - Implements `IAuthenticationContext` for auth middleware integration
   - Middleware uses these properties to validate permissions and log access
   - Account/isSecondary used for shared-key/SAS auth routing

## Change propagation notes
- If Context base class adds new fields, wrap them in QueueStorageContext too
- Keep wrapper pattern consistent with blob-layer equivalents (BlobStorageContext from Phase 11)
- If IAuthenticationContext interface changes, update all property accessors to match
- The xMsRequestID→contextID mapping is a TS quirk; preserve exact field names
- All properties are optional (undefined valid) — do not add required fields without contract review

