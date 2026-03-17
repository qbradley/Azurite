# Table Authentication Translation - Quick Reference Card

## Permission Letters at a Glance

### Table SAS (4 letters)
```
r = Query (read)
a = Add (insert)
u = Update
d = Delete
```
**Validation**: ANY letter match = authorized

### Account SAS (8 letters)
```
r = Read        c = Create      l = List        p = Process
a = Add         w = Write       d = Delete      u = Update
```
**Validation**: ALL THREE match (service + resourceType + permission)

---

## String-to-Sign Formats

### SharedKey
```
METHOD\nCONTENT_MD5\nCONTENT_TYPE\nDATE\n/{ACCOUNT}{PATH}[?comp=VALUE]
```

### SharedKeyLite
```
DATE\n/{ACCOUNT}{PATH}
```

### Table SAS
```
PERMS\nSTART\nEXPIRY\n/table/{ACCOUNT}/{TABLE}\nID\nIP\nPROTO\nVERSION\nSPK\nSRK\nEPK\nERK
```
(All optional fields as empty strings if missing)

### Account SAS (new: >= 2020-12-06)
```
ACCOUNT\nPERMS\nSERVICES\nRESORCE_TYPES\nSTART\nEXPIRY\nIP\nPROTO\nVERSION\nENC\n
```

### Account SAS (old: < 2020-12-06)
```
PERMS\nSERVICES\nRESORCE_TYPES\nSTART\nEXPIRY\nIP\nPROTO\nVERSION
```

---

## Query Parameters

| Param | Meaning | SAS Type |
|-------|---------|----------|
| `sv` | Version (required) | All |
| `sp` | Permissions (w/ se or si) | All |
| `se` | Expiry (w/ sp or si) | All |
| `st` | Start time | All |
| `spr` | Protocol | All |
| `sip` | IP range | All |
| `sig` | Signature (required) | All |
| `si` | Identifier/ACL ID | Table/Blob |
| `spk`, `srk`, `epk`, `erk` | Partition/Row key ranges | Table only |
| `ss` | Services (required) | Account only |
| `srt` | Resource types (required) | Account only |

---

## Authorization Headers

```
SharedKey {ACCOUNT}:{SIGNATURE}
SharedKeyLite {ACCOUNT}:{SIGNATURE}
Bearer {JWT}
```

---

## Implementation Order (Risk→Complexity)

1. **SharedKey** (ref: blob_shared_key_authenticator.rs)
2. **SharedKeyLite** (simplified SharedKey)
3. **AccountSAS** (reuse azurite-common)
4. **TableSAS** (complex: ACL lookup)
5. **Token** (JWT decode)

---

## Critical Code Patterns

### Permission Check (ANY-matching)
```rust
// Required: "u", Granted: "raud"
if granted.contains(required_char) {
    // Authorized
}
```

### Signature Generation
```rust
let stringToSign = /* build per format */;
let signature = computeHMACSHA256(&stringToSign, account_key);
let matches = computed_signature == request_signature;
```

### Secondary Account Path
```rust
if is_secondary && path.find(account) == Some(1) {
    // Also try with "{account}-secondary"
    let alt_path = path.replace(account, &format!("{account}-secondary"));
}
```

### Stored Access Policy
```rust
if let Some(identifier) = sas_values.identifier {
    let policy = tableStore.getTableACL(account, table).await?;
    values.permissions = policy.permission;
    values.expiryTime = policy.expiry;
}
```

---

## Reusable Rust Types (azurite-common)

```rust
use azurite_common::authentication::{
    DateOrString, SASProtocol, SASProtocolOrString,
    IIPRange, ipRangeToString,
    IAccountSASSignatureValues, generateAccountSASSignature,
};
use azurite_common::utils::utils::{
    computeHMACSHA256, truncatedISO8061Date, getURLQueries,
};
```

---

## Testing Checklist

- [ ] Table SAS: Permission ANY-match
- [ ] Account SAS: All 3 constraints match
- [ ] SharedKey: Canonical resource + headers
- [ ] SharedKeyLite: Date only
- [ ] Secondary paths: Both variants tried
- [ ] ACL policy: Override perms after sig
- [ ] Token: nbf, exp, iss, aud validation
- [ ] Empty strings: In signature when fields missing
- [ ] Version branching: Account SAS >= 2020-12-06
- [ ] URL decoding: Before sig check

---

## Common Pitfalls

❌ **Wrong**: Omitting empty fields from signature
✅ **Right**: Always include empty string for missing optional fields

❌ **Wrong**: Iterating through granted permissions to find required
✅ **Right**: Check if any required permission letter is IN granted string

❌ **Wrong**: Not lowercasing table name in canonical resource
✅ **Right**: Always use `.toLowerCase()` on table name

❌ **Wrong**: Only trying one signature variant for secondary account
✅ **Right**: Try BOTH with and without "-secondary" suffix

❌ **Wrong**: Verifying JWT signature in BASIC mode
✅ **Right**: Just check structure, claims, and validity windows

---

## File References (TS Source)

| File | Key Section | Lines |
|------|-------------|-------|
| ITableSASSignatureValues.ts | String-to-sign | 169-181, 229-241 |
| OperationTableSASPermission.ts | Validation logic | 11-18 |
| TableSASAuthenticator.ts | Full flow | 24-280 |
| TableSharedKeyAuthenticator.ts | SharedKey | 51-64 (sig), 212-252 (canon) |
| TableSharedKeyLiteAuthenticator.ts | SharedKeyLite | 54-64 (sig), 209-251 (canon) |
| OperationAccountSASPermission.ts | Account SAS mapping | 15-51 |

