# Porting Record — `src/common/ServerBase.ts`

## File info
- Source path: `src/common/ServerBase.ts`
- Source lines: `201`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/server_base.rs`
- Crate: `azurite-common`
- Module: `server_base`
- Phase: `4.8`
- Status: `ported`

## Exported API
### Type alias `RequestListener`
- `type RequestListener = (request: http.IncomingMessage, response: http.ServerResponse) => void`

### Enum `ServerStatus`
- `Closed = "Closed"`
- `Starting = "Starting"`
- `Running = "Running"`
- `Closing = "Closing"`

### Default abstract class `ServerBase implements ICleaner`
- Property: `protected status: ServerStatus = ServerStatus.Closed`
- Property: `public readonly httpServer: (http.Server | https.Server) & stoppable.WithStop`
- Constructor:
  `constructor(host: string, port: number, httpServer: http.Server | https.Server, requestListenerFactory: IRequestListenerFactory, config: ConfigurationBase)`
- Method: `getHttpServerAddress(): string`
- Method: `getStatus(): ServerStatus`
- Method: `start(): Promise<void>`
- Method: `close(): Promise<void>`
- Method: `clean(): Promise<void>`
- Protected hook: `beforeStart(): Promise<void>`
- Protected hook: `afterStart(): Promise<void>`
- Protected hook: `beforeClose(): Promise<void>`
- Protected hook: `afterClose(): Promise<void>`

## Dependencies
- Node built-ins: `http`, `https`.
- External package: `stoppable`.
- Internal imports:
  - `./ConfigurationBase` — Phase `4.7`, analyzed in this pass.
  - `./ICleaner` — Phase `1.2`, already ported.
  - `./IRequestListenerFactory` — Phase `1.7`, already ported.
- Downstream subclasses in later phases: blob/queue/table server implementations.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| request-listener callback | `tower::Service<Request<Body>>` / axum `Router` | `ServerBase` itself is transport/lifecycle glue, not request business logic. |
| `http.Server | https.Server` union | server handle enum or generic transport wrapper | Need to preserve HTTP vs HTTPS selection driven by config/certs. |
| `stoppable.WithStop` intersection | graceful-shutdown handle | Rust equivalent should expose a shutdown trigger/future. |
| abstract base class | base struct + lifecycle trait hooks | Matches project decision to use composition instead of inheritance. |
| string enum `ServerStatus` | Rust enum with same variant names | Keep transition visibility for diagnostics and tests. |

## Recommended Rust translation
- Model this as a `ServerBase` struct holding host, port, config, status, and a shutdown handle, plus a trait or hook object for `before_*` / `after_*` lifecycle steps.
- Use `axum`/`hyper` for request serving and a `tokio::sync::watch`/`oneshot` shutdown signal to replace `stoppable.stop()`.
- Preserve the strict state machine and lifecycle ordering even if the actual server runtime is more idiomatic in Rust.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| `RequestListener` | router/service factory output | In TS this can be an Express-compatible function; in Rust it should be the service/router value produced by the factory. |
| constructor | `fn new(host, port, transport, request_listener_factory, config) -> Self` | Preserve keep-alive timeout setup, listener replacement, and request-factory binding at construction time. |
| `getHttpServerAddress()` | `fn http_server_address(&self) -> String` | Preserve empty-string result before the listener has an address and protocol choice based on `config.hasCert()`. |
| `getStatus()` | `fn status(&self) -> ServerStatus` | Direct status accessor. |
| `start()` | `async fn start(&mut self) -> Result<()>` | Preserve Closed→Starting→Running transition and reset to Closed on startup failure. |
| `close()` | `async fn close(&mut self) -> Result<()>` | Preserve Running→Closing→Closed transition plus request-rejection before shutdown. |
| lifecycle hooks | trait methods with default no-op async impls | Keep overridable extension points for subclasses. |

## Special handling
- Constructor behavior matters: if `config.keepAliveTimeout > 0`, the timeout is applied in seconds×1000 to the Node server before request listeners are rebound.
- The constructor removes all pre-existing `request` listeners and attaches exactly one listener from `requestListenerFactory.createRequestListener()`. Preserve the “single bound listener” model when mapping to axum/tower.
- `getHttpServerAddress()` reflects the actual bound address from `httpServer.address()`, not the configured `(host, port)` pair. This matters when `port` is `0` and the OS chooses a random port.
- `start()` calls `beforeStart()` first, then waits for `listen()`, then marks `Running`, then calls `afterStart()`. `afterStart()` is not inside the startup `try/catch`, so if it throws the server stays in `Running`.
- `close()` calls `beforeClose()`, removes all request listeners, calls `httpServer.stop()`, then `afterClose()`, then marks `Closed`. There is no `try/finally`, so if `beforeClose()` or `afterClose()` throws, status remains `Closing`.
- `clean()` is a no-op default even though the class implements `ICleaner`.

## Patterns requiring special handling
- **Node HTTP/HTTPS server lifecycle → axum/hyper**: translate the transport-specific Node server plus `stoppable` shutdown wrapper into an axum/hyper server task with an explicit graceful-shutdown signal.
- **Request listener factory abstraction**: TS binds a callback from `IRequestListenerFactory`; Rust should preserve an analogous factory boundary rather than wiring routers directly into every subclass.
- **Composition over inheritance**: concrete Rust servers should embed `ServerBase` and delegate lifecycle hooks rather than trying to simulate TS inheritance.

## Change propagation notes
- If TS changes lifecycle hook order or adds new hooks, update both the base struct and every concrete server subclass in Rust.
- If request binding logic changes (for example, middleware installed before factory binding), revisit both `ServerBase` and each request-listener factory together.
- If TS adds more server statuses, preserve them explicitly rather than collapsing into boolean running/stopped flags.

## Fidelity risks and edge cases
- The strict state machine is behaviorally significant. Starting twice or closing while not running currently throws.
- `afterStart()` failures leave the server running; `afterClose()` failures leave status `Closing`. Rust error handling should document these asymmetries instead of silently “improving” them.
- `stoppable.stop()` is Node-specific and does not directly map to axum. The Rust translation must preserve the intent—stop accepting new requests and drain in-flight work—without hiding the shutdown boundary.

## Rust port notes
- Ported the lifecycle wrapper into `rust/crates/azurite-common/src/server_base.rs` using axum/tokio for bind/start/close while preserving the TS `Closed → Starting → Running → Closing` state machine.
- The Rust version keeps the explicit before/after hook ordering and the address lookup behavior that reflects the bound socket rather than only the configured port.
