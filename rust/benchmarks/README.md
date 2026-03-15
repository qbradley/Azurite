# Azurite Performance Benchmarks

Comparative benchmarks between the TypeScript and Rust implementations of Azurite,
measuring latency, startup time, memory usage, and CPU utilization.

## Quick Start

```bash
# From the repository root:
bash rust/benchmarks/run-benchmarks.sh
```

This will:
1. Build both TS (`npm run build`) and Rust (`cargo build --release`) implementations
2. Run 4 benchmark configurations (TS×memory, TS×disk, Rust×memory, Rust×disk)
3. Print a comparison table and save raw JSON results

## What It Measures

### Operations (from the [in-memory-persistence design doc](../../docs/designs/2023-10-in-memory-persistence.md))

Each iteration performs a **write-then-read** cycle:

| Service | Operations per Iteration |
|---------|------------------------|
| **Blob** | Upload 8 KiB → Set metadata → Delete |
| **Queue** | Send message → Receive → Delete |
| **Table** | Upsert entity → Query by partition key |

### Metrics

| Metric | Description |
|--------|-------------|
| **Startup time** | Time from process spawn to first successful HTTP response |
| **Operation latency** | Per-iteration timing (mean, median, P95, P99, stddev) |
| **Memory (median)** | Median RSS of the server process during the benchmark |
| **Memory (peak)** | Peak RSS of the server process |
| **CPU (avg/peak)** | CPU utilization of the server process (sampled every 200ms) |

### Configurations

| Config | Server | Persistence |
|--------|--------|-------------|
| `ts/memory` | TypeScript (Node.js) | `--inMemoryPersistence` |
| `ts/disk` | TypeScript (Node.js) | Disk (temp directory) |
| `rust/memory` | Rust (native binary) | `--inMemoryPersistence` |
| `rust/disk` | Rust (native binary) | Disk (temp directory) |

## Options

```bash
# Custom iteration count
bash rust/benchmarks/run-benchmarks.sh --iterations 500 --warmup 50

# Skip building (if already built)
SKIP_BUILD=1 bash rust/benchmarks/run-benchmarks.sh

# Run a single configuration manually
node rust/benchmarks/benchmark.js --impl rust --mode memory --iterations 100

# Output JSON (for programmatic consumption)
node rust/benchmarks/benchmark.js --impl rust --mode memory --json
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SKIP_BUILD` | unset | If set, skip building TS and Rust |
| `ITERATIONS` | 200 | Number of benchmark iterations |
| `WARMUP` | 20 | Number of warmup iterations |

## Output

Results are saved as JSON files in `rust/benchmarks/results/` with timestamps:
```
results/20260315_120000_ts_memory.json
results/20260315_120000_ts_disk.json
results/20260315_120000_rust_memory.json
results/20260315_120000_rust_disk.json
```

The summary table printed to stdout includes:
- Startup & resource usage comparison
- Operation latency with percentiles
- Speedup ratios (Rust vs TypeScript)

## Prerequisites

- Node.js 18+ with project dependencies installed (`npm install`)
- Rust toolchain (`cargo build --release`)
- Linux (uses `/proc` for memory/CPU sampling)
- Ports 13000-13002 must be free

## Notes

- Benchmarks use ports 13000-13002 to avoid conflicts with running Azurite instances
- Each configuration runs in isolation (server started and stopped per config)
- Disk mode uses a temporary directory that is cleaned up after each run
- The benchmark client runs single-threaded (sequential operations) to measure per-request latency
