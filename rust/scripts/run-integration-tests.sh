#!/usr/bin/env bash
# rust/scripts/run-integration-tests.sh
# Integration test runner for Rust Azurite against TypeScript test suite
# Current status: the unified Rust azurite binary does not yet support the full integration-test startup flow.
# Today it rejects queue/table CLI flags and later panics during queue startup, so this runner fails fast with clear diagnostics until unified startup wiring is complete.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${RUST_ROOT}/.." && pwd)"

AZURITE_TEST_SERVICES="${AZURITE_TEST_SERVICES:-all}"
export RUST_LOG="${RUST_LOG:-info}"

BLOB_HOST="127.0.0.1"
BLOB_PORT="11000"
QUEUE_HOST="127.0.0.1"
QUEUE_PORT="11001"
TABLE_HOST="127.0.0.1"
TABLE_PORT="11002"

SERVER_PID=""
SERVER_LOG=""

cleanup() {
  local exit_code="${1:-0}"

  trap - EXIT INT TERM

  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    echo "Stopping Rust Azurite server (pid ${SERVER_PID})..."
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi

  if [[ ${exit_code} -eq 0 && -n "${SERVER_LOG}" && -f "${SERVER_LOG}" ]]; then
    rm -f "${SERVER_LOG}"
  fi

  exit "${exit_code}"
}

trap 'cleanup $?' EXIT
trap 'cleanup 130' INT TERM

usage() {
  cat <<'EOF'
Usage: rust/scripts/run-integration-tests.sh

Environment variables:
  AZURITE_TEST_SERVICES  blob | queue | table | all (default: all)
  RUST_LOG               Rust log filter passed to the server (default: info)
  SKIP_BUILD             If set, skip cargo build --release
EOF
}

require_command() {
  local command_name="$1"

  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Required command not found: ${command_name}" >&2
    exit 1
  fi
}

find_azurite_binary() {
  local direct_path="${RUST_ROOT}/target/release/azurite"
  local discovered_path=""

  if [[ -x "${direct_path}" ]]; then
    printf '%s\n' "${direct_path}"
    return 0
  fi

  discovered_path="$(find "${RUST_ROOT}/target" -path '*/release/azurite' -type f 2>/dev/null | head -n 1 || true)"
  if [[ -n "${discovered_path}" && -x "${discovered_path}" ]]; then
    printf '%s\n' "${discovered_path}"
    return 0
  fi

  return 1
}

validate_service_selection() {
  case "${AZURITE_TEST_SERVICES}" in
    blob|queue|table|all)
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unsupported AZURITE_TEST_SERVICES value: ${AZURITE_TEST_SERVICES}" >&2
      usage >&2
      exit 1
      ;;
  esac
}

service_selected() {
  local service="$1"
  [[ "${AZURITE_TEST_SERVICES}" == "all" || "${AZURITE_TEST_SERVICES}" == "${service}" ]]
}

print_server_log_tail() {
  if [[ -n "${SERVER_LOG}" && -f "${SERVER_LOG}" ]]; then
    echo "--- Rust Azurite log tail ---" >&2
    tail -n 40 "${SERVER_LOG}" >&2 || true
    echo "--- end log tail ---" >&2
  fi
}

wait_for_endpoint() {
  local service_name="$1"
  local url="$2"
  local http_code=""

  echo "Waiting for ${service_name} readiness at ${url}..."

  for attempt in $(seq 1 30); do
    if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
      echo "Rust Azurite exited before ${service_name} became ready." >&2
      print_server_log_tail
      return 1
    fi

    http_code="$(curl --silent --output /dev/null --write-out '%{http_code}' --max-time 2 "${url}" || true)"
    if [[ "${http_code}" != "000" ]]; then
      echo "${service_name} responded with HTTP ${http_code}."
      return 0
    fi

    sleep 1
  done

  echo "Timed out waiting for ${service_name} after 30 seconds." >&2
  print_server_log_tail
  return 1
}

