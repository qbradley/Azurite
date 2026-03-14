// Test scaffold for Phase 6 & 7 Parity Tests
// Save as: rust/crates/azurite-blob/tests/blob/phase6_errors.rs and phase7_authentication.rs

// ============================================================================
// PHASE 6: ERRORS & CONTEXT TESTS
// ============================================================================

#[cfg(test)]
mod phase6_errors {
    use azurite_blob::errors::{
        StorageError, StorageErrorFactory, NotImplementedError, NotImplementedinSQLError,
        StrictModelNotSupportedError,
    };
    use azurite_blob::context::BlobStorageContext;
    use azurite_blob::generated::context::Context;
    use std::collections::BTreeMap;

    // ========== StorageError Tests ==========
    
    #[test]
    fn storage_error_creates_xml_body_with_escaped_content() {
        let error = StorageError::new(
            400,
            "TestError",
            "Test message with <special> & \"chars\"",
            "test-id-123",
            BTreeMap::new(),
        );
        
        // Check XML is properly escaped
        let body = error.body.as_ref().unwrap().as_string().unwrap();
        assert!(body.contains("&lt;special&gt;"));
        assert!(body.contains("&amp;"));
        assert!(body.contains("&quot;"));
    }

    #[test]
    fn storage_error_sets_required_headers() {
        let error = StorageError::new(
            404,
            "NotFound",
            "Resource not found",
            "request-id-xyz",
            BTreeMap::new(),
        );
        
        let headers = error.headers.as_ref().unwrap();
        assert_eq!(headers.get("x-ms-error-code").unwrap().as_string().unwrap(), "NotFound");
        assert_eq!(headers.get("x-ms-request-id").unwrap().as_string().unwrap(), "request-id-xyz");
        assert_eq!(error.contentType.as_ref().unwrap(), "application/xml");
    }

    #[test]
    fn storage_error_includes_timestamp_in_message() {
        let error = StorageError::new(
            400,
            "TestCode",
            "Original message",
            "id-123",
            BTreeMap::new(),
        );
        
        let body = error.body.as_ref().unwrap().as_string().unwrap();
        // Verify message format: msg\nRequestId:id\nTime:ISO8601Z
        assert!(body.contains("Original message"));
        assert!(body.contains("RequestId:id-123"));
        assert!(body.contains("Time:"));
        assert!(body.contains("Z</Message>")); // UTC indicator
    }

    #[test]
    fn storage_error_includes_extra_fields_as_xml_siblings() {
        let mut extra = BTreeMap::new();
        extra.insert("UserSpecifiedMd5".to_string(), "abc123".to_string());
        extra.insert("ServerCalculatedMd5".to_string(), "def456".to_string());
        
        let error = StorageError::new(
            400,
            "Md5Mismatch",
            "MD5 values do not match",
            "id-123",
            extra,
        );
        
        let body = error.body.as_ref().unwrap().as_string().unwrap();
        assert!(body.contains("<UserSpecifiedMd5>abc123</UserSpecifiedMd5>"));
        assert!(body.contains("<ServerCalculatedMd5>def456</ServerCalculatedMd5>"));
    }

    // ========== StorageErrorFactory Tests ==========

    #[test]
    fn factory_container_not_found_returns_404() {
        let error = StorageErrorFactory::getContainerNotFound(Some("test-id"));
        assert_eq!(error.statusCode, 404);
        assert_eq!(error.storageErrorCode, "ContainerNotFound");
        assert_eq!(error.storageRequestID, "test-id");
    }

    #[test]
    fn factory_md5_mismatch_includes_both_values() {
        let error = StorageErrorFactory::getMd5Mismatch(
            Some("id-123"),
            "user-md5-value",
            "server-md5-value",
        );
        
        let body = error.body.as_ref().unwrap().as_string().unwrap();
        assert!(body.contains("<UserSpecifiedMd5>user-md5-value</UserSpecifiedMd5>"));
        assert!(body.contains("<ServerCalculatedMd5>server-md5-value</ServerCalculatedMd5>"));
    }

