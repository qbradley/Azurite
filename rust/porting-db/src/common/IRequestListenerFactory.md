# Porting Record — `src/common/IRequestListenerFactory.ts`

## File info
- Source path: `src/common/IRequestListenerFactory.ts`
- Source lines: `11`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_request_listener_factory.rs`
- Crate: `azurite-common`
- Module: `i_request_listener_factory`
- Phase: `1.7`
- Status: `ported`

## Exported API
### Default interface `IRequestListenerFactory`
- `createRequestListener(): RequestListener`

### Imported type dependency
- `RequestListener` from `src/common/ServerBase.ts`
- TS shape: `(request: http.IncomingMessage, response: http.ServerResponse) => void`

## Dependencies
- `./ServerBase`
- Implementations: blob, queue, and table request listener factories.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `RequestListener` callback | `axum::Router` or boxed `tower::Service<Request<Body>>` | Strategy selects axum as the HTTP layer. |
| `void` listener return | Service/handler side effects | The TS callback writes directly to the response stream. |

## Recommended Rust translation
```rust
pub type RequestListener = axum::Router;

pub trait RequestListenerFactory: Send + Sync {
    fn create_request_listener(&self) -> RequestListener;
}
```

## Special handling
- The TS type is a raw Node HTTP request callback. In Rust, the closest faithful abstraction is a prebuilt router/service that the server binds before listening.
- Keep the factory boundary even if each service only has one concrete listener builder; `ServerBase` depends on this indirection.
- If the port needs more literal fidelity than `Router` provides, switch the alias to a boxed Tower service rather than collapsing the abstraction.

## Change propagation notes
- If `RequestListener` gains parameters or async setup in TS, update this record together with `ServerBase` and all service-specific factories.
- Future listener middleware additions should remain behind the factory method instead of leaking into server construction.

## Rust port notes
- Ported to `rust/crates/azurite-common/src/i_request_listener_factory.rs`.
- Forced deviation: `RequestListener` is currently aliased to `axum::Router` in `server_base.rs` because the Phase 1 common crate does not yet have a direct Rust equivalent for the raw Node request callback.
