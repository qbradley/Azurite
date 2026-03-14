# Azurite integration/E2E test strategy for the Rust port

## Bottom line

Yes — Azurite already has a substantial end-to-end test surface, and most of the valuable coverage is reusable against the Rust implementation.

The best reusable assets are the **TypeScript Mocha suites under `tests/blob/`, `tests/queue/`, and `tests/table/` that drive Azurite through official Azure SDK clients**, plus the **table-specific .NET and Go conformance suites**. The main blocker is not protocol compatibility; it is that the current Node test harness usually starts the **TypeScript** server directly through local test factories instead of pointing at an arbitrary external endpoint.

So the answer is:

- **Does Azurite have integration/E2E tests?** Yes, extensively.
- **Can they validate the Rust port end-to-end?** Yes, for a large percentage of the suite.
- **Can they run literally unchanged today just by swapping an environment variable?** **Mostly no.** A thin external-server adapter layer is needed first.

---

## 1. Inventory of test infrastructure

### 1.1 Primary TypeScript test tree

The repository has a single top-level `tests/` tree and **no separate `test/` directory**.

### 1.2 Test runner and scripts

Azurite’s test runner is configured directly in `package.json`, not through a separate Jest/Mocha config file.

Key facts:

- **Mocha + ts-node** is the test stack (`package.json:83,88,314-325`).
- Main scripts:
  - `npm run test` → all Loki-tagged TS tests (`package.json:314`)
  - `npm run test:blob` / `test:blob:in-memory` / `test:blob:sql` / `test:blob:sql:ci` (`package.json:316-319`)
  - `npm run test:queue` / `test:queue:in-memory` (`package.json:320-321`)
  - `npm run test:table` / `test:table:in-memory` (`package.json:322-323`)
  - `npm run test:exe` / `test:linux` for packaged binaries (`package.json:324-325`)
- Related dependencies:
  - `@azure/data-tables` `^13.0.1` (`package.json:50`)
  - `@azure/storage-blob` `^12.9.0` (`package.json:51`)
  - `@azure/storage-queue` `^12.8.0` (`package.json:52`)
  - `azure-storage` `^2.10.3` (`package.json:76`)
  - `mocha` `^10.8.2` (`package.json:83`)
  - `ts-node` `^10.0.0` (`package.json:88`)

### 1.3 CI wiring

CI runs the same Mocha suites in `.github/workflows/PrValidation.yml`:

- Blob tests across Ubuntu/Windows/macOS and Node 16/18/20/22 (`PrValidation.yml:10-97`)
- Blob MySQL-backed tests via Dockerized MySQL (`PrValidation.yml:98-128`)
- Queue tests across Ubuntu/Windows/macOS (`PrValidation.yml:130-194`)
- Table tests across Ubuntu/Windows/macOS (`PrValidation.yml:196-260`)

Important detail: there is **no `docker-compose.test.*`** file; MySQL is started inline with `docker run` inside CI (`PrValidation.yml:106-112`, `122-128`).

### 1.4 Config-file search result

I did not find repo-level test config files such as `jest.config.*`, `mocha.*`, `.nycrc`, or `docker-compose.test.*`. Test selection is script-driven from `package.json`, and service startup is mostly test-helper-driven under `tests/`.

### 1.5 Counts by category

#### TypeScript test entry files

- **Blob:** 23 files under `tests/blob/`
- **Queue:** 11 files under `tests/queue/`
- **Table:** 20 TypeScript test files plus 1 Go conformance harness under `tests/table/`
- **Top-level binary smoke tests:** 2 files (`tests/exe.test.ts`, `tests/linuxbinary.test.ts`)

#### Cross-language table conformance assets

- **4 NUnit suite files** under `tests/table/dotnet/AzuriteTableTest/`
  - `AzureStorageTests.cs`
  - `CosmosTableTests.cs`
  - `DataTableTests.cs`
  - `DataTablesBatchTests.cs`
- **2 C# helper model files** in the same directory
- **1 Go harness** at `tests/table/go/main.go`

#### Functional categorization

