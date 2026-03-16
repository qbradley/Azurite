# Aragorn parity fixes

## Proposed decisions

1. **Preserve xml2js XML declaration in Rust serializers**
   - Blob, queue, and table `stringifyXML`/`jsonToXML` helpers should emit `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` exactly.
   - Rationale: TS uses `xml2js.Builder`, whose default declaration is observable in every XML response body.

2. **Preserve raw copy-source query strings during validation**
   - Cross-account copy validation should append/replace `comp=metadata` without reconstructing the rest of the source query.
   - Rationale: SAS-bearing copy-source URLs are part of externally supplied auth material; keeping the raw query mirrors TS `URLBuilder` behavior and avoids validation-only drift.

3. **Use atomic single-lock upsert/merge for table entities**
   - `insertOrUpdateTableEntity` and `insertOrMergeTableEntity` should read the current entity snapshot and mutate/insert within one lock scope.
   - Rationale: separate existence queries create the same TOCTOU class already fixed in blob metadata paths.
