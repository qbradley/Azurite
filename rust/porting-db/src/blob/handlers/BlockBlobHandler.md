# Porting Record — `src/blob/handlers/BlockBlobHandler.ts`

## File info
- Source path: `src/blob/handlers/BlockBlobHandler.ts`
- Source lines: `507`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/block_blob_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::block_blob_handler`
- Phase: `11.5`
- Status: `not_started`

## Exported API
### Default class `BlockBlobHandler extends BaseHandler implements IBlockBlobHandler`
- Methods:
  - `upload()`
  - `putBlobFromUrl()` — unimplemented
  - `stageBlock()`
  - `stageBlockFromURL()` — unimplemented
  - `commitBlockList()`
  - `getBlockList()`
- Private helpers: `parseTier()`, `validateBlockId()`

## Dependencies
- `convertRawHeadersToMetadata()`, `getMD5FromStream()`, `getMD5FromString()`, `newEtag()`.
- `parseXML()` for commit-block-list body reparse.
- `IBlobMetadataStore` block/blob APIs.
- `getTagsFromString()` for header-based blob tag ingestion.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| streamed upload + immediate reread for MD5 | staged write followed by validation stream | Preserve the current write-then-reread flow instead of optimizing it away. |
| XML block list reparsing | explicit ordered XML parse helper | The generated parameter object loses element order, so raw XML is authoritative. |
| base64 block-id validation | helper fn returning bytes + original text | Canonical base64 round-trip and max-length checks are observable. |

## Special handling
- `BlockBlobHandler.ts:48-52` uses a ternary conditioned on `content-md5 || x-ms-blob-content-md5`, but the returned value path only falls back to `blobHTTPHeaders.blobContentMD5 || content-md5`; the presence of `x-ms-blob-content-md5` can affect control flow without supplying the value.
- `BlockBlobHandler.ts:60-69`, `199-209` append the whole body to extent storage first, then reject if stored byte count mismatches `content-length`.
- `BlockBlobHandler.ts:71-96`, `211-236` reread stored content to validate MD5 and compare string-vs-byte-array forms separately.
- `BlockBlobHandler.ts:127-139`, `363-383`, `469-483` default block blobs to `Hot` tier, set `accessTierInferred = true` when no explicit tier is provided, and accept `Cold` in addition to `Hot/Cool/Archive`.
- `BlockBlobHandler.ts:293-330` reparses the raw XML request body with ordered children (`parsed.$$`) because the deserialized `blocks` parameter does not preserve sequence.
- `BlockBlobHandler.ts:308-309` maps malformed commit XML to `InvalidXmlDocument`, not the more generic `InvalidOperation` used elsewhere.
- `BlockBlobHandler.ts:384-395` returns `contentMD5` computed from the raw XML block-list body, not from committed blob content.
- `BlockBlobHandler.ts:421-422` documents remaining gaps around synthetic uncommitted block blobs and conditional headers in `getBlockList()`.
- `BlockBlobHandler.ts:486-505` requires canonical base64 text (`blockId === raw.toString("base64")`) and limits decoded IDs to 64 bytes.

## Change propagation notes
- The commit-block-list XML reparse is architecture-critical; if swagger/autorest ever starts preserving block order, Aragorn can simplify only after proving wire equivalence.
- If TS later fixes the MD5-header precedence bug, keep the change visible in the Rust porting record because current behavior is compatibility-sensitive.