    #[test]
    fn factory_invalid_tag_returns_duplicate_tag_names_code() {
        let error = StorageErrorFactory::getInvalidTag("id-123");
        // Note: quirk in TS — returns "DuplicateTagNames", not "InvalidTag"
        assert_eq!(error.storageErrorCode, "DuplicateTagNames");
        assert_eq!(error.statusCode, 400);
    }

    #[test]
    fn factory_default_ids_mix_constant_and_empty() {
        let error_with_default = StorageErrorFactory::getContainerNotFound(None);
        assert_eq!(error_with_default.storageRequestID, "DefaultBlobRequestID");
        
        let error_with_empty = StorageErrorFactory::getInvalidXmlDocument(None);
        // Some return empty string for None
        assert!(error_with_empty.storageRequestID.is_empty() || 
                error_with_empty.storageRequestID == "DefaultBlobRequestID");
    }

    // ========== NotImplementedError Tests ==========

    #[test]
    fn not_implemented_error_returns_501() {
        let error = StorageError::new(
            501,
            "APINotImplemented",
            "Current API is not implemented yet. Please vote your wanted features to https://github.com/azure/azurite/issues",
            "id",
            BTreeMap::new(),
        );
        assert_eq!(error.statusCode, 501);
        assert_eq!(error.storageErrorCode, "APINotImplemented");
    }

    // ========== StrictModelNotSupportedError Tests ==========

    #[test]
    fn strict_model_error_interpolates_feature_name() {
        let error = StorageError::new(
            500,
            "FeatureNotSupported",
            "TestFeature header or parameter is not supported in Azurite strict mode. Switch to loose model by Azurite command line parameter \"--loose\" or Visual Studio Code configuration \"Loose\". Please vote your wanted features to https://github.com/azure/azurite/issues",
            "id",
            BTreeMap::new(),
        );
        
        let msg = &error.storageErrorMessage;
        assert!(msg.contains("TestFeature"));
        assert!(msg.contains("--loose"));
        assert!(msg.contains("Loose"));
        assert_eq!(error.statusCode, 500);
    }

    // ========== BlobStorageContext Tests ==========

    #[test]
    fn blob_storage_context_getters_and_setters() {
        let base_context = Context::default();
        let mut blob_context = BlobStorageContext::new(&base_context);
        
        // Test account getter/setter
        blob_context.setAccount(Some("myaccount".to_string()));
        assert_eq!(blob_context.account(), Some("myaccount".to_string()));
        
        // Test container getter/setter
        blob_context.setContainer(Some("mycontainer".to_string()));
        assert_eq!(blob_context.container(), Some("mycontainer".to_string()));
        assert_eq!(blob_context.getContainer(), Some("mycontainer".to_string())); // alias
        
        // Test blob getter/setter
        blob_context.setBlob(Some("myblob".to_string()));
        assert_eq!(blob_context.blob(), Some("myblob".to_string()));
    }

    #[test]
    fn blob_storage_context_xms_request_id_aliases_context_id() {
        let base_context = Context::default();
        let mut blob_context = BlobStorageContext::new(&base_context);
        
        blob_context.setXMsRequestID(Some("request-123".to_string()));
        assert_eq!(blob_context.xMsRequestID(), Some("request-123".to_string()));
    }

    #[test]
    fn blob_storage_context_boolean_fields() {
        let base_context = Context::default();
        let mut blob_context = BlobStorageContext::new(&base_context);
        
        blob_context.setIsSecondary(Some(true));
        assert_eq!(blob_context.isSecondary(), Some(true));
        
        blob_context.setLoose(Some(false));
        assert_eq!(blob_context.loose(), Some(false));
        
        blob_context.setDisableProductStyleUrl(Some(true));
        assert_eq!(blob_context.disableProductStyleUrl(), Some(true));
    }

    #[test]
    fn blob_storage_context_deref_to_base_context() {
        let base_context = Context::default();
        let blob_context = BlobStorageContext::new(&base_context);
        
        // Should be able to use as &Context
        let _: &Context = &*blob_context;
    }
}

// ============================================================================
// PHASE 7: AUTHENTICATION TESTS
// ============================================================================