run_service_tests() {
  local service="$1"

  # We run mocha directly (bypassing npm run test:*) so we can exclude
  # HTTPS / OAuth / CORS test files that require TLS support which
  # the Rust server does not yet provide.
  local mocha_base="npx cross-env NODE_TLS_REJECT_UNAUTHORIZED=0 mocha --require ts-node/register --no-timeouts --recursive --exit"

  case "${service}" in
    blob)
      echo "Running TypeScript blob integration tests (excluding HTTPS/OAuth/CORS)..."
      ${mocha_base} --grep @loki \
        --ignore 'tests/blob/https.test.ts' \
        --ignore 'tests/blob/oauth.test.ts' \
        --ignore 'tests/blob/blobCorsRequest.test.ts' \
        'tests/blob/*.test.ts' 'tests/blob/**/*.test.ts'
      ;;
    queue)
      echo "Running TypeScript queue integration tests (excluding HTTPS/OAuth)..."
      ${mocha_base} --grep @loki \
        --ignore 'tests/queue/https.test.ts' \
        --ignore 'tests/queue/oauth.test.ts' \
        'tests/queue/*.test.ts' 'tests/queue/**/*.test.ts'
      ;;
    table)
      echo "Running TypeScript table integration tests..."
      ${mocha_base} \
        'tests/table/*.test.ts' 'tests/table/**/*.test.ts'
      ;;
    *)
      echo "Unknown test service: ${service}" >&2
      return 1
      ;;
  esac
}

run_requested_tests() {
  case "${AZURITE_TEST_SERVICES}" in
    all)
      run_service_tests blob
      run_service_tests queue
      run_service_tests table
      ;;
    blob|queue|table)
      run_service_tests "${AZURITE_TEST_SERVICES}"
      ;;
  esac
}

if [[ $# -gt 1 ]]; then
  echo "Usage error: expected zero or one argument." >&2
  usage >&2
  exit 1
fi

if [[ $# -eq 1 ]]; then
  case "$1" in
    blob|queue|table|all)
      AZURITE_TEST_SERVICES="$1"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
fi

validate_service_selection
require_command curl
require_command npm

cd "${REPO_ROOT}"

if [[ -z "${SKIP_BUILD:-}" ]]; then
  require_command cargo
  echo "Building Rust Azurite binary..."
  (
    cd "${RUST_ROOT}"
    cargo build --release
  )
else
  echo "Skipping Rust build because SKIP_BUILD is set."
fi

AZURITE_BIN="$(find_azurite_binary || true)"
if [[ -z "${AZURITE_BIN}" ]]; then
  echo "Rust Azurite binary not found under ${RUST_ROOT}/target. Build it first or unset SKIP_BUILD." >&2
  exit 1
fi

SERVER_LOG="$(mktemp "${TMPDIR:-/tmp}/azurite-integration.XXXXXX.log")"
echo "Starting Rust Azurite from ${AZURITE_BIN}..."
echo "Server log: ${SERVER_LOG}"

"${AZURITE_BIN}" \
  --blobHost "${BLOB_HOST}" \
  --blobPort "${BLOB_PORT}" \
  --queueHost "${QUEUE_HOST}" \
  --queuePort "${QUEUE_PORT}" \
  --tableHost "${TABLE_HOST}" \
  --tablePort "${TABLE_PORT}" \
  --inMemoryPersistence \
  >"${SERVER_LOG}" 2>&1 &
SERVER_PID=$!

sleep 1
if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
  echo "Rust Azurite failed to stay running after startup." >&2
  print_server_log_tail
  exit 1
fi

if service_selected blob; then
  wait_for_endpoint "blob service" "http://${BLOB_HOST}:${BLOB_PORT}/devstoreaccount1?comp=properties"
fi
if service_selected queue; then
  wait_for_endpoint "queue service" "http://${QUEUE_HOST}:${QUEUE_PORT}/devstoreaccount1?comp=properties"
fi
if service_selected table; then
  wait_for_endpoint "table service" "http://${TABLE_HOST}:${TABLE_PORT}/devstoreaccount1/Tables"
fi

export AZURITE_EXTERNAL_SERVER=1
export AZURITE_BLOB_HOST="${BLOB_HOST}"
export AZURITE_BLOB_PORT="${BLOB_PORT}"
export AZURITE_QUEUE_HOST="${QUEUE_HOST}"
export AZURITE_QUEUE_PORT="${QUEUE_PORT}"
export AZURITE_TABLE_HOST="${TABLE_HOST}"
export AZURITE_TABLE_PORT="${TABLE_PORT}"
export NODE_TLS_REJECT_UNAUTHORIZED=0

echo "Running TypeScript integration tests for service selection: ${AZURITE_TEST_SERVICES}"
set +e
run_requested_tests
test_exit_code=$?
set -e

if [[ ${test_exit_code} -ne 0 ]]; then
  echo "TypeScript integration tests failed with exit code ${test_exit_code}." >&2
else
  echo "TypeScript integration tests completed successfully."
fi

exit "${test_exit_code}"
