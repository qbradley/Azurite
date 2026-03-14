# Phase 15 Table Service Translation Status

## Translation Progress: 12 of 116 files (10%)

### ✅ COMPLETED (12 files):

#### Entity Type System (Complete)
- [x] `src/table/entity/IEdmType.ts` → `entity/i_edm_type.rs`
- [x] `src/table/entity/EntityProperty.ts` → `entity/entity_property.rs`
- [x] `src/table/entity/EdmString.ts` → `entity/edm_string.rs`
- [x] `src/table/entity/EdmNull.ts` → `entity/edm_null.rs`
- [x] `src/table/entity/EdmBoolean.ts` → `entity/edm_boolean.rs`
- [x] `src/table/entity/EdmInt32.ts` → `entity/edm_int32.rs`
- [x] `src/table/entity/EdmInt64.ts` → `entity/edm_int64.rs`
- [x] `src/table/entity/EdmDouble.ts` → `entity/edm_double.rs`
- [x] `src/table/entity/EdmDateTime.ts` → `entity/edm_date_time.rs`
- [x] `src/table/entity/EdmGuid.ts` → `entity/edm_guid.rs`
- [x] `src/table/entity/EdmBinary.ts` → `entity/edm_binary.rs`
- [x] `src/table/entity/NormalizedEntity.ts` → `entity/normalized_entity.rs`

**Status:** Compiles cleanly, no clippy warnings. Complete EDM type system with 9 type implementations.

### ⬜ REMAINING (104 files):

#### Generated Framework (30 files) - HIGH PRIORITY
- [ ] `src/table/generated/artifacts/models.ts` (1,661 LOC)
- [ ] `src/table/generated/artifacts/mappers.ts` (1,254 LOC)
- [ ] `src/table/generated/artifacts/specifications.ts` (574 LOC)
- [ ] `src/table/generated/artifacts/parameters.ts` (298 LOC)
- [ ] `src/table/generated/artifacts/operation.ts` (27 LOC)
- [ ] `src/table/generated/*.ts` (7 framework files)
- [ ] `src/table/generated/errors/*.ts` (4 files)
- [ ] `src/table/generated/handlers/*.ts` (4 files)
- [ ] `src/table/generated/middleware/*.ts` (6 files)
- [ ] `src/table/generated/utils/*.ts` (4 files)

#### Errors (3 files)
- [ ] `src/table/errors/StorageError.ts`
- [ ] `src/table/errors/StorageErrorFactory.ts`
- [ ] `src/table/errors/NotImplementedError.ts`

#### Context (1 file)
- [ ] `src/table/context/TableStorageContext.ts`

#### Authentication (11 files)
- [ ] `src/table/authentication/IAuthenticator.ts`
- [ ] `src/table/authentication/IAuthenticationContext.ts`
- [ ] `src/table/authentication/ITableSASSignatureValues.ts`
- [ ] `src/table/authentication/AccountSASAuthenticator.ts`
- [ ] `src/table/authentication/TableSASAuthenticator.ts`
- [ ] `src/table/authentication/TableSASPermissions.ts`
- [ ] `src/table/authentication/TableSharedKeyAuthenticator.ts`
- [ ] `src/table/authentication/TableSharedKeyLiteAuthenticator.ts`
- [ ] `src/table/authentication/TableTokenAuthenticator.ts`
- [ ] `src/table/authentication/OperationAccountSASPermission.ts`
- [ ] `src/table/authentication/OperationTableSASPermission.ts`

#### Persistence (21 files) - CRITICAL PATH
- [ ] `src/table/persistence/ITableMetadataStore.ts`
- [ ] `src/table/persistence/LokiTableMetadataStore.ts` (1,088 LOC)
- [ ] `src/table/persistence/LokiTableStoreQueryGenerator.ts`
- [ ] QueryInterpreter subsystem (18 files):
  - [ ] `src/table/persistence/QueryInterpreter/IQueryContext.ts`
  - [ ] `src/table/persistence/QueryInterpreter/QueryLexer.ts`
  - [ ] `src/table/persistence/QueryInterpreter/QueryParser.ts`
  - [ ] `src/table/persistence/QueryInterpreter/QueryValidator.ts`
  - [ ] `src/table/persistence/QueryInterpreter/QueryInterpreter.ts`
  - [ ] QueryNodes (13 files): AndNode, OrNode, NotNode, Equals, LessThan, etc.

