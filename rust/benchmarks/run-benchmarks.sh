#!/usr/bin/env bash
# rust/benchmarks/run-benchmarks.sh
#
# Runs the full benchmark matrix (TS vs Rust × disk vs in-memory),
# collects results, and prints a comparison table.
#
# Usage:
#   ./rust/benchmarks/run-benchmarks.sh [--iterations N] [--warmup N]
#
# Prerequisites:
#   - Node.js with npm packages installed (npm install in repo root)
#   - Rust binary built (cargo build --release in rust/)
#   - TS build available (npm run build in repo root)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${RUST_ROOT}/.." && pwd)"

ITERATIONS="${ITERATIONS:-200}"
WARMUP="${WARMUP:-20}"

# Parse CLI args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --iterations) ITERATIONS="$2"; shift 2 ;;
    --warmup) WARMUP="$2"; shift 2 ;;
    -h|--help)
      cat <<'EOF'
Usage: run-benchmarks.sh [OPTIONS]

Options:
  --iterations N   Number of benchmark iterations per operation (default: 200)
  --warmup N       Number of warmup iterations (default: 20)

Environment variables:
  SKIP_BUILD       Skip building TS and Rust binaries
  ITERATIONS       Same as --iterations
  WARMUP           Same as --warmup

The script runs 4 benchmark configurations:
  1. TypeScript + in-memory
  2. TypeScript + disk
  3. Rust + in-memory
  4. Rust + disk

Results are saved to rust/benchmarks/results/ as JSON and a summary table
is printed to stdout.
EOF
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

RESULTS_DIR="${SCRIPT_DIR}/results"
mkdir -p "${RESULTS_DIR}"

# Ensure ports are free
kill_port_users() {
  for port in 13000 13001 13002; do
    local pids
    pids="$(lsof -ti ":${port}" 2>/dev/null || true)"
    if [[ -n "${pids}" ]]; then
      echo "Killing existing processes on port ${port}: ${pids}"
      echo "${pids}" | xargs kill 2>/dev/null || true
      sleep 1
    fi
  done
}

# Build step
if [[ -z "${SKIP_BUILD:-}" ]]; then
  echo "=== Building TypeScript ==="
  (cd "${REPO_ROOT}" && npm run build 2>&1 | tail -1)

  echo "=== Building Rust (release) ==="
  (cd "${RUST_ROOT}" && cargo build --release 2>&1 | tail -1)
else
  echo "Skipping builds (SKIP_BUILD is set)"
fi

# Verify prerequisites
if [[ ! -f "${REPO_ROOT}/dist/src/azurite.js" ]]; then
  echo "ERROR: TS build not found at dist/src/azurite.js. Run: npm run build" >&2
  exit 1
fi

RUST_BIN=""
for candidate in "${RUST_ROOT}/target/release/azurite" "${RUST_ROOT}/target/x86_64-unknown-linux-gnu/release/azurite"; do
  if [[ -x "${candidate}" ]]; then
    RUST_BIN="${candidate}"
    break
  fi
done
if [[ -z "${RUST_BIN}" ]]; then
  echo "ERROR: Rust binary not found. Run: cd rust && cargo build --release" >&2
  exit 1
fi

echo ""
echo "=== Benchmark Configuration ==="
echo "Iterations: ${ITERATIONS}"
echo "Warmup:     ${WARMUP}"
echo "Rust binary: ${RUST_BIN}"
echo "Results dir: ${RESULTS_DIR}"
echo ""

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
CONFIGS=("ts/memory" "ts/disk" "rust/memory" "rust/disk")
JSON_FILES=()

for config in "${CONFIGS[@]}"; do
  impl="${config%%/*}"
  mode="${config##*/}"
  label="${impl}_${mode}"
  json_file="${RESULTS_DIR}/${TIMESTAMP}_${label}.json"

  echo "================================================================"
  echo "  Running: ${impl} / ${mode}"
  echo "================================================================"

  kill_port_users

  node "${SCRIPT_DIR}/benchmark.js" \
    --impl "${impl}" \
    --mode "${mode}" \
    --iterations "${ITERATIONS}" \
    --warmup "${WARMUP}" \
    --json > "${json_file}" 2>&1 || {
      echo "WARNING: Benchmark ${label} failed. Check ${json_file}" >&2
      # Write a placeholder so the summary script doesn't crash
      echo '{"label":"'"${label}"'","error":true}' > "${json_file}"
    }

  JSON_FILES+=("${json_file}")
  echo ""
done

kill_port_users

# Generate summary
echo ""
echo "================================================================"
echo "  RESULTS SUMMARY"
echo "================================================================"
echo ""

