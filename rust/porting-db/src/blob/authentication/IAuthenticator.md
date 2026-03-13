# Porting Record — `src/blob/authentication/IAuthenticator.ts`

## File info
- Source path: `src/blob/authentication/IAuthenticator.ts`
- Source lines: `6`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/i_authenticator.rs`
- Crate: `azurite-blob`
- Module: `authentication::i_authenticator`
- Phase: `7.1`
- Status: `not_started`

## Exported API
### Default interface `IAuthenticator`
- Single method:
  - `validate(req: IRequest, content: Context): Promise<boolean | undefined>`

## Dependencies
- Imports:
  - `../generated/Context` — Phase `5.7`.
  - `../generated/IRequest` — Phase `5.1` generated request interface.
- Key implementers:
  - `BlobSharedKeyAuthenticator`
  - `AccountSASAuthenticator`
  - `BlobSASAuthenticator`
  - `BlobTokenAuthenticator`
  - `PublicAccessAuthenticator`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| interface with async method | `#[async_trait] pub trait IAuthenticator` | Keep the trait small and chain-oriented. |
| `Promise<boolean | undefined>` | `Result<Option<bool>, StorageError>` or similar | `Some(true)` = matched/passed, `Some(false)` = matched/failed, `None` = authenticator not applicable. |

## Special handling
- The tri-state return is the contract: this file underpins the blob authentication chain-of-responsibility.
- The second parameter is named `content` rather than `context`; that typo is not behaviorally important but is part of the source surface.

## Change propagation notes
- Any signature change here ripples to all concrete authenticators and the middleware that composes them.
- Keep the Rust abstraction narrow; later authenticators throw `StorageErrorFactory` errors directly, so the trait should not hide the difference between “not applicable” and “hard failure”.
