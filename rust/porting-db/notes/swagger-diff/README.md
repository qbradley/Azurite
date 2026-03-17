# Swagger-Based Differential Test Framework

This framework provides automated differential testing between TypeScript and Rust Azurite implementations using OpenAPI/Swagger specifications as the source of truth.

## Architecture

The framework consists of two main modules:

1. **`swagger_parser.py`** (Samwise): Parses Swagger specs, builds request templates, manages operation dependencies
2. **`runner.py`** (Boromir): Executes differential tests, compares responses, reports results

## Components

### swagger_parser.py - Swagger Parser Module ✅ COMPLETE

Parses the Azure Blob Storage Swagger specification and generates HTTP requests.

**Status:** Implemented by Samwise (2026-03-17)

**Key Classes:**

- `SwaggerSpec`: Load and parse swagger spec, resolve $refs, organize operations by tier
- `Operation`: Single API operation with resolved parameters
- `Parameter`: Resolved parameter (no $refs)
- `DependencyDAG`: Operation dependency graph (5 tiers)
- `RequestBuilder`: Build HTTP requests with SharedKey auth
- `Request`: Complete HTTP request ready to execute

**Features:**
- Parses 72 blob operations from `swagger/blob-storage-2021-10-04.json`
- Resolves all $ref references (103 parameters, 56 definitions)
- 5-tier dependency system (Service → Container → Blob → Lease → Snapshot)
- SharedKey authentication with HMAC-SHA256
- Smart parameter defaults (container/blob names, headers, query params)
- Filters x-ms-paths discriminators (BlockBlob, PageBlob, AppendBlob)

**Usage:**
```python
from swagger_diff.swagger_parser import SwaggerSpec, RequestBuilder

# Load spec
spec = SwaggerSpec('swagger/blob-storage-2021-10-04.json')

# Get operations
ops = spec.get_operations()  # All 72 operations sorted by tier
container_create = spec.get_operation('Container_Create')

# Get dependency chain
chain = spec.get_dependency_chain('Blob_SetMetadata')
# Returns: [Container_Create, BlockBlob_Upload]

# Build request
builder = RequestBuilder('devstoreaccount1', account_key, '127.0.0.1', 10000)
request = builder.build_request(container_create)
# Request has: method, url, headers, body, path, query_params
```

See inline documentation in `swagger_parser.py` for full API details.

### runner.py - Differential Test Runner (TODO - Boromir)

The main test execution engine that:
- Connects to both TS and Rust Azurite servers
- Executes operation scenarios against both servers simultaneously
- Compares responses using normalized comparison logic
- Manages test state between operations
- Reports results with coverage matrix

**Key Classes:**

- `DifferentialRunner`: Main test orchestrator
- `ComparisonEngine`: Response comparison with dynamic field normalization (adapted from `traffic_replay.py`)
- `ScenarioGenerator`: Generates test scenarios (Phase 1: happy-path only)
- `ReportGenerator`: Console and JSON output

### Comparison Logic (Adapted from traffic_replay.py)

The comparison engine uses sophisticated normalization to handle dynamic values:

**Header Comparison:**
- Ignores transport headers (connection, content-length, keep-alive, server, transfer-encoding)
- Normalizes dynamic headers (date, etag, x-ms-request-id, etc.) to placeholders

**Body Comparison:**
- **XML**: Structural comparison with element sorting and dynamic field normalization
- **JSON**: Recursive normalization of dynamic values
- **Binary**: Direct byte comparison

**Dynamic Field Constants:**
- `TRANSPORT_IGNORED_HEADER_NAMES`: Headers to skip during comparison
- `DYNAMIC_HEADER_PLACEHOLDERS`: Header name → placeholder mappings
- `DYNAMIC_FIELD_PLACEHOLDERS`: Field name → placeholder mappings

## Usage

### Basic Usage

```bash
# Run all operations
python3 -m swagger_diff.runner

# Run specific operations
python3 -m swagger_diff.runner --operations Container_Create,Blob_Upload

# Run a specific tier
python3 -m swagger_diff.runner --tier 0

# Run a resource group
python3 -m swagger_diff.runner --group Container

# Verbose output
python3 -m swagger_diff.runner --verbose

# Save results to JSON
python3 -m swagger_diff.runner --output results.json
```

### Server Configuration

By default, the runner expects:
- **TS Azurite**: `127.0.0.1:10001` (blob)
- **Rust Azurite**: `127.0.0.1:10000` (blob)

Override with:
```bash
python3 -m swagger_diff.runner --ts-port 10001 --rust-port 10000
```

### Output Format

**Console Output:**
```
=== Differential Test Results ===
Total: 72 scenarios
  ✓ Passed: 68
  ✗ Failed: 4

[FAIL] Container_Create / happy_path
  TS Status: 201, Rust Status: 409
  Status code: TS=201 != Rust=409
```

**JSON Output:**
```json
[
  {
    "operation_id": "Container_Create",
    "scenario_name": "happy_path",
    "ts_status": 201,
    "rust_status": 201,
    "passed": true,
    "differences": [],
    "setup_divergence": false,
    "error": null
  }
]
```

## State Management

The runner manages state between operations:

1. **Fresh state per operation**: Each operation gets a clean environment
2. **Container cleanup**: All containers are deleted after each operation
3. **No cross-contamination**: Operations don't affect each other

This eliminates cascade failures seen in traffic replay testing.

## Phase 1 Scope

Current implementation (Phase 1):
- ✓ Happy-path scenarios only (valid requests with all required params)
- ✓ 72 blob operations covered
- ✓ Comparison engine fully functional
- ✓ Fresh state per operation
- ✓ Coverage matrix output
- ✓ JSON results for analysis

Phase 2 (future):
- Parameter combinatorics (missing params, invalid values)
- Error scenarios and boundary conditions
- Setup chain execution
- Dependency tier management

## Integration with swagger_parser

The swagger_parser module is complete and ready to use:

```python
from swagger_diff.swagger_parser import SwaggerSpec, RequestBuilder, DependencyDAG

# Load spec
spec = SwaggerSpec('swagger/blob-storage-2021-10-04.json')

# Get operation
op = spec.get_operation('Container_Create')
# op.operation_id, op.method, op.path_template, op.tier, op.parameters, etc.

# Build request
builder = RequestBuilder('devstoreaccount1', account_key, host, port)
request = builder.build_request(op, overrides={'_containerName': 'test'})
# request.method, request.url, request.headers, request.body

# Get dependencies
chain = spec.get_dependency_chain('Blob_SetMetadata')
# Returns: [Container_Create, BlockBlob_Upload] operations

# Get operations by tier/group
tier_0_ops = [op for op in spec.get_operations() if op.tier == 0]
container_ops = [op for op in spec.get_operations() if op.group == 'Container']
```

## Key Differences from Traffic Replay

1. **Two live servers** vs one server + recorded responses
2. **Fresh state per operation** vs cumulative state with artifacts
3. **No ETag/snapshot mapping** needed (both servers start fresh)
4. **Setup chains executed on both** servers
5. **Auth computed fresh** for each request

## Testing the Runner

```bash
# Check servers are reachable (will use mock spec)
python3 -m swagger_diff.runner

# With verbose output
python3 -m swagger_diff.runner -v
```

The runner will check connectivity to both servers and report if they're reachable.

## File Structure

```
swagger_diff/
├── __init__.py          # Package initialization
├── __main__.py          # Module entry point
├── runner.py            # Main differential test runner (this file)
├── swagger_parser.py    # Swagger spec parser (Samwise's module)
└── README.md            # This file
```
