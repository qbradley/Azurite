# Table Entity Types (Phase 15.4-15.7)

## File Info
- **Source Path:** `src/table/entity/` (12 files, ~973 LOC)
- **Rust Target:** `azurite-table/src/entity/`
- **Type:** EDM type system for Azure Table Storage
- **Phase:** 15.4-15.7
- **Complexity:** M (medium)
- **Status:** ported

## Exports

### IEdmType Interface & EdmType Enum (15.4)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdmType {
    Binary,     // bytes (base64-encoded)
    Boolean,    // true/false
    DateTime,   // ISO 8601
    Double,     // IEEE 754 double
    Guid,       // UUID
    Int32,      // 32-bit signed integer
    Int64,      // 64-bit signed integer (BigInt)
    String,     // UTF-8 string
    Null,       // Null value
}

pub trait IEdmType: Send + Sync {
    fn to_json_property_value_pair(
        &self,
        name: &str,
    ) -> Option<(String, JsonValue)>;
    
    fn to_json_property_value_string(&self, name: &str) -> Option<String>;
    
    fn to_json_property_type_pair(
        &self,
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
        force: Option<bool>,
    ) -> Option<(String, String)>;
    
    fn to_json_property_type_string(
        &self,
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
    ) -> Option<String>;
}

pub fn get_edm_type(type_str: &str) -> Option<EdmType> { ... }
```

### EdmXxx Type Implementations (15.5)
```rust
// EdmString (minimal)
pub struct EdmString {
    value: String,
}

impl IEdmType for EdmString { ... }

// EdmInt32 (54 LOC)
pub struct EdmInt32 {
    value: i32,
}

impl IEdmType for EdmInt32 { ... }

// EdmInt64 (64 LOC) — Uses BigNumber for precision
pub struct EdmInt64 {
    value: i64,  // Or BigInt if needed
}

impl IEdmType for EdmInt64 { ... }

// EdmDouble (88 LOC)
pub struct EdmDouble {
    value: f64,
}

impl IEdmType for EdmDouble { ... }

// EdmBoolean (47 LOC)
pub struct EdmBoolean {
    value: bool,
}

impl IEdmType for EdmBoolean { ... }

// EdmDateTime (71 LOC) — ISO 8601
pub struct EdmDateTime {
    value: DateTime<Utc>,  // or String for wire format
}

impl IEdmType for EdmDateTime { ... }

// EdmGuid (104 LOC) — UUID
pub struct EdmGuid {
    value: Uuid,
}

impl IEdmType for EdmGuid { ... }

// EdmBinary (64 LOC) — Base64-encoded
pub struct EdmBinary {
    value: Vec<u8>,
}

impl IEdmType for EdmBinary { ... }

// EdmNull (40 LOC) — Null type
pub struct EdmNull;

impl IEdmType for EdmNull { ... }
```

### EntityProperty (15.6)
```rust
#[derive(Clone)]
pub struct EntityProperty {
    pub name: String,
    pub value: Box<dyn IEdmType>,
    pub edm_type: EdmType,
    pub is_system_property: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationLevel {
    Full,      // All type annotations
    Minimal,   // Only non-standard types
    No,        // No type annotations
}

impl EntityProperty {
    pub fn new(
        name: String,
        value: Box<dyn IEdmType>,
        edm_type: EdmType,
        is_system_property: bool,
    ) -> Self { ... }
    
    pub fn to_json_property_value_pair(&self) -> Option<(String, JsonValue)> { ... }
    pub fn to_json_property_value_string(&self) -> Option<String> { ... }
    pub fn to_json_property_type_pair(
        &self,
        annotation_level: AnnotationLevel,
        force: Option<bool>,
    ) -> Option<(String, String)> { ... }
    pub fn to_json_property_type_string(
        &self,
        annotation_level: AnnotationLevel,
    ) -> Option<String> { ... }
    
    pub fn to_response_string(&self, annotation_level: AnnotationLevel) -> String { ... }
    
    pub fn from_xml(name: &str, value: &str, type_str: Option<&str>) -> Result<Self, ParseError> { ... }
}
```

### NormalizedEntity (15.7)
```rust
#[derive(Clone)]
pub struct NormalizedEntity {
    pub partition_key: String,
    pub row_key: String,
    pub etag: String,
    pub last_modified_time: DateTime<Utc>,  // Or ISO 8601 String
    pub timestamp: Option<i64>,  // Unix timestamp
    pub properties: HashMap<String, EntityProperty>,
}

impl NormalizedEntity {
    pub fn new(
        partition_key: String,
        row_key: String,
        etag: String,
        last_modified_time: DateTime<Utc>,
        properties: HashMap<String, EntityProperty>,
    ) -> Self { ... }
    
    pub fn to_response_string(&self, annotation_level: AnnotationLevel) -> String { ... }
    
