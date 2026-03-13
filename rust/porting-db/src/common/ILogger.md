# Porting Record — `src/common/ILogger.ts`

## File info
- Source path: `src/common/ILogger.ts`
- Source lines: `13`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_logger.rs`
- Crate: `azurite-common`
- Module: `i_logger`
- Phase: `1.3`
- Status: `analyzed`

## Exported API
### Default interface `ILogger`
- `error(message: string, contextID?: string): void`
- `warn(message: string, contextID?: string): void`
- `info(message: string, contextID?: string): void`
- `verbose(message: string, contextID?: string): void`
- `debug(message: string, contextID?: string): void`

## Dependencies
- Imports: none.
- Primary downstream users: `AccountDataStore`, `OperationQueue`, extent stores, telemetry, GC managers, and the singleton `Logger` adapter.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `string` | `&str` / `String` | Borrow at call sites; own only when storage is required. |
| `string | undefined` | `Option<&str>` / `Option<String>` | Optional context token. |
| `void` | `()` | Logging stays synchronous in the interface. |

## Recommended Rust translation
```rust
pub trait Logger: Send + Sync {
    fn error(&self, message: &str, context_id: Option<&str>);
    fn warn(&self, message: &str, context_id: Option<&str>);
    fn info(&self, message: &str, context_id: Option<&str>);
    fn verbose(&self, message: &str, context_id: Option<&str>);
    fn debug(&self, message: &str, context_id: Option<&str>);
}
```

## Special handling
- Preserve the TypeScript spelling note that the optional correlation field is named `contextID` here, not `contextId`.
- The wider codebase mixes `contextID` and `contextId` (`Telemetry.ts` handles both). Rust field names can be idiomatic snake_case, but the porting notes must preserve the TS inconsistency for future change propagation.
- This interface is intentionally thin because `Logger.ts` adapts it onto `ILoggerStrategy`.

## Change propagation notes
- Any added log level here must also be added to `LogLevels`, `Logger.ts`, and all logger strategies.
- If the context field is renamed in TS, audit telemetry and persistence code for mixed casing assumptions.
