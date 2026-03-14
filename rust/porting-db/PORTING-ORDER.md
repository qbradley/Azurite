# Azurite Porting Order — File-Level Plan

> **Author:** Gandalf (Lead Architect)
> **Date:** 2026-03-13
> **Reference:** See [STRATEGY.md](./STRATEGY.md) for rationale, type mappings, and decisions.
> **Governing Principle:** Fidelity with TypeScript. 1:1 structural correspondence.

---

## How to Read This Document

- **Phase**: Sequential porting phase. Lower phases must complete before higher ones.
- **Depends On**: Files that must be ported before this file.
- **Rust Target**: Target file path in `rust/crates/`.
- **LOC**: Approximate lines of TypeScript code.
- **Complexity**: L (low), M (medium), H (high) — based on async patterns, type complexity, and external deps.
- **Status**: `⬜ not_started` | `🔧 in_progress` | `✅ ported` | `✔️ verified`

---

## Phase 0: Project Scaffolding

No TypeScript files — create Rust project infrastructure.

| Task | Description | Status |
|------|-------------|--------|
| 0.1 | Create `rust/Cargo.toml` workspace with 5 crates | ✅ |
| 0.2 | Create `rust/crates/azurite-common/Cargo.toml` | ✅ |
| 0.3 | Create `rust/crates/azurite-blob/Cargo.toml` | ✅ |
| 0.4 | Create `rust/crates/azurite-queue/Cargo.toml` | ✅ |
| 0.5 | Create `rust/crates/azurite-table/Cargo.toml` | ✅ |
| 0.6 | Create `rust/crates/azurite/Cargo.toml` (combined binary) | ✅ |
| 0.7 | Set up CI (cargo build, cargo test, cargo clippy) | ⬜ |
| 0.8 | Create `.rustfmt.toml` with project formatting config | ⬜ |

---

## Phase 1: Common Core Interfaces (~15 files)

Foundation types and traits that everything depends on.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 1.1 | `src/common/IDataStore.ts` | `azurite-common/src/i_data_store.rs` | ~15 | L | — | ⬜ |
| 1.2 | `src/common/ICleaner.ts` | `azurite-common/src/i_cleaner.rs` | ~5 | L | — | ⬜ |
| 1.3 | `src/common/ILogger.ts` | `azurite-common/src/i_logger.rs` | ~15 | L | — | ⬜ |
| 1.4 | `src/common/ILoggerStrategy.ts` | `azurite-common/src/i_logger_strategy.rs` | ~10 | L | 1.3 | ⬜ |
| 1.5 | `src/common/models.ts` | `azurite-common/src/models.rs` | ~20 | L | — | ⬜ |
| 1.6 | `src/common/IAccountDataStore.ts` | `azurite-common/src/i_account_data_store.rs` | ~30 | L | 1.1 | ⬜ |
| 1.7 | `src/common/IRequestListenerFactory.ts` | `azurite-common/src/i_request_listener_factory.rs` | ~10 | L | — | ⬜ |
| 1.8 | `src/common/IServerFactory.ts` | `azurite-common/src/i_server_factory.rs` | ~10 | L | — | ⬜ |
| 1.9 | `src/common/IGCExtentProvider.ts` | `azurite-common/src/i_gc_extent_provider.rs` | ~15 | L | — | ⬜ |
| 1.10 | `src/common/IGCManager.ts` | `azurite-common/src/i_gc_manager.rs` | ~10 | L | — | ⬜ |
| 1.11 | `src/common/IEnvironment.ts` | `azurite-common/src/i_environment.rs` | ~50 | M | 1.5 | ⬜ |
| 1.12 | `src/common/persistence/IExtentMetadata.ts` | `azurite-common/src/persistence/i_extent_metadata.rs` | ~10 | L | — | ⬜ |
| 1.13 | `src/common/persistence/IExtentStore.ts` | `azurite-common/src/persistence/i_extent_store.rs` | ~40 | M | 1.1, 1.12 | ⬜ |
| 1.14 | `src/common/persistence/IExtentMetadataStore.ts` | `azurite-common/src/persistence/i_extent_metadata_store.rs` | ~30 | M | 1.1, 1.9, 1.12 | ⬜ |
| 1.15 | `src/common/persistence/IOperationQueue.ts` | `azurite-common/src/persistence/i_operation_queue.rs` | ~10 | L | — | ⬜ |

---

## Phase 2: Common Persistence Implementations (~8 files)

Extent storage — the binary data layer all services depend on.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 2.1 | `src/common/persistence/OperationQueue.ts` | `azurite-common/src/persistence/operation_queue.rs` | ~123 | M | 1.15 | ✅ |
| 2.2 | `src/common/persistence/MemoryExtentStore.ts` | `azurite-common/src/persistence/memory_extent_store.rs` | ~150 | M | 1.13 | ✅ |
| 2.3 | `src/common/persistence/FSExtentStore.ts` | `azurite-common/src/persistence/fs_extent_store.rs` | ~677 | H | 1.13, 2.1 | ✅ |
| 2.4 | `src/common/persistence/LokiExtentMetadataStore.ts` | `azurite-common/src/persistence/loki_extent_metadata_store.rs` | ~200 | M | 1.14 | ✅ |
| 2.5 | `src/common/persistence/AllExtentsAsyncIterator.ts` | `azurite-common/src/persistence/all_extents_async_iterator.rs` | ~50 | M | 1.14 | ✅ |
| 2.6 | `src/common/ZeroBytesStream.ts` | `azurite-common/src/zero_bytes_stream.rs` | ~30 | L | — | ✅ |
| 2.7 | `src/common/Mutex.ts` | `azurite-common/src/mutex.rs` | ~76 | M | — | ✅ |

