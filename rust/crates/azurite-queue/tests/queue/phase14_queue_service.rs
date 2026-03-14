#[cfg(test)]
mod phase14_queue_auth_tests {
    use azurite_common::authentication::{
        AccountSASPermission, AccountSASResourceType, AccountSASService,
    };
    use azurite_queue::authentication::operation_account_sas_permission::OperationAccountSASPermission;

    #[test]
    fn test_operation_account_sas_permission_creation() {
        let perm = OperationAccountSASPermission::new("q", "s", "r");

        assert_eq!(perm.service, "q");
        assert_eq!(perm.resourceType, "s");
        assert_eq!(perm.permission, "r");
    }

    #[test]
    fn test_validate_services() {
        let perm = OperationAccountSASPermission::new("q", "s", "r");

        // Service 'q' should be in the services string
        assert!(perm.validateServices("bqtf"));
        assert!(perm.validateServices("q"));
        assert!(!perm.validateServices("btf"));
    }

    #[test]
    fn test_validate_resource_types() {
        let perm = OperationAccountSASPermission::new("q", "so", "r");

        // At least one of 's' or 'o' should be in resourceTypes
        assert!(perm.validateResourceTypes("sco"));
        assert!(perm.validateResourceTypes("s"));
        assert!(perm.validateResourceTypes("o"));
        assert!(perm.validateResourceTypes("co"));
        assert!(!perm.validateResourceTypes("c"));
    }

    #[test]
    fn test_validate_permissions() {
        let perm = OperationAccountSASPermission::new("q", "s", "rwdl");

        // At least one of 'r', 'w', 'd', or 'l' should be in permissions
        assert!(perm.validatePermissions("rwdxlacuptfiy"));
        assert!(perm.validatePermissions("r"));
        assert!(perm.validatePermissions("w"));
        assert!(perm.validatePermissions("d"));
        assert!(perm.validatePermissions("l"));
        assert!(!perm.validatePermissions("acuptfiy"));
    }

    #[test]
    fn test_validate_all_conditions() {
        let perm = OperationAccountSASPermission::new("q", "s", "r");

        // All three conditions must match
        assert!(perm.validate("bqtf", "sco", "rwdxlacuptfiy"));
        assert!(!perm.validate("btf", "sco", "rwdxlacuptfiy")); // service mismatch
        assert!(!perm.validate("bqtf", "co", "rwdxlacuptfiy")); // resourceType mismatch
        assert!(!perm.validate("bqtf", "sco", "wdxlacuptfiy")); // permission mismatch
    }

    #[test]
    fn test_account_sas_service_queue_value() {
        // Verify Queue service character matches TS
        assert_eq!(AccountSASService::Queue.as_str(), "q");
    }

    #[test]
    fn test_account_sas_resource_types() {
        // Verify resource type characters match TS
        assert_eq!(AccountSASResourceType::Service.as_str(), "s");
        assert_eq!(AccountSASResourceType::Container.as_str(), "c");
        assert_eq!(AccountSASResourceType::Object.as_str(), "o");
    }

    #[test]
    fn test_account_sas_permissions() {
        // Verify permission characters match TS
        assert_eq!(AccountSASPermission::Read.as_str(), "r");
        assert_eq!(AccountSASPermission::Write.as_str(), "w");
        assert_eq!(AccountSASPermission::Delete.as_str(), "d");
        assert_eq!(AccountSASPermission::List.as_str(), "l");
        assert_eq!(AccountSASPermission::Add.as_str(), "a");
        assert_eq!(AccountSASPermission::Create.as_str(), "c");
        assert_eq!(AccountSASPermission::Update.as_str(), "u");
        assert_eq!(AccountSASPermission::Process.as_str(), "p");
    }
}

#[cfg(test)]
mod phase14_queue_metadata_store_tests {
    use azurite_queue::persistence::loki_queue_metadata_store::LokiQueueMetadataStore;
    use std::path::PathBuf;

    #[test]
    fn test_loki_queue_metadata_store_creation() {
        let store = LokiQueueMetadataStore::new(PathBuf::from("__azurite_db_queue__.json"), false);

        assert_eq!(store.lokiDBPath, PathBuf::from("__azurite_db_queue__.json"));
    }

    #[test]
    fn test_loki_queue_metadata_store_in_memory() {
        let _store = LokiQueueMetadataStore::new(PathBuf::from(":memory:"), true);

        // In-memory store created successfully
    }

    // Note: Full async tests for queue operations (enqueue/dequeue/peek/clear)
    // would require async test runtime and mock contexts
}

