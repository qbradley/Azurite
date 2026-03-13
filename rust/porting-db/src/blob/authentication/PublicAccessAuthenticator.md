# Porting Record — `src/blob/authentication/PublicAccessAuthenticator.ts`

## File info
- Source path: `src/blob/authentication/PublicAccessAuthenticator.ts`
- Source lines: `149`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/public_access_authenticator.rs`
- Crate: `azurite-blob`
- Module: `authentication::public_access_authenticator`
- Phase: `7.14`
- Status: `not_started`

## Exported API
### Default class `PublicAccessAuthenticator`
- Implements `IAuthenticator`.
- Constructor:
  - `new PublicAccessAuthenticator(blobMetadataStore: IBlobMetadataStore, logger: ILogger)`
- Public method:
  - `validate(req: IRequest, context: Context): Promise<boolean | undefined>`

### Internal helper
- `getContainerPublicAccessType(account, container, context): Promise<PublicAccessType | undefined>`

### Internal allowlists
- `CONTAINER_PUBLIC_READ_OPERATIONS`
- `BLOB_PUBLIC_READ_OPERATIONS`

## Dependencies
- Imports:
  - `../../common/ILogger`
  - generated `PublicAccessType`, `Operation`, `Context`, `IRequest`
  - `../persistence/IBlobMetadataStore`
  - `./IAuthenticator`
- Key consumers:
  - Blob authentication middleware/request-listener chain, typically as the permissive fallback after stronger auth modes.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| authenticator class | struct with metadata store and logger | Keep allowlists as explicit static sets. |
| `Set<Operation>` | `Lazy<HashSet<Operation>>` | Operation membership is the whole policy surface here. |
| `Promise<boolean | undefined>` | `Result<Option<bool>, StorageError>` | Returns `None` when public access does not apply. |

## Special handling
- Public access only applies when `containerName` is present; otherwise validation returns `undefined`.
- `getContainerPublicAccessType()` catches all metadata-store errors and returns `undefined` instead of propagating them.
- `PublicAccessType.Container` and `PublicAccessType.Blob` map to different read allowlists.
- The allowlists include TODO comments questioning `PageBlob_GetPageRanges`, `PageBlob_GetPageRangesDiff`, and `BlockBlob_GetBlockList`. Preserve those uncertain entries as source truth.
- When the container has a known public access level but the operation is not allowlisted, `validate()` returns `undefined`, not `false`. The TODO comment explicitly frames this as the “not this validation pattern, go to next authenticator” case.
- Unsupported `containerPublicAccessType` values throw an error rather than failing closed quietly.

## Change propagation notes
- If TS changes the public-read allowlists, update the exact sets here rather than deriving permissions from `PublicAccessType` semantics.
- If metadata-store error handling changes from swallow-and-skip to hard-fail, document that as a real auth behavior change.