---

## Phase 3: Common Authentication (~5 files)

Shared SAS token primitives used by all services.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 3.1 | `src/common/authentication/IIPRange.ts` | `azurite-common/src/authentication/i_ip_range.rs` | ~20 | L | — | ✅ |
| 3.2 | `src/common/authentication/AccountSASPermissions.ts` | `azurite-common/src/authentication/account_sas_permissions.rs` | ~80 | M | — | ✅ |
| 3.3 | `src/common/authentication/AccountSASServices.ts` | `azurite-common/src/authentication/account_sas_services.rs` | ~40 | L | — | ✅ |
| 3.4 | `src/common/authentication/AccountSASResourceTypes.ts` | `azurite-common/src/authentication/account_sas_resource_types.rs` | ~40 | L | — | ✅ |
| 3.5 | `src/common/authentication/IAccountSASSignatureValues.ts` | `azurite-common/src/authentication/i_account_sas_signature_values.rs` | ~200 | H | 3.1-3.4 | ✅ |

---

## Phase 4: Common Utilities, Config, Logger, Environment (~10 files)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 4.1 | `src/common/utils/constants.ts` | `azurite-common/src/utils/constants.rs` | ~50 | L | — | ✅ |
| 4.2 | `src/common/utils/utils.ts` | `azurite-common/src/utils/utils.rs` | ~172 | M | — | ✅ |
| 4.3 | `src/common/utils/BufferStream.ts` | `azurite-common/src/utils/buffer_stream.rs` | ~30 | L | — | ✅ |
| 4.4 | `src/common/Logger.ts` | `azurite-common/src/logger.rs` | ~80 | M | 1.3, 1.4 | ✅ |
| 4.5 | `src/common/NoLoggerStrategy.ts` | `azurite-common/src/no_logger_strategy.rs` | ~20 | L | 1.4 | ✅ |
| 4.6 | `src/common/WinstonLoggerStrategy.ts` | `azurite-common/src/winston_logger_strategy.rs` | ~60 | M | 1.4 | ✅ |
| 4.7 | `src/common/ConfigurationBase.ts` | `azurite-common/src/configuration_base.rs` | ~120 | M | 1.5 | ✅ |
| 4.8 | `src/common/ServerBase.ts` | `azurite-common/src/server_base.rs` | ~180 | H | 4.7 | ✅ |
| 4.9 | `src/common/AccountDataStore.ts` | `azurite-common/src/account_data_store.rs` | ~150 | M | 1.6 | ✅ |
| 4.10 | `src/common/Environment.ts` | `azurite-common/src/environment.rs` | ~200 | M | 1.11, 4.7 | ✅ |
| 4.11 | `src/common/Telemetry.ts` | `azurite-common/src/telemetry.rs` | ~410 | H | — | ✅ |

---

## Phase 5: Generated Framework — Blob (template for queue/table) (~34 files)

