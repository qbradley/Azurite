#!/bin/bash
#
# Capture and Replay Differential Testing Harness
#
# This script orchestrates the full traffic capture and replay workflow:
# 1. Record HTTP traffic from TypeScript integration tests
# 2. Replay that traffic against Rust Azurite
# 3. Compare responses using normalization logic
#
# Usage:
#   ./capture-and-replay.sh          # Full workflow: record + replay
#   ./capture-and-replay.sh record   # Record only
#   ./capture-and-replay.sh replay   # Replay only (assumes corpus exists)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RUST_ROOT="$REPO_ROOT/rust"
CORPUS_DIR="$RUST_ROOT/scripts/traffic_corpus"

TRAFFIC_RECORDER="$RUST_ROOT/scripts/traffic_recorder.py"
TRAFFIC_REPLAY="$RUST_ROOT/scripts/traffic_replay.py"

TS_BLOB_PORT=10000
TS_QUEUE_PORT=10001
TS_TABLE_PORT=10002

PROXY_BLOB_PORT=11000
PROXY_QUEUE_PORT=11001
PROXY_TABLE_PORT=11002

RUST_BLOB_PORT=10000
RUST_QUEUE_PORT=10001
RUST_TABLE_PORT=10002

TS_BLOB_PID=""
TS_QUEUE_PID=""
TS_TABLE_PID=""
PROXY_PID=""
RUST_BLOB_PID=""
RUST_QUEUE_PID=""
RUST_TABLE_PID=""

cleanup_ts_servers() {
    echo "=== Cleaning up TypeScript servers ==="
    
    if [ -n "$TS_BLOB_PID" ] && kill -0 "$TS_BLOB_PID" 2>/dev/null; then
        echo "Stopping TS blob server (PID $TS_BLOB_PID)"
        kill "$TS_BLOB_PID" 2>/dev/null || true
        wait "$TS_BLOB_PID" 2>/dev/null || true
    fi
    
    if [ -n "$TS_QUEUE_PID" ] && kill -0 "$TS_QUEUE_PID" 2>/dev/null; then
        echo "Stopping TS queue server (PID $TS_QUEUE_PID)"
        kill "$TS_QUEUE_PID" 2>/dev/null || true
        wait "$TS_QUEUE_PID" 2>/dev/null || true
    fi
    
    if [ -n "$TS_TABLE_PID" ] && kill -0 "$TS_TABLE_PID" 2>/dev/null; then
        echo "Stopping TS table server (PID $TS_TABLE_PID)"
        kill "$TS_TABLE_PID" 2>/dev/null || true
        wait "$TS_TABLE_PID" 2>/dev/null || true
    fi
}

cleanup_proxy() {
    echo "=== Cleaning up recording proxy ==="
    
    if [ -n "$PROXY_PID" ] && kill -0 "$PROXY_PID" 2>/dev/null; then
        echo "Stopping recording proxy (PID $PROXY_PID)"
        kill -INT "$PROXY_PID" 2>/dev/null || true
        sleep 2
        if kill -0 "$PROXY_PID" 2>/dev/null; then
            kill -TERM "$PROXY_PID" 2>/dev/null || true
        fi
        wait "$PROXY_PID" 2>/dev/null || true
    fi
}

cleanup_rust_servers() {
    echo "=== Cleaning up Rust servers ==="
    
    if [ -n "$RUST_BLOB_PID" ] && kill -0 "$RUST_BLOB_PID" 2>/dev/null; then
        echo "Stopping Rust blob server (PID $RUST_BLOB_PID)"
        kill "$RUST_BLOB_PID" 2>/dev/null || true
        wait "$RUST_BLOB_PID" 2>/dev/null || true
    fi
    
    if [ -n "$RUST_QUEUE_PID" ] && kill -0 "$RUST_QUEUE_PID" 2>/dev/null; then
        echo "Stopping Rust queue server (PID $RUST_QUEUE_PID)"
        kill "$RUST_QUEUE_PID" 2>/dev/null || true
        wait "$RUST_QUEUE_PID" 2>/dev/null || true
    fi
    
    if [ -n "$RUST_TABLE_PID" ] && kill -0 "$RUST_TABLE_PID" 2>/dev/null; then
        echo "Stopping Rust table server (PID $RUST_TABLE_PID)"
        kill "$RUST_TABLE_PID" 2>/dev/null || true
        wait "$RUST_TABLE_PID" 2>/dev/null || true
    fi
}

