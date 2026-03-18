# Porting Record — `src/queue/utils/utils.ts`

## File info
- Source path: `src/queue/utils/utils.ts`
- Source lines: `209`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/utils/utils.rs`
- Crate: `azurite-queue`
- Module: `utils::utils`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `Date` | `DateTime<Utc>` | Timestamp handling |
| `Buffer` | `Vec<u8>` | Binary data |

## Special handling
Queue utility functions: message encoding/decoding (base64), time calculations (visibility timeout, TTL), queue name validation, URL path parsing. Follows same pattern as blob equivalent. See `porting-db/src/blob/utils/utils.md` for detailed fidelity notes.
