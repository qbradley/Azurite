# Porting Record — `src/blob/errors/StrictModelNotSupportedError.ts`

## File info
- Source path: `src/blob/errors/StrictModelNotSupportedError.ts`
- Source lines: `12`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/errors/strict_model_error.rs`
- Crate: `azurite-blob`
- Module: `errors::strict_model_error`
- Phase: `6.4`
- Status: `ported`

## Exported API
### Default class `StrictModelNotSupportedError`
- Extends `StorageError`.
- Constructor: `new StrictModelNotSupportedError(feature: string, requestID: string = "")`
- Fixed payload:
  - status `500`
  - code `FeatureNotSupported`
  - message template `${feature} header or parameter is not supported in Azurite strict mode. Switch to loose model by Azurite command line parameter "--loose" or Visual Studio Code configuration "Loose". Please vote your wanted features to https://github.com/azure/azurite/issues`

## Dependencies
- Imports:
  - `./StorageError` — Phase `6.1`.
- Key consumers:
  - `AccountSASAuthenticator.ts` and `BlobSASAuthenticator.ts` throw it when `ses` is present outside loose mode.
  - Strict-mode middleware can reuse it for other unsupported headers/parameters.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| tiny subclass with one dynamic field | helper constructor over `StorageError` | No extra behavior beyond message interpolation. |
| `feature: string` | `&str` / `String` | Preserve caller-supplied text verbatim in the user-facing message. |

## Special handling
- Although the failure is triggered by unsupported request syntax, TS returns HTTP `500`, not `400` or `409`. Preserve that observable behavior unless the team explicitly decides otherwise.
- The message embeds both the CLI flag `--loose` and the VS Code setting `Loose`; those exact strings are part of the guidance surface.

## Change propagation notes
- Any new strict-mode incompatibility should either reuse this helper or document why a different error code is required.
- If the team ever decides to normalize strict-mode failures to a 4xx class, record that as an explicit decision before touching the Rust translation.
