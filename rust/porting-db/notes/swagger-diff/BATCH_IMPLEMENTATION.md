# Azure Storage Batch Request Implementation

## Overview

The swagger differential test harness now properly generates Azure Storage batch request bodies for `Service_SubmitBatch` and `Container_SubmitBatch` operations.

## Problem

Previously, these operations were sending:
- **Content-Type:** `application/octet-stream`
- **Body:** Empty

This caused both operations to fail in differential testing.

## Solution

Implemented proper multipart/mixed batch request generation that conforms to the Azure Storage REST API specification.

## Implementation Details

### Modified Files

1. **rust/scripts/swagger_diff/swagger_parser.py**
   - Added `uuid` import for boundary generation
   - Modified `_generate_param_values()` to skip Content-Type for batch operations
   - Modified `_build_body()` to delegate batch operations to specialized handler
   - Added `_build_batch_body()` method to generate proper multipart bodies

### Batch Request Format

A batch request now includes:

```
POST /?comp=batch HTTP/1.1
Host: 127.0.0.1:10000
Content-Type: multipart/mixed; boundary=batch_<uuid>
Authorization: SharedKey devstoreaccount1:<signature>
Content-Length: <length>
x-ms-version: 2021-10-04
x-ms-date: <RFC1123-date>

--batch_<uuid>
Content-Type: application/http
Content-Transfer-Encoding: binary
Content-ID: 0

DELETE /devstoreaccount1/container/blob HTTP/1.1
x-ms-version: 2021-10-04
x-ms-date: <RFC1123-date>
Content-Length: 0
Authorization: SharedKey devstoreaccount1:<sub-signature>

--batch_<uuid>--
```

### Key Features

1. **Proper Content-Type:** `multipart/mixed; boundary=batch_<uuid>`
2. **MIME Part Headers:** Each sub-request has Content-Type, Content-Transfer-Encoding, and Content-ID
3. **Embedded HTTP Request:** Full HTTP request with method, path, and headers
4. **Dual Authentication:**
   - Outer request has SharedKey auth for batch submission
   - Each sub-request has independent SharedKey auth for the operation
5. **CRLF Line Endings:** Uses `\r\n` per HTTP multipart specification

## Testing

Run the validation test:

```bash
cd /home/azureuser/Azurite
python3 -m rust.scripts.swagger_diff.test_batch_requests
```

## Azure Storage Specification Compliance

The implementation follows the Azure Storage Batch REST API specification:
- Multipart MIME format per RFC 2046
- Each sub-request is a complete HTTP message
- Independent authentication for each sub-request
- Proper boundary markers and CRLF line endings

## Future Work

This implementation provides:
1. Correct test data for differential testing of batch operations
2. Reference implementation for batch request parsing in Rust port
3. Validation that both TS and Rust implementations handle batch requests correctly
