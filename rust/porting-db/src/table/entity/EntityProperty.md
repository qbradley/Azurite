# Porting Record — `src/table/entity/EntityProperty.ts`

## File info
- Source path: `src/table/entity/EntityProperty.ts`
- Source lines: `~120`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/entity_property.rs`
- Crate: `azurite-table`
- Module: `entity::entity_property`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `EntityProperty` | `EntityProperty struct` | Property wrapper |
| `AnnotationLevel` | `enum AnnotationLevel` | Full/Minimal/No |
| `Box<dyn IEdmType>` | `Box<dyn IEdmType>` | Polymorphic EDM value |

## Special handling
Wraps an EDM value with name, type tag, and system-property flag. Serialization methods respect AnnotationLevel (Full → all type annotations, Minimal → only non-default types, No → no annotations). System properties (PartitionKey, RowKey, Timestamp) never get type annotations.
