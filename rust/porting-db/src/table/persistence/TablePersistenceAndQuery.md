# Table Persistence & Query Interpreter (Phase 15.9-15.12)

## File Info
- **Source Path:** `src/table/persistence/` (4 files + QueryInterpreter subsystem, ~2,022 LOC)
- **Rust Target:** `azurite-table/src/persistence/`
- **Type:** Metadata persistence + OData query interpreter
- **Phase:** 15.9-15.12
- **Complexity:** H (high)
- **Status:** analyzed

## Part 1: ITableMetadataStore & Implementations (15.9-15.11)

### ITableMetadataStore.ts (141 LOC)
```rust
pub trait ITableMetadataStore: ICleaner + Send + Sync {
    // Table operations
    async fn create_table(
        &self,
        context: &Context,
        table_model: &Table,
    ) -> Result<(), TableError>;
    
    async fn query_tables(
        &self,
        context: &Context,
        account: &str,
        query_options: &QueryOptions,
        next_table: Option<&str>,
    ) -> Result<(Vec<Table>, Option<String>), TableError>;
    
    async fn delete_table(
        &self,
        context: &Context,
        table: &Table,
        account: &str,
    ) -> Result<(), TableError>;
    
    async fn get_table(
        &self,
        account: &str,
        table: &str,
        context: &Context,
    ) -> Result<Table, TableError>;
    
    async fn set_table_acl(
        &self,
        account: &str,
        table: &str,
        context: &Context,
        acl: Option<&Vec<SignedIdentifier>>,
    ) -> Result<(), TableError>;
    
    // Entity operations
    async fn query_table_entities(
        &self,
        context: &Context,
        account: &str,
        table: &str,
        query_options: &QueryOptions,
        next_partition_key: Option<&str>,
        next_row_key: Option<&str>,
    ) -> Result<(Vec<Entity>, Option<String>, Option<String>), EntityError>;
    
    async fn query_table_entities_with_partition_and_row_key(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        partition_key: &str,
        row_key: &str,
        batch_id: Option<&str>,
    ) -> Result<Option<Entity>, EntityError>;
    
    async fn insert_or_update_table_entity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: &Entity,
        if_match: Option<&str>,
        batch_id: Option<&str>,
    ) -> Result<Entity, EntityError>;
    
    async fn insert_or_merge_table_entity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: &Entity,
        if_match: Option<&str>,
        batch_id: Option<&str>,
    ) -> Result<Entity, EntityError>;
    
    async fn delete_table_entity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        partition_key: &str,
        row_key: &str,
        if_match: Option<&str>,
        batch_id: Option<&str>,
    ) -> Result<(), EntityError>;
    
    // Batch operations
    async fn begin_batch_transaction(
        &self,
        account: &str,
        table: &str,
        batch_id: &str,
    ) -> Result<(), BatchError>;
    
    async fn commit_batch_transaction(
        &self,
        batch_id: &str,
    ) -> Result<(), BatchError>;
    
    async fn abort_batch_transaction(
        &self,
        batch_id: &str,
    ) -> Result<(), BatchError>;
}

// Type definitions
pub type Table = {
    account: String,
    table: String,
    table_acl: Option<Vec<SignedIdentifier>>,
} & OdataAnnotationsOptional;

pub type Entity = {
    partition_key: String,
    row_key: String,
    etag: String,
    last_modified_time: DateTime<Utc>,
    properties: HashMap<String, EntityProperty>,
} & OdataAnnotationsOptional;

pub struct OdataAnnotationsOptional {
    pub odata_metadata: Option<String>,
    pub odata_type: Option<String>,
    pub odata_id: Option<String>,
    pub odata_edit_link: Option<String>,
}

pub struct QueryOptions {
    pub filter: Option<String>,      // OData filter expression
    pub select: Option<Vec<String>>, // Columns to return
    pub top: Option<u32>,            // Max results
}
```

### LokiTableMetadataStore.ts (1,088 LOC)
```rust
pub struct LokiTableMetadataStore {
    db: Arc<Mutex<LokiDatabase>>,
    tables_collection: Arc<Mutex<LokiCollection>>,
    entities_collection: Arc<Mutex<LokiCollection>>,
    transactions_collection: Arc<Mutex<LokiCollection>>,
}

impl ITableMetadataStore for LokiTableMetadataStore {
    // All 21 methods from interface
}

impl ICleaner for LokiTableMetadataStore {
    async fn clear_async(&self) -> Result<(), PersistenceError>;
}
```

**Data model** (3 Loki collections):
1. **tables**: Table metadata (name, ACL, created time)
2. **entities**: Entity records (PartitionKey, RowKey, properties, eTag, timestamp)
3. **transactions**: Batch transaction state

