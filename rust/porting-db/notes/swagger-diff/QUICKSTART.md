# Swagger Differential Runner — Quick Start

## Purpose
Execute differential tests between TypeScript and Rust Azurite by sending identical requests to both servers and comparing responses.

## Prerequisites
1. Both TS and Rust Azurite must be running:
   - TS: `127.0.0.1:10001` (blob)
   - Rust: `127.0.0.1:10000` (blob)

## Basic Usage

```bash
cd /home/azureuser/Azurite/rust/scripts

# Run all operations (72 total)
python3 -m swagger_diff.runner

# Run specific tier (operations with no prerequisites)
python3 -m swagger_diff.runner --tier 0

# Run specific operations
python3 -m swagger_diff.runner --operations Container_Create,BlockBlob_Upload

# Run operations by resource group
python3 -m swagger_diff.runner --group Container

# Verbose mode with JSON output
python3 -m swagger_diff.runner -v --output results.json
```

## Operation Tiers

- **Tier 0** (10 ops): No prerequisites — service operations, Container_Create
- **Tier 1** (17 ops): Needs container — container operations, initial blob creates
- **Tier 2** (36 ops): Needs blob — most blob operations
- **Tier 3** (8 ops): Needs lease — lease lifecycle operations
- **Tier 4** (1 op): Needs snapshot — Blob_CreateSnapshot

## Understanding Results

### Console Output
```
=== Differential Test Results ===
Total: 72 scenarios
  ✓ Passed: 68
  ✗ Failed: 4

[FAIL] Container_Create / happy_path
  TS Status: 201, Rust Status: 409
  Status code: TS=201 != Rust=409
```

### JSON Output
Each result includes:
- `operation_id`: The swagger operation
- `scenario_name`: Test scenario (currently just "happy_path")
- `ts_status`, `rust_status`: HTTP status codes
- `passed`: Boolean result
- `differences`: List of specific differences found
- `error`: Error message if request failed

## Common Issues

### "Server NOT reachable"
- Ensure both servers are running
- Check ports: TS on 10001, Rust on 10000
- Verify with: `curl http://127.0.0.1:10001/?comp=list`

### "Operation X not found in spec"
- Verify swagger spec path (default: `swagger/blob-storage-2021-10-04.json`)
- Use `--spec` flag to specify different path

### Auth Failures
- Runner uses default Azure Storage Emulator credentials
- Both servers must be configured to accept devstoreaccount1

## Architecture

```
runner.py
├── DifferentialRunner: Orchestrates test execution
├── ComparisonEngine: Normalizes and compares responses
├── ScenarioGenerator: Creates test scenarios
└── ReportGenerator: Formats output

Integrates with:
└── swagger_parser.py (Samwise's module)
    ├── SwaggerSpec: Parses swagger specifications
    ├── RequestBuilder: Builds authenticated requests
    └── DependencyDAG: Manages operation dependencies
```

## Next Steps

1. Run tier 0 first to validate basic operations
2. Progress through tiers to test dependent operations
3. Review failures to identify implementation gaps
4. Create issues for Rust implementation fixes

## Phase 2 (Future)

Current implementation only tests happy-path scenarios. Phase 2 will add:
- Missing required parameters
- Invalid parameter values
- Boundary conditions
- Error scenarios
- Multi-step operation sequences

---

**Questions?** See `rust/scripts/swagger_diff/README.md` for detailed documentation.