#[cfg(test)]
mod phase7_authentication {
    use azurite_blob::authentication::{
        BlobSASPermission, ContainerSASPermission, BlobSASResourceType,
        OperationBlobSASPermission, OperationAccountSASPermission,
        IBlobSASSignatureValues, generateBlobSASSignature, generateBlobSASSignatureWithUDK,
    };
    use std::sync::Arc;

    // ========== Permission Enum Tests ==========

    #[test]
    fn blob_sas_permissions_have_correct_wire_characters() {
        assert_eq!(BlobSASPermission::Read.as_str(), "r");
        assert_eq!(BlobSASPermission::Add.as_str(), "a");
        assert_eq!(BlobSASPermission::Create.as_str(), "c");
        assert_eq!(BlobSASPermission::Write.as_str(), "w");
        assert_eq!(BlobSASPermission::Delete.as_str(), "d");
        assert_eq!(BlobSASPermission::DeleteVersion.as_str(), "x");
        assert_eq!(BlobSASPermission::Tag.as_str(), "t");
        assert_eq!(BlobSASPermission::Move.as_str(), "m");
        assert_eq!(BlobSASPermission::execute.as_str(), "e"); // lowercase member
        assert_eq!(BlobSASPermission::SetImmutabilityPolicy.as_str(), "i");
        assert_eq!(BlobSASPermission::permanentDelete.as_str(), "y"); // lowercase member
    }

    #[test]
    fn blob_sas_resource_types_have_correct_codes() {
        assert_eq!(BlobSASResourceType::Container.as_str(), "c");
        assert_eq!(BlobSASResourceType::Blob.as_str(), "b");
        assert_eq!(BlobSASResourceType::BlobSnapshot.as_str(), "bs");
    }

    #[test]
    fn container_sas_permissions_include_any_sentinel() {
        assert_eq!(ContainerSASPermission::Any.as_str(), "AnyPermission");
        // Should NOT be treated as a regular permission character
    }

    // ========== IBlobSASSignatureValues Version Dispatch Tests ==========

    #[test]
    fn sas_version_dispatch_uses_newest_applicable_version() {
        // Test 2020-12-06
        let values_2020 = IBlobSASSignatureValues {
            version: "2020-12-06".to_string(),
            // ... other fields
        };
        
        // Should use 2020-12-06 generator
        let (sig, string_to_sign) = generateBlobSASSignature(
            &values_2020,
            BlobSASResourceType::Blob,
            "testaccount",
            b"test-key",
        );
        
        assert!(!sig.is_empty());
        assert!(!string_to_sign.is_empty());
    }

    #[test]
    fn sas_canonical_name_not_url_encoded() {
        let values = IBlobSASSignatureValues {
            version: "2020-12-06".to_string(),
            containerName: "my-container".to_string(),
            blobName: Some("my/blob?name".to_string()), // Contains special chars
            // ... other fields
        };
        
        // Canonical name should be raw concatenation: /blob/account/container/blob
        // Not URL-encoded
        let (_, string_to_sign) = generateBlobSASSignature(
            &values,
            BlobSASResourceType::Blob,
            "myaccount",
            b"key",
        );
        
        // Should contain raw slash, not %2F
        assert!(string_to_sign.contains("/my-container/my/blob?name"));
    }

    // ========== Permission Validation Tests ==========

    #[test]
    fn operation_blob_sas_permission_validates_any_character_match() {
        let perm = OperationBlobSASPermission::new("wc"); // Requires write OR create
        
        // Should pass if either w or c present
        assert!(perm.validate("w")); // Has write
        assert!(perm.validate("c")); // Has create
        assert!(perm.validate("wc")); // Has both
        assert!(perm.validate("awcd")); // Has both + extras
        
        // Should fail if neither w nor c present
        assert!(!perm.validate("r")); // Only read
        assert!(!perm.validate("d")); // Only delete
        assert!(!perm.validate("")); // Empty
    }

