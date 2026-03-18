# Porting Record — `src/queue/QueueEnvironment.ts`

## File info
- Source path: `src/queue/QueueEnvironment.ts`
- Source lines: `170`
- Source type: `handwritten implementation class`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/queue_environment.rs`
- Crate: `azurite-queue`
- Module: `queue_environment`
- Phase: `14.17`
- Status: `ported`

## Exported API
### Default class `QueueEnvironment`
- Implements `IQueueEnvironment` (Phase 14.16)
- Constructor: `new QueueEnvironment()` — parses CLI args during construction via `args` library
- Public methods: 15 methods matching `IQueueEnvironment` interface exactly
  - All are accessor methods (getters in TypeScript, methods in interface)

## Dependencies
- `args` npm package: CLI argument parser (lines 1, 9, 62-65)
- `IQueueEnvironment` (Phase 14.16): Interface contract
- Queue constants: `DEFAULT_QUEUE_LISTENING_PORT`, `DEFAULT_QUEUE_SERVER_HOST_NAME`
- Inherits TS pattern from `BlobEnvironment` (Phase 12)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `args.parse(process.argv)` | `std::env::args()` + custom parser | CLI arg parsing; use clap crate for ergonomic alternative |
| `private flags = ...` | `struct` with parsed field values | Cache parsed arguments |
| `!== undefined` checks | `Option::is_some()` | Convert TS flag checks to Rust optionals |
| `RangeError` exceptions | `Result<T, RangeError>` or `panic!()` | Validation errors during init |

## Special handling
1. **CLI parsing architecture** (`QueueEnvironment.ts:1, 9-62`)
   - Uses external `args` library (not Node.js `process.argv` directly)
   - Parsing happens during class construction via `new QueueEnvironment()`
   - Constructor has zero parameters — configuration entirely from environment/CLI
   - `(args as any).config.name = "azurite-queue"` (line 62) sets program name for help text

2. **Boolean flag coercion** (`QueueEnvironment.ts:84-96`, `122-136`, etc.)
   - `!== undefined` check converts CLI presence (flag without value) to boolean
   - This pattern differs from `parseFloat()` for numeric values
   - Default is always `false` when flag absent
   - Preserves exact TS semantics: `--silent` alone → `silent() === true`, flag absent → `false`

3. **Validation: inMemoryPersistence ↔ location** (`QueueEnvironment.ts:138-150`)
   - `inMemoryPersistence()` and `location()` are mutually exclusive
   - If both set, throw `RangeError` with specific message
   - Validation happens in getter, not constructor — lazy validation on first call
   - Preserve exact error messages for CLI compatibility

4. **Validation: extentMemoryLimit requires inMemoryPersistence** (`QueueEnvironment.ts:145-147`)
   - `extentMemoryLimit()` is only valid with `inMemoryPersistence === true`
   - If set without inMemoryPersistence, throw `RangeError`
   - Again, lazy validation in getter

5. **Debug flag parsing quirk** (`QueueEnvironment.ts:156-169`)
   - `debug()` async method returns `Promise<string | boolean | undefined>`
   - If flag is a string (file path), return that string
   - If flag is `true` (no value provided), throw RangeError asking for file path
   - If flag absent, return `undefined` (implicit return at line 169)
   - Note: Line 169 has no explicit return — TypeScript implicitly returns `undefined`

6. **OAuth flag parsing** (`QueueEnvironment.ts:39`)
   - OAuth flag accepts string value (not boolean)
   - Returns `undefined` if not set
   - No validation — any string is accepted (validation deferred to downstream)

7. **ConfigurationBase fields absent**
   - QueueEnvironment does NOT extend ConfigurationBase (unlike Phase 4)
   - It only implements IQueueEnvironment interface (which includes some Phase 4 methods)
   - Separation of concerns: IQueueEnvironment handles environment/CLI, ConfigurationBase handles validation/defaults

## Change propagation notes
- If new CLI options are added to queue service, update args.option() calls (lines 9-60) and add corresponding getter methods
- Boolean flag parsing must preserve `!== undefined` semantics (not truthiness)
- Lazy validation in getters means order of calls matters — if inMemoryPersistence is checked before location, different errors may surface
- If args library is replaced, preserve exact flag names and default values for CLI backwards compatibility
- Match `BlobEnvironment.ts` structure (Phase 12) for consistency across services