Autorest-generated code for blob. Establishes patterns for queue and table.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 5.1 | `src/blob/generated/IRequest.ts` | `azurite-blob/src/generated/i_request.rs` | ~20 | L | — | ✅ |
| 5.2 | `src/blob/generated/IResponse.ts` | `azurite-blob/src/generated/i_response.rs` | ~20 | L | — | ✅ |
| 5.3 | `src/blob/generated/utils/ILogger.ts` | `azurite-blob/src/generated/utils/i_logger.rs` | ~15 | L | — | ✅ |
| 5.4 | `src/blob/generated/utils/utils.ts` | `azurite-blob/src/generated/utils/utils.rs` | ~50 | L | — | ✅ |
| 5.5 | `src/blob/generated/utils/serializer.ts` | `azurite-blob/src/generated/utils/serializer.rs` | ~100 | M | — | ✅ |
| 5.6 | `src/blob/generated/utils/xml.ts` | `azurite-blob/src/generated/utils/xml.rs` | ~80 | M | — | ✅ |
| 5.7 | `src/blob/generated/Context.ts` | `azurite-blob/src/generated/context.rs` | ~120 | M | 5.1, 5.2 | ✅ |
| 5.8 | `src/blob/generated/artifacts/models.ts` | `azurite-blob/src/generated/artifacts/models.rs` | ~800 | H | — | ✅ |
| 5.9 | `src/blob/generated/artifacts/parameters.ts` | `azurite-blob/src/generated/artifacts/parameters.rs` | ~300 | M | 5.8 | ✅ |
| 5.10 | `src/blob/generated/artifacts/mappers.ts` | `azurite-blob/src/generated/artifacts/mappers.rs` | ~7394 | H | 5.8 | ✅ |
| 5.11 | `src/blob/generated/artifacts/operation.ts` | `azurite-blob/src/generated/artifacts/operation.rs` | ~100 | M | — | ✅ |
| 5.12 | `src/blob/generated/artifacts/specifications.ts` | `azurite-blob/src/generated/artifacts/specifications.rs` | ~500 | H | 5.8-5.11 | ✅ |
| 5.13 | `src/blob/generated/errors/*.ts` (4 files) | `azurite-blob/src/generated/errors/` | ~100 | L | — | ✅ |
| 5.14 | `src/blob/generated/handlers/I*Handler.ts` (6 files) | `azurite-blob/src/generated/handlers/` | ~200 | M | 5.7, 5.8 | ✅ |
| 5.15 | `src/blob/generated/handlers/IHandlers.ts` | `azurite-blob/src/generated/handlers/i_handlers.rs` | ~20 | L | 5.14 | ✅ |
| 5.16 | `src/blob/generated/handlers/handlerMappers.ts` | `azurite-blob/src/generated/handlers/handler_mappers.rs` | ~100 | M | 5.14 | ✅ |
| 5.17 | `src/blob/generated/MiddlewareFactory.ts` | `azurite-blob/src/generated/middleware_factory.rs` | ~80 | M | 5.3, 5.15 | ✅ |
| 5.18 | `src/blob/generated/ExpressMiddlewareFactory.ts` | `azurite-blob/src/generated/express_middleware_factory.rs` | ~100 | M | 5.17 | ✅ |
| 5.19 | `src/blob/generated/ExpressRequestAdapter.ts` | `azurite-blob/src/generated/express_request_adapter.rs` | ~80 | M | 5.1 | ✅ |
| 5.20 | `src/blob/generated/ExpressResponseAdapter.ts` | `azurite-blob/src/generated/express_response_adapter.rs` | ~80 | M | 5.2 | ✅ |
| 5.21 | `src/blob/generated/middleware/dispatch.middleware.ts` | `azurite-blob/src/generated/middleware/dispatch.rs` | ~150 | H | 5.7, 5.12 | ✅ |
| 5.22 | `src/blob/generated/middleware/deserializer.middleware.ts` | `azurite-blob/src/generated/middleware/deserializer.rs` | ~200 | H | 5.7, 5.10 | ✅ |
| 5.23 | `src/blob/generated/middleware/serializer.middleware.ts` | `azurite-blob/src/generated/middleware/serializer.rs` | ~150 | H | 5.7, 5.10 | ✅ |
| 5.24 | `src/blob/generated/middleware/HandlerMiddlewareFactory.ts` | `azurite-blob/src/generated/middleware/handler_middleware_factory.rs` | ~80 | M | 5.15, 5.16 | ✅ |
| 5.25 | `src/blob/generated/middleware/error.middleware.ts` | `azurite-blob/src/generated/middleware/error.rs` | ~50 | M | 5.7 | ✅ |
| 5.26 | `src/blob/generated/middleware/end.middleware.ts` | `azurite-blob/src/generated/middleware/end.rs` | ~30 | L | 5.7 | ✅ |

---

## Phase 6: Blob Errors + Context (~5 files)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 6.1 | `src/blob/errors/StorageError.ts` | `azurite-blob/src/errors/storage_error.rs` | ~66 | M | 5.13 | ✅ |
| 6.2 | `src/blob/errors/StorageErrorFactory.ts` | `azurite-blob/src/errors/storage_error_factory.rs` | ~854 | H | 6.1 | ✅ |
| 6.3 | `src/blob/errors/NotImplementedError.ts` | `azurite-blob/src/errors/not_implemented_error.rs` | ~15 | L | — | ✅ |
| 6.4 | `src/blob/errors/StrictModelNotSupportedError.ts` | `azurite-blob/src/errors/strict_model_error.rs` | ~15 | L | — | ✅ |
| 6.5 | `src/blob/context/BlobStorageContext.ts` | `azurite-blob/src/context/blob_storage_context.rs` | ~80 | M | 5.7 | ✅ |

---

## Phase 7: Blob Authentication (~12 files)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 7.1 | `src/blob/authentication/IAuthenticator.ts` | `azurite-blob/src/authentication/i_authenticator.rs` | ~15 | L | — | ✅ |
| 7.2 | `src/blob/authentication/IAuthenticationContext.ts` | `azurite-blob/src/authentication/i_authentication_context.rs` | ~20 | L | — | ✅ |
| 7.3 | `src/blob/authentication/IBlobSASSignatureValues.ts` | `azurite-blob/src/authentication/i_blob_sas_signature_values.rs` | ~818 | H | 3.5 | ✅ |
| 7.4 | `src/blob/authentication/BlobSASPermissions.ts` | `azurite-blob/src/authentication/blob_sas_permissions.rs` | ~80 | M | — | ✅ |
| 7.5 | `src/blob/authentication/BlobSASResourceType.ts` | `azurite-blob/src/authentication/blob_sas_resource_type.rs` | ~30 | L | — | ✅ |
| 7.6 | `src/blob/authentication/ContainerSASPermissions.ts` | `azurite-blob/src/authentication/container_sas_permissions.rs` | ~60 | L | — | ✅ |
| 7.7 | `src/blob/authentication/IRange.ts` | `azurite-blob/src/authentication/i_range.rs` | ~15 | L | — | ✅ |
| 7.8 | `src/blob/authentication/OperationAccountSASPermission.ts` | `azurite-blob/src/authentication/operation_account_sas_permission.rs` | ~660 | H | 3.2 | ✅ |
| 7.9 | `src/blob/authentication/OperationBlobSASPermission.ts` | `azurite-blob/src/authentication/operation_blob_sas_permission.rs` | ~548 | H | 7.4 | ✅ |
| 7.10 | `src/blob/authentication/BlobSharedKeyAuthenticator.ts` | `azurite-blob/src/authentication/blob_shared_key_authenticator.rs` | ~200 | M | 7.1 | ✅ |
| 7.11 | `src/blob/authentication/AccountSASAuthenticator.ts` | `azurite-blob/src/authentication/account_sas_authenticator.rs` | ~200 | M | 7.1, 7.3 | ✅ |
| 7.12 | `src/blob/authentication/BlobSASAuthenticator.ts` | `azurite-blob/src/authentication/blob_sas_authenticator.rs` | ~627 | H | 7.1, 7.3 | ✅ |
| 7.13 | `src/blob/authentication/BlobTokenAuthenticator.ts` | `azurite-blob/src/authentication/blob_token_authenticator.rs` | ~100 | M | 7.1 | ✅ |
| 7.14 | `src/blob/authentication/PublicAccessAuthenticator.ts` | `azurite-blob/src/authentication/public_access_authenticator.rs` | ~80 | M | 7.1 | ✅ |

