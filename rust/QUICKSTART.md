# Azurite Rust — Quick Start Guide

Welcome! This guide will help you get up and running with **Azurite Rust**, a high-performance, cross-platform emulator for Azure Storage (Blob, Queue, and Table services).

> **Note:** This is the Rust implementation of Azurite. It maintains feature parity with the TypeScript version while offering improved performance and deployment flexibility.

## Prerequisites

You'll need **Rust** installed on your system. If you haven't already:

1. **Install Rust** using [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

2. **Minimum Rust version:** 1.77 or later
   ```bash
   rustc --version
   ```

3. **Verify cargo is available:**
   ```bash
   cargo --version
   ```

## Building Azurite Rust

Navigate to the `rust/` directory and build the release binary:

```bash
cd rust
cargo build --release
```

The compiled binary will be available at:
```
rust/target/release/azurite-rust
```

> **Tip:** Building in release mode optimizes performance. Use `cargo build` (without `--release`) for faster builds during development.

## Running Azurite

### Quick Start (All Services)

Run all three services (Blob, Queue, Table) combined:

```bash
cargo run --release
```

Or use the compiled binary directly:

```bash
./target/release/azurite-rust
```

Both commands will:
- Start Blob service on `http://127.0.0.1:10000`
- Start Queue service on `http://127.0.0.1:10001`
- Start Table service on `http://127.0.0.1:10002`
- Store data in the current working directory

### Individual Services

You can run a single service independently:

**Blob service only:**
```bash
cargo run --release -p azurite-blob
# or: ./target/release/azurite-blob-rust
```

**Queue service only:**
```bash
cargo run --release -p azurite-queue
# or: ./target/release/azurite-queue-rust
```

**Table service only:**
```bash
cargo run --release -p azurite-table
# or: ./target/release/azurite-table-rust
```

## Common Options

### Custom Storage Location

Store data in a specific directory:

```bash
cargo run --release -- --location /data/azurite
```

Or with the binary:
```bash
./target/release/azurite-rust -l /data/azurite
```

The directory will be created if it doesn't exist.

### Custom Host and Port

Listen on a different address or port:

```bash
cargo run --release -- --blobHost 0.0.0.0 --blobPort 9000
```

Individual service ports:
```bash
./target/release/azurite-rust \
  --blobHost 0.0.0.0 --blobPort 10000 \
  --queueHost 0.0.0.0 --queuePort 10001 \
  --tableHost 0.0.0.0 --tablePort 10002
```

### In-Memory Mode (No Persistence)

Disable disk persistence — all data is lost when the process stops:

```bash
./target/release/azurite-rust --inMemoryPersistence
```

Optionally set a memory limit (in megabytes):
```bash
./target/release/azurite-rust --inMemoryPersistence --extentMemoryLimit 1024
```

### Silent Mode

Suppress access logs in the console:

```bash
./target/release/azurite-rust -s
```

Or:
```bash
./target/release/azurite-rust --silent
```

### Debug Logging

Enable detailed debug logs to a file:

```bash
./target/release/azurite-rust --debug /var/log/azurite/debug.log
```

The log file will help diagnose issues.

### Loose Mode

Enable loose validation mode (useful for compatibility with some test tools):

```bash
./target/release/azurite-rust -L
# or:
./target/release/azurite-rust --loose
```

### Skip API Version Check

Allow requests with any API version:

```bash
./target/release/azurite-rust --skipApiVersionCheck
```

### Blob Query Compatibility Mode

Blob Query defaults to TypeScript-compatible behavior and returns HTTP 400 for empty `comp=query` requests. Disable that bug-for-bug mode to get the semantically correct 501 response instead:

```bash
./target/release/azurite-rust --disableBugForBugCompatibility
```

## Connecting from Client Applications

Azurite uses the same connection strings and endpoints as Azure Storage.

### Default Connection String

For local development, use:
```
UseDevelopmentStorage=true
```

This connects to all three services on the default local ports (Blob: 10000, Queue: 10001, Table: 10002).

### Explicit Connection String

For custom configurations:
```
DefaultEndpointsProtocol=http;
AccountName=devstoreaccount1;
AccountKey=Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBekX7Gx==;
BlobEndpoint=http://127.0.0.1:10000/devstoreaccount1;
QueueEndpoint=http://127.0.0.1:10001/devstoreaccount1;
TableEndpoint=http://127.0.0.1:10002/devstoreaccount1;
```

### Azure SDK for Python

```python
from azure.storage.blob import BlobServiceClient

connection_string = "UseDevelopmentStorage=true"
client = BlobServiceClient.from_connection_string(connection_string)

# Create a container
container = client.create_container("my-container")

# Upload a blob
blob = container.upload_blob("my-blob", b"Hello, World!")
```

### Azure SDK for Node.js

```javascript
const { BlobServiceClient } = require("@azure/storage-blob");

const connectionString = "UseDevelopmentStorage=true";
const blobServiceClient = BlobServiceClient.fromConnectionString(connectionString);

// Create a container
const containerClient = blobServiceClient.getContainerClient("my-container");
await containerClient.create();

// Upload a blob
const blockBlobClient = containerClient.getBlockBlobClient("my-blob");
await blockBlobClient.upload("Hello, World!", 13);
```

### Azure SDK for .NET

```csharp
using Azure.Storage.Blobs;
using Azure.Storage.Queues;
using Azure.Data.Tables;

string connectionString = "UseDevelopmentStorage=true";

// Blob
var blobServiceClient = new BlobServiceClient(
    new Uri("http://127.0.0.1:10000/devstoreaccount1"),
    new StorageSharedKeyCredential("devstoreaccount1", "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBekX7Gx==")
);

var container = await blobServiceClient.CreateBlobContainerAsync("my-container");
var blob = container.Value.GetBlobClient("my-blob");
await blob.UploadAsync(BinaryData.FromString("Hello, World!"), overwrite: true);
```

### Azure CLI

```bash
# Set environment for local development
export AZURE_STORAGE_ACCOUNT=devstoreaccount1
export AZURE_STORAGE_KEY=Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBekX7Gx==

# List containers
az storage container list --connection-string "UseDevelopmentStorage=true"

# Upload a blob
az storage blob upload \
  --connection-string "UseDevelopmentStorage=true" \
  --container-name my-container \
  --name my-blob \
  --file ./my-file.txt
```

### cURL for REST API

**Blob service example:**
```bash
# Create a container
curl -X PUT http://127.0.0.1:10000/devstoreaccount1/my-container

# Upload a blob (PUT request with body)
curl -X PUT \
  -H "Content-Type: application/octet-stream" \
  -d "Hello, World!" \
  http://127.0.0.1:10000/devstoreaccount1/my-container/my-blob

# Download a blob
curl http://127.0.0.1:10000/devstoreaccount1/my-container/my-blob
```

**Queue service example:**
```bash
# Create a queue
curl -X PUT http://127.0.0.1:10001/devstoreaccount1/myqueue

# Send a message
curl -X POST http://127.0.0.1:10001/devstoreaccount1/myqueue/messages \
  -d "messagetype=text&messagetext=Hello"

# Receive messages
curl http://127.0.0.1:10001/devstoreaccount1/myqueue/messages
```

## Advanced Configuration

### Certificate for HTTPS

You can enable HTTPS with a certificate:

```bash
./target/release/azurite-rust \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem
```

For `.pfx` certificates:
```bash
./target/release/azurite-rust \
  --cert /path/to/cert.pfx \
  --pwd your-password
```

### OAuth Support

Enable basic OAuth support:

```bash
./target/release/azurite-rust --oauth basic
```

### Disable Product-Style URLs

By default, Azurite accepts both styles of storage URLs. To enforce path-style only:

```bash
./target/release/azurite-rust --disableProductStyleUrl
```

## Implementation Status

### Implemented Services

- **Blob Storage** — Fully featured blob operations, including:
  - Container management (create, list, delete, properties, ACL)
  - Blob operations (create, read, update, delete, copy)
  - Snapshots and leases
  - SAS and authentication (SharedKey, Account SAS, Service SAS, OAuth)
  - Metadata and properties
  - Block and page blob support

- **Queue Storage** — Core queue operations including:
  - Queue management (create, list, delete, properties)
  - Message operations (put, get, peek, delete, update, clear)
  - Visibility timeout and TTL
  - SAS and authentication support

- **Table Storage** — Basic table operations (in progress):
  - Table management (create, list, delete)
  - Entity operations (insert, update, query, delete)
  - Batch operations

### Known Limitations

- **No read-only secondary endpoint (RA-GRS)** — Azurite emulates a primary endpoint only
- **Limited scalability** — Azurite is optimized for development, not production workloads
- **In-memory storage** — Without `--inMemoryPersistence`, data persists to disk in the workspace directory
- **No Azure AD** — OAuth support is basic; full Azure AD is not implemented

For detailed API coverage, refer to the main [README.md](../README.md).

## Troubleshooting

### Port Already in Use

If a port is in use, specify a different one:

```bash
./target/release/azurite-rust --blobPort 9000 --queuePort 9001 --tablePort 9002
```

### Workspace Errors

Ensure the workspace directory is writable:

```bash
./target/release/azurite-rust --location ./my-workspace
```

Check the debug log for details:

```bash
./target/release/azurite-rust --debug ./debug.log
```

### Connection Failures

Verify Azurite is listening:

```bash
curl http://127.0.0.1:10000
```

If needed, bind to all interfaces:

```bash
./target/release/azurite-rust --blobHost 0.0.0.0
```

## Docker

To run Azurite in Docker, build a container from the Rust binary:

```dockerfile
FROM rust:latest as builder
WORKDIR /azurite
COPY . .
RUN cd rust && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /azurite/rust/target/release/azurite-rust /usr/local/bin/
EXPOSE 10000 10001 10002
CMD ["azurite-rust"]
```

Build and run:

```bash
docker build -t azurite-rust .
docker run -it -p 10000:10000 -p 10001:10001 -p 10002:10002 azurite-rust
```

## For More Information

- **TS Reference:** See the TypeScript Azurite [README.md](../README.md) for comprehensive feature documentation
- **Porting Records:** Implementation details are tracked in [rust/porting-db/](./porting-db/)
- **Contributing:** This is an active port; contributions are welcome on [GitHub](https://github.com/Azure/Azurite)

---

**Happy emulating! 🚀**