#[cfg(test)]
mod phase14_queue_errors_tests {
    use azurite_queue::errors::StorageErrorFactory;
    use std::collections::BTreeMap;

    #[test]
    fn test_storage_error_not_implement() {
        let err = StorageErrorFactory::notImplement(Some("test-context-id"));

        assert_eq!(err.statusCode, 500);
        assert_eq!(err.storageErrorCode, "functionNotImplement");
        assert_eq!(err.storageErrorMessage, "No function.");
        assert_eq!(err.storageRequestID, "test-context-id");
    }

    #[test]
    fn test_storage_error_internal_error() {
        let err = StorageErrorFactory::InternalError(Some("test-context-id"));

        assert_eq!(err.statusCode, 500);
        assert_eq!(err.storageErrorCode, "InternalError");
        assert!(err.storageErrorMessage.contains("internal error"));
        assert_eq!(err.storageRequestID, "test-context-id");
    }

    #[test]
    fn test_storage_error_invalid_header_value() {
        let mut details = BTreeMap::new();
        details.insert("HeaderName".to_string(), "x-ms-invalid".to_string());

        let err =
            StorageErrorFactory::getInvalidHeaderValue(Some("test-context-id"), Some(details));

        assert_eq!(err.statusCode, 400);
        assert_eq!(err.storageErrorCode, "InvalidHeaderValue");
        assert_eq!(err.storageRequestID, "test-context-id");
    }

    #[test]
    fn test_storage_error_invalid_api_version() {
        let err =
            StorageErrorFactory::getInvalidAPIVersion(Some("test-context-id"), Some("2018-01-01"));

        assert_eq!(err.statusCode, 400);
        assert_eq!(err.storageErrorCode, "InvalidHeaderValue");
        assert!(err.storageErrorMessage.contains("2018-01-01"));
        assert!(err.storageErrorMessage.contains("not supported"));
    }

    #[test]
    fn test_storage_error_cors_preflight_failure() {
        let mut details = BTreeMap::new();
        details.insert("MessageDetails".to_string(), "No matching rule".to_string());

        let err = StorageErrorFactory::corsPreflightFailure(Some("test-context-id"), Some(details));

        assert_eq!(err.statusCode, 403);
        assert_eq!(err.storageErrorCode, "CorsPreflightFailure");
        assert!(err.storageErrorMessage.contains("CORS"));
    }

    #[test]
    fn test_storage_error_invalid_uri() {
        let err = StorageErrorFactory::getInvalidUri(Some("test-context-id"), None);

        assert_eq!(err.statusCode, 400);
        assert_eq!(err.storageErrorCode, "InvalidUri");
        assert!(err.storageErrorMessage.contains("invalid characters"));
    }

    #[test]
    fn test_storage_error_authentication_failed() {
        let err = StorageErrorFactory::getAuthenticationFailed(
            Some("test-context-id"),
            "Signature mismatch",
        );

        assert_eq!(err.statusCode, 403);
        assert_eq!(err.storageErrorCode, "AuthenticationFailed");
        assert!(err.storageErrorMessage.contains("authenticate"));
    }

    #[test]
    fn test_storage_error_invalid_authentication_info() {
        let err = StorageErrorFactory::getInvalidAuthenticationInfo(Some("test-context-id"));

        assert_eq!(err.statusCode, 400);
        assert_eq!(err.storageErrorCode, "InvalidAuthenticationInfo");
        assert!(err.storageErrorMessage.contains("Authorization"));
    }

    #[test]
    fn test_storage_error_authorization_failure() {
        let err = StorageErrorFactory::getAuthorizationFailure(Some("test-context-id"));

        assert_eq!(err.statusCode, 403);
        assert_eq!(err.storageErrorCode, "AuthenticationFailed");
    }

    #[test]
    fn test_storage_error_invalid_operation() {
        let err = StorageErrorFactory::getInvalidOperation(
            "test-context-id",
            Some("Operation not allowed"),
        );

        assert_eq!(err.statusCode, 400);
        assert_eq!(err.storageErrorCode, "InvalidOperation");
        assert_eq!(err.storageErrorMessage, "Operation not allowed");
    }

    #[test]
    fn test_storage_error_resource_not_found() {
        let err = StorageErrorFactory::ResourceNotFound("test-context-id");

        assert_eq!(err.statusCode, 404);
        assert_eq!(err.storageErrorCode, "ResourceNotFound");
        assert!(err.storageErrorMessage.contains("does not exist"));
    }