### LokiTableStoreQueryGenerator.ts (99 LOC)
```rust
pub struct LokiQueryGenerator;

impl LokiQueryGenerator {
    pub fn generate_loki_filter(
        odata_filter: &str,
    ) -> Result<String, QueryError> { ... }
}

// Generates Loki query syntax from OData filters
// Example: `PartitionKey eq 'key1'` → Loki filter for partition_key == "key1"
// Supports operators: ==, !=, <, <=, >, >=, and, or, not
```

## Part 2: Query Interpreter Subsystem (15.12)

### QueryInterpreter Architecture (18 files, ~1,135 LOC)

**High-level design:**
- **Lexer** → tokenize OData filter string
- **Parser** → parse tokens into AST (Abstract Syntax Tree)
- **Validator** → validate AST semantics
- **Interpreter** → execute AST against entities (visitor pattern)

### Core Files

#### QueryLexer.ts
```rust
pub struct QueryLexer {
    input: String,
    position: usize,
}

impl QueryLexer {
    pub fn new(input: &str) -> Self { ... }
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> { ... }
}

pub enum Token {
    // Literals
    Identifier(String),
    StringLiteral(String),
    IntLiteral(i32),
    LongLiteral(i64),
    FloatLiteral(f64),
    DateTimeLiteral(DateTime<Utc>),
    GuidLiteral(Uuid),
    BinaryLiteral(Vec<u8>),
    
    // Operators
    Eq,              // eq
    Ne,              // ne
    Lt,              // lt
    Le,              // le
    Gt,              // gt
    Ge,              // ge
    And,             // and
    Or,              // or
    Not,             // not
    
    // Keywords
    True,
    False,
    Null,
    
    // Punctuation
    LeftParen,       // (
    RightParen,      // )
    Eof,
}
```

#### QueryParser.ts
```rust
pub struct QueryParser {
    tokens: Vec<Token>,
    position: usize,
}

impl QueryParser {
    pub fn new(tokens: Vec<Token>) -> Self { ... }
    pub fn parse(&mut self) -> Result<Box<dyn IQueryNode>, ParseError> { ... }
}

// Recursive descent parser producing AST
// Grammar:
// expression := or_expression
// or_expression := and_expression ('or' and_expression)*
// and_expression := not_expression ('and' not_expression)*
// not_expression := ('not')? comparison_expression
// comparison_expression := additive_expression (comp_op additive_expression)?
// additive_expression := multiplicative_expression (('+' | '-') multiplicative_expression)*
// multiplicative_expression := unary_expression (('*' | '/') unary_expression)*
// unary_expression := ('-')? primary_expression
// primary_expression := literal | identifier | '(' expression ')'
```

#### QueryValidator.ts
```rust
pub struct QueryValidator;

impl QueryValidator {
    pub fn validate(
        &self,
        ast: &dyn IQueryNode,
        entity_schema: &EntitySchema,
    ) -> Result<(), ValidationError> { ... }
}

// Validates:
// - Property names exist in entity schema
// - Type conversions are valid
// - Operator operands are compatible types
```

#### QueryInterpreter.ts
```rust
pub struct QueryInterpreter;

impl QueryInterpreter {
    pub fn evaluate(
        &self,
        ast: &dyn IQueryNode,
        entity: &NormalizedEntity,
    ) -> Result<bool, EvaluationError> { ... }
    
    pub fn select(
        &self,
        entity: &NormalizedEntity,
        select_fields: &[String],
    ) -> NormalizedEntity { ... }
}

// Visitor pattern: evaluates AST against entity
// Returns true/false for filter expressions
// Selects subset of properties for select clauses
```

### Query Node Types (22 types)

```rust
pub trait IQueryNode: Send + Sync {
    fn evaluate(&self, entity: &NormalizedEntity) -> Result<bool, EvaluationError>;
    fn accept<V: QueryNodeVisitor>(&self, visitor: &V) -> Result<V::Output, Error>;
}

// Binary operators (logical & comparison)
pub struct AndNode {
    left: Box<dyn IQueryNode>,
    right: Box<dyn IQueryNode>,
}

pub struct OrNode {
    left: Box<dyn IQueryNode>,
    right: Box<dyn IQueryNode>,
}

pub struct EqualsNode {
    left: Box<dyn IQueryNode>,
    right: Box<dyn IQueryNode>,
}

pub struct NotEqualsNode { ... }
pub struct LessThanNode { ... }
pub struct LessThanEqualNode { ... }
pub struct GreaterThanNode { ... }
pub struct GreaterThanEqualNode { ... }

// Unary operators
pub struct NotNode {
    operand: Box<dyn IQueryNode>,
}

// Literals
pub struct ConstantNode {
    value: JsonValue,
}

pub struct DateTimeNode {
    value: DateTime<Utc>,
}

pub struct GuidNode {
    value: Uuid,
}

pub struct BinaryDataNode {
    value: Vec<u8>,
}

pub struct BigNumberNode {
    value: String,  // For Int64 representation
}

// Variables (property references)
pub struct IdentifierNode {
    name: String,
}

pub struct ValueNode {
    property_name: String,
    entity_property: Option<EntityProperty>,
}

// Query context
pub trait IQueryContext: Send + Sync {
    fn get_property(&self, name: &str) -> Option<&EntityProperty>;
    fn get_system_property(&self, name: &str) -> Option<String>;
}
```

