# Traffic Capture & Replay Differential Testing Harness

## Overview

This harness turns the existing 998 TypeScript integration tests into thousands of differential assertions by:

1. **Recording** all HTTP traffic from TS integration tests through a proxy
2. **Replaying** that recorded traffic against Rust Azurite
3. **Comparing** each response using semantic normalization

This provides exhaustive differential coverage without writing new tests.

## Architecture

```
RECORDING PHASE:
  Mocha tests → Recording Proxy (:11000/:11001/:11002) → TS Azurite (:10000/:10001/:10002)
  All request/response pairs saved to corpus files

REPLAY PHASE:
  Replay Tool → Rust Azurite (:10000/:10001/:10002)
  Each response compared to recorded corpus with normalization
```

## Components

### 1. `traffic_recorder.py` — Recording Proxy

A Python HTTP proxy (stdlib only, no external dependencies) that:
- Listens on ports 11000, 11001, 11002
- Forwards requests to TS Azurite on ports 10000, 10001, 10002
- Records each request/response pair as JSON
- Handles binary bodies, chunked encoding, keep-alive
- Thread-safe for concurrent test connections

**Corpus format**: JSON files with arrays of exchanges:
```json
{
  "service": "blob",
  "recorded_at": 1234567890.123,
  "exchange_count": 1247,
  "exchanges": [
    {
      "sequence_number": 1,
      "timestamp": 1234567890.123,
      "service": "blob",
      "request": {
        "method": "PUT",
        "path": "/devstoreaccount1/container1?restype=container",
        "headers": {...},
        "body": {"encoding": "utf-8", "data": "..."}
      },
      "response": {
        "status_code": 201,
        "reason": "Created",
        "headers": {...},
        "body": {"encoding": "empty", "data": ""}
      }
    },
    ...
  ]
}
```

Output: `blob_traffic.json`, `queue_traffic.json`, `table_traffic.json`

### 2. `traffic_replay.py` — Replay & Comparison Tool

Replays recorded traffic and compares responses using normalization from `differential_test.py`:

**Normalization applied**:
- Headers: Ignore transport headers (Connection, Transfer-Encoding), replace dynamic values (Date, ETag, x-ms-request-id) with placeholders
- JSON bodies: Recursively normalize dynamic fields (timestamps, ETags, message IDs)
- XML bodies: Normalize dynamic text (RequestId, Time), sort children for order-independence
- Binary bodies: Direct byte comparison

**Comparison logic**:
1. Status code must match exactly
2. Headers must match (after normalization, order-independent)
3. Body must match (content-type aware normalization)

**Output format**:
```
=== Traffic Replay: blob (1247 exchanges) ===
[PASS] #0001 PUT /devstoreaccount1/container1?restype=container → 201
[PASS] #0002 PUT /devstoreaccount1/container1/blob1 → 201
[FAIL] #0003 GET /devstoreaccount1/container1?restype=container&comp=list → 200
  Headers differ:
    x-ms-something: value_a != value_b
  Body differs:
    ... (diff details)
...
=== Results: 1240/1247 passed (7 failures) ===
```

### 3. `capture-and-replay.sh` — Orchestration Script

Wrapper script for the full workflow:

```bash
# Full workflow: record + replay
./rust/scripts/capture-and-replay.sh

# Record only (corpus saved to rust/scripts/traffic_corpus/)
./rust/scripts/capture-and-replay.sh record

# Replay only (assumes corpus exists)
./rust/scripts/capture-and-replay.sh replay
```

The script:
- Starts TS Azurite on :10000/:10001/:10002
- Starts recording proxy on :11000/:11001/:11002
- Runs TS integration tests with `AZURITE_EXTERNAL_SERVER=1`
- Stops proxy and TS servers, saves corpus
- Builds Rust Azurite
- Starts Rust Azurite on :10000/:10001/:10002
- Runs replay tool against corpus
- Reports results

## Usage

### Quick Start

From the repository root:

```bash
# Full workflow (record + replay)
./rust/scripts/capture-and-replay.sh
```

This will:
1. Start TS Azurite and recording proxy
2. Run all TS integration tests (998 tests)
3. Record all HTTP traffic to `rust/scripts/traffic_corpus/`
4. Build and start Rust Azurite
5. Replay all recorded traffic
6. Compare responses and report differences

### Separate Phases

```bash
# Record traffic only
./rust/scripts/capture-and-replay.sh record

# Replay recorded traffic (after making Rust changes)
./rust/scripts/capture-and-replay.sh replay
```

### Manual Usage

#### Recording:

