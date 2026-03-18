# Queue Authentication (Phase 14.6)

## File Info
- **Source Path:** `src/queue/authentication/` (10 files, ~1,790 LOC)
- **Rust Target:** `azurite-queue/src/authentication/`
- **Type:** Authentication implementations
- **Phase:** 14.6
- **Complexity:** M (medium)
- **Status:** ported

## Exports

```rust
// Authenticator trait
pub trait IAuthenticator: Send + Sync {
    async fn validate(
        &self,
        req: &dyn IRequest,
        context: &Context,
    ) -> Result<Option<bool>, AuthenticationError>;
}

// Concrete authenticators
pub struct QueueSASAuthenticator { ... }        // (398 LOC)
pub struct QueueSharedKeyAuthenticator { ... }   // (356 LOC)
pub struct QueueTokenAuthenticator { ... }       // (253 LOC)
pub struct AccountSASAuthenticator { ... }       // (315 LOC)

// SAS/Permission types
pub struct IQueueSASSignatureValues {            // (155 LOC)
    pub canonical_resource: String,
    pub string_to_sign: String,
}

pub enum QueueSASPermissions {                   // (6 LOC)
    Read,
    Add,
    Update,
    Process,
}

// Permission mapping
pub struct OperationAccountSASPermission { ... } // (227 LOC)
    // Maps operations to account-level permissions (raup)
pub struct OperationQueueSASPermission { ... }   // (71 LOC)
    // Maps operations to queue-level permissions

// Marker interfaces
pub trait IAuthenticationContext: Send + Sync {} // (3 LOC)
pub trait IAuthenticator: Send + Sync { ... }    // (6 LOC)
```

## Dependencies

- Phase 3: AccountSASPermissions, AccountSASServices, AccountSASResourceTypes, IAccountSASSignatureValues
- Phase 5 Queue: IRequest, Context (from generated)
- Common: IIPRange, HMAC-SHA256 signing
- StorageErrorFactory (14.3)

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `string` | `String` | Queue names, account names, access keys |
| `Buffer` | `Vec<u8>` | HMAC signatures, key material |
| `Promise<boolean \| undefined>` | `Result<Option<bool>, AuthError>` | Validation result |
| `IRequest` | `&dyn IRequest` | Request abstraction |
| `Context` | `&Context` | Request context |
| `URLSearchParams` | `HashMap<String, String>` | Query string parsing |
| `BaseAuthenticator` (base class) | Trait with default impl or shared code | Shared auth logic |
| `CryptoJS.HmacSHA256` | `hmac_sha2::HmacSha256` | HMAC signing |

## Special Handling / Fidelity Flags

### Queue SAS Permissions
- **Permission letters**: `r` (read), `a` (add), `u` (update), `p` (process)
- **Differs from blob**: Blob uses `racwd` (read, add, create, write, delete)
- **Validation order**: Operations mapped to letter sets; validation checks letter membership

### SAS Signature Generation
1. **Canonical resource format**: `/queueservices/accountname/queuename`
2. **String-to-sign components** (in order):
   - Permission letters (e.g., "raup")
   - Start time (ISO 8601 with Z suffix)
   - Expiry time (ISO 8601 with Z suffix)
   - Canonical resource
   - Resource type (either 'sco' for service/container/object)
   - IP range (optional, exact format preservation required)
   - API version (e.g., "2020-08-04")
3. **HMAC signing**: Base64(HMAC-SHA256(UTF-8(string-to-sign), Base64Decode(account_key)))
4. **Authorization header format**: `SharedAccessSignature sv=VERSION&sr=RESOURCE&sig=SIGNATURE&...`

### Shared Key Authentication
- **Header format**: `Authorization: SharedKey accountname:signature`
- **Signature algorithm**: Same as Phase 3 blob auth, but with queue-specific headers
- **Header ordering**: Specific order required for canonical string (like blob)
- **Case sensitivity**: Header names are case-insensitive, but value parsing is case-sensitive

### Token (Azure AD) Authentication
- **Bearer token validation**: Extract from `Authorization: Bearer <token>`
- **Token parsing**: JWT structure (header.payload.signature); verify signature if needed
- **Scope validation**: Check if token has queue service scope

### Error Handling
- All authenticators return `Result<Option<bool>, AuthenticationError>`
  - `Ok(Some(true))` = authentication succeeded
  - `Ok(Some(false))` = authentication failed (not an error condition; may fallback to anon)
  - `Ok(None)` = authenticator cannot handle (skip, try next)
  - `Err(_)` = authentication error (malformed header, invalid signature)

### Comparison to Blob Phase 7
- **Identical pattern**: Same authenticator trait + concrete implementations
- **Permission set differs**: `raup` (queue) vs `racwd` (blob)
- **Canonical resource format**: `/queueservices/...` vs `/blobservices/...`
- **Shared Key Lite**: Queue doesn't have Shared Key Lite (unlike table)
- **SAS scoping**: Both support IP range + time range; queue is simpler

## Change Propagation

**Authenticator chain impact:**
- Adding/removing authenticator from chain requires updating middleware factory
- Authenticator ordering matters (e.g., Bearer tokens before SAS before Shared Key)

**Permission mapping changes:**
- Changes to OperationAccountSASPermission break validation for operations that map to different letters
- Changes to OperationQueueSASPermission break queue-level SAS authorization

**SAS signature format changes:**
- Order of string-to-sign components is critical; reordering breaks all existing SAS tokens
- Adding new parameters (e.g., IP range) requires updating both generation and validation

## Rust Porting Notes

1. **Trait objects**: Use `Box<dyn IAuthenticator>` in authenticator chain
2. **HMAC signing**: Use `hmac_sha2::HmacSha256` from `hmac` and `sha2` crates
3. **Base64 encoding/decoding**: Use `base64` crate
4. **String-to-sign order**: Must match TS exactly (consider test vectors from existing code)
5. **Header parsing**: Case-insensitive header name lookups; case-sensitive value parsing
6. **Time formatting**: ISO 8601 with Z suffix (e.g., "2021-10-20T15:30:00Z") — use `chrono` crate
7. **Error hierarchy**: Implement custom error type with HTTP status codes
8. **Permission validation**: Use bitfield or set-based approach for efficient letter checking
9. **Token validation**: For Bearer tokens, either validate JWT signature (using `jsonwebtoken` crate) or skip signature validation (if auth service handles it)
10. **Async boundary**: Mark validate() as async; may need to call external auth service

## Implementation Strategy

Propose 3-file porting units for Phase 14.6:
1. **Unit 1**: Trait + permission types + error types (IAuthenticator, QueueSASPermissions, errors)
2. **Unit 2**: SAS authenticators (QueueSASAuthenticator, IQueueSASSignatureValues, OperationQueueSASPermission)
3. **Unit 3**: Shared Key + Token + Account SAS (QueueSharedKeyAuthenticator, QueueTokenAuthenticator, AccountSASAuthenticator, OperationAccountSASPermission)
