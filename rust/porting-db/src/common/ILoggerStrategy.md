# Porting Record — `src/common/ILoggerStrategy.ts`

## File info
- Source path: `src/common/ILoggerStrategy.ts`
- Source lines: `11`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_logger_strategy.rs`
- Crate: `azurite-common`
- Module: `i_logger_strategy`
- Phase: `1.4`
- Status: `analyzed`

## Exported API
### Enum `LogLevels`
- `Error = "error"`
- `Warn = "warn"`
- `Info = "info"`
- `Verbose = "verbose"`
- `Debug = "debug"`

### Default interface `ILoggerStrategy`
- `log(level: LogLevels, message: string, contextID?: string): void`

## Dependencies
- Imports: none.
- Primary downstream users: `Logger.ts`, `NoLoggerStrategy.ts`, `WinstonLoggerStrategy.ts`, `VSCChannelLoggerStrategy.ts`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum `LogLevels` | `enum LogLevel` + `as_str()`/`Display` | Preserve lowercase string payloads. |
| `string` | `&str` / `String` | Message body. |
| `string | undefined` | `Option<&str>` / `Option<String>` | Optional correlation ID. |
| `void` | `()` | Strategy call stays synchronous. |

## Recommended Rust translation
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Verbose,
    Debug,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Verbose => "verbose",
            Self::Debug => "debug",
        }
    }
}

pub trait LoggerStrategy: Send + Sync {
    fn log(&self, level: LogLevel, message: &str, context_id: Option<&str>);
}
```

## Special handling
- Keep the string-valued enum semantics. The TS implementation writes these lowercase values into formatted logs.
- Preserve the `contextID` spelling in the analysis notes, even if Rust parameter names use `context_id`.
- This interface is the strategy side of the logger/strategy pair; it should remain single-entry-point even though `ILogger` exposes one method per level.

## Change propagation notes
- New log levels require synchronized updates to the enum, the `Logger` adapter, and every strategy implementation.
- If TS changes enum payload strings, verify formatter output and any tests or docs that rely on literal log level text.