---

## Phase 8: Blob Lease Subsystem (~14 files)

State machine pattern — important to port as a unit.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 8.1 | `src/blob/lease/ILeaseState.ts` | `azurite-blob/src/lease/i_lease_state.rs` | ~31 | L | — | ✅ |
| 8.2 | `src/blob/lease/LeaseStateBase.ts` | `azurite-blob/src/lease/lease_state_base.rs` | ~29 | L | 8.1 | ✅ |
| 8.3 | `src/blob/lease/LeaseAvailableState.ts` | `azurite-blob/src/lease/lease_available_state.rs` | ~50 | M | 8.2 | ✅ |
| 8.4 | `src/blob/lease/LeaseLeasedState.ts` | `azurite-blob/src/lease/lease_leased_state.rs` | ~80 | M | 8.2 | ✅ |
| 8.5 | `src/blob/lease/LeaseBreakingState.ts` | `azurite-blob/src/lease/lease_breaking_state.rs` | ~60 | M | 8.2 | ✅ |
| 8.6 | `src/blob/lease/LeaseBrokenState.ts` | `azurite-blob/src/lease/lease_broken_state.rs` | ~50 | M | 8.2 | ✅ |
| 8.7 | `src/blob/lease/LeaseExpiredState.ts` | `azurite-blob/src/lease/lease_expired_state.rs` | ~50 | M | 8.2 | ✅ |
| 8.8 | `src/blob/lease/LeaseFactory.ts` | `azurite-blob/src/lease/lease_factory.rs` | ~63 | M | 8.3-8.7 | ✅ |
| 8.9 | `src/blob/lease/BlobLeaseAdapter.ts` | `azurite-blob/src/lease/blob_lease_adapter.rs` | ~40 | L | 8.1 | ✅ |
| 8.10 | `src/blob/lease/ContainerLeaseAdapter.ts` | `azurite-blob/src/lease/container_lease_adapter.rs` | ~40 | L | 8.1 | ✅ |
| 8.11 | `src/blob/lease/BlobLeaseSyncer.ts` | `azurite-blob/src/lease/blob_lease_syncer.rs` | ~40 | L | 8.1 | ✅ |
| 8.12 | `src/blob/lease/ContainerLeaseSyncer.ts` | `azurite-blob/src/lease/container_lease_syncer.rs` | ~40 | L | 8.1 | ✅ |
| 8.13 | `src/blob/lease/BlobReadLeaseValidator.ts` | `azurite-blob/src/lease/blob_read_lease_validator.rs` | ~50 | M | 8.1 | ✅ |
| 8.14 | `src/blob/lease/BlobWriteLeaseValidator.ts` | `azurite-blob/src/lease/blob_write_lease_validator.rs` | ~50 | M | 8.1 | ✅ |
| 8.15 | `src/blob/lease/BlobWriteLeaseSyncer.ts` | `azurite-blob/src/lease/blob_write_lease_syncer.rs` | ~40 | L | 8.1 | ✅ |
| 8.16 | `src/blob/lease/ContainerDeleteLeaseValidator.ts` | `azurite-blob/src/lease/container_delete_lease_validator.rs` | ~40 | L | 8.1 | ✅ |
| 8.17 | `src/blob/lease/ContainerReadLeaseValidator.ts` | `azurite-blob/src/lease/container_read_lease_validator.rs` | ~40 | L | 8.1 | ✅ |

---

## Phase 9: Blob Conditions (~7 files)

