# Porting Record — `src/blob/generated/utils/ILogger.ts`

## File info
- Source path: `src/blob/generated/utils/ILogger.ts`
- Source lines: `13`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/utils/i_logger.rs`
- Crate: `azurite-blob`
- Module: `generated::utils::i_logger`
- Phase: `5.3`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface ILogger {
  error(message: string, contextID?: string): void;
  warn(message: string, contextID?: string): void;
  info(message: string, contextID?: string): void;
  verbose(message: string, contextID?: string): void;
  debug(message: string, contextID?: string): void;
}
```

## Dependencies
- None.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `ILogger` interface | `trait ILogger` | Keep five methods (`error`, `warn`, `info`, `verbose`, `debug`) rather than collapsing them. |
| `contextID?: string` | `Option<&str>` / `Option<String>` | Generated code logs with `contextId` values threaded through middleware. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- This tiny interface is the only logging dependency shared by the generated framework. Queue/table generated code should mirror it.

## Middleware chain ordering
- Every generated middleware receives this logger and emits stage-specific messages.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If log levels or method names change, update every generated middleware factory and helper because they call these methods directly.
