# Porting Record — `src/common/Telemetry.ts`

## File info
- Source path: `src/common/Telemetry.ts`
- Source lines: `410`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/telemetry.rs`
- Crate: `azurite-common`
- Module: `telemetry`
- Phase: `4.11`
- Status: `analyzed`

## Exported API
### Exported class `AzuriteTelemetryClient`
- Public static field: `isVSC: boolean = false`
- Public static method: `init(location: string, enableTelemetry: boolean, env: any, isVSC: boolean = false): void`
- Public static method: `createAppInsigntClient(cloudRole: string, samplingPercentage: number | undefined, maxBatchSize: number | undefined): TelemetryClient`
- Public static method: `TraceRequest(context: any): void`
- Public static method: `TraceStartEvent(serviceType: string = ""): Promise<void>`
- Public static method: `TraceStopEvent(serviceType: string = ""): void`
- Private static method: `removeRoleInstance(envelope: Contracts.EnvelopeTelemetry): boolean`
- Private static method: `GetRequestUri(endpoint: string): string`
- Private static method: `GetInstanceID(inMemoryPersistence: boolean = false): string`
- Private static method: `GetRequestAuthentication(authorizationHeader: string | undefined, sigQuery: string | undefined): string`
- Private static method: `GetAllParameterString(): Promise<string>`

### Static state that matters to translation
- `eventClient: TelemetryClient | undefined`
- `requestClient: TelemetryClient | undefined`
- `enableTelemetry: boolean = true`
- `location: string`
- `_totalIngressSize: number = 0`
- `_totalEgressSize: number = 0`
- `_totalBlobRequestCount: number = 0`
- `_totalQueueRequestCount: number = 0`
- `_totalTableRequestCount: number = 0`
- `sessionID = uuid()`
- `instanceID = ""`
- `initialized = false`
- `env: any = undefined`
- debug configuration statics: `isDebug`, `requestCollectPercentage`, `enableAppInsightLog`, `cloudRole`, `requestMaxBatchSize`
- `appInsights = require("applicationinsights")`

## Dependencies
- External packages:
  - `applicationinsights` / `applicationinsights/out/Library/TelemetryClient`
  - `uuid`
- Node built-ins: `crypto`, `fs`, `path.join`, `URL`, `process.argv`, `process.env`.
- Internal imports:
  - `./Logger` — Phase `4.4`, analyzed in this pass.
  - `../blob/generated/Context` — Phase `5.7`, not yet ported.
  - `../blob/generated/artifacts/operation` — Phase `5.11`, not yet ported.
  - `../queue/generated/` — Phase `14.1` generated group, not yet ported.
  - `../table/generated/` — Phase `15.1` generated group, not yet ported.
  - `../blob/utils/constants` — Phase `12.1`, not yet ported.
  - `../queue/utils/constants` — Phase `14.23`, not yet ported.
  - `../table/utils/constants` — Phase `15.30`, not yet ported.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| all-static singleton class | global `TelemetryState` inside `OnceCell<Mutex<_>>` / `Lazy<RwLock<_>>` | TS design is process-global and mutable. |
| `TelemetryClient` | wrapper around App Insights client or custom HTTP emitter | Keep two logical clients if sampling policies differ. |
| `env: any` | enum/trait object representing CLI args vs VS Code configuration | TS passes either CLI-ish environment object or VS Code workspace config. |
| counters as mutable numbers | `u64` behind mutex or atomics | Preserve separate blob/queue/table request counts and ingress/egress totals. |
| generated `Context` types | trait/enum abstraction over service contexts | `TraceRequest` inspects operation, request, response, headers, and context IDs. |
| `Promise<void>` helpers | `async fn` | `TraceStartEvent()` and `GetAllParameterString()` are async today. |

## Recommended Rust translation
- Keep telemetry as a dedicated common-layer singleton service, not as free functions sprinkled across request handlers.
- Represent the mutable static fields as one explicit Rust state struct so future TS diffs map mechanically.
- Preserve the split between `eventClient` and `requestClient`, because TS uses different sampling policies for lifecycle events vs per-request telemetry.
- Prefer an explicit adapter around the telemetry SDK (or a custom HTTP emitter) so privacy filtering and payload shaping remain local to this module.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| `init()` | `fn init(location: &Path, enable_telemetry: bool, env: TelemetryEnv, is_vsc: bool)` | Preserve one-time initialization, instance/session ID setup, and logger side effects. |
| `createAppInsigntClient()` | `fn create_app_insignt_client(...) -> TelemetryClient` | Keep the typo in the analysis name visible; it exists in TS. |
| `TraceRequest()` | `fn trace_request(context: &dyn TelemetryContext)` or `async fn` wrapper | Preserve service-type detection, request counting, property collection, and result classification. |
| `TraceStartEvent()` | `async fn trace_start_event(service_type: &str)` | Includes parameter summary via `GetAllParameterString()`. |
| `TraceStopEvent()` | `fn trace_stop_event(service_type: &str)` | Current TS version is sync, unlike `TraceStartEvent()`. |
| `removeRoleInstance()` | telemetry processor/filter callback | Preserve SHA-256 hashing of `ai.cloud.roleInstance` and blanking of `ai.operation.name`. |
| `GetRequestUri()` | helper that sanitizes endpoint string | Preserve current host-hiding attempt, including its bug/quirk. |
| `GetInstanceID()` | helper reading/writing config file | Preserve on-disk key typo (`instaceID`) unless the team explicitly approves normalization. |
| `GetRequestAuthentication()` | helper mapping auth header + SAS query into text label | Keep output labels such as `Sas`, `Anonymous`, and `Bearer,Sas`. |
| `GetAllParameterString()` | async helper building comma-separated parameter-name list | Records parameter names, not values. |

## Special handling
- `init()` is intentionally defensive and side-effect heavy: it stores location/env flags, computes `instanceID`, logs the `InstaceID`/`SessionID`, creates clients if needed, calls `appInsights.start()`, and marks `initialized = true`. Repeated calls mostly log a “Don't need initialize Telemetry” message.
- `createAppInsigntClient()` hard-codes the full Application Insights connection string in source and disables most auto-collection features before creating a dedicated `TelemetryClient`.
- `removeRoleInstance()` does not actually remove the role instance; it hashes `ai.cloud.roleInstance` with SHA-256 and blanks `ai.operation.name`.
- `TraceRequest()` uses `instanceof BlobContext/QueueContext/TableContext` to determine service type and request-name prefix (`B_`, `Q_`, `T_`). It increments separate per-service counters and records `ReqNo` in properties.
- `TraceRequest()` records:
  - `apiVersion` as `"v" + x-ms-version` (so missing headers become `vundefined`).
  - `authorization` using the first auth-header token plus `,Sas` if a `sig` query param exists.
  - `ingress` from request `content-length` when parseable.
  - `egress` from response `content-length` when the method is not `HEAD`.
  - `duration` from `new Date().getTime() - context.startTime.getTime()`.
  - `id` from `context.contextId` only, even though the logger call later falls back to `context.contextID`.
  - placeholder `contextObjects.operationName` / `operation_Name` values of `"test"`.
- `GetRequestUri()` tries to hide local hostnames by replacing them with `[hidden]`, but it checks `if (uri.hostname.toLowerCase() in knownHosts)`. Because `knownHosts` is an array, this uses the JS `in` operator against indices rather than values, so the intended redaction does not trigger for `127.0.0.1`, `localhost`, or `host.docker.internal`.
- `GetInstanceID()` persists JSON at `<location>/AzuriteConfig` using the misspelled key `instaceID`. The typo is used consistently for read and write.
- If `inMemoryPersistence` is true, `GetInstanceID()` skips disk entirely and returns a fresh UUID each init.
- `GetAllParameterString()` has two modes:
  - VS Code mode reads `workspaceConfiguration.get(flag)` and suppresses values equal to the default host/timeout/port values.
  - CLI mode scans `process.argv` for long flags and selected short flags (`d`, `l`, `L`, `s`).
- In VS Code mode, default suppression for all `*Host` and `*KeepAliveTimeout` flags compares against blob defaults (`DEFAULT_BLOB_SERVER_HOST_NAME`, `DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT`). That is harmless today because queue/table defaults match blob defaults, but it is brittle if the services diverge later.
- The module uses broad `try/catch` blocks that log warnings and swallow telemetry errors.

## Patterns requiring special handling
- **Application Insights SDK → Rust telemetry equivalent**: this file is tightly coupled to the Node SDK's setup/client/processor model.
- **Process-global mutable singleton**: every field is static and shared across requests.
- **Generated service context inspection**: request telemetry depends on generated blob/queue/table `Context` and operation enums.
- **File-backed instance identity**: persistent ID is stored in a local config file unless in-memory persistence is enabled.

## Change propagation notes
- If TS changes the telemetry payload shape or privacy filtering, update this record before Aragorn changes the Rust emitter.
- If service-specific generated contexts or operation enums change names, revisit `TraceRequest()` request-name generation and `instanceof` dispatch.
- If blob/queue/table default host/timeout values ever diverge, `GetAllParameterString()` needs an audit because it currently assumes blob defaults for all services in VSC mode.
- If the team decides to normalize the `instaceID` typo or fix the `knownHosts` redaction bug in Rust, document that as an explicit divergence/decision rather than silently correcting it.

## Fidelity risks and edge cases
- Hard-coded telemetry connection string is a real source dependency and may have governance/privacy implications.
- The host-redaction bug in `GetRequestUri()` is observable privacy behavior today; fixing it in Rust without approval would be a semantic change.
- `TraceStopEvent()` is synchronous and there is no explicit flush path in this file, so shutdown delivery guarantees are weak.
- Telemetry state is unsynchronized mutable global state in TS. Rust must serialize access explicitly without hiding the singleton shape.
- `contextId` vs `contextID` mismatch appears again here: telemetry request ID uses only `contextId`, while logger output falls back between both spellings.