## Dependencies

- Phase 1: IDataStore, ICleaner
- Phase 2: LokiDatabase, LokiCollection
- Phase 15.4-15.7: EntityProperty, NormalizedEntity, EdmTypes
- Phase 15.1: Context, Entity models
- Common: DateTime utilities, UUID

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `ITableMetadataStore` | `pub trait ITableMetadataStore` | Interface |
| `Table` | `Table` struct | Table metadata |
| `Entity` | `Entity` struct | Entity record |
| `QueryOptions` | `QueryOptions` struct | Filter/select/top |
| `Loki.Collection` | `Arc<Mutex<LokiCollection>>` | Shared storage |
| `Promise<T>` | `Result<T, Error>` | Async result |
| `IQueryNode` | `trait IQueryNode` | AST node |
| `Token` | `enum Token` | Lexer token |
| `string` | `String` | Text |
| `number` | `i32`, `i64`, `f64` | Numeric values |

## Special Handling / Fidelity Flags

### Critical Query Interpreter Patterns
1. **OData filter is lazy-parsed**:
   - Filter string not validated until evaluation time
   - Unknown properties in filters don't error (may return no matches)
   - Type mismatches in comparisons use string-based ordering

2. **String-based type coercion**:
   - Numeric properties compared as strings if type info missing
   - Comparison operators use lexicographic ordering by default
   - Int64 represented as string to avoid JS precision loss

3. **Visitor pattern for extensibility**:
   - Each node type accepts visitor for pattern matching
   - Allows different evaluation strategies (filter vs select vs count)
   - Critical for batch operation evaluation

4. **Property access is case-sensitive**:
   - OData filter property names must match entity property case exactly
   - System properties (PartitionKey, RowKey, Timestamp, eTag) are special
   - Custom properties case-preserved from insertion

5. **Time-based comparisons**:
   - DateTime literals parsed from ISO 8601 with Z suffix
   - Comparison uses UTC timestamp ordering
   - Null/missing timestamps sorted to end

6. **Batch evaluation context**:
   - QueryInterpreter used during batch transaction processing
   - Filter expressions evaluated against each entity in batch
   - Results affect conditional PUT/MERGE/DELETE operations

### Comparison to Blob Query System
- **Different scope**: Blob uses simpler tag-based filtering
- **Full OData support**: Table implements subset of OData query language
- **Type-aware**: Table interpreter understands EDM types
- **Property-based**: Table filters on entity properties vs blob tags

## Change Propagation

**OData filter changes:**
- New operators → lexer + parser updates required
- New keywords → token type additions
- Grammar changes → parser update

**Entity schema changes:**
- New property types → validator must understand new types
- Property removal → may break existing queries (error or empty result?)
- Type changes → comparison semantics may change

**Query result changes:**
- Select clause changes → property filtering logic
- Filter operator behavior changes → evaluation results differ
- Sort order changes (if added) → ordering behavior differs

## Rust Porting Notes

1. **Lexer**: Character-by-character parsing with lookahead
2. **Parser**: Recursive descent with precedence handling
3. **AST nodes**: Use `Box<dyn IQueryNode>` for tree construction
4. **Visitor pattern**: Trait object for different evaluation strategies
5. **DateTime parsing**: ISO 8601 with Z suffix; use `chrono::DateTime::parse_from_rfc3339()`
6. **Type coercion**: Explicit string conversion for numeric properties
7. **Property lookup**: Case-sensitive HashMap access
8. **Null handling**: Use `Option<T>` for missing properties
9. **Error propagation**: Define QueryError, ParseError, ValidationError types
10. **Short-circuit evaluation**: AND/OR nodes stop on first decisive result
11. **String comparisons**: Lexicographic ordering for all types (fallback)
12. **Batch context**: Pass entity to evaluate() for property resolution

## Implementation Strategy

Propose 3-file porting units for Phase 15.9-15.12:
1. **Unit 1**: ITableMetadataStore interface + LokiTableMetadataStore (persistence)
2. **Unit 2**: QueryLexer + QueryParser + QueryValidator (parsing layer)
3. **Unit 3**: QueryInterpreter + 22 node types (evaluation layer)

**Critical dependency**: Phase 15.4-15.7 (EntityProperty, NormalizedEntity) must complete before query interpreter can be fully ported.
