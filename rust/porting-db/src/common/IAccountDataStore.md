# Porting Record — `src/common/IAccountDataStore.ts`

## File info
- Source path: `src/common/IAccountDataStore.ts`
- Source lines: `17`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_account_data_store.rs`
- Crate: `azurite-common`
- Module: `i_account_data_store`
- Phase: `1.6`
- Status: `ported`

## Exported API
### Interface `IAccountProperties`
- `name: string`
- `key1: Buffer`
- `key2?: Buffer`

### Default interface `IAccountDataStore extends IDataStore, ICleaner`
- `getAccount(name: string): IAccountProperties | undefined`

## Dependencies
- `./ICleaner`
- `./IDataStore`
- Important downstream users: shared key, SAS, and token authenticators across blob/queue/table.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `string` | `String` / `&str` | Account names are lookup keys. |
| `Buffer` | `Vec<u8>` | Suitable for owned account keys decoded from base64. |
| `Buffer | undefined` | `Option<Vec<u8>>` | Optional secondary key. |
| `IAccountProperties | undefined` | `Option<AccountProperties>` | Missing account is a cache miss, not an exception. |

## Recommended Rust translation
```rust
pub struct AccountProperties {
    pub name: String,
    pub key1: Vec<u8>,
    pub key2: Option<Vec<u8>>,
}

#[async_trait]
pub trait AccountDataStore: DataStore + Cleaner + Send + Sync {
    fn get_account(&self, name: &str) -> Option<AccountProperties>;
}
```

## Special handling
- Preserve multiple inheritance (`IDataStore` + `ICleaner`) as Rust supertraits.
- `getAccount()` is synchronous in TS and used on hot authentication paths; keep it sync unless a future TS change makes lookup async.
- `AccountDataStore.ts` decodes keys from base64 into `Buffer`s and quietly returns `undefined` for missing accounts. Do not change that behavior into exception-driven lookup.

## Change propagation notes
- If TS adds fields to `IAccountProperties`, audit every authenticator and SAS path that reads account keys.
- If lookup becomes async later, revisit authentication call sites before changing the Rust trait surface.

## Rust port notes
- Ported to `rust/crates/azurite-common/src/i_account_data_store.rs` with `IAccountProperties` and `IAccountDataStore`.
- Kept `getAccount()` synchronous and Option-returning to match the TypeScript hot-path authentication usage.