Conditional header validation (ETag, If-Match, If-Modified-Since, etc.)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 9.1 | `src/blob/conditions/IConditionalHeaders.ts` | `azurite-blob/src/conditions/i_conditional_headers.rs` | ~20 | L | — | ✅ |
| 9.2 | `src/blob/conditions/IConditionResource.ts` | `azurite-blob/src/conditions/i_condition_resource.rs` | ~15 | L | — | ✅ |
| 9.3 | `src/blob/conditions/IConditionalHeadersValidator.ts` | `azurite-blob/src/conditions/i_conditional_headers_validator.rs` | ~15 | L | 9.1, 9.2 | ✅ |
| 9.4 | `src/blob/conditions/ConditionalHeadersAdapter.ts` | `azurite-blob/src/conditions/conditional_headers_adapter.rs` | ~50 | M | 9.1 | ✅ |
| 9.5 | `src/blob/conditions/ConditionResourceAdapter.ts` | `azurite-blob/src/conditions/condition_resource_adapter.rs` | ~40 | L | 9.2 | ✅ |
| 9.6 | `src/blob/conditions/ReadConditionalHeadersValidator.ts` | `azurite-blob/src/conditions/read_conditional_headers_validator.rs` | ~100 | M | 9.3 | ✅ |
| 9.7 | `src/blob/conditions/WriteConditionalHeadersValidator.ts` | `azurite-blob/src/conditions/write_conditional_headers_validator.rs` | ~120 | M | 9.3 | ✅ |

---

## Phase 10: Blob Persistence (~8 files)

The metadata store and query interpreter — the heart of blob storage logic.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 10.1 | `src/blob/persistence/IBlobMetadataStore.ts` | `azurite-blob/src/persistence/i_blob_metadata_store.rs` | ~1165 | H | Phase 1 | ✅ |
| 10.2 | `src/blob/persistence/QueryInterpreter/IQueryContext.ts` | `azurite-blob/src/persistence/query_interpreter/i_query_context.rs` | ~15 | L | — | ✅ |
| 10.3 | `src/blob/persistence/QueryInterpreter/QueryNodes/IQueryNode.ts` | `azurite-blob/src/persistence/query_interpreter/query_nodes/i_query_node.rs` | ~10 | L | 10.2 | ✅ |
| 10.4 | `src/blob/persistence/QueryInterpreter/QueryNodes/*.ts` (12 files) | `azurite-blob/src/persistence/query_interpreter/query_nodes/` | ~300 | M | 10.3 | ✅ |
| 10.5 | `src/blob/persistence/QueryInterpreter/QueryParser.ts` | `azurite-blob/src/persistence/query_interpreter/query_parser.rs` | ~605 | H | 10.3, 10.4 | ✅ |
| 10.6 | `src/blob/persistence/QueryInterpreter/QueryInterpreter.ts` | `azurite-blob/src/persistence/query_interpreter/query_interpreter.rs` | ~100 | M | 10.4 | ✅ |
| 10.7 | `src/blob/persistence/LokiBlobMetadataStore.ts` | `azurite-blob/src/persistence/loki_blob_metadata_store.rs` | ~3565 | H | 10.1 | ⬜ |
| 10.8 | `src/blob/persistence/BlobReferredExtentsAsyncIterator.ts` | `azurite-blob/src/persistence/blob_referred_extents_async_iterator.rs` | ~80 | M | 10.1 | ✅ |
| 10.9 | `src/blob/persistence/FilterBlobPage.ts` | `azurite-blob/src/persistence/filter_blob_page.rs` | ~60 | M | 10.1 | ✅ |
| 10.10 | `src/blob/persistence/PageWithDelimiter.ts` | `azurite-blob/src/persistence/page_with_delimiter.rs` | ~80 | M | 10.1 | ✅ |

---

## Phase 11: Blob Handlers (~12 files)

All API operation implementations.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 11.1 | `src/blob/handlers/BaseHandler.ts` | `azurite-blob/src/handlers/base_handler.rs` | ~30 | L | 10.1 | ⬜ |
| 11.2 | `src/blob/handlers/ServiceHandler.ts` | `azurite-blob/src/handlers/service_handler.rs` | ~416 | H | 11.1 | ⬜ |
| 11.3 | `src/blob/handlers/ContainerHandler.ts` | `azurite-blob/src/handlers/container_handler.rs` | ~865 | H | 11.1, Phase 8 | ⬜ |
| 11.4 | `src/blob/handlers/BlobHandler.ts` | `azurite-blob/src/handlers/blob_handler.rs` | ~1350 | H | 11.1, Phase 8, 9 | ⬜ |
| 11.5 | `src/blob/handlers/BlockBlobHandler.ts` | `azurite-blob/src/handlers/block_blob_handler.rs` | ~507 | H | 11.1 | ⬜ |
| 11.6 | `src/blob/handlers/PageBlobHandler.ts` | `azurite-blob/src/handlers/page_blob_handler.rs` | ~495 | H | 11.1 | ⬜ |
| 11.7 | `src/blob/handlers/AppendBlobHandler.ts` | `azurite-blob/src/handlers/append_blob_handler.rs` | ~200 | M | 11.1 | ⬜ |
| 11.8 | `src/blob/handlers/IPageBlobRangesManager.ts` | `azurite-blob/src/handlers/i_page_blob_ranges_manager.rs` | ~15 | L | — | ⬜ |
| 11.9 | `src/blob/handlers/PageBlobRangesManager.ts` | `azurite-blob/src/handlers/page_blob_ranges_manager.rs` | ~481 | H | 11.8 | ⬜ |
| 11.10 | `src/blob/handlers/BlobBatchHandler.ts` | `azurite-blob/src/handlers/blob_batch_handler.rs` | ~576 | H | 11.1 | ⬜ |
| 11.11 | `src/blob/handlers/BlobBatchSubRequest.ts` | `azurite-blob/src/handlers/blob_batch_sub_request.rs` | ~80 | M | — | ⬜ |
| 11.12 | `src/blob/handlers/BlobBatchSubResponse.ts` | `azurite-blob/src/handlers/blob_batch_sub_response.rs` | ~50 | L | — | ⬜ |
| 11.13 | `src/blob/handlers/SubResponseTextBodyStream.ts` | `azurite-blob/src/handlers/sub_response_text_body_stream.rs` | ~40 | L | — | ⬜ |