wait_for_port() {
    local port=$1
    local timeout=${2:-30}
    local elapsed=0
    
    echo "Waiting for port $port to be ready..."
    while ! nc -z 127.0.0.1 "$port" 2>/dev/null; do
        sleep 1
        elapsed=$((elapsed + 1))
        if [ $elapsed -ge $timeout ]; then
            echo "ERROR: Timeout waiting for port $port"
            return 1
        fi
    done
    echo "Port $port is ready"
}

record_traffic() {
    echo ""
    echo "=============================================="
    echo "=== PHASE 1: RECORDING TRAFFIC ==="
    echo "=============================================="
    echo ""
    
    mkdir -p "$CORPUS_DIR"
    
    echo "=== Starting TypeScript Azurite servers ==="
    
    cd "$REPO_ROOT"
    
    echo "Starting TS blob server on port $TS_BLOB_PORT..."
    node -r ts-node/register src/blob/main.ts \
        --blobHost 127.0.0.1 \
        --blobPort $TS_BLOB_PORT \
        --location "$CORPUS_DIR/ts-state/blob" \
        --silent \
        --disableTelemetry \
        > "$CORPUS_DIR/ts-blob.log" 2>&1 &
    TS_BLOB_PID=$!
    
    echo "Starting TS queue server on port $TS_QUEUE_PORT..."
    node -r ts-node/register src/queue/main.ts \
        --queueHost 127.0.0.1 \
        --queuePort $TS_QUEUE_PORT \
        --location "$CORPUS_DIR/ts-state/queue" \
        --silent \
        --disableTelemetry \
        > "$CORPUS_DIR/ts-queue.log" 2>&1 &
    TS_QUEUE_PID=$!
    
    echo "Starting TS table server on port $TS_TABLE_PORT..."
    node -r ts-node/register src/table/main.ts \
        --tableHost 127.0.0.1 \
        --tablePort $TS_TABLE_PORT \
        --location "$CORPUS_DIR/ts-state/table" \
        --silent \
        --disableTelemetry \
        > "$CORPUS_DIR/ts-table.log" 2>&1 &
    TS_TABLE_PID=$!
    
    wait_for_port $TS_BLOB_PORT
    wait_for_port $TS_QUEUE_PORT
    wait_for_port $TS_TABLE_PORT
    
    echo ""
    echo "=== Starting recording proxy ==="
    
    python3 "$TRAFFIC_RECORDER" --output-dir "$CORPUS_DIR" > "$CORPUS_DIR/proxy.log" 2>&1 &
    PROXY_PID=$!
    
    sleep 3
    
    wait_for_port $PROXY_BLOB_PORT
    wait_for_port $PROXY_QUEUE_PORT
    wait_for_port $PROXY_TABLE_PORT
    
    echo ""
    echo "=== Running TypeScript integration tests ==="
    echo "Tests will connect to recording proxy on ports $PROXY_BLOB_PORT, $PROXY_QUEUE_PORT, $PROXY_TABLE_PORT"
    echo ""
    
    cd "$REPO_ROOT"
    
    AZURITE_EXTERNAL_SERVER=1 \
    AZURITE_BLOB_PORT=$PROXY_BLOB_PORT \
    AZURITE_QUEUE_PORT=$PROXY_QUEUE_PORT \
    AZURITE_TABLE_PORT=$PROXY_TABLE_PORT \
    NODE_TLS_REJECT_UNAUTHORIZED=0 \
    npx mocha \
        --require ts-node/register \
        --no-timeouts \
        --grep @loki \
        --recursive \
        --exit \
        'tests/**/*.test.ts' 2>&1 | tee "$CORPUS_DIR/test-run.log"
    
    TEST_EXIT_CODE=${PIPESTATUS[0]}
    
    echo ""
    echo "=== Test run completed (exit code: $TEST_EXIT_CODE) ==="
    
    cleanup_proxy
    cleanup_ts_servers
    
    echo ""
    echo "=== Recording phase complete ==="
    echo "Corpus saved to: $CORPUS_DIR"
    ls -lh "$CORPUS_DIR"/*.json 2>/dev/null || echo "No corpus files found"
    echo ""
    
    return $TEST_EXIT_CODE
}

replay_traffic() {
    echo ""
    echo "=============================================="
    echo "=== PHASE 2: REPLAYING TRAFFIC ==="
    echo "=============================================="
    echo ""
    
    if [ ! -d "$CORPUS_DIR" ]; then
        echo "ERROR: Corpus directory not found: $CORPUS_DIR"
        echo "Run './capture-and-replay.sh record' first"
        return 1
    fi
    
    if ! ls "$CORPUS_DIR"/*.json >/dev/null 2>&1; then
        echo "ERROR: No corpus files found in $CORPUS_DIR"
        return 1
    fi
    
    echo "=== Building Rust Azurite ==="
    
    cd "$RUST_ROOT"
    cargo build --release \
        --bin azurite-blob-rust \
        --bin azurite-queue-rust \
        --bin azurite-table-rust
    
    echo ""
    echo "=== Starting Rust Azurite servers ==="
    
    RUST_BIN_DIR="$RUST_ROOT/target/release"
    if [ ! -d "$RUST_BIN_DIR" ]; then
        RUST_BIN_DIR=$(find "$RUST_ROOT/target" -type d -name release | head -n1)
    fi
    
    if [ -z "$RUST_BIN_DIR" ] || [ ! -d "$RUST_BIN_DIR" ]; then
        echo "ERROR: Rust binary directory not found"
        return 1
    fi
    
    echo "Starting Rust blob server on port $RUST_BLOB_PORT..."
    "$RUST_BIN_DIR/azurite-blob-rust" \
        --blobHost 127.0.0.1 \
        --blobPort $RUST_BLOB_PORT \
        --location "$CORPUS_DIR/rust-state/blob" \
        --silent \
        > "$CORPUS_DIR/rust-blob.log" 2>&1 &
    RUST_BLOB_PID=$!
    
    echo "Starting Rust queue server on port $RUST_QUEUE_PORT..."
    "$RUST_BIN_DIR/azurite-queue-rust" \
        --queueHost 127.0.0.1 \
        --queuePort $RUST_QUEUE_PORT \
        --location "$CORPUS_DIR/rust-state/queue" \
        --silent \
        > "$CORPUS_DIR/rust-queue.log" 2>&1 &
    RUST_QUEUE_PID=$!
    
    echo "Starting Rust table server on port $RUST_TABLE_PORT..."
    "$RUST_BIN_DIR/azurite-table-rust" \
        --tableHost 127.0.0.1 \
        --tablePort $RUST_TABLE_PORT \
        --location "$CORPUS_DIR/rust-state/table" \
        --silent \
        > "$CORPUS_DIR/rust-table.log" 2>&1 &
    RUST_TABLE_PID=$!
    
    wait_for_port $RUST_BLOB_PORT
    wait_for_port $RUST_QUEUE_PORT
    wait_for_port $RUST_TABLE_PORT
    
    echo ""
    echo "=== Running replay and comparison ==="
    echo ""
    
    python3 "$TRAFFIC_REPLAY" --corpus-dir "$CORPUS_DIR" 2>&1 | tee "$CORPUS_DIR/replay-results.log"
    REPLAY_EXIT_CODE=${PIPESTATUS[0]}
    
    cleanup_rust_servers
    
    echo ""
    echo "=== Replay phase complete ==="
    echo "Results saved to: $CORPUS_DIR/replay-results.log"
    echo ""
    
    return $REPLAY_EXIT_CODE
}

main() {
    local mode="${1:-full}"
    
    trap 'cleanup_proxy; cleanup_ts_servers; cleanup_rust_servers' EXIT INT TERM
    
    case "$mode" in
        record)
            record_traffic
            ;;
        replay)
            replay_traffic
            ;;
        full|"")
            if record_traffic; then
                echo "Recording successful, proceeding to replay..."
                replay_traffic
            else
                echo "ERROR: Recording phase failed"
                exit 1
            fi
            ;;
        *)
            echo "Usage: $0 [record|replay|full]"
            echo ""
            echo "Modes:"
            echo "  record  - Record traffic from TypeScript integration tests"
            echo "  replay  - Replay recorded traffic against Rust Azurite"
            echo "  full    - Run both record and replay phases (default)"
            exit 1
            ;;
    esac
}

main "$@"