```bash
# Start recording proxy
python3 rust/scripts/traffic_recorder.py --output-dir rust/scripts/traffic_corpus

# In another terminal: start TS Azurite on :10000/:10001/:10002
npm run blob &
npm run queue &
npm run table &

# Run tests pointing to proxy ports
AZURITE_EXTERNAL_SERVER=1 \
AZURITE_BLOB_PORT=11000 \
AZURITE_QUEUE_PORT=11001 \
AZURITE_TABLE_PORT=11002 \
npx mocha --require ts-node/register --grep @loki --recursive --exit 'tests/**/*.test.ts'

# Stop proxy (Ctrl+C) - corpus files saved automatically
```

#### Replaying:

```bash
# Start Rust Azurite on :10000/:10001/:10002
cargo run --release --bin azurite-blob -- --blobHost 127.0.0.1 --blobPort 10000 &
cargo run --release --bin azurite-queue -- --queueHost 127.0.0.1 --queuePort 10001 &
cargo run --release --bin azurite-table -- --tableHost 127.0.0.1 --tablePort 10002 &

# Run replay
python3 rust/scripts/traffic_replay.py --corpus-dir rust/scripts/traffic_corpus
```

## Key Features

### No External Dependencies

Both Python scripts use only stdlib:
- `http.server`, `http.client` for HTTP
- `json` for corpus format
- `threading` for concurrency
- `xml.etree.ElementTree` for XML normalization

### Thread-Safe Recording

The proxy handles concurrent connections from parallel test execution:
- Shared exchange list protected by locks
- Monotonic sequence numbers
- Each exchange logged with timestamp

### Binary Body Support

Bodies are stored in the corpus with encoding metadata:
- `encoding: "utf-8"` for text/JSON/XML
- `encoding: "base64"` for binary data
- `encoding: "empty"` for no body

### Semantic Normalization

Comparison ignores noise and focuses on semantic differences:
- Dynamic IDs, timestamps, ETags replaced with placeholders
- Transport headers (Connection, Transfer-Encoding) ignored
- XML children sorted for order-independence
- JSON deep comparison with field-aware normalization

### Stateful Replay

Replay sends ALL requests in recorded sequence order per service. The corpus includes all setup (container creation, blob upload, etc.), so starting Rust Azurite fresh before replay ensures correct state evolution.

## Expected Results

On a clean TS→Rust port with perfect parity:
- **Pass rate**: ~100% (all exchanges match)
- **Known differences**: Dynamic field variations already normalized

On a port with regressions:
- **Failures** indicate genuine parity bugs
- **Diff output** shows exact header/body mismatches
- **Sequence numbers** help locate failing operations

## Troubleshooting

### Port conflicts

If ports 10000-10002 or 11000-11002 are in use:
```bash
# Check what's using the ports
lsof -i :10000
lsof -i :11000

# Kill orphaned processes
pkill -f azurite
```

### Corpus not found

```bash
# Ensure recording phase completed
ls -lh rust/scripts/traffic_corpus/*.json

# If missing, run record phase again
./rust/scripts/capture-and-replay.sh record
```

### Rust build failures

```bash
# Build manually with more detail
cd rust
cargo build --release --bin azurite-blob --bin azurite-queue --bin azurite-table
```

### Test timeouts

The TS integration tests can take 10-30 minutes. Ensure:
- Adequate system resources (disk I/O for Loki persistence)
- No other Azurite instances running
- Proxy forwarding is working (check `traffic_corpus/proxy.log`)

## Integration with CI

The harness can be integrated into CI pipelines:

```yaml
# Example GitHub Actions step
- name: Differential testing harness
  run: |
    cd rust/scripts
    ./capture-and-replay.sh
  timeout-minutes: 60
```

Exit codes:
- `0` = all exchanges passed
- `1` = one or more failures

## Maintenance

When TS behavior changes:
1. Re-record the corpus: `./capture-and-replay.sh record`
2. Commit the new corpus (or regenerate in CI)
3. Update Rust implementation to match
4. Verify: `./capture-and-replay.sh replay`

## Limitations

- **Corpus size**: Full 998-test corpus can be 10-50 MB (depends on blob sizes in tests)
- **Replay time**: Replaying thousands of exchanges takes 5-15 minutes
- **State coupling**: Corpus assumes fresh server state at replay start
- **Non-determinism**: Tests with time-based behavior may have variability

## Future Enhancements

Potential improvements:
- Parallel replay for faster execution
- Incremental corpus updates (record only changed tests)
- Corpus filtering (replay specific services or test patterns)
- HTML diff report generation
- Performance metrics (latency comparison TS vs Rust)

---

**Author**: Boromir (QA Expert)  
**Date**: 2025-01-XX  
**Status**: Production-ready