---

## Phase 12: Blob Middleware, Server, Config, Environment (~10 files)

Wire everything together into a running service.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 12.1 | `src/blob/utils/constants.ts` | `azurite-blob/src/utils/constants.rs` | ~50 | L | — | ⬜ |
| 12.2 | `src/blob/utils/utils.ts` | `azurite-blob/src/utils/utils.rs` | ~100 | M | — | ⬜ |
| 12.3 | `src/blob/middlewares/blobStorageContext.middleware.ts` | `azurite-blob/src/middlewares/blob_storage_context.rs` | ~80 | M | 6.5 | ⬜ |
| 12.4 | `src/blob/middlewares/AuthenticationMiddlewareFactory.ts` | `azurite-blob/src/middlewares/authentication_middleware_factory.rs` | ~100 | M | Phase 7 | ⬜ |
| 12.5 | `src/blob/middlewares/PreflightMiddlewareFactory.ts` | `azurite-blob/src/middlewares/preflight_middleware_factory.rs` | ~465 | H | — | ⬜ |
| 12.6 | `src/blob/middlewares/StrictModelMiddlewareFactory.ts` | `azurite-blob/src/middlewares/strict_model_middleware_factory.rs` | ~80 | M | — | ⬜ |
| 12.7 | `src/blob/middlewares/telemetry.middleware.ts` | `azurite-blob/src/middlewares/telemetry.rs` | ~50 | L | — | ⬜ |
| 12.8 | `src/blob/IBlobEnvironment.ts` | `azurite-blob/src/i_blob_environment.rs` | ~50 | L | 1.11 | ⬜ |
| 12.9 | `src/blob/BlobEnvironment.ts` | `azurite-blob/src/blob_environment.rs` | ~100 | M | 12.8 | ⬜ |
| 12.10 | `src/blob/BlobConfiguration.ts` | `azurite-blob/src/blob_configuration.rs` | ~50 | L | 4.7 | ⬜ |
| 12.11 | `src/blob/BlobRequestListenerFactory.ts` | `azurite-blob/src/blob_request_listener_factory.rs` | ~200 | H | Phase 5, 12.3-12.7 | ⬜ |
| 12.12 | `src/blob/BlobServer.ts` | `azurite-blob/src/blob_server.rs` | ~245 | H | 4.8, 12.10, 12.11 | ⬜ |
| 12.13 | `src/blob/BlobServerFactory.ts` | `azurite-blob/src/blob_server_factory.rs` | ~60 | M | 12.12 | ⬜ |
| 12.14 | `src/blob/main.ts` | `azurite-blob/src/main.rs` | ~80 | M | 12.13 | ⬜ |

---

## Phase 13: Blob GC (~2 files)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 13.1 | `src/blob/gc/BlobGCManager.ts` | `azurite-blob/src/gc/blob_gc_manager.rs` | ~293 | H | 1.10, 10.1 | ⬜ |

**🏁 MILESTONE: Blob service fully ported — integration tests can begin for blob.**

---

## Phase 14: Queue Service (full) (~50 files)

