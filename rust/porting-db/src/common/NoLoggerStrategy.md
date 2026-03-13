# Porting Record — `src/common/NoLoggerStrategy.ts`

## File info
- Source path: `src/common/NoLoggerStrategy.ts`
- Source lines: `18`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/no_logger_strategy.rs`
- Crate: `azurite-common`
- Module: `no_logger_strategy`
- Phase: `4.5`
- Status: `analyzed`

## Exported API
### Default class `NoLoggerStrategy implements ILoggerStrategy`
- Method: `log(level: LogLevels, message: string, contextID?: string | undefined): void`

## Dependencies
- Internal imports:
  - `./ILoggerStrategy` — Phase `1.4`, already ported.
- No external packages.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| null-object class | zero-sized struct implementing `ILoggerStrategy` | No internal state required. |
| `void` log method | `fn log(...) {}` | Preserve the intentional no-op. |

## Recommended Rust translation
- Translate directly to a trivial `struct NoLoggerStrategy;` implementing the logging strategy trait.
- Keep it available as the logger singleton's default strategy to mirror TS startup behavior.

## Function / method mapping notes
| TS member | Rust recommendation | Fidelity notes |
|---|---|---|
| `log()` | `fn log(&self, level: LogLevels, message: &str, context_id: Option<&str>)` | Intentionally ignores all arguments. |

## Special handling
- This is a classic null-object implementation. Do not add filtering, counters, or debug assertions; silence is the behavior.

## Change propagation notes
- If `ILoggerStrategy` grows new requirements, this class should usually remain the minimal no-op implementation of them.

## Fidelity risks and edge cases
- There are no algorithmic risks here. The important point is preserving the existence of a dedicated no-op strategy rather than encoding “disabled logging” as `None`.