#### Batch Processing (14 files)
- [ ] `src/table/batch/BatchOperation.ts`
- [ ] `src/table/batch/BatchRequest.ts`
- [ ] `src/table/batch/BatchSerialization.ts`
- [ ] `src/table/batch/TableBatchOrchestrator.ts` (755 LOC)
- [ ] `src/table/batch/TableBatchSerialization.ts` (656 LOC)
- [ ] `src/table/batch/TableBatchRepository.ts`
- [ ] `src/table/batch/ITableBatchRepository.ts`
- [ ] `src/table/batch/*.ts` (7 more batch param files)

#### Handlers (3 files)
- [ ] `src/table/handlers/BaseHandler.ts`
- [ ] `src/table/handlers/ServiceHandler.ts`
- [ ] `src/table/handlers/TableHandler.ts` (1,188 LOC - HUGE)

#### Middleware (4 files)
- [ ] `src/table/middleware/AuthenticationMiddlewareFactory.ts`
- [ ] `src/table/middleware/PreflightMiddlewareFactory.ts`
- [ ] `src/table/middleware/tableStorageContext.middleware.ts`
- [ ] `src/table/middleware/telemetry.middleware.ts`

#### Configuration/Server (7 files)
- [ ] `src/table/ITableEnvironment.ts`
- [ ] `src/table/TableEnvironment.ts`
- [ ] `src/table/TableConfiguration.ts`
- [ ] `src/table/TableRequestListenerFactory.ts`
- [ ] `src/table/TableServer.ts`
- [ ] `src/table/main.ts`
- [ ] `src/table/utils/*.ts` (2 files: constants, utils)

## Dependencies
- ✅ regex (added)
- ✅ base64 (added)
- ✅ chrono (already present)
- ✅ serde_json (already present)

## Next Steps Priority Order

1. **Utils/Constants** (2 files) - Small, needed everywhere
2. **Errors** (3 files) - Foundation for error handling
3. **Generated Framework** (30 files) - Large but mechanical, use blob/queue patterns
4. **Persistence** (21 files) - Core functionality, includes complex QueryInterpreter
5. **Authentication** (11 files) - Similar to queue auth
6. **Context** (1 file) - Small
7. **Handlers** (3 files) - Business logic, depends on everything above
8. **Batch** (14 files) - Complex transactional logic
9. **Middleware** (4 files) - Wiring layer
10. **Server/Config** (7 files) - Entry point and configuration

## Critical Fidelity Notes From Analysis

### EDM Type System (✅ Complete)
- 9 EDM types with specific annotation rules
- Type annotation controlled by AnnotationLevel (FULL/MINIMAL/NO)
- System properties (PartitionKey, RowKey, Timestamp) have special handling
- EdmDouble has special handling for NaN/Infinity
- EdmGuid stores as base64 internally
- EdmDateTime auto-appends "Z" for UTC
- Int64/Double/Guid/Binary cannot be system properties

### QueryInterpreter (Pending)
- Full OData filter parser with lexer→parser→validator→interpreter
- 22 AST node types for query expressions
- String-based type coercion for comparisons
- Case-sensitive property access

### Batch Processing (Pending)
- Multipart MIME format
- Atomic transaction semantics
- Isolation per batch ID

### Generated Framework (Pending)
- Follow blob/queue patterns exactly
- 2 handlers (Service, Table) vs Queue's 4
- OData JSON serialization with type annotations
- Atom XML optional format

## Estimated Remaining Effort
- ~104 files remaining
- ~9,000 LOC to translate
- Critical path: Generated framework + Persistence + Handlers (~5,000 LOC)
- Complex subsystems: QueryInterpreter (800 LOC), Batch (1,400 LOC), TableHandler (1,188 LOC)