Follows identical patterns to blob. Smaller and simpler.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 14.1 | `src/queue/generated/` (32 files) | `azurite-queue/src/generated/` | ~4000 | H | Phase 5 patterns | ⬜ |
| 14.2 | `src/queue/errors/StorageError.ts` | `azurite-queue/src/errors/storage_error.rs` | ~67 | M | — | ⬜ |
| 14.3 | `src/queue/errors/StorageErrorFactory.ts` | `azurite-queue/src/errors/storage_error_factory.rs` | ~300 | M | 14.2 | ⬜ |
| 14.4 | `src/queue/errors/NotImplementedError.ts` | `azurite-queue/src/errors/not_implemented_error.rs` | ~15 | L | — | ⬜ |
| 14.5 | `src/queue/context/QueueStorageContext.ts` | `azurite-queue/src/context/queue_storage_context.rs` | ~60 | M | 14.1 | ⬜ |
| 14.6 | `src/queue/authentication/*.ts` (10 files) | `azurite-queue/src/authentication/` | ~800 | M | Phase 3 | ⬜ |
| 14.7 | `src/queue/persistence/IQueueMetadataStore.ts` | `azurite-queue/src/persistence/i_queue_metadata_store.rs` | ~200 | M | Phase 1 | ⬜ |
| 14.8 | `src/queue/persistence/LokiQueueMetadataStore.ts` | `azurite-queue/src/persistence/loki_queue_metadata_store.rs` | ~881 | H | 14.7 | ⬜ |
| 14.9 | `src/queue/persistence/QueueReferredExtentsAsyncIterator.ts` | `azurite-queue/src/persistence/queue_referred_extents_async_iterator.rs` | ~50 | M | 14.7 | ⬜ |
| 14.10 | `src/queue/handlers/BaseHandler.ts` | `azurite-queue/src/handlers/base_handler.rs` | ~30 | L | 14.7 | ⬜ |
| 14.11 | `src/queue/handlers/ServiceHandler.ts` | `azurite-queue/src/handlers/service_handler.rs` | ~150 | M | 14.10 | ⬜ |
| 14.12 | `src/queue/handlers/QueueHandler.ts` | `azurite-queue/src/handlers/queue_handler.rs` | ~300 | M | 14.10 | ⬜ |
| 14.13 | `src/queue/handlers/MessagesHandler.ts` | `azurite-queue/src/handlers/messages_handler.rs` | ~200 | M | 14.10 | ⬜ |
| 14.14 | `src/queue/handlers/MessageIdHandler.ts` | `azurite-queue/src/handlers/message_id_handler.rs` | ~150 | M | 14.10 | ⬜ |
| 14.15 | `src/queue/middlewares/*.ts` (4 files) | `azurite-queue/src/middlewares/` | ~600 | M | — | ⬜ |
| 14.16 | `src/queue/IQueueEnvironment.ts` | `azurite-queue/src/i_queue_environment.rs` | ~30 | L | 1.11 | ⬜ |
| 14.17 | `src/queue/QueueEnvironment.ts` | `azurite-queue/src/queue_environment.rs` | ~80 | M | 14.16 | ⬜ |
| 14.18 | `src/queue/QueueConfiguration.ts` | `azurite-queue/src/queue_configuration.rs` | ~40 | L | 4.7 | ⬜ |
| 14.19 | `src/queue/QueueRequestListenerFactory.ts` | `azurite-queue/src/queue_request_listener_factory.rs` | ~150 | M | 14.1 | ⬜ |
| 14.20 | `src/queue/QueueServer.ts` | `azurite-queue/src/queue_server.rs` | ~200 | M | 4.8, 14.18 | ⬜ |
| 14.21 | `src/queue/main.ts` | `azurite-queue/src/main.rs` | ~60 | L | 14.20 | ⬜ |
| 14.22 | `src/queue/gc/QueueGCManager.ts` | `azurite-queue/src/gc/queue_gc_manager.rs` | ~200 | M | 1.10 | ⬜ |
| 14.23 | `src/queue/utils/constants.ts` | `azurite-queue/src/utils/constants.rs` | ~30 | L | — | ⬜ |
| 14.24 | `src/queue/utils/utils.ts` | `azurite-queue/src/utils/utils.rs` | ~50 | L | — | ⬜ |

**🏁 MILESTONE: Queue service fully ported.**

---

## Phase 15: Table Service (full) (~70 files)

Most complex service due to batch processing and EDM type system.

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 15.1 | `src/table/generated/` (30 files) | `azurite-table/src/generated/` | ~3500 | H | Phase 5 patterns | ⬜ |
| 15.2 | `src/table/errors/*.ts` (3 files) | `azurite-table/src/errors/` | ~500 | M | — | ⬜ |
| 15.3 | `src/table/context/TableStorageContext.ts` | `azurite-table/src/context/table_storage_context.rs` | ~80 | M | 15.1 | ⬜ |
| 15.4 | `src/table/entity/IEdmType.ts` | `azurite-table/src/entity/i_edm_type.rs` | ~20 | L | — | ⬜ |
| 15.5 | `src/table/entity/Edm*.ts` (9 files) | `azurite-table/src/entity/` | ~300 | M | 15.4 | ⬜ |
| 15.6 | `src/table/entity/EntityProperty.ts` | `azurite-table/src/entity/entity_property.rs` | ~50 | M | 15.4 | ⬜ |
| 15.7 | `src/table/entity/NormalizedEntity.ts` | `azurite-table/src/entity/normalized_entity.rs` | ~60 | M | 15.6 | ⬜ |
| 15.8 | `src/table/authentication/*.ts` (11 files) | `azurite-table/src/authentication/` | ~900 | M | Phase 3 | ⬜ |
| 15.9 | `src/table/persistence/ITableMetadataStore.ts` | `azurite-table/src/persistence/i_table_metadata_store.rs` | ~200 | M | Phase 1 | ⬜ |
| 15.10 | `src/table/persistence/LokiTableMetadataStore.ts` | `azurite-table/src/persistence/loki_table_metadata_store.rs` | ~1088 | H | 15.9 | ⬜ |
| 15.11 | `src/table/persistence/LokiTableStoreQueryGenerator.ts` | `azurite-table/src/persistence/loki_table_store_query_generator.rs` | ~100 | M | 15.9 | ⬜ |
| 15.12 | `src/table/persistence/QueryInterpreter/` (18 files) | `azurite-table/src/persistence/query_interpreter/` | ~800 | H | — | ⬜ |
| 15.13 | `src/table/batch/BatchOperation.ts` | `azurite-table/src/batch/batch_operation.rs` | ~30 | L | — | ⬜ |
| 15.14 | `src/table/batch/BatchRequest.ts` | `azurite-table/src/batch/batch_request.rs` | ~80 | M | — | ⬜ |
| 15.15 | `src/table/batch/BatchSerialization.ts` | `azurite-table/src/batch/batch_serialization.rs` | ~100 | M | — | ⬜ |
| 15.16 | `src/table/batch/TableBatchOrchestrator.ts` | `azurite-table/src/batch/table_batch_orchestrator.rs` | ~755 | H | 15.13-15.15 | ⬜ |
| 15.17 | `src/table/batch/TableBatchSerialization.ts` | `azurite-table/src/batch/table_batch_serialization.rs` | ~656 | H | — | ⬜ |
| 15.18 | `src/table/batch/TableBatchRepository.ts` | `azurite-table/src/batch/table_batch_repository.rs` | ~100 | M | 15.9 | ⬜ |
| 15.19 | `src/table/batch/*.ts` (remaining) | `azurite-table/src/batch/` | ~300 | M | — | ⬜ |
| 15.20 | `src/table/handlers/BaseHandler.ts` | `azurite-table/src/handlers/base_handler.rs` | ~30 | L | 15.9 | ⬜ |
| 15.21 | `src/table/handlers/ServiceHandler.ts` | `azurite-table/src/handlers/service_handler.rs` | ~150 | M | 15.20 | ⬜ |
| 15.22 | `src/table/handlers/TableHandler.ts` | `azurite-table/src/handlers/table_handler.rs` | ~1188 | H | 15.20, 15.16 | ⬜ |
| 15.23 | `src/table/middleware/*.ts` (4 files) | `azurite-table/src/middleware/` | ~600 | M | — | ⬜ |
| 15.24 | `src/table/ITableEnvironment.ts` | `azurite-table/src/i_table_environment.rs` | ~30 | L | 1.11 | ⬜ |
| 15.25 | `src/table/TableEnvironment.ts` | `azurite-table/src/table_environment.rs` | ~80 | M | 15.24 | ⬜ |
| 15.26 | `src/table/TableConfiguration.ts` | `azurite-table/src/table_configuration.rs` | ~40 | L | 4.7 | ⬜ |
| 15.27 | `src/table/TableRequestListenerFactory.ts` | `azurite-table/src/table_request_listener_factory.rs` | ~200 | M | 15.1 | ⬜ |
| 15.28 | `src/table/TableServer.ts` | `azurite-table/src/table_server.rs` | ~200 | M | 4.8, 15.26 | ⬜ |
| 15.29 | `src/table/main.ts` | `azurite-table/src/main.rs` | ~60 | L | 15.28 | ⬜ |
| 15.30 | `src/table/utils/constants.ts` | `azurite-table/src/utils/constants.rs` | ~30 | L | — | ⬜ |
| 15.31 | `src/table/utils/utils.ts` | `azurite-table/src/utils/utils.rs` | ~50 | L | — | ⬜ |

