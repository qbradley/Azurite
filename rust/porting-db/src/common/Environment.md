# Porting Record — `src/common/Environment.ts`

## File info
- Source path: `src/common/Environment.ts`
- Source lines: `247`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/environment.rs`
- Crate: `azurite-common`
- Module: `environment`
- Phase: `4.10`
- Status: `ported`

## Exported API
### Default class `Environment implements IEnvironment`
- Private field: `flags = args.parse(process.argv)`
- Method: `blobHost(): string | undefined`
- Method: `blobPort(): number | undefined`
- Method: `blobKeepAliveTimeout(): number | undefined`
- Method: `queueHost(): string | undefined`
- Method: `queuePort(): number | undefined`
- Method: `queueKeepAliveTimeout(): number | undefined`
- Method: `tableHost(): string | undefined`
- Method: `tablePort(): number | undefined`
- Method: `tableKeepAliveTimeout(): number | undefined`
- Method: `location(): Promise<string>`
- Method: `silent(): boolean`
- Method: `loose(): boolean`
- Method: `skipApiVersionCheck(): boolean`
- Method: `disableProductStyleUrl(): boolean`
- Method: `cert(): string | undefined`
- Method: `key(): string | undefined`
- Method: `pwd(): string | undefined`
- Method: `oauth(): string | undefined`
- Method: `inMemoryPersistence(): boolean`
- Method: `extentMemoryLimit(): number | undefined`
- Method: `disableTelemetry(): boolean`
- Method: `debug(): Promise<string | undefined>`

## Dependencies
- External package: `args`.
- Internal imports:
  - `../blob/utils/constants` — Phase `12.1`, not yet ported.
  - `../queue/utils/constants` — Phase `14.23`, not yet ported.
  - `../table/utils/constants` — Phase `15.30`, not yet ported.
  - `./IEnvironment` — Phase `1.11`, already ported.
- Runtime dependencies: `process.argv`, `process.cwd()`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `args` parser global configuration | `clap::Parser` / builder-style CLI parser | Keep option names and defaults close to TS. |
| `flags` bag with optional properties | concrete struct of `Option<T>` fields | Mirrors presence-vs-value semantics clearly. |
| `Promise<string>` / `Promise<string | undefined>` | async trait methods or `async fn` returning `String` / `Option<String>` | TS keeps these async only because the interface contract does. |
| boolean flags checked by presence | `bool` plus explicit `was_present` handling if needed | Current TS treats defined flag presence as `true`. |
| service-specific getter families | flattened Rust environment trait/struct | Preserve blob/queue/table-specific accessors rather than collapsing into generic arrays. |

## Recommended Rust translation
- Translate CLI parsing with `clap` (or equivalent) into a concrete `Environment` struct that still exposes the same getter surface as `IEnvironment`.
- Preserve the current split between parsing and validation: some invalid combinations are only rejected when specific getters are called.
- Keep the common-layer flattened environment shape already documented in Phase 1, because `azurite-common` cannot depend on later service crates.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| module-level `args.option(...)` chain | `clap` command/derive definition | Preserve long names, short names (`-l`, `-s`, `-L`, `-d`), help text intent, and defaults. |
| constructor/`flags` parse | `fn new_from_env()` / `Parser::parse()` | TS reparses `process.argv` for each new instance. |
| service host/port/keepAlive getters | trivial field accessors returning `Option<T>` | Keep one getter per service and property. |
| `location()` | `async fn location(&self) -> String` | Preserve fallback to `process.cwd()` when option omitted or `<cwd>` sentinel is used. |
| `silent()` / `loose()` / `skipApiVersionCheck()` / `disableProductStyleUrl()` / `disableTelemetry()` | presence-based boolean getters | Current TS checks `!== undefined`, not literal truthy coercion. |
| `cert()` / `key()` / `pwd()` / `oauth()` | optional string getters | No extra validation here. |
| `inMemoryPersistence()` | getter with validation | Throws when `--location` is also set; also guards `extentMemoryLimit`. |
| `extentMemoryLimit()` | numeric optional getter | Parser converts `-1` sentinel to `undefined`. |
| `debug()` | `async fn debug(&self) -> Result<Option<String>, RangeError>` | Bare `--debug` / `-d` without path throws. |

## Special handling
- The module configures the `args` parser at import time, before any `Environment` instance is created. `(args as any).config.name = "azurite"` is also a module-level side effect.
- `disableProductStyleUrl` is registered twice in the option chain (once at lines 84–87 and again at lines 106–109). That duplication is real TS source behavior and should stay documented rather than silently disappearing from the analysis.
- `location` uses a parser callback that maps the literal default placeholder `<cwd>` to `undefined`, then `location()` falls back to `process.cwd()`.
- `extentMemoryLimit` uses `-1` as a parser-level sentinel for “not provided”, converted to `undefined`; any other input is passed through `parseFloat`.
- `location()` and `debug()` are marked `async` even though they do not await anything. That is driven by the `IEnvironment` contract and should remain visible to Aragorn.
- Validation is lazy and getter-triggered:
  - `inMemoryPersistence()` throws if `--location` is also set.
  - `inMemoryPersistence()` also throws if `--extentMemoryLimit` was set without `--inMemoryPersistence`.
  - `debug()` throws if the flag is present but does not carry a file path.
- Boolean getters treat defined flag presence as `true`; they do not inspect an explicit boolean payload.

## Patterns requiring special handling
- **Command-line parsing (`args` → Rust CLI parser)**: preserve the current option surface, defaults, and getter-triggered validation using `clap` or equivalent.
- **Aggregate environment interface**: this class implements the common `IEnvironment` surface that merges blob, queue, and table environment contracts.
- **Process-global CLI source**: the implementation reads `process.argv` directly and is designed for CLI/runtime environments, not VS Code configuration.

## Change propagation notes
- If TS adds or removes flags, update both the parser registration and the matching getter(s); the two are intentionally kept in the same file.
- If service defaults diverge in future, also audit `Telemetry.ts`, which currently compares VSC settings against blob defaults for host/timeout suppression.
- If `IEnvironment` changes, revisit this file and the flattened Rust trait together.

## Fidelity risks and edge cases
- Duplicate `disableProductStyleUrl` registration is easy to “clean up” accidentally, but doing so changes the literal TS source shape Faramir is tracking for propagation.
- Lazy validation means errors surface only when certain getters are called. A Rust parser that eagerly rejects combinations at startup would change timing/behavior.
- `debug()` returning `undefined` by default and throwing only for bare `--debug` is subtle; preserve that distinction.

## Rust port notes
- Ported CLI parsing into `rust/crates/azurite-common/src/environment.rs` with clap-backed getters that preserve the TypeScript option names, defaults, lazy validation, and the duplicate `disableProductStyleUrl` registration shape.
- `debug()` now returns the requested path or throws for bare `--debug`, and `extentMemoryLimit()` stays float-shaped so the downstream NaN path remains visible to future ports.
