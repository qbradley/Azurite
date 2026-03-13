# Porting Record — `src/common/WinstonLoggerStrategy.ts`

## File info
- Source path: `src/common/WinstonLoggerStrategy.ts`
- Source lines: `52`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/winston_logger_strategy.rs`
- Crate: `azurite-common`
- Module: `winston_logger_strategy`
- Phase: `4.6`
- Status: `analyzed`

## Exported API
### Default class `WinstonLoggerStrategy implements ILoggerStrategy`
- Private field: `winstonLogger: IWinstonLogger`
- Private field: `consoleTransport?: transports.ConsoleTransportInstance`
- Private field: `fileTransport?: transports.FileTransportInstance`
- Constructor: `constructor(level: LogLevels = LogLevels.Debug, logfile?: string)`
- Method: `log(level: LogLevels, message: string, contextID: string = "\t"): void`

## Dependencies
- External package: `winston` (`createLogger`, `format`, `Logger`, `transports`).
- Internal imports:
  - `./ILoggerStrategy` — Phase `1.4`, already ported.
- Downstream user: `Logger.configLogger()` installs this strategy when logging is enabled.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `winston.Logger` | strategy-owned `tracing_subscriber` writer or custom sink object | Rust equivalent should keep one strategy object responsible for output formatting. |
| optional console/file transport | enum/struct wrapping stdout vs file writer | TS installs exactly one transport path, not both. |
| string enum level | shared `LogLevels` enum | Preserve lowercase textual level values. |
| optional logfile path | `Option<PathBuf>` | File path decides transport type. |

## Recommended Rust translation
- Implement this as the concrete logging strategy behind the common `ILoggerStrategy` trait.
- Prefer `tracing` + `tracing-subscriber` or a small direct writer abstraction instead of trying to mimic Winston internals literally.
- Preserve the observable formatting contract: timestamp, space, `contextID`, space, lowercase level, colon, message.

## Function / method mapping notes
| TS member | Rust recommendation | Fidelity notes |
|---|---|---|
| constructor | `fn new(level: LogLevels, logfile: Option<PathBuf>) -> Self` | If `logfile` is `Some`, write to file; otherwise write to console. |
| `log()` | `fn log(&self, level: LogLevels, message: &str, context_id: Option<&str>)` | Preserve the TS default of `"\t"` when the caller omits `contextID`. |
| `winstonLogger.log({ level, message, contextID })` | write formatted line to sink | Structured logging is not exposed to callers; formatted text output is the observable behavior. |

## Special handling
- The formatter is inline and simple: `${info.timestamp} ${info.contextID} ${info.level}: ${info.message}`. Do not silently add JSON output, ANSI coloring, or log targets unless TS does.
- `contextID` defaults to a tab character (`"\t"`), not an empty string. That odd default affects the literal spacing of log lines and should be preserved if exact parity matters.
- The constructor always uses the passed log level to initialize the underlying logger, but current callers always pass `LogLevels.Debug`.
- There is no transport rotation, buffering policy, or multi-sink fan-out in this file.

## Change propagation notes
- If TS later adds file rotation or multiple transports, this file is where the Rust strategy should mirror the expanded behavior instead of pushing it into generic logging infrastructure.
- If TS changes the formatted message template, update any Rust tests that compare literal log lines.

## Fidelity risks and edge cases
- The largest fidelity risk is over-idiomatizing into generic `tracing` macros without preserving the strategy object and its runtime-selected sink.
- The tab default for missing `contextID` is easy to lose in Rust if `Option::None` is formatted as an empty string.
- File-vs-console selection is mutually exclusive today; a Rust port that broadcasts to both would diverge from current TS behavior.