node -e '
const fs = require("fs");
const files = process.argv.slice(1);
const results = files.map(f => {
  const raw = fs.readFileSync(f, "utf8");
  // The JSON output may be preceded by stderr lines; find the JSON line
  const lines = raw.split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("{")) {
      try { return JSON.parse(trimmed); } catch {}
    }
  }
  return { label: f, error: true };
});

const valid = results.filter(r => !r.error);
if (valid.length === 0) {
  console.log("No valid results collected.");
  process.exit(1);
}

function fmtUs(us) {
  if (us >= 1e6) return (us / 1e6).toFixed(2) + " s";
  if (us >= 1e3) return (us / 1e3).toFixed(1) + " ms";
  return us.toFixed(0) + " µs";
}
function fmtMB(mb) { return mb.toFixed(1) + " MB"; }
function fmtPct(p) { return p.toFixed(1) + "%"; }

// Startup & Memory table
console.log("### Startup & Resource Usage\n");
console.log("| Configuration    | Startup   | Memory (med) | Memory (peak) | CPU (avg) | CPU (peak) |");
console.log("|------------------|-----------|--------------|---------------|-----------|------------|");
for (const r of valid) {
  console.log(
    "| " + r.label.padEnd(16) +
    " | " + (r.startupMs + " ms").padStart(9) +
    " | " + fmtMB(r.memory.medianMemMB).padStart(12) +
    " | " + fmtMB(r.memory.peakMemMB).padStart(13) +
    " | " + fmtPct(r.memory.avgCpu).padStart(9) +
    " | " + fmtPct(r.memory.peakCpu).padStart(10) + " |"
  );
}

// Operation latency table
console.log("\n### Operation Latency (mean ± stddev)\n");
console.log("| Configuration    |        Blob |       Queue |       Table |");
console.log("|------------------|-------------|-------------|-------------|");
for (const r of valid) {
  const b = r.blob, q = r.queue, t = r.table;
  console.log(
    "| " + r.label.padEnd(16) +
    " | " + (fmtUs(b.mean) + " ± " + fmtUs(b.stddev)).padStart(11) +
    " | " + (fmtUs(q.mean) + " ± " + fmtUs(q.stddev)).padStart(11) +
    " | " + (fmtUs(t.mean) + " ± " + fmtUs(t.stddev)).padStart(11) + " |"
  );
}

// Detailed per-operation table
console.log("\n### Detailed Latency Percentiles\n");
console.log("| Configuration    | Op    | Mean       | Median     | P95        | P99        |");
console.log("|------------------|-------|------------|------------|------------|------------|");
for (const r of valid) {
  for (const [op, s] of [["Blob", r.blob], ["Queue", r.queue], ["Table", r.table]]) {
    console.log(
      "| " + r.label.padEnd(16) +
      " | " + op.padEnd(5) +
      " | " + fmtUs(s.mean).padStart(10) +
      " | " + fmtUs(s.median).padStart(10) +
      " | " + fmtUs(s.p95).padStart(10) +
      " | " + fmtUs(s.p99).padStart(10) + " |"
    );
  }
}

// Speedup comparison
const tsMemory = valid.find(r => r.label === "ts/memory");
const rustMemory = valid.find(r => r.label === "rust/memory");
const tsDisk = valid.find(r => r.label === "ts/disk");
const rustDisk = valid.find(r => r.label === "rust/disk");

if (tsMemory && rustMemory) {
  console.log("\n### Speedup (Rust vs TypeScript)\n");
  console.log("| Mode   | Blob    | Queue   | Table   | Startup | Memory  |");
  console.log("|--------|---------|---------|---------|---------|---------|");
  for (const [mode, ts, rust] of [["memory", tsMemory, rustMemory], ["disk", tsDisk, rustDisk]]) {
    if (!ts || !rust) continue;
    const bSpeedup = (ts.blob.mean / rust.blob.mean).toFixed(2) + "x";
    const qSpeedup = (ts.queue.mean / rust.queue.mean).toFixed(2) + "x";
    const tSpeedup = (ts.table.mean / rust.table.mean).toFixed(2) + "x";
    const sSpeedup = (ts.startupMs / rust.startupMs).toFixed(2) + "x";
    const mRatio = (ts.memory.peakMemMB / rust.memory.peakMemMB).toFixed(2) + "x";
    console.log(
      "| " + mode.padEnd(6) +
      " | " + bSpeedup.padStart(7) +
      " | " + qSpeedup.padStart(7) +
      " | " + tSpeedup.padStart(7) +
      " | " + sSpeedup.padStart(7) +
      " | " + mRatio.padStart(7) + " |"
    );
  }
  console.log("\n(Values > 1.0x = Rust is faster/smaller)");
}
' "${JSON_FILES[@]}"

echo ""
echo "Raw JSON results saved to: ${RESULTS_DIR}/"
ls -1 "${RESULTS_DIR}/${TIMESTAMP}"_*.json 2>/dev/null
