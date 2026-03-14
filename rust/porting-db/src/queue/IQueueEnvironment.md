# Porting Record — `src/queue/IQueueEnvironment.ts`

## File info
- Source path: `src/queue/IQueueEnvironment.ts`
- Source lines: `17`
- Source type: `interface`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/i_queue_environment.rs`
- Crate: `azurite-queue`
- Module: `i_queue_environment`
- Phase: `14.16`
- Status: `not_started`

## Exported API
### Default interface `IQueueEnvironment`
- 15 getter methods (all non-mutating):
  - `queueHost(): string | undefined`
  - `queuePort(): number | undefined`
  - `queueKeepAliveTimeout(): number | undefined`
  - `location(): Promise<string>`
  - `silent(): boolean`
  - `loose(): boolean`
  - `skipApiVersionCheck(): boolean`
  - `disableProductStyleUrl(): boolean`
  - `cert(): string | undefined`
  - `key(): string | undefined`
  - `pwd(): string | undefined`
  - `debug(): Promise<string | boolean | undefined>`
  - `inMemoryPersistence(): boolean`
  - `extentMemoryLimit(): number | undefined`
  - `disableTelemetry(): boolean`

## Dependencies
- None (pure interface definition)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `interface` | `pub trait` | Interface contract with default implementations optional |
| `string \| undefined` | `Option<String>` | Nullable string return |
| `number \| undefined` | `Option<f64>` or `Option<i32>` | Nullable numeric return; queue port is i32, timeouts are f64 |
| `Promise<string>` | `impl Future<Output = String>` or async method | Async location resolution |
| `Promise<string \| boolean \| undefined>` | `impl Future<Output = Option<String>>` or `impl Future<Output = Option<bool>>` | Union return; see Special handling |
| `boolean` | `bool` | Direct mapping |

## Special handling
1. **Async methods** (`location()`, `debug()`)
   - Queue environment differs from blob by having TWO async methods
   - Both must be async for compatibility with CLI argument handling and file I/O
   - Preserve exact signatures in trait

2. **Debug return type variance** (`debug()`)
   - Returns `Promise<string | boolean | undefined>`
   - In Rust, represent as `impl Future<Output = Option<String>>` (strings for file path, undefined for disabled)
   - Boolean return is a quirk from TypeScript pattern but Rust will only use string variant in practice
   - See QueueEnvironment.ts implementation for actual behavior

3. **Port type specificity**
   - `queuePort()` specifically returns `number` not undefined by default (unlike blob port which can be undefined)
   - `queueKeepAliveTimeout()` returns `number | undefined` (matches blob pattern)

4. **Configuration coherence**
   - All methods are configuration accessors — no side effects
   - Maps 1:1 to CLI arguments defined in QueueEnvironment.ts
   - No overlap with ConfigurationBase interface (Phase 4)

## Change propagation notes
- If new CLI options are added to `QueueEnvironment`, update this interface first
- Async methods must stay async even if implementation becomes synchronous (to preserve caller expectations)
- Keep consistent with `IBlobEnvironment` interface (Phase 12) for future consolidation opportunities

