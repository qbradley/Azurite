#!/bin/bash
# Test script to reproduce Bug 6: PUT blob ignoring active lease

set -e

ACCOUNT="devstoreaccount1"
CONTAINER="testcontainer"
BLOB="testblob"
BASE_URL="http://127.0.0.1:10000/$ACCOUNT"

echo "==> Creating container..."
curl -X PUT "$BASE_URL/$CONTAINER?restype=container" \
  -H "x-ms-date: $(date -u '+%a, %d %b %Y %H:%M:%S GMT')" \
  -H "x-ms-version: 2025-11-05" \
  -w "\nStatus: %{http_code}\n"

echo ""
echo "==> Uploading initial blob..."
curl -X PUT "$BASE_URL/$CONTAINER/$BLOB" \
  -H "x-ms-date: $(date -u '+%a, %d %b %Y %H:%M:%S GMT')" \
  -H "x-ms-version: 2025-11-05" \
  -H "x-ms-blob-type: BlockBlob" \
  -H "Content-Length: 11" \
  -d "hello world" \
  -w "\nStatus: %{http_code}\n"

echo ""
echo "==> Acquiring lease..."
LEASE_RESPONSE=$(curl -s -X PUT "$BASE_URL/$CONTAINER/$BLOB?comp=lease" \
  -H "x-ms-date: $(date -u '+%a, %d %b %Y %H:%M:%S GMT')" \
  -H "x-ms-version: 2025-11-05" \
  -H "x-ms-lease-action: acquire" \
  -H "x-ms-lease-duration: 60" \
  -H "x-ms-proposed-lease-id: abcdefgh-1234-5678-9012-abcdefghijkl" \
  -i)

echo "$LEASE_RESPONSE"
LEASE_ID=$(echo "$LEASE_RESPONSE" | grep -i "x-ms-lease-id:" | cut -d' ' -f2 | tr -d '\r')
echo "Lease ID: $LEASE_ID"

echo ""
echo "==> Attempting to overwrite blob WITHOUT lease-id (should return 412)..."
curl -X PUT "$BASE_URL/$CONTAINER/$BLOB" \
  -H "x-ms-date: $(date -u '+%a, %d %b %Y %H:%M:%S GMT')" \
  -H "x-ms-version: 2025-11-05" \
  -H "x-ms-blob-type: BlockBlob" \
  -H "Content-Length: 9" \
  -d "new content" \
  -w "\nStatus: %{http_code}\n" \
  -i

echo ""
echo "==> Test complete. Expected status: 412, but Bug 6 says Rust returns 201"
