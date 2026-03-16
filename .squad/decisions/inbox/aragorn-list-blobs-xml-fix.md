# Aragorn: EnumerationResults XML attributes

## Context
The Rust blob serializer was emitting `EnumerationResults` metadata like `ServiceEndpoint` and `ContainerName` as child elements, while TypeScript Azurite emits them as XML attributes via `xmlIsAttribute: true` mapper metadata.

## Decision
Preserve the TypeScript contract by carrying `xmlIsAttribute` through the Rust blob mapper model and by mapping attribute-bearing XML properties to quick-xml's `@AttributeName` convention during serialization.

## Scope
- `ListBlobsFlatSegmentResponse`
- `ListBlobsHierarchySegmentResponse`
- `ListContainersSegmentResponse`
- `FilterBlobSegment`
- Other blob models already marked with XML attributes in TypeScript metadata (`BlobMetadata.encrypted`, `BlobName.encoded`)

## Implementation notes
- Added `xmlIsAttribute` to the Rust `Mapper` type.
- Patched both generated metadata snapshots used by blob serialization: `mappers.generated.json` and `specifications.generated.json`.
- Updated `apply_model_mapping()` to serialize attribute fields as `@...` keys so quick-xml emits root attributes instead of child elements.
- Added a direct integration test that asserts raw list-blobs XML contains root attributes and does not contain child-element fallbacks.

## Why
This keeps the Rust port mechanically aligned with the TypeScript mapper contract and fixes the differential mismatch without introducing custom per-handler XML logic.
