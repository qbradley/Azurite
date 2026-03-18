# Porting Record — `src/table/batch/ (18 files)`

## File info
- Source path: `src/table/batch/ (18 files)`
- Source lines: `~2500`
- Source type: `handwritten`
- Rust target: `azurite-table/src/batch/mod.rs`
- Crate: `azurite-table`
- Module: `batch`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `BatchRequest` | `BatchRequest struct` | Parsed multipart batch |
| `TableBatchOrchestrator` | `TableBatchOrchestrator` | Executes batch operations transactionally |
| `TableBatchSerialization` | `batch serialization fns` | Multipart MIME parsing/generation |
| `BatchOperation` | `enum BatchOperation` | Insert/Update/Merge/Delete/Query |
| `IOptionalParams` | `trait/struct` | Operation-specific optional params |

## Special handling
Table batch is the most complex table-specific subsystem. Handles OData multipart batch protocol ($batch endpoint).

Key fidelity concerns:
1. **Multipart MIME parsing**: Boundary detection, Content-ID tracking, changeset atomicity
2. **Transaction semantics**: All operations in a changeset succeed or fail together
3. **Entity group transactions**: All entities in a batch must share the same PartitionKey
4. **Sub-request routing**: Each batch sub-request is parsed and dispatched to the appropriate handler method
5. **Error rollback**: If any operation fails, all prior operations in the changeset must be rolled back
6. **Response serialization**: Multipart response with per-operation status codes and bodies

No blob equivalent — blob batch uses a different protocol (JSON-based). Table batch is OData multipart.

## Change propagation notes
Batch protocol changes require updating both parsing (TableBatchSerialization) and orchestration (TableBatchOrchestrator). Entity group transaction constraints are hard-coded.
