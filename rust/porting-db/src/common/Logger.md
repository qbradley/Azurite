# Porting Record — `src/common/Logger.ts`

## File info
- Source path: `src/common/Logger.ts`
- Source lines: `51`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/logger.rs`
- Crate: `azurite-common`
- Module: `logger`
- Phase: `4.4`
- Status: `analyzed`

## Exported API
### Class `Logger implements ILogger`
- Constructor: `constructor(strategy: ILoggerStrategy)`
- Property: `public strategy: ILoggerStrategy`
- Method: `error(message: string, contextID?: string): void`
- Method: `warn(message: string, contextID?: string): void`
- Method: `info(message: string, contextID?: string): void`
- Method: `verbose(message: string, contextID?: string): void`
- Method: `debug(message: string, contextID?: string): void`

### Function `configLogger`
- `configLogger(enable: boolean, logFile?: string): void`

### Default export `logger`
- `logger: Logger = new Logger(new NoLoggerStrategy())`

## Dependencies
- Internal imports:
  - `./ILogger` — Phase `1.3`, already ported.
  - `./ILoggerStrategy` — Phase `1.4`, already ported.
  - `./NoLoggerStrategy` — Phase `4.5`, analyzed in this pass.
  - `./WinstonLoggerStrategy` — Phase `4.6`, analyzed in this pass.
- No external package imports in this file.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| class with mutable strategy field | `struct Logger { strategy: Arc<dyn ILoggerStrategy> }` behind `RwLock`/`Mutex` when global | Strategy swapping is the key behavior to preserve. |
| default singleton export | `static LOGGER: Lazy<Arc<RwLock<Logger>>>` or dedicated global facade | This file is intentionally process-global in TS. |
| `string | undefined` context ID | `Option<&str>` / `Option<String>` | Keep optional correlation identifier semantics. |
| `void` | `()` | Logging remains synchronous in the TS surface. |

## Recommended Rust translation
- Keep a thin adapter object that maps level-specific methods (`error`, `warn`, etc.) onto a single strategy method.
- Preserve the singleton/global access pattern because a large amount of TS code imports the default logger directly.
- Hide synchronization at the singleton boundary so call sites still look like direct logger usage.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| `Logger` constructor | `fn new(strategy: Arc<dyn ILoggerStrategy>) -> Self` | Strategy injection is explicit and should stay testable. |
| `error()`/`warn()`/`info()`/`verbose()`/`debug()` | level-specific methods forwarding to `strategy.log()` | Preserve direct 1:1 mapping to `LogLevels`. |
| `configLogger(enable, logFile?)` | `fn config_logger(enable: bool, log_file: Option<&Path>)` | Preserve runtime replacement of the global strategy. |
| default `logger` export | `pub fn logger() -> &'static GlobalLogger` or `static LOGGER` | TS relies on module-level singleton behavior. |

## Special handling
- `Logger` does not add any logic beyond level dispatch. Resist the urge to collapse it into direct `tracing` macros in call sites, because future TS diffs land here first.
- The singleton starts as `NoLoggerStrategy`, so logging is disabled until `configLogger()` is called. Preserve that startup behavior.
- `configLogger(true, logFile)` always installs `WinstonLoggerStrategy(LogLevels.Debug, logFile)`; it does not preserve any previous level configuration.

## Change propagation notes
- If TS adds a new log level to `ILogger`/`ILoggerStrategy`, this adapter must grow a matching forwarding method.
- If TS changes singleton initialization order or starts reading configuration from elsewhere, audit every module that imports the default logger directly.

## Fidelity risks and edge cases
- Global mutable logger strategy is the main translation risk. A Rust port that removes runtime swapping would diverge from current TS behavior.
- `contextID` spelling is preserved here; keep the naming inconsistency visible in notes because the codebase also uses `contextId` elsewhere.
