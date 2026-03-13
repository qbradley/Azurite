# Porting Record — `src/common/authentication/AccountSASResourceTypes.ts`

## File info
- Source path: `src/common/authentication/AccountSASResourceTypes.ts`
- Source lines: `104`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/authentication/account_sas_resource_types.rs`
- Crate: `azurite-common`
- Module: `authentication::account_sas_resource_types`
- Phase: `3.4`
- Status: `ported`

## Exported API
### Enum `AccountSASResourceType`
- `Service = "s"`
- `Container = "c"`
- `Object = "o"`
- `Any = "AnyResourceType"` — sentinel used later by blob batch authorization logic, not by the parser/serializer in this file.

### Default class `AccountSASResourceTypes`
- **Static method:** `parse(resourceTypes: string): AccountSASResourceTypes`
- **Public flags:**
  - `service: boolean`
  - `container: boolean`
  - `object: boolean`
- **Instance method:** `toString(): string`

## Dependencies
- Imports: none.
- Porting status:
  - No direct imports.
- Key consumers:
  - `src/common/authentication/IAccountSASSignatureValues.ts` (Phase 3.5; analyzed in this pass) calls `.toString()` on this helper or a raw string.
  - `src/blob/authentication/OperationAccountSASPermission.ts` (later phase) compares raw resource type strings against `AccountSASResourceType.Any` and the one-character enum members.
  - `src/queue/authentication/OperationAccountSASPermission.ts` and `src/table/authentication/OperationAccountSASPermission.ts` (later phases) consume the one-character enum members.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum `AccountSASResourceType` | `enum AccountSASResourceType` with `as_str() -> &'static str` | Needed because `Any` is a multi-character sentinel. |
| default class with public booleans | `pub struct AccountSASResourceTypes` | Preserve field-level fidelity. |
| `parse(resourceTypes: string)` | `pub fn parse(resource_types: &str) -> Result<Self, AccountSasError>` | Preserve duplicate and invalid-character failures. |
| `toString(): string` | `impl Display` plus canonical helper | Canonical output is `sco`. |

## Function and method mappings
- `AccountSASResourceTypes.parse(resourceTypes: string): AccountSASResourceTypes`
  - Rust: `pub fn parse(resource_types: &str) -> Result<Self, AccountSasError>`
  - Behavior: iterate each character, set booleans, reject duplicates and unknown resource types.
- `AccountSASResourceTypes#toString(): string`
  - Rust: `pub fn to_string_ordered(&self) -> String` and `impl Display`
  - Behavior: emit resource types in exact TS order `sco`.

## Recommended Rust translation
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountSASResourceType {
    Service,
    Container,
    Object,
    Any,
}

impl AccountSASResourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Service => "s",
            Self::Container => "c",
            Self::Object => "o",
            Self::Any => "AnyResourceType",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccountSASResourceTypes {
    pub service: bool,
    pub container: bool,
    pub object: bool,
}
```

## Special handling
- `AccountSASResourceType.Any` is a validation-only sentinel for later blob batch authorization code. This file never parses or serializes it, so Rust should keep it separate from canonical parse/display logic.
- Duplicate detection throws `RangeError("Duplicated permission character: ${c}")`; the message uses "permission character" even though this file parses resource types. Preserve the quirk if exact error text matters.
- `toString()` order is the same as the enum's canonical account-SAS order: `service`, then `container`, then `object` (`sco`).
- Empty string is valid and yields an all-false helper object.
- No trimming, sorting, or normalization is applied to the incoming string.

## Change propagation notes
- Any new account-level resource type must update the enum, parser switch, canonical serializer order, and later blob/queue/table operation permission validators.
- If the blob batch `AnyResourceType` sentinel changes semantics, update this record and the later blob `OperationAccountSASPermission` record together.
- Any serializer-order change must be audited in `IAccountSASSignatureValues.ts`, because signature generation consumes this helper's exact byte sequence.

## Rust port notes
- Ported in `rust/crates/azurite-common/src/authentication/account_sas_resource_types.rs` with the TS field names and canonical `sco` serializer order intact.
- `AccountSASResourceType::Any` remains validation-only and is excluded from parse/display logic.
- Preserved the TS duplicate-error wording quirk while adding serde string renames for later union handling.
