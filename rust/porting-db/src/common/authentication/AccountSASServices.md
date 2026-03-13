# Porting Record — `src/common/authentication/AccountSASServices.ts`

## File info
- Source path: `src/common/authentication/AccountSASServices.ts`
- Source lines: `121`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/authentication/account_sas_services.rs`
- Crate: `azurite-common`
- Module: `authentication::account_sas_services`
- Phase: `3.3`
- Status: `analyzed`

## Exported API
### Enum `AccountSASService`
- `Blob = "b"`
- `File = "f"`
- `Queue = "q"`
- `Table = "t"`

### Default class `AccountSASServices`
- **Static method:** `parse(services: string): AccountSASServices`
- **Public flags:**
  - `blob: boolean`
  - `file: boolean`
  - `queue: boolean`
  - `table: boolean`
- **Instance method:** `toString(): string`

## Dependencies
- Imports: none.
- Porting status:
  - No direct imports.
- Key consumers:
  - `src/common/authentication/IAccountSASSignatureValues.ts` (Phase 3.5; analyzed in this pass) calls `.toString()` on this helper or a raw string.
  - `src/blob/authentication/OperationAccountSASPermission.ts`, `src/queue/authentication/OperationAccountSASPermission.ts`, and `src/table/authentication/OperationAccountSASPermission.ts` (later phases) use the enum members to build per-operation requirements.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum `AccountSASService` | `enum AccountSASService` with `as_str() -> &'static str` | String-backed single-character service identifiers. |
| default class with public booleans | `pub struct AccountSASServices` with explicit bool fields | Keeps the helper object close to TS. |
| `parse(services: string)` | `pub fn parse(services: &str) -> Result<Self, AccountSasError>` | Preserve duplicate and invalid-character errors. |
| `toString(): string` | `impl Display` plus canonical helper | Output order is behaviorally significant. |

## Function and method mappings
- `AccountSASServices.parse(services: string): AccountSASServices`
  - Rust: `pub fn parse(services: &str) -> Result<Self, AccountSasError>`
  - Behavior: iterate each character, set booleans, reject duplicates and unknown characters immediately.
- `AccountSASServices#toString(): string`
  - Rust: `pub fn to_string_ordered(&self) -> String` and `impl Display`
  - Behavior: emit services in exact TS order `btqf`.

## Recommended Rust translation
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountSASService {
    Blob,
    File,
    Queue,
    Table,
}

impl AccountSASService {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blob => "b",
            Self::File => "f",
            Self::Queue => "q",
            Self::Table => "t",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccountSASServices {
    pub blob: bool,
    pub file: bool,
    pub queue: bool,
    pub table: bool,
}
```

## Special handling
- Despite the doc comment saying "ONLY AVAILABLE IN NODE.JS RUNTIME", the implementation is pure string parsing/formatting with no Node-only APIs. Rust can translate it directly as normal common-library code.
- `toString()` order is **not** enum declaration order. TS emits `blob`, then `table`, then `queue`, then `file` (`btqf`). Preserve that exact order.
- Duplicate detection throws `RangeError("Duplicated permission character: ${c}")`; note the message says "permission character" even though this file parses services. Preserve this wording if error text must stay stable.
- Empty string is valid and yields an all-false helper object.
- No trimming or normalization is applied to input.

## Change propagation notes
- If Azure adds new account-level services, update both the enum and the canonical serializer order intentionally; do not append blindly.
- Any serializer-order change must be audited in `IAccountSASSignatureValues.ts`, because signature generation trusts the helper's exact byte order.
- Later operation-permission tables depend on the one-character enum values directly, so changes here ripple into blob, queue, and table authentication phases.
