# Porting Record — `src/common/AccountDataStore.ts`

## File info
- Source path: `src/common/AccountDataStore.ts`
- Source lines: `126`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/account_data_store.rs`
- Crate: `azurite-common`
- Module: `account_data_store`
- Phase: `4.9`
- Status: `ported`

## Exported API
### Default class `AccountDataStore implements IAccountDataStore`
- Constructor: `constructor(logger: ILogger)`
- Method: `getAccount(name: string): IAccountProperties | undefined`
- Method: `init(): Promise<void>`
- Method: `isInitialized(): boolean`
- Method: `close(): Promise<void>`
- Method: `isClosed(): boolean`
- Method: `clean(): Promise<void>`
- Private helper: `refresh(): void`
- Private helper: `parserAccountsEnvironmentString(accounts: string): IAccounts`

### Internal-only constructs that matter to translation
- Private enum `Status { Initializing, Initialized, Closing, Closed }`
- Private interface `IAccounts { [key: string]: IAccountProperties }`
- Private constant `DEFAULT_EMULATOR_ACCOUNTS: IAccounts`

## Dependencies
- Internal imports:
  - `../common/utils/constants` (`EMULATOR_ACCOUNT_KEY`, `EMULATOR_ACCOUNT_NAME`) — Phase `4.1`, analyzed in this pass.
  - `./IAccountDataStore` — Phase `1.6`, already ported.
  - `./utils/constants` (`AZURITE_ACCOUNTS_ENV`, `DEFAULT_ACCOUNTS_REFRESH_INTERVAL`) — Phase `4.1`, analyzed in this pass.
- Cross-layer import:
  - `../queue/generated/utils/ILogger` — `PORTING-ORDER` Phase `5.3`, not yet ported. The file depends on the queue-generated logger interface instead of `src/common/ILogger.ts`.
- Runtime dependency: `process.env`, `setInterval`, `clearInterval`, `Buffer.from(..., "base64")`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string-keyed object map | `HashMap<String, IAccountProperties>` | Closest match to TS index-signature storage. |
| `Buffer` account keys | `Vec<u8>` | Preserve raw decoded key bytes. |
| `IAccountProperties | undefined` | `Option<AccountProperties>` | Missing account is not an error. |
| timer handle typed as `any` | `tokio::task::JoinHandle<()>` / interval guard | Need explicit lifecycle in Rust. |
| private enum `Status` | internal Rust enum | Only `Initialized` and `Closed` are meaningfully used today. |

## Recommended Rust translation
- Keep this as a concrete in-memory account store with a refresh loop driven by `AZURITE_ACCOUNTS`.
- Use a shared `HashMap<String, AccountProperties>` behind `RwLock` if the store is accessed concurrently.
- Preserve the timer-driven refresh behavior rather than re-reading the environment only once at startup.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| constructor | `fn new(logger: Arc<dyn ILogger>) -> Self` | Preserve logger injection rather than reaching for the global logger. |
| `getAccount()` | `fn get_account(&self, name: &str) -> Option<AccountProperties>` | Sync lookup on hot auth path. |
| `init()` | `async fn init(&mut self) -> Result<()>` | Call `refresh()` immediately, then start periodic refresh every 60 seconds. |
| `isInitialized()` | `fn is_initialized(&self) -> bool` | Direct state check. |
| `close()` | `async fn close(&mut self) -> Result<()>` | Stop refresh loop and mark closed. |
| `isClosed()` | `fn is_closed(&self) -> bool` | Direct state check. |
| `clean()` | `async fn clean(&self) -> Result<()>` | No-op by default. |
| `refresh()` | private refresh helper | Reads env, logs masked presence, and swaps account map or falls back to defaults. |
| `parserAccountsEnvironmentString()` | `fn parse_accounts_environment_string(input: &str) -> Result<HashMap<String, AccountProperties>, RangeError>` | Preserve `account:key1[:key2];...` grammar and permissive trailing semicolon handling. |

## Special handling
- `init()` does not use the `Initializing` status even though the enum defines it; it calls `refresh()`, starts the timer, `unref()`s the timer, and then sets `Initialized`.
- `close()` clears the interval and sets `Closed`; the `Closing` enum member is never used.
- `refresh()` intentionally masks the raw `AZURITE_ACCOUNTS` value in logs (`*****`) while still logging whether the variable exists.
- Fallback behavior is forgiving: if parsing fails, the store logs an error and silently reverts to the default emulator account instead of propagating failure.
- `parserAccountsEnvironmentString()` accepts entries of the form `account:key1` or `account:key1:key2`, separated by semicolons; empty entries are ignored.
- `Buffer.from(key, "base64")` is the current decoder. Rust should preserve the decoded-byte outcome, but note that Node base64 decoding is permissive and may not reject every malformed string as strictly as a Rust crate would.
- The default account map is a shared constant object in TS (`DEFAULT_EMULATOR_ACCOUNTS`). Reassignments often point back to that same default object.

## Patterns requiring special handling
- **Environment-driven account refresh**: this is process-env polling, not file-based config watching.
- **Timer lifecycle**: TS uses `setInterval(...).unref()` so the interval does not keep the Node process alive. Rust needs an equivalent background task that does not block shutdown.
- **Cross-layer logger import**: the file imports `ILogger` from queue-generated code rather than common. Aragorn should keep this coupling visible or add an explicit adapter rather than silently rewriting the dependency graph.

## Change propagation notes
- If TS adds more account properties, update both the parser and every authentication path that reads `IAccountProperties`.
- If the env-string grammar changes, audit documentation, CLI help, and any tests that depend on semicolon/colon parsing.
- If upstream changes the logger import to use common `ILogger`, note that this cross-layer asymmetry disappeared.

## Fidelity risks and edge cases
- Silent fallback to the emulator account can hide operator misconfiguration; Rust should preserve the behavior unless the team explicitly decides otherwise.
- `Initializing` and `Closing` are currently dead states. Do not remove them from the record; future TS changes may start using them.
- The base64 decoder permissiveness may differ across languages. If Rust uses stricter decoding, document any deliberate divergence.

## Rust port notes
- Ported the in-memory account store into `rust/crates/azurite-common/src/account_data_store.rs`, including environment-driven refresh, masked logging, default-emulator fallback, and the periodic timer loop.
- The Rust parser keeps the `account:key1[:key2];...` grammar and still falls back silently to the emulator account when refresh parsing fails.
