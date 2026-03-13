# Porting Record — `src/common/authentication/AccountSASPermissions.ts`

## File info
- Source path: `src/common/authentication/AccountSASPermissions.ts`
- Source lines: `290`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/authentication/account_sas_permissions.rs`
- Crate: `azurite-common`
- Module: `authentication::account_sas_permissions`
- Phase: `3.2`
- Status: `analyzed`

## Exported API
### Enum `AccountSASPermission`
- `Read = "r"`
- `Write = "w"`
- `Delete = "d"`
- `DeleteVersion = "x"`
- `List = "l"`
- `Add = "a"`
- `Create = "c"`
- `Update = "u"`
- `Process = "p"`
- `Tag = "t"`
- `Filter = "f"`
- `SetImmutabilityPolicy = "i"`
- `PermanentDelete = "y"`
- `Any = "AnyPermission"` — sentinel used later by blob batch permission validation, not by the parser/serializer in this file.

### Default class `AccountSASPermissions`
- **Static method:** `parse(permissions: string): AccountSASPermissions`
- **Public flags:**
  - `read: boolean`
  - `write: boolean`
  - `delete: boolean`
  - `deleteVersion: boolean`
  - `list: boolean`
  - `add: boolean`
  - `create: boolean`
  - `update: boolean`
  - `process: boolean`
  - `tag: boolean`
  - `filter: boolean`
  - `setImmutabilityPolicy: boolean`
  - `permanentDelete: boolean`
- **Instance method:** `toString(): string`

## Dependencies
- Imports: none.
- Porting status:
  - No direct imports.
- Key consumers:
  - `src/common/authentication/IAccountSASSignatureValues.ts` (Phase 3.5; analyzed in this pass) calls `.toString()` on either this helper or a raw string.
  - `src/blob/authentication/OperationAccountSASPermission.ts` (later phase) compares raw permission strings against `AccountSASPermission.Any` and the one-character enum members.
  - `src/queue/authentication/OperationAccountSASPermission.ts` and `src/table/authentication/OperationAccountSASPermission.ts` (later phases) consume the one-character enum members.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum `AccountSASPermission` | `enum AccountSASPermission` with `as_str() -> &'static str` | Needed because `Any` is multi-character while the others are single-character. |
| default class with public booleans | `pub struct AccountSASPermissions` with explicit bool fields | Preserve the mutable helper-object surface instead of collapsing to bitflags. |
| `boolean` flags | `bool` | Default all flags to `false`. |
| `parse(permissions: string)` | `pub fn parse(permissions: &str) -> Result<Self, AccountSasError>` | Preserve duplicate/invalid-character failures. |
| `toString(): string` | `impl Display` plus inherent `to_string_ordered(&self) -> String` | Canonical string order is behaviorally significant. |
| `RangeError` | custom parse error or shared `StorageError` | Keep duplicate vs invalid cases distinct if possible. |

## Function and method mappings
- `AccountSASPermissions.parse(permissions: string): AccountSASPermissions`
  - Rust: `pub fn parse(permissions: &str) -> Result<Self, AccountSasError>`
  - Behavior: iterate each character, reject duplicates immediately, reject unknown characters immediately, allow empty string.
- `AccountSASPermissions#toString(): string`
  - Rust: `pub fn to_string_ordered(&self) -> String` and `impl Display`
  - Behavior: emit permissions in exact TS order `rwdxlacuptfiy`, regardless of input order used when parsing or mutating fields.

## Recommended Rust translation
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountSASPermission {
    Read,
    Write,
    Delete,
    DeleteVersion,
    List,
    Add,
    Create,
    Update,
    Process,
    Tag,
    Filter,
    SetImmutabilityPolicy,
    PermanentDelete,
    Any,
}

impl AccountSASPermission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "r",
            Self::Write => "w",
            Self::Delete => "d",
            Self::DeleteVersion => "x",
            Self::List => "l",
            Self::Add => "a",
            Self::Create => "c",
            Self::Update => "u",
            Self::Process => "p",
            Self::Tag => "t",
            Self::Filter => "f",
            Self::SetImmutabilityPolicy => "i",
            Self::PermanentDelete => "y",
            Self::Any => "AnyPermission",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccountSASPermissions {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub delete_version: bool,
    pub list: bool,
    pub add: bool,
    pub create: bool,
    pub update: bool,
    pub process: bool,
    pub tag: bool,
    pub filter: bool,
    pub set_immutability_policy: bool,
    pub permanent_delete: bool,
}
```

## Special handling
- `AccountSASPermission.Any` is a validation-only sentinel for later blob batch authorization code. This file never parses or serializes it, so Rust should preserve it as a non-canonical helper constant/variant rather than folding it into normal parse or display logic.
- `toString()` order is fixed and canonical: `r`, `w`, `d`, `x`, `l`, `a`, `c`, `u`, `p`, `t`, `f`, `i`, `y`. Reordering these characters will change generated SAS strings.
- `parse()` accepts any incoming order but throws on duplicates. Empty string is valid and produces an instance with every flag still `false`.
- TS exposes all flags as mutable public fields. Prefer an explicit Rust struct over compact bitflags so field-by-field future TS changes remain easy to propagate.
- The parser does not trim or lowercase input. Preserve exact character matching.

## Change propagation notes
- If TS adds a new permission, update the enum, parser switch, canonical serializer order, and every later `OperationAccountSASPermission` table that relies on character membership.
- If blob batch semantics around `AnyPermission` change, update both this record and the later blob authentication permission validator record together.
- Any behavioral change in `toString()` must be audited in `IAccountSASSignatureValues.ts`, because the signature generator trusts this helper's canonical ordering.
