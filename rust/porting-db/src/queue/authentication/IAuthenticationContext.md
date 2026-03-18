# Porting Record — `src/queue/authentication/IAuthenticationContext.ts`

## File info
- Source path: `src/queue/authentication/IAuthenticationContext.ts`
- Source lines: `3`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/i_authentication_context.rs`
- Crate: `azurite-queue`
- Module: `authentication::i_authentication_context`
- Status: `ported`

## Special handling
Marker trait for authentication context. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/IAuthenticationContext.md` for detailed fidelity notes.
