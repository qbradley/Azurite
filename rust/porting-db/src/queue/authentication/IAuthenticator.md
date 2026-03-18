# Porting Record — `src/queue/authentication/IAuthenticator.ts`

## File info
- Source path: `src/queue/authentication/IAuthenticator.ts`
- Source lines: `6`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/i_authenticator.rs`
- Crate: `azurite-queue`
- Module: `authentication::i_authenticator`
- Status: `ported`

## Special handling
Core authenticator trait with `validate()` method. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/IAuthenticator.md` for detailed fidelity notes.