**🏁 MILESTONE: Table service fully ported.**

---

## Phase 16: Combined Binary (~2 files)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 16.1 | `src/azurite.ts` | `azurite/src/main.rs` | ~150 | M | Phases 12-15 | ⬜ |

**🏁 MILESTONE: Full Azurite emulator running in Rust.**

---

## Phase 17: SQL Persistence (Optional/Deferred)

| # | Source File | Rust Target | LOC | Complexity | Depends On | Status |
|---|-----------|------------|-----|------------|------------|--------|
| 17.1 | `src/blob/SqlBlobConfiguration.ts` | `azurite-blob/src/sql_blob_configuration.rs` | ~40 | L | 12.10 | ⬜ |
| 17.2 | `src/blob/SqlBlobServer.ts` | `azurite-blob/src/sql_blob_server.rs` | ~200 | M | 12.12 | ⬜ |
| 17.3 | `src/blob/persistence/SqlBlobMetadataStore.ts` | `azurite-blob/src/persistence/sql_blob_metadata_store.rs` | ~3579 | H | 10.1 | ⬜ |
| 17.4 | `src/common/persistence/SqlExtentMetadataStore.ts` | `azurite-common/src/persistence/sql_extent_metadata_store.rs` | ~200 | M | 1.14 | ⬜ |

---

## Summary Statistics

| Phase | Files | Est. Lines | Critical Path? |
|-------|-------|-----------|---------------|
| 0 (Scaffolding) | — | — | Yes |
| 1 (Core Interfaces) | 15 | ~300 | Yes |
| 2 (Persistence Impl) | 7 | ~1,100 | Yes |
| 3 (Auth Primitives) | 5 | ~380 | Yes |
| 4 (Utils/Config/Logger) | 11 | ~1,400 | Yes |
| 5 (Generated Framework) | 34 | ~10,000 | Yes |
| 6-13 (Blob Service) | ~60 | ~12,000 | Yes |
| 14 (Queue Service) | ~50 | ~7,500 | No (parallel possible) |
| 15 (Table Service) | ~70 | ~10,000 | No (parallel possible) |
| 16 (Combined Binary) | 2 | ~150 | Yes (after 14+15) |
| 17 (SQL Persistence) | 4 | ~4,000 | No (deferred) |
| **TOTAL** | **~258** | **~47,000** | |

> **Note:** Line counts are estimated for Rust output. Actual Rust code may be 20-40% larger than TypeScript due to explicit types, error handling, and trait implementations.

---

## Out of Scope (Not Ported)

| File(s) | Reason |
|---------|--------|
| `src/extension.ts` | VS Code extension API — JavaScript only |
| `src/main.ts` | Re-export for VS Code extension |
| `src/common/VSC*.ts` (14 files) | VS Code integration — not applicable to Rust |
| Test files (`tests/`) | Tests will be rewritten using Rust Azure SDK clients |

---

*This document is maintained by Gandalf. Status updates by Aragorn during implementation.*
