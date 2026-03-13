# Porting Record — `src/common/ConfigurationBase.ts`

## File info
- Source path: `src/common/ConfigurationBase.ts`
- Source lines: `107`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/configuration_base.rs`
- Crate: `azurite-common`
- Module: `configuration_base`
- Phase: `4.7`
- Status: `analyzed`

## Exported API
### Enum `CertOptions`
- `Default`
- `PEM`
- `PFX`

### Function `setExtentMemoryLimit`
- `setExtentMemoryLimit(env: IBlobEnvironment | IQueueEnvironment | IEnvironment, logToConsole: boolean): void`

### Default abstract class `ConfigurationBase`
- Constructor:
  `constructor(host: string, port: number, keepAliveTimeout: number, enableAccessLog: boolean = false, accessLogWriteStream?: NodeJS.WritableStream, enableDebugLog: boolean = false, debugLogFilePath?: string, loose: boolean = false, skipApiVersionCheck: boolean = false, cert: string = "", key: string = "", pwd: string = "", oauth?: string, disableProductStyleUrl: boolean = false)`
- Method: `hasCert(): CertOptions`
- Method: `getCert(option: any): { cert: Buffer; key: Buffer } | { pfx: Buffer; passphrase: string } | null`
- Method: `getOAuthLevel(): undefined | OAuthLevel`
- Method: `getHttpServerAddress(): string`

## Dependencies
- Node built-ins: `fs`, `os.totalmem()`.
- Internal imports:
  - `./models` (`OAuthLevel`) — Phase `1.5`, already ported.
  - `../blob/IBlobEnvironment` — Phase `12.8`, not yet ported.
  - `../queue/IQueueEnvironment` — Phase `14.16`, not yet ported.
  - `./IEnvironment` — Phase `1.11`, already ported.
  - `./persistence/MemoryExtentStore` (`DEFAULT_EXTENT_MEMORY_LIMIT`, `SharedChunkStore`) — Phase `2.2`, already ported.
  - `./Logger` — Phase `4.4`, analyzed in this pass.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| abstract base class with readonly fields | `struct ConfigurationBase` embedded via composition | Matches project decision to use composition over inheritance. |
| `NodeJS.WritableStream` | boxed writer trait object / `Option<Arc<Mutex<dyn Write + Send>>>` | Only needed if Rust keeps access-log stream injection. |
| `oauth?: string` | `Option<String>` plus `OAuthLevel` parser helper | This file keeps raw string and converts lazily. |
| `IBlobEnvironment | IQueueEnvironment | IEnvironment` | flattened trait/object accepted by helper | `setExtentMemoryLimit()` only needs `inMemoryPersistence()` and `extentMemoryLimit()`. |
| `Buffer` return from `fs.readFileSync()` | `Vec<u8>` | Cert and key are file contents, not paths once loaded. |
| enum `CertOptions` | small Rust enum | Preserve PEM-vs-PFX precedence rules. |

## Recommended Rust translation
- Keep `ConfigurationBase` as a common base struct used by concrete service configuration types via composition.
- Keep CLI/environment parsing outside this file. `ConfigurationBase.ts` consumes already-parsed values and offers helper methods for TLS, OAuth, and address formatting.
- Model `setExtentMemoryLimit()` as a free helper function in the same module so future TS diffs remain easy to compare.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| `CertOptions` | `enum CertOptions { Default, Pem, Pfx }` | Preserve the simple three-state discriminator. |
| `setExtentMemoryLimit()` | `fn set_extent_memory_limit(env: &dyn ExtentMemoryEnvironment, log_to_console: bool) -> Result<(), Error>` | Preserve logging side effects and `SharedChunkStore.setSizeLimit(...)` behavior. |
| constructor fields | embedded base struct fields | Keep field names aligned with TS (`enable_access_log`, `debug_log_file_path`, etc.). |
| `hasCert()` | `fn has_cert(&self) -> CertOptions` | Preserve precedence: PEM wins when both key and pwd are present. |
| `getCert(option)` | `fn get_cert(&self, option: CertOptions) -> io::Result<Option<CertMaterial>>` | Use enum instead of `any`, but preserve output shapes. |
| `getOAuthLevel()` | `fn get_oauth_level(&self) -> Option<OAuthLevel>` | Only `basic` maps to `BASIC`; anything else yields `None`. |
| `getHttpServerAddress()` | `fn get_http_server_address(&self) -> String` | Prefix switches to `https` whenever `hasCert() != Default`. |

## Special handling
- `hasCert()` is intentionally heuristic: `(cert && key) => PEM`, else `(cert && pwd) => PFX`, else `Default`. If all three are present, PEM wins because it is checked first.
- `getCert()` performs blocking file reads with `fs.readFileSync()`. Rust can use async file I/O internally, but the analysis should preserve that this helper currently loads whole files eagerly and returns raw bytes.
- `getOAuthLevel()` is permissive: it lowercases the string and only recognizes `basic`; unsupported values quietly return `undefined` instead of throwing.
- `setExtentMemoryLimit()` logs to both `console.log()` and the shared logger when `logToConsole` is true. Keep the duplicate side effect visible.
- `setExtentMemoryLimit()` has a subtle NaN path: after defaulting, it throws only when `mb < 0`; if `mb` is `NaN`, the `mb >= 0` branch is false and execution falls into the nominally “no limit” else branch. This is reachable if upstream parsing ever yields `NaN`.
- The file does not parse command-line arguments itself; that happens in `Environment.ts`. Preserve that separation so future TS changes in CLI parsing still land in the right Rust layer.

## Change propagation notes
- Any new config field added to the constructor must be audited through every concrete service configuration type that embeds this base.
- If TS tightens OAuth validation later, update this helper and re-check every request-listener/authenticator call site.
- If in-memory extent-limit semantics change here, audit `MemoryExtentStore` and startup logging together.

## Fidelity risks and edge cases
- PEM/PFX precedence is easy to accidentally reorder during translation.
- Returning `None` for unsupported OAuth strings is current behavior; converting that into an error would be a semantic change.
- The `setExtentMemoryLimit()` helper currently mixes configuration interpretation, logging, and global `SharedChunkStore` mutation. Splitting those concerns in Rust may improve design, but keep the visible call flow close to TS.