    pub fn from_entity(entity: &Entity) -> Result<Self, ConversionError> { ... }
}
```

## Type Hierarchy

```
IEdmType (trait)
  ├── EdmString
  ├── EdmInt32
  ├── EdmInt64
  ├── EdmDouble
  ├── EdmBoolean
  ├── EdmDateTime
  ├── EdmGuid
  ├── EdmBinary
  └── EdmNull

EntityProperty (composite)
  └── Contains: IEdmType + name + is_system_property

NormalizedEntity (row container)
  └── Contains: partition_key + row_key + eTag + timestamp + HashMap<PropertyName, EntityProperty>
```

## Dependencies

- Phase 15.1: Generated context and models
- Common: DateTime utilities, UUID, JSON serialization
- Phase 1: IDataStore patterns

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `IEdmType` | `trait IEdmType` | Type interface |
| `EdmString` | `pub struct EdmString` | String wrapper |
| `EdmInt32` | `pub struct EdmInt32` | 32-bit integer |
| `EdmInt64` | `pub struct EdmInt64` | 64-bit integer |
| `EdmDouble` | `pub struct EdmDouble` | IEEE 754 float |
| `EdmBoolean` | `pub struct EdmBoolean` | Boolean wrapper |
| `EdmDateTime` | `pub struct EdmDateTime` | ISO 8601 datetime |
| `EdmGuid` | `pub struct EdmGuid` | UUID wrapper |
| `EdmBinary` | `pub struct EdmBinary` | Base64-encoded bytes |
| `EdmNull` | `pub struct EdmNull` | Null type |
| `EntityProperty` | `pub struct EntityProperty` | Property wrapper |
| `NormalizedEntity` | `pub struct NormalizedEntity` | Entity container |
| `AnnotationLevel` | `pub enum AnnotationLevel` | Metadata level enum |
| `any` | `Box<dyn IEdmType>` | Trait object |
| `Map<K, V>` | `HashMap<K, V>` | Property map |
| `Date` | `DateTime<Utc>` | Timestamp |
| `string` | `String` | Text |

## Special Handling / Fidelity Flags

### Critical EDM Type Patterns
1. **Type system is serialization-driven**:
   - Each type knows how to serialize to JSON with OData annotations
   - `to_json_property_*()` methods generate correct JSON representation
   - Type information encoded in `@odata.type` field based on annotation level

2. **Annotation levels control serialization detail**:
   - `Full`: All type annotations (e.g., `@odata.type: "Edm.Int32"`)
   - `Minimal`: Only non-standard types (e.g., Int64, DateTime, Guid, Binary)
   - `No`: No type annotations; type inferred from value format

3. **System properties are special**:
   - PartitionKey, RowKey, Timestamp, eTag are NOT in properties map
   - System properties never have type annotations (even in Full mode)
   - Timestamp is special: both regular field and Unix epoch conversion

4. **String type is implicit default**:
   - If no type annotation, assume string type
   - Comparison operations use string-based ordering
   - Number parsing required for numeric comparisons

5. **DateTime is ISO 8601 on wire**:
   - Stored as ISO 8601 string in JSON (`"2021-10-20T15:30:00Z"`)
   - Represented as `DateTime<Utc>` internally
   - Must preserve Z suffix exactly

6. **Int64 requires BigNumber handling**:
   - JavaScript limitation: cannot represent 64-bit integers safely
   - Some implementations use string representation for Int64
   - Rust: use `i64` with explicit overflow checks if needed

7. **Guid is UUID format**:
   - Stored as RFC 4122 UUID string (lowercase hex with dashes)
   - Example: `"550e8400-e29b-41d4-a716-446655440000"`
   - Case-insensitive parsing but preserve lowercase on output

8. **Binary is base64-encoded**:
   - Raw bytes encoded as base64 in JSON
   - Stored as string in properties map
   - Decoded on retrieval

### Comparison to Blob Types
- **Simpler type system**: Blob uses fewer types (mostly strings/metadata)
- **EDM standard compliance**: Table follows Azure Table Storage EDM spec
- **OData annotations**: Table includes OData metadata; blob doesn't

## Change Propagation

**Adding new EDM type:**
- Must implement IEdmType trait
- Must update get_edm_type() dispatch
- Must update serialization in EntityProperty
- Must update XML parsing logic

**Changing annotation levels:**
- Affects JSON serialization format
- May break clients expecting specific `@odata.type` format
- Must test with both minimal and full annotation levels

**Property name casing:**
- Property names preserved exactly as provided
- Case-sensitive lookups in properties map
- Must match TS behavior for case preservation

## Rust Porting Notes

1. **Trait objects**: Use `Box<dyn IEdmType>` for polymorphism
2. **Serialization**: Implement Serialize/Deserialize derive or manual impl
3. **DateTime handling**: Use `chrono::DateTime<Utc>` with explicit format strings
4. **UUID handling**: Use `uuid` crate for GUID type
5. **Base64 encoding**: Use `base64` crate for binary type
6. **BigInt handling**: Use `i64` for Int64 type; consider `num-bigint` if needed
7. **JSON serialization**: Use `serde_json` with custom serializers for OData format
8. **String ordering**: Use standard Rust string comparison (lexicographic)
9. **Null type**: Use `Option<T>` or dedicated `EdmNull` struct
10. **Property maps**: Use `HashMap<String, EntityProperty>` with case-sensitive keys
11. **Type dispatch**: Implement `match` statement for get_edm_type() function
12. **Clone requirement**: EDM types must be Clone for property copies

## Implementation Strategy

Propose 3-file porting units for Phase 15.4-15.7:
1. **Unit 1**: IEdmType trait + EdmNull + EdmString + EdmBoolean (core types)
2. **Unit 2**: EdmInt32 + EdmInt64 + EdmDouble + EdmDateTime + EdmGuid + EdmBinary (numeric/datetime/guid/binary types)
3. **Unit 3**: EntityProperty + NormalizedEntity (composition + serialization)

**Critical dependency**: Phase 15.1 (Context, generated models) must provide any required JSON/serialization utilities before implementing this phase.