    #[test]
    fn test_storage_error_default_context_id() {
        let err = StorageErrorFactory::notImplement(None);

        // Should use default ID when None provided
        assert_eq!(err.storageRequestID, "DefaultID");
    }

    #[test]
    fn test_storage_error_invalid_cors_header_value() {
        let mut details = BTreeMap::new();
        details.insert(
            "MessageDetails".to_string(),
            "Missing Origin header".to_string(),
        );

        let err =
            StorageErrorFactory::getInvalidCorsHeaderValue(Some("test-context-id"), Some(details));

        assert_eq!(err.statusCode, 400);
        assert_eq!(err.storageErrorCode, "InvalidHeaderValue");
        assert!(err.storageErrorMessage.contains("CORS"));
    }
}

#[cfg(test)]
mod phase14_queue_constants_tests {
    use azurite_common::utils::constants::EMULATOR_ACCOUNT_NAME;
    use azurite_queue::utils::constants::*;

    #[test]
    fn test_queue_api_version() {
        assert_eq!(QUEUE_API_VERSION, "2025-11-05");
    }

    #[test]
    fn test_default_queue_server_settings() {
        assert_eq!(DEFAULT_QUEUE_SERVER_HOST_NAME, "127.0.0.1");
        assert_eq!(DEFAULT_QUEUE_LISTENING_PORT, 10001);
    }

    #[test]
    fn test_queue_database_paths() {
        assert_eq!(DEFAULT_QUEUE_LOKI_DB_PATH, "__azurite_db_queue__.json");
        assert_eq!(
            DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH,
            "__azurite_db_queue_extent__.json"
        );
        assert_eq!(DEFAULT_QUEUE_PERSISTENCE_PATH, "__queuestorage__");
    }

    #[test]
    fn test_queue_message_limits() {
        assert_eq!(MESSAGETEXT_LENGTH_MAX, 64 * 1024);
        assert_eq!(MESSAGETTL_MIN, 1);
        assert_eq!(DEFAULT_MESSAGETTL, 7 * 24 * 3600);
        assert_eq!(DEQUEUE_NUMOFMESSAGES_MIN, 1);
        assert_eq!(DEQUEUE_NUMOFMESSAGES_MAX, 32);
    }

    #[test]
    fn test_visibility_timeout_limits() {
        assert_eq!(DEFAULT_DEQUEUE_VISIBILITYTIMEOUT, 30);
        assert_eq!(DEQUEUE_VISIBILITYTIMEOUT_MIN, 1);
        assert_eq!(DEQUEUE_VISIBILITYTIMEOUT_MAX, 7 * 24 * 3600);
        assert_eq!(ENQUEUE_VISIBILITYTIMEOUT_MIN, 0);
        assert_eq!(ENQUEUE_VISIBILITYTIMEOUT_MAX, 7 * 24 * 3600);
    }

    #[test]
    fn test_queue_list_limits() {
        assert_eq!(LIST_QUEUE_MAXRESULTS_MIN, 1);
        assert_eq!(LIST_QUEUE_MAXRESULTS_MAX, 2147483647);
    }

    #[test]
    fn test_queue_status_code() {
        assert_eq!(QUEUE_STATUSCODE::CREATED as u16, 201);
        assert_eq!(QUEUE_STATUSCODE::NOCONTENT as u16, 204);
    }

    #[test]
    fn test_emulator_account_constants() {
        assert_eq!(EMULATOR_ACCOUNT_NAME, "devstoreaccount1");
    }

    #[test]
    fn test_queue_header_constants() {
        assert_eq!(HeaderConstants.AUTHORIZATION, "authorization");
        assert_eq!(HeaderConstants.CONTENT_TYPE, "content-type");
        assert_eq!(HeaderConstants.X_MS_VERSION, "x-ms-version");
    }
}

#[cfg(test)]
mod phase14_queue_sas_tests {
    use azurite_queue::authentication::queue_sas_permissions::QueueSASPermission;

    #[test]
    fn test_queue_sas_permission_values() {
        // Verify permission string values match TS
        assert_eq!(QueueSASPermission::Read.as_str(), "r");
        assert_eq!(QueueSASPermission::Add.as_str(), "a");
        assert_eq!(QueueSASPermission::Update.as_str(), "u");
        assert_eq!(QueueSASPermission::Process.as_str(), "p");
    }

    // Note: QueueSASPermission is a simple enum without parse/toString methods
    // Those would need to be added if required for parity with TS
}
