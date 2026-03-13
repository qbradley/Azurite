# Porting Record — `src/common/IServerFactory.ts`

## File info
- Source path: `src/common/IServerFactory.ts`
- Source lines: `5`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_server_factory.rs`
- Crate: `azurite-common`
- Module: `i_server_factory`
- Phase: `1.8`
- Status: `analyzed`

## Exported API
### Default interface `IServerFactory`
- `createServer(): Promise<ServerBase>`

## Dependencies
- `./ServerBase`
- Observed usage: the common interface exists, but concrete factories such as `BlobServerFactory` do not explicitly implement it and may accept optional environment arguments.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Promise<ServerBase>` | `async fn -> Result<Self::Server, StorageError>` | Associated type preserves concrete server subtype. |
| abstract `ServerBase` | `trait Server` or `Box<dyn Server>` | Depends on how `ServerBase` is translated. |

## Recommended Rust translation
```rust
#[async_trait]
pub trait ServerFactory: Send + Sync {
    type Server: Server;
    async fn create_server(&self) -> Result<Self::Server, StorageError>;
}
```

## Special handling
- The TS interface is narrower than actual factory implementations. `BlobServerFactory.createServer(blobEnvironment?)` returns a concrete union type and takes an optional argument not present here.
- Prefer an associated type over a forced boxed trait object so service factories can keep their concrete return types.
- Treat this interface as a common abstraction point, not proof that every factory is already substitutable at runtime.

## Change propagation notes
- If TS aligns the interface with concrete factories by adding parameters, update both this record and all factory implementations.
- Revisit the return strategy if `ServerBase` translation changes from trait-object friendly to concrete composition.
