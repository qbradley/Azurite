# Rust integration test scripts

## What this script does

`run-integration-tests.sh` builds the Rust `azurite` binary, starts it in the background, waits for selected services to respond on the TypeScript test ports, runs the existing Mocha integration suites against that external server, then stops the Rust server and returns the test exit code.

The script is intended to validate the Rust port with the existing TypeScript black-box suites once the external-server harness patch is present.

## Prerequisites

- Rust toolchain capable of building the workspace (`cargo`)
- Node.js and npm
- Repository dependencies installed at the repo root (`npm ci --legacy-peer-deps` is the safest setup)
- `curl` for readiness polling

## How to run it

From the repository root:

```bash
./rust/scripts/run-integration-tests.sh
```

You can also run it from another directory; it resolves the repo root automatically.

## Environment variables

- `AZURITE_TEST_SERVICES` — which Mocha suites to run: `blob`, `queue`, `table`, or `all` (default: `all`)
- `RUST_LOG` — Rust log filter for the server process (default: `info`)
- `SKIP_BUILD` — if set, skip `cargo build --release` and reuse the existing built `azurite` binary under `rust/target/`

The script also exports the external-server settings expected by the companion TypeScript harness work:

- `AZURITE_EXTERNAL_SERVER=1`
- `AZURITE_BLOB_HOST=127.0.0.1`
- `AZURITE_BLOB_PORT=11000`
- `AZURITE_QUEUE_HOST=127.0.0.1`
- `AZURITE_QUEUE_PORT=11001`
- `AZURITE_TABLE_HOST=127.0.0.1`
- `AZURITE_TABLE_PORT=11002`
- `NODE_TLS_REJECT_UNAUTHORIZED=0`

## Example invocations

Run all reusable suites:

```bash
./rust/scripts/run-integration-tests.sh
```

Run only queue tests:

```bash
AZURITE_TEST_SERVICES=queue ./rust/scripts/run-integration-tests.sh
```

Reuse an existing release build and increase Rust logging:

```bash
SKIP_BUILD=1 RUST_LOG=debug ./rust/scripts/run-integration-tests.sh
```

## Current limitations / known issues

- The companion TypeScript harness patch is required so `AZURITE_EXTERNAL_SERVER=1` makes the factories return no-op server objects instead of starting the TypeScript emulator.
- Validation during this change showed that the unified Rust binary currently rejects `--queueHost` / `--queuePort` / `--tableHost` / `--tablePort` at process startup, so the script currently fails before readiness polling can succeed.
- Validation also showed that even when started with blob-only flags, the same unified binary later panics during queue startup inside `azurite-queue`, so there is not yet a successful end-to-end run through the combined binary.
- Because of those two startup issues, no current `AZURITE_TEST_SERVICES` selection is expected to reach the Mocha execution step yet; the script is future-ready and intentionally reports those failures clearly.
- Some table REST coverage still depends on test-helper configurability for embedded endpoint strings described in `rust/porting-db/INTEGRATION-TEST-STRATEGY.md`.