- **Reusable black-box integration/E2E TS suites:** **41 files**
- **TS internal/unit-style suites:** **13 files**
- **Binary/package smoke suites:** **2 files**
- **Cross-language conformance artifacts:** **7 files** (4 C# suite files, 2 C# helpers, 1 Go program)

Roughly speaking, the TS tree contains hundreds of assertions; the large majority of user-visible API coverage lives in the 41 black-box suites.

---

## 2. What kinds of tests exist?

## 2.1 Unit / internal tests

These test TypeScript implementation details directly and are **not** good Rust black-box candidates.

Representative files:

- `tests/blob/unit/query.parser.unit.test.ts` (`parseQuery` imported from `src/blob/...`) (`tests/blob/unit/query.parser.unit.test.ts:1-19`)
- `tests/blob/conditions.test.ts`
- `tests/blob/fsStore.test.ts`
- `tests/blob/handlers/AppendBlobHandler.test.ts`
- `tests/blob/handlers/PageBlobRangesManager.test.ts`
- `tests/blob/memoryStore.unit.test.ts`
- `tests/blob/pagewithdelimiter.test.ts`
- `tests/blob/utils.test.ts`
- `tests/table/unit/deserialization.unit.test.ts`
- `tests/table/unit/query.interpreter.unit.test.ts`
- `tests/table/unit/query.lexer.unit.test.ts`
- `tests/table/unit/query.parser.unit.test.ts`
- `tests/table/unit/serialization.unit.test.ts`

**Why they are not reusable as Rust E2E:** they import `src/...` modules directly and validate parser/storage/helper internals rather than the wire protocol.

## 2.2 Integration / E2E tests

These are the most valuable suites for the Rust port.

They generally:

1. Start an Azurite server,
2. Create SDK clients or raw HTTP clients,
3. Exercise the public storage API,
4. Assert on responses, headers, auth behavior, and persisted outcomes.

Representative files:

- Blob SDK suites: `tests/blob/apis/*.test.ts`, `tests/blob/sas.test.ts`, `tests/blob/oauth.test.ts`, `tests/blob/https.test.ts`, `tests/blob/blobCorsRequest.test.ts`
- Queue SDK suites: `tests/queue/apis/*.test.ts`, `tests/queue/queueSas.test.ts`, `tests/queue/oauth.test.ts`, `tests/queue/https.test.ts`, `tests/queue/queueCorsRequest.test.ts`
- Table SDK suites: `tests/table/apis/*.test.ts`, `tests/table/auth/*.test.ts`, `tests/table/KeepAlive/tableKeepAliveTimeout.test.ts`
- Table REST suites: `tests/table/apis/table.entity.rest.test.ts`, `tests/table/apis/table.validation.rest.test.ts`

## 2.3 Conformance tests

The table service also has language-external compatibility tests:

- NUnit + `Microsoft.WindowsAzure.Storage` / `Azure.Data.Tables` (`tests/table/dotnet/AzuriteTableTest/*.cs`)
- Go SDK harness (`tests/table/go/main.go`)

These are especially valuable because they validate Azurite as a wire-compatible server, not as a TS implementation.

## 2.4 Packaging / binary smoke tests

- `tests/exe.test.ts`
- `tests/linuxbinary.test.ts`

These are closer to installable-binary smoke tests than reusable API parity suites.

---

## 3. How the reusable suites talk to Azurite

## 3.1 Shared emulator account and key

Tests use the standard Azurite emulator credentials from `tests/testutils.ts`:

- account name `devstoreaccount1` (`tests/testutils.ts:9`)
- standard emulator key (`tests/testutils.ts:10-11`)

This is exactly what the Rust port should expose if it wants to reuse the existing clients without rewriting credentials.

## 3.2 Server factories and default ports

The reusable TS suites mostly create local servers through test factories:

- Blob factory hardcodes `127.0.0.1:11000` and reads `AZURITE_TEST_DB` / `AZURITE_TEST_INMEMORYPERSISTENCE` (`tests/BlobTestServerFactory.ts:16-21`)
- Queue factory hardcodes `127.0.0.1:11001` and reads `AZURITE_TEST_INMEMORYPERSISTENCE` (`tests/queue/utils/QueueTestServerFactory.ts:20-23`)
- Table factory hardcodes `127.0.0.1:11002` and reads `AZURITE_TEST_INMEMORYPERSISTENCE` (`tests/table/utils/TableTestServerFactory.ts:16-24`)

### Consequence for Rust reuse

These tests are **not currently written as “just point to another endpoint” tests**. They assume the test process can construct and start a TS server object locally.

## 3.3 SDK-based connection styles

### Blob

Representative blob API test:

- `tests/blob/apis/blob.test.ts` builds `BlobServiceClient(baseURL, newPipeline(new StorageSharedKeyCredential(...)))` against `http://${server.config.host}:${server.config.port}/devstoreaccount1` (`tests/blob/apis/blob.test.ts:27-45`) and starts the test server in `before()` (`tests/blob/apis/blob.test.ts:55-62`).

Blob suites cover:

- service/container/blob APIs
- block/page/append blob flows
- tags and conditions
- SAS
- OAuth bearer auth
- HTTPS
- CORS
- keep-alive behavior
- special naming
- high-level client operations

### Queue

Representative queue API test:

- `tests/queue/apis/queue.test.ts` builds `QueueServiceClient` against `http://127.0.0.1:11001/devstoreaccount1` using `@azure/storage-queue` (`tests/queue/apis/queue.test.ts:24-52`) and starts a queue server in `before()` (`tests/queue/apis/queue.test.ts:58-65`).

Queue suites cover:

- service properties
- queue CRUD
- message CRUD / dequeue / update / clear
- access policies and SAS
- OAuth bearer auth
- HTTPS
- CORS
- keep-alive behavior
- special naming

### Table

Representative table SDK suites:

- `tests/table/apis/table.entity.azure.data-tables.test.ts` uses `@azure/data-tables` `TableClient` / `TableTransaction` (`tests/table/apis/table.entity.azure.data-tables.test.ts:1-18`) and starts HTTPS table server in `before()` (`tests/table/apis/table.entity.azure.data-tables.test.ts:36-49`).
- `tests/table/utils/table.entity.test.utils.ts` can create legacy connection strings or `TableClient`/`TableServiceClient` objects (`tests/table/utils/table.entity.test.utils.ts:23-32`, `99-120`, `158-219`).

Table suites cover:

- table CRUD
- entity CRUD
- batch transactions
- OData query behavior
- validation edge cases
- legacy `azure-storage` compatibility
- modern `@azure/data-tables` compatibility
- SAS
- OAuth bearer auth
- HTTPS
- CORS
- keep-alive behavior

## 3.4 Raw HTTP / REST tests

Table has explicit REST-only suites using Axios:

- `tests/table/apis/table.entity.rest.test.ts` starts a table server (`tests/table/apis/table.entity.rest.test.ts:32-46`) and then submits raw HTTP and multipart batch payloads (`tests/table/apis/table.entity.rest.test.ts:53-126`)
- `tests/table/apis/table.validation.rest.test.ts` does raw validation requests (`tests/table/apis/table.validation.rest.test.ts:23-48`, `55-116`)
- REST helper URLs are built from `TableEntityTestConfig.protocol/host/port/accountName` in `tests/table/utils/table.entity.tests.rest.submitter.ts` (`tests/table/utils/table.entity.tests.rest.submitter.ts:24-37`, `73-85`, `117-147`)
- That config is currently hardcoded to `http://127.0.0.1:11002` in `tests/table/models/table.entity.test.config.ts` (`tests/table/models/table.entity.test.config.ts:5-13`)

### Important Rust-port caveat

`table.entity.rest.test.ts` embeds **literal `http://127.0.0.1:10002/...` URLs inside multipart batch bodies** (`tests/table/apis/table.entity.rest.test.ts:70-71`, `110-111`). That makes the REST table suite **not fully endpoint-agnostic today**, even though the outer request URL comes from test config.

---

## 4. Which suites are black-box and reusable against Rust?

## 4.1 Best candidates: TypeScript SDK suites

These are the most reusable because they validate server behavior through public SDKs:

### Blob (15 black-box TS files)

- `tests/blob/apis/appendblob.test.ts`
- `tests/blob/apis/blob.test.ts`
- `tests/blob/apis/blobbatch.test.ts`
- `tests/blob/apis/blockblob.test.ts`
- `tests/blob/apis/container.test.ts`
- `tests/blob/apis/pageblob.test.ts`
- `tests/blob/apis/service.test.ts`
- `tests/blob/authentication.test.ts`
- `tests/blob/blobCorsRequest.test.ts`
- `tests/blob/blobKeepAliveTimeout.test.ts`
- `tests/blob/blockblob.highlevel.test.ts`
- `tests/blob/https.test.ts`
- `tests/blob/oauth.test.ts`
- `tests/blob/sas.test.ts`
- `tests/blob/specialnaming.test.ts`

### Queue (11 black-box TS files)

- `tests/queue/apis/messageid.test.ts`
- `tests/queue/apis/messages.test.ts`
- `tests/queue/apis/queue.test.ts`
- `tests/queue/apis/queueService.test.ts`
- `tests/queue/https.test.ts`
- `tests/queue/oauth.test.ts`
- `tests/queue/queueAuthentication.test.ts`
- `tests/queue/queueCorsRequest.test.ts`
- `tests/queue/queueKeepAliveTimeout.test.ts`
- `tests/queue/queueSas.test.ts`
- `tests/queue/queueSpecialnaming.test.ts`

### Table (15 black-box TS files)

- `tests/table/KeepAlive/tableKeepAliveTimeout.test.ts`
- `tests/table/apis/table.batch.errorhandling.test.ts`
- `tests/table/apis/table.entity.apostrophe.azure-storage.test.ts`
- `tests/table/apis/table.entity.apostrophe.data-tables.test.ts`
- `tests/table/apis/table.entity.azure.data-tables.test.ts`
- `tests/table/apis/table.entity.issues.test.ts`
- `tests/table/apis/table.entity.query.test.ts`
- `tests/table/apis/table.entity.rest.test.ts`
- `tests/table/apis/table.entity.test.ts`
- `tests/table/apis/table.service.test.ts`
- `tests/table/apis/table.test.ts`
- `tests/table/apis/table.validation.rest.test.ts`
- `tests/table/auth/oauth.test.ts`
- `tests/table/auth/sas.test.ts`
- `tests/table/auth/tableCorsRequest.test.ts`

**Assessment:** These are mostly black-box API checks. They should become strong Rust parity suites once the startup/endpoint assumptions are externalized.

## 4.2 Cross-language table conformance suites

### .NET

Example: `tests/table/dotnet/AzuriteTableTest/AzureStorageTests.cs`

- Uses `CloudStorageAccount.Parse("UseDevelopmentStorage=true")` (`AzureStorageTests.cs:18-22`)
- Exercises table creation, entity insert, and query through Microsoft’s .NET SDK (`AzureStorageTests.cs:24-52`)

Example: `tests/table/dotnet/AzuriteTableTest/DataTablesBatchTests.cs`

- Uses `new TableServiceClient("UseDevelopmentStorage=true")` (`DataTablesBatchTests.cs:18-22`)
- Requires Azurite to be started externally (`DataTablesBatchTests.cs:18-21`)
- Exercises `Azure.Data.Tables` batch submission (`DataTablesBatchTests.cs:25-55`)

### Go

`tests/table/go/main.go`:

- Builds an explicit connection string pointing at `http://127.0.0.1:10002/devstoreaccount1` (`tests/table/go/main.go:40-44`)
- Uses both `github.com/Azure/azure-sdk-for-go/sdk/data/aztables` and the older Go storage package (`tests/table/go/main.go:10-13`, `62-79`, `96-152`)
- Validates table creation, inserts, batch work, and queries (`tests/table/go/main.go:25-35`, `47-59`, `154-180`)

**Assessment:** Very high value for Rust table parity, but they assume the classic emulator port `10002`, not the TS test port `11002`.

---

## 5. What is *not* reusable as Rust black-box validation?

## 5.1 Internal unit suites

The 13 internal/unit-style TS tests should be reimplemented as Rust unit tests rather than reused via Mocha:

- they import TS source modules directly,
- they assert on TS parser/storage/helper internals,
- they do not validate the observable wire protocol.

## 5.2 Packaging/binary smoke tests

`tests/exe.test.ts` and `tests/linuxbinary.test.ts` are only partially reusable.

Why they are awkward:

- they expect specific binary names (`azurite.exe`, `azuritelinux`) (`tests/exe.test.ts:57-63`, `tests/linuxbinary.test.ts:44-49`)
- they spawn the binaries directly (`tests/exe.test.ts:84-88`, `tests/linuxbinary.test.ts:72-73`)
- they assert exact startup log text for all three services (`tests/exe.test.ts:92-120`, `tests/linuxbinary.test.ts:76-94`)

These could be adapted later if the Rust CLI intentionally mirrors Azurite’s flags and log text, but they are **not** the best first-phase correctness harness.

---

## 6. Endpoint configurability: what already exists vs what is hardcoded

## 6.1 Existing configurability

- Blob factory switches between Loki and SQL metadata via `AZURITE_TEST_DB` (`tests/BlobTestServerFactory.ts:16-18`)
- Blob/Queue/Table factories all honor `AZURITE_TEST_INMEMORYPERSISTENCE` (`tests/BlobTestServerFactory.ts:18`, `tests/queue/utils/QueueTestServerFactory.ts:20`, `tests/table/utils/TableTestServerFactory.ts:16-18`)
- Table helper has an alternate cloud path using env vars:
  - `AZURE_TABLE_STORAGE`
  - `AZURE_DATATABLES_STORAGE_STRING`
  - `AZURE_DATATABLES_SAS`
  - `AZURITE_TABLE_BASE_URL`
  (`tests/table/utils/table.entity.test.utils.ts:29-32`, `99-120`, `158-219`)
- All npm test scripts already set `NODE_TLS_REJECT_UNAUTHORIZED=0`, which helps with self-signed HTTPS during test execution (`package.json:314-325`)

## 6.2 Hardcoded assumptions that block “run against Rust as-is”

### Blob / Queue / Table local startup

The largest issue is that many tests instantiate TS server classes directly through test factories instead of consuming an external endpoint.

### Hardcoded ports

- Blob `11000` (`tests/BlobTestServerFactory.ts:20`)
- Queue `11001` (`tests/queue/utils/QueueTestServerFactory.ts:22`)
- Table `11002` (`tests/table/utils/TableTestServerFactory.ts:23`)
- Table REST helper config `11002` (`tests/table/models/table.entity.test.config.ts:6-8`)
- Table cross-language conformance tests expect `10002` via `UseDevelopmentStorage=true` or explicit connection string (`AzureStorageTests.cs:21`, `DataTablesBatchTests.cs:21`, `tests/table/go/main.go:40-44`)

### Hardcoded “local Azurite” switches

Several table-oriented tests set `const testLocalAzuriteInstance = true`, which prevents using the alternate environment-based cloud path without editing the tests. Examples:

- `tests/table/apis/table.entity.azure.data-tables.test.ts:26-31`
- `tests/exe.test.ts:50-56`
- `tests/linuxbinary.test.ts:37-43`

### Hardcoded REST batch inner URLs

The raw table REST suite embeds `10002` inside batch payloads (`tests/table/apis/table.entity.rest.test.ts:70-71`, `110-111`), so port changes are not fully centralized.

---

## 7. Reusability assessment for the Rust implementation

## 7.1 What can be reused with only a thin harness layer?

**Recommended first-wave reusable coverage:**

- all 15 blob black-box TS files
- all 11 queue black-box TS files
- all 15 table black-box TS files
- table Go and .NET conformance after the table service is stable

This is enough to give strong end-to-end signal across:

- blob / queue / table protocol behavior
- SDK compatibility
- auth (shared key, SAS, OAuth)
- HTTPS
- CORS
- keep-alive / client behavior
- batch/multipart paths
- special naming and validation rules

## 7.2 What needs to change first?

### Required one-time harness work

The cleanest approach is to add an **external-server mode** to the existing test helpers instead of rewriting individual tests.

Recommended changes:

1. **Blob/Queue/Table test factories** should support an env-controlled external mode, e.g.:
   - `AZURITE_EXTERNAL_SERVER=1`
   - `AZURITE_BLOB_URL`
   - `AZURITE_QUEUE_URL`
   - `AZURITE_TABLE_URL`

2. In external mode, each factory should return a lightweight object with:
   - `config.host`
   - `config.port`
   - `start()` → no-op
   - `close()` → no-op
   - `clean()` → no-op

   That lets the existing suites keep their shape while hitting the Rust server.

3. **Table URL helpers** should read protocol/host/port/account from env instead of static `TableEntityTestConfig` constants.

4. **Table raw batch payload tests** should replace embedded `10002` strings dynamically.

### Why this approach is better than rewriting tests

- it preserves the existing, battle-tested assertions
- it keeps TS→Rust parity checks tied to the original suite
- it minimizes long-term maintenance because future upstream test additions remain reusable

## 7.3 Which suites still need special handling?

- **Blob SQL suite** (`npm run test:blob:sql` / `test:blob:sql:ci`) is valuable only if the Rust port grows equivalent SQL-backed blob metadata support.
- **Binary smoke suites** should wait until the Rust CLI is meant to emulate Azurite’s packaging/flags/log format.
- **Internal unit tests** should be reauthored in Rust, not run through Node.

---

## 8. Step-by-step instructions for running reusable tests against Rust

## Phase 0: one-time setup

1. From the repo root, install Node dependencies:
   ```bash
   npm ci --legacy-peer-deps
   ```
2. Implement the thin external-server adapter described above in the TS test helpers.
3. Keep the Rust server on Azurite-compatible credentials:
   - account: `devstoreaccount1`
   - key: the standard emulator key from `tests/testutils.ts`

## Phase 1: Blob first

1. Start the Rust blob service on **`127.0.0.1:11000`**.
2. Ensure it supports the same account/key as the TS suite.
3. If running HTTPS blob suites, serve TLS compatible with the test cert/key flow used by Azurite tests.
4. Run:
   ```bash
   npm run test:blob
   ```
5. Once basic blob parity is working, also run:
   ```bash
   npm run test:blob:in-memory
   ```
6. Defer `npm run test:blob:sql` until Rust has equivalent SQL metadata behavior.

## Phase 2: Queue second

1. Start the Rust queue service on **`127.0.0.1:11001`**.
2. Run:
   ```bash
   npm run test:queue
   npm run test:queue:in-memory
   ```
3. After basic parity, enable HTTPS/OAuth subsets if the Rust queue service supports them.

## Phase 3: Table third

1. Start the Rust table service on **`127.0.0.1:11002`** for the TS Mocha suites.
2. Run:
   ```bash
   npm run test:table
   npm run test:table:in-memory
   ```
3. After adapter work for raw REST payloads, include:
   - `tests/table/apis/table.entity.rest.test.ts`
   - `tests/table/apis/table.validation.rest.test.ts`
4. Once table parity is strong, also run the cross-language conformance suites with the Rust table service on **`10002`** (or patch those suites to a configurable port).

## Phase 4: Full parity sweep

After blob/queue/table service-level stability:

```bash
npm run test
```

That gives one aggregated Node-based E2E pass over the reusable TS suites.

---

## 9. Recommended phased adoption order

This should follow the existing porting priority:

1. **Blob first**
   - richest SDK-based API surface
   - already aligned with project phase order
   - best early confidence in shared auth/error/path handling

2. **Queue second**
   - smaller and faster suite
   - good follow-up validation of shared account/auth/cors/https behavior

3. **Table third**
   - highest harness complexity because it mixes:
     - legacy and new Node SDKs
     - raw REST repro suites
     - Go/.NET conformance
     - both 11002 and 10002 assumptions

---

## 10. Gaps and new tests still worth adding for Rust

## 10.1 Missing today

- No existing “external arbitrary server” mode in the TS harness
- No blob/queue cross-language conformance suites comparable to the table .NET/Go coverage
- Binary smoke tests are tied to TS packaging behavior, not generic API parity

## 10.2 Recommended additions for the Rust port

1. **Add external-server mode to the upstream TS harness** rather than building a Rust-only duplicate suite.
2. **Add one Rust CLI smoke suite** that only checks:
   - process starts
   - ports bind
   - basic service ping or CRUD works
   This replaces the fragile TS binary-output assertions as a Rust-native packaging check.
3. **Eventually add blob/queue cross-language SDK checks** (similar to existing table Go/.NET tests) if long-term compatibility confidence becomes important.

---

## 11. Final recommendation

**Recommended strategy:** reuse Azurite’s existing Node/Mocha integration suites as the primary Rust black-box parity harness, with a very small amount of one-time harness adaptation.

Do **not** start by rewriting tests in Rust. The existing suites already encode years of Azurite compatibility behavior. The highest-value move is to make those suites target an external server cleanly, then run them against the Rust implementation in this order:

1. blob (`11000`)
2. queue (`11001`)
3. table (`11002`), then table conformance (`10002`)

That gives the Rust port a realistic, protocol-focused validation path that stays close to upstream TypeScript behavior and remains useful when future TS tests are added.
