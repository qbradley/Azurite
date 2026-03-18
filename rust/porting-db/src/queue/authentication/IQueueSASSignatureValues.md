# Porting Record — `src/queue/authentication/IQueueSASSignatureValues.ts`

## File info
- Source path: `src/queue/authentication/IQueueSASSignatureValues.ts`
- Source lines: `155`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/i_queue_sas_signature_values.rs`
- Crate: `azurite-queue`
- Module: `authentication::i_queue_sas_signature_values`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
SAS signature values struct for queue service. Canonical resource format: `/queueservices/accountname/queuename`. String-to-sign component order is critical.
