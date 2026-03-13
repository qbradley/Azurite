# Porting Record — `src/common/utils/constants.ts`

## File info
- Source path: `src/common/utils/constants.ts`
- Source lines: `62`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/utils/constants.rs`
- Crate: `azurite-common`
- Module: `utils::constants`
- Phase: `4.1`
- Status: `ported`

## Exported API
### Constant `AZURITE_ACCOUNTS_ENV`
- `AZURITE_ACCOUNTS_ENV: "AZURITE_ACCOUNTS"`

### Constant `DEFAULT_ACCOUNTS_REFRESH_INTERVAL`
- `DEFAULT_ACCOUNTS_REFRESH_INTERVAL: 60 * 1000`

### Constant `DEFAULT_FD_CACHE_NUMBER`
- `DEFAULT_FD_CACHE_NUMBER: 100`

### Constant `FD_CACHE_NUMBER_MIN`
- `FD_CACHE_NUMBER_MIN: 1`

### Constant `FD_CACHE_NUMBER_MAX`
- `FD_CACHE_NUMBER_MAX: 100`

### Constant `DEFAULT_MAX_EXTENT_SIZE`
- `DEFAULT_MAX_EXTENT_SIZE: 64 * 1024 * 1024`

### Constant `DEFAULT_READ_CONCURRENCY`
- `DEFAULT_READ_CONCURRENCY: 100`

### Constant `DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS`
- `DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS: 10 * 60 * 1000`

### Constant `DEFAULT_SQL_CHARSET`
- `DEFAULT_SQL_CHARSET: "utf8mb4"`

### Constant `IP_REGEX`
- `IP_REGEX: RegExp = /^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$/`

### Constant `NO_ACCOUNT_HOST_NAMES`
- `NO_ACCOUNT_HOST_NAMES: Set<string> = new Set().add("host.docker.internal")`

### Constant `DEFAULT_SQL_COLLATE`
- `DEFAULT_SQL_COLLATE: "utf8mb4_bin"`

### Constant `DEFAULT_SQL_OPTIONS`
- `DEFAULT_SQL_OPTIONS: { logging: boolean; pool: { max: number; min: number; acquire: number; idle: number }; charset: string; collate: string; dialectOptions: { timezone: string } }`

### Constant `BEARER_TOKEN_PREFIX`
- `BEARER_TOKEN_PREFIX: "Bearer"`

### Constant `HTTPS`
- `HTTPS: "https"`

### Constant `VALID_ISSUE_PREFIXES`
- `VALID_ISSUE_PREFIXES: string[] = [
  "https://sts.windows.net/",
  "https://sts.microsoftonline.de/",
  "https://sts.chinacloudapi.cn/",
  "https://sts.windows-ppe.net"
]`

### Constant `EMULATOR_ACCOUNT_NAME`
- `EMULATOR_ACCOUNT_NAME: "devstoreaccount1"`

### Constant `EMULATOR_ACCOUNT_KEY`
- `EMULATOR_ACCOUNT_KEY: Buffer = Buffer.from("Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==", "base64")`

### Constant `VALID_CSHARP_IDENTIFIER_REGEX`
- `VALID_CSHARP_IDENTIFIER_REGEX: RegExp = /^[a-zA-Z_][a-zA-Z0-9_]*$/`

## Dependencies
- Imports: none.
- Downstream users in Phase 4: `AccountDataStore.ts`, `utils.ts`.
- Downstream users in later phases: service constants/env parsing, authentication helpers, SQL-backed persistence.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string literal constant | `pub const NAME: &str` | Keep exact casing and punctuation for env vars, protocol labels, and issuer prefixes. |
| numeric constant in milliseconds/bytes | `pub const NAME: u64` / `usize` / `Duration` helper | Prefer raw integers for change propagation; wrap in `Duration` only at call sites. |
| `Buffer` constant | `pub const` base64 string + lazy decoded `Vec<u8>` / `[u8; N]` | Preserve emulator account key bytes exactly. |
| `RegExp` | `once_cell::sync::Lazy<Regex>` | Compile once and preserve current patterns. |
| `Set<string>` | `Lazy<HashSet<&'static str>>` | Only one hostname today, but preserve set semantics. |
| object literal config | dedicated Rust struct or builder function | Keep field names close to TS for future diffing. |

## Recommended Rust translation
- Keep this module as a pure constants module with no runtime initialization beyond lazy regex/set construction.
- Model `DEFAULT_SQL_OPTIONS` as a small config struct or helper function returning the exact default pool/timezone settings.
- Store the emulator account key either as a base64 string constant with a decoding helper or as a byte array generated once, but keep the source value visible for change propagation.

## Function / value mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| `AZURITE_ACCOUNTS_ENV` | `pub const AZURITE_ACCOUNTS_ENV: &str` | Environment-variable name is contract-sensitive. |
| `DEFAULT_ACCOUNTS_REFRESH_INTERVAL` | `pub const DEFAULT_ACCOUNTS_REFRESH_INTERVAL_MS: u64` | Used by `AccountDataStore` timer cadence. |
| `IP_REGEX` | `static IP_REGEX: Lazy<Regex>` | Preserve IP-only matching; do not replace with host parser heuristics unless behavior is covered. |
| `NO_ACCOUNT_HOST_NAMES` | `static NO_ACCOUNT_HOST_NAMES: Lazy<HashSet<&str>>` | Currently only `host.docker.internal`; future TS additions should diff cleanly. |
| `DEFAULT_SQL_OPTIONS` | `fn default_sql_options() -> SqlOptions` | Preserve UTC timezone and case-sensitive collation. |
| `VALID_ISSUE_PREFIXES` | `pub const VALID_ISSUE_PREFIXES: &[&str]` | Note the last entry lacks the trailing slash used by the other prefixes. |
| `EMULATOR_ACCOUNT_KEY` | `static EMULATOR_ACCOUNT_KEY: Lazy<Vec<u8>>` | Exact bytes must remain compatible with existing Azurite clients. |

## Special handling
- `DEFAULT_SQL_COLLATE` intentionally uses `utf8mb4_bin`, not a case-insensitive collation. This is tied to metadata casing behavior and should not be normalized.
- `NO_ACCOUNT_HOST_NAMES` is set-based even though it currently contains one value; preserve the structure because hostname exceptions are expected to grow.
- `VALID_ISSUE_PREFIXES` encodes Azure cloud issuer allow-list behavior. Keep the exact order and strings because upstream diffs will likely be literal additions/removals.

## Change propagation notes
- If the emulator account name or key changes in TS, audit all shared-key auth paths and any hard-coded test fixtures before updating Rust.
- If more hostnames are added to `NO_ACCOUNT_HOST_NAMES`, re-check any Rust hostname parsing optimizations so they still preserve the exception list.
- If SQL defaults change here, audit both blob/queue/table SQL metadata stores and setup documentation.

## Fidelity risks and edge cases
- The hard-coded emulator key is compatibility-critical and must remain byte-for-byte identical.
- Regex behavior should stay regex-based; do not silently replace with stricter IP parsing without reviewing all hostname/account-name extraction sites.
- `VALID_ISSUE_PREFIXES` is slightly inconsistent (`windows-ppe` entry has no trailing slash). Preserve that exact literal list unless TS changes it.

## Rust port notes
- Implemented the full constants surface in `rust/crates/azurite-common/src/utils/constants.rs`, including lazy regex/set/key initialization and a struct-backed `DEFAULT_SQL_OPTIONS` value.
- Kept literal compatibility-sensitive values unchanged, including the emulator account key bytes and the issuer prefix list order.
