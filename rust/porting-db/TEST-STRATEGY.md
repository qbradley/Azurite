# Rust Port Test Strategy

## Purpose
Rust parity testing in Azurite follows Boromir's core rule: the TypeScript implementation defines the expected behavior, and the Rust port must produce the same outputs for the same inputs. When direct dual-execution is possible, the same fixture or request should be run against both implementations and compared at the response, error, and observable side-effect levels.

## Test layers

### 1. Unit tests
Use crate-local unit and integration tests for small, deterministic behaviors that do not require a running service.

Current Phase 1 focus:
- enum and string mappings (`OAuthLevel`, `LogLevels`)
- logger adapter forwarding (`ILogger` -> `ILoggerStrategy`)
- common trait contracts (`IDataStore`, `ICleaner`, `IAccountDataStore`)

### 2. Contract / parity tests
Contract tests encode TypeScript behavior in a way the Rust port can consume before full services exist. For early phases, this means trait-level assertions and ignored scaffolding for modules that are not translated yet. As concrete Rust implementations appear, contract tests should move from ignored placeholders to executable parity suites.

### 3. Service integration tests
Once blob, queue, and table services are translated far enough to boot, integration tests should mirror the TypeScript tree and validate SDK-visible behavior:
- Blob: request listener, middleware, and API parity
- Queue: service and message API parity
- Table: serialization, query, and REST parity

These suites should compare:
- HTTP status codes
- headers and response payloads
- error codes / messages
- persistence side effects when observable

## Test organization
The Rust workspace should mirror the TypeScript test layout by crate responsibility:

- `rust/crates/azurite-common/tests/common/`
  - Phase 1 shared contracts and mapping tests
- `rust/crates/azurite-blob/tests/blob/`
  - future `apis`, `unit`, and middleware parity suites
- `rust/crates/azurite-queue/tests/queue/`
  - future queue API parity suites
- `rust/crates/azurite-table/tests/table/`
  - future table API, serialization, and query parity suites

Ignored tests are preferred over comments for not-yet-translated areas so that `cargo test` validates compilation and preserves the intended coverage map.

## Running tests

### Rust
From the workspace root:

```bash
cd rust
cargo test --workspace
```

Target a crate while iterating:

```bash
cargo test -p azurite-common
cargo test -p azurite-blob
cargo test -p azurite-queue
cargo test -p azurite-table
```

### TypeScript
Use the existing TypeScript suites as the reference implementation:

```bash
npm run test:blob
npm run test:queue
npm run test:table
```

For parity work, prefer reusing existing TS fixtures, names, and setup patterns from `tests/` rather than inventing Rust-only scenarios.

## Parity workflow
1. Identify the closest existing TypeScript test or interface contract.
2. Encode the same input/output expectation in Rust.
3. If the Rust implementation is not ready, add an ignored scaffold with the exact missing milestone called out.
4. Once the implementation exists, unignore the test and compare behavior against TS output.
5. Record divergences with a minimal reproduction: input, TS result, Rust result, and affected module.

## CI / CD considerations
- Run `cargo test --workspace` on every Rust-port PR.
- Keep ignored parity tests in CI output; they act as an explicit backlog of parity coverage.
- When a service becomes runnable, add side-by-side CI jobs that execute representative TS and Rust API suites against the same fixtures.
- Favor stable, deterministic fixtures over time-sensitive or environment-sensitive assertions.
- Any discovered divergence should block acceptance until either the Rust implementation is corrected or the porting database explicitly documents why behavior intentionally differs.
