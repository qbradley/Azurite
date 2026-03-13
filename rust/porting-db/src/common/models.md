# Porting Record — `src/common/models.ts`

## File info
- Source path: `src/common/models.ts`
- Source lines: `3`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/models.rs`
- Crate: `azurite-common`
- Module: `models`
- Phase: `1.5`
- Status: `analyzed`

## Exported API
### Enum `OAuthLevel`
- `BASIC` (numeric enum member, value `0` in TS)

## Dependencies
- Imports: none.
- Downstream users: `ConfigurationBase.getOAuthLevel()`, blob/queue/table request listener factories, and token authenticators.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| numeric enum `OAuthLevel` | `enum OAuthLevel { Basic }` | Preserve the single supported mode. |
| CLI string `"basic"` | `impl FromStr for OAuthLevel` | `ConfigurationBase` lowercases input before mapping to `BASIC`. |

## Recommended Rust translation
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OAuthLevel {
    Basic,
}

impl std::str::FromStr for OAuthLevel {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "basic" => Ok(Self::Basic),
            _ => Err(StorageError::invalid_header_value(/* context */ "")),
        }
    }
}
```

## Special handling
- TS currently exposes only a single OAuth level, but it is already threaded through configuration and authenticators as an enum rather than a bool. Preserve that extensibility.
- The TS enum is numeric, but the public-facing configuration input is string-based (`"basic"`). The Rust port should document both representations.

## Change propagation notes
- Any new enum member must be audited through `ConfigurationBase`, all request listener factories, and service token authenticators.
- If the TS project changes CLI strings without changing enum members, update the Rust string parser but keep enum names aligned with TS.