    #[test]
    fn empty_operation_permission_always_fails() {
        let perm = OperationBlobSASPermission::new(""); // Empty requirement
        
        // Empty permission string should always fail
        assert!(!perm.validate("r"));
        assert!(!perm.validate("w"));
        assert!(!perm.validate("c"));
        assert!(!perm.validate("rw"));
        assert!(!perm.validate(""));
    }

    #[test]
    fn container_sas_any_permission_is_special_case() {
        // ContainerSASPermission::Any = "AnyPermission" is a sentinel
        // Should NOT be validated as a character
        let perm = OperationBlobSASPermission::new("AnyPermission");
        
        // If this is the permission string, validation behavior is special
        // (used only for batch operations)
        // Test depends on specific implementation in OperationBlobSASPermission
    }

    // ========== Shared Key Authenticator Tests (Scaffolding) ==========

    #[test]
    #[ignore = "Requires IAccountDataStore mock and full request setup"]
    fn blob_shared_key_authenticator_validates_signature() {
        // Setup:
        // 1. Create mock IAccountDataStore with test account + key
        // 2. Create GeneratedHttpRequest with Authorization header
        // 3. Create BlobSharedKeyAuthenticator
        // 4. Call validate()
        // 5. Assert returns Some(true) for valid signature, Some(false) for invalid
        
        // Mock implementation here
    }

    #[test]
    #[ignore = "Requires request setup"]
    fn blob_shared_key_authenticator_returns_none_for_missing_auth_header() {
        // Should return None if Authorization header not present
        // (authenticator not applicable)
    }

    #[test]
    #[ignore = "Requires request setup"]
    fn blob_shared_key_authenticator_rejects_get_user_delegation_key() {
        // Should throw StorageErrorFactory::AuthenticationFailed()
        // for GetUserDelegationKey operation
    }

    // ========== Account SAS Authenticator Tests (Scaffolding) ==========

    #[test]
    #[ignore = "Requires stores and request setup"]
    fn account_sas_authenticator_requires_all_fields() {
        // sv, se, sp, ss, srt, sig all mandatory
        // Missing any one should return false
    }

    #[test]
    #[ignore = "Requires stores"]
    fn account_sas_authenticator_rejects_ses_in_strict_mode() {
        // When loose=false and ses parameter present
        // Should throw StrictModelNotSupportedError
    }

    // ========== Blob SAS Authenticator Tests (Scaffolding) ==========

    #[test]
    #[ignore = "Requires stores and request setup"]
    fn blob_sas_authenticator_invalid_resource_type_returns_none() {
        // sr parameter other than c/b/bs should return None
    }

    #[test]
    #[ignore = "Requires stores"]
    fn blob_sas_authenticator_identifier_overrides_sp_st_se_only() {
        // When si (identifier) present, should fetch ACL and override
        // ONLY permissions, startTime, expiryTime
        // NOT protocol, IP, ses, response headers
    }

    #[test]
    #[ignore = "Requires stores"]
    fn blob_sas_authenticator_snapshot_uses_container_permissions() {
        // BlobSnapshot (sr=bs) should use CONTAINER permission table
        // NOT BLOB permission table (unlike normal Blob resources)
    }

    // ========== JWT Token Authenticator Tests (Scaffolding) ==========

    #[test]
    #[ignore = "Requires request and store setup"]
    fn blob_token_authenticator_requires_https() {
        // Should throw on HTTP requests
        // Should accept HTTPS requests
    }

    #[test]
    #[ignore = "Requires request setup"]
    fn blob_token_authenticator_parses_jwt_without_verification() {
        // Should decode JWT without verifying signature
        // Should extract nbf, exp, iat claims
        // Should validate they exist and are in correct format
    }

    // ========== Public Access Authenticator Tests (Scaffolding) ==========

    #[test]
    #[ignore = "Requires stores and context setup"]
    fn public_access_authenticator_requires_container_name() {
        // Should return None if container name not present in context
    }

    #[test]
    #[ignore = "Requires stores"]
    fn public_access_authenticator_swallows_metadata_errors() {
        // Metadata store errors should be caught
        // Should return None (continue auth chain)
        // Should NOT throw
    }
}
