#![allow(non_snake_case)]

#[cfg(test)]
mod phase12_middleware_config_tests {
    use azurite_blob::blob_configuration::BlobConfiguration;
    use azurite_blob::blob_environment::BlobEnvironment;
    use azurite_blob::i_blob_environment::IBlobEnvironment;
    use azurite_blob::middlewares::preflight_middleware_factory::PreflightMiddlewareFactory;
    use azurite_blob::utils::constants::*;
    use azurite_common::i_logger::ILogger;
    use std::sync::Arc;

    struct MockLogger;
    impl ILogger for MockLogger {
        fn info(&self, _message: &str, _contextID: Option<&str>) {}
        fn error(&self, _message: &str, _contextID: Option<&str>) {}
        fn warn(&self, _message: &str, _contextID: Option<&str>) {}
        fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
        fn debug(&self, _message: &str, _contextID: Option<&str>) {}
    }

    #[test]
    fn test_preflight_middleware_factory_creation() {
        let logger = Arc::new(MockLogger) as Arc<dyn ILogger + Send + Sync>;
        let _factory = PreflightMiddlewareFactory::new(logger.clone());

        // Factory should be created successfully
        // Note: Testing CORS logic would require public test methods or integration tests
        // with full middleware pipeline
    }

    #[test]
    fn test_blob_configuration_defaults() {
        // Test default configuration values match TS constants
        let config = BlobConfiguration::default();

        assert_eq!(config.base.host, DEFAULT_BLOB_SERVER_HOST_NAME);
        assert_eq!(config.base.port, DEFAULT_BLOB_LISTENING_PORT);
        assert_eq!(
            config.base.keepAliveTimeout,
            DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT
        );
        assert_eq!(config.base.enableAccessLog, DEFAULT_ENABLE_ACCESS_LOG);
        assert_eq!(config.base.enableDebugLog, DEFAULT_ENABLE_DEBUG_LOG);
        assert!(config.bugForBugCompatibility);
        assert_eq!(config.metadataDBPath, DEFAULT_BLOB_LOKI_DB_PATH);
        assert_eq!(config.extentDBPath, DEFAULT_BLOB_EXTENT_LOKI_DB_PATH);
    }

    #[test]
    fn test_blob_configuration_custom_values() {
        use azurite_common::persistence::i_extent_store::StoreDestinationArray;

        let persistence_array: StoreDestinationArray = vec![];
        let config = BlobConfiguration::new(
            "localhost".to_string(),
            11000,
            10,
            "/custom/metadata.json".to_string(),
            "/custom/extent.json".to_string(),
            persistence_array.clone(),
            false,
            None,
            false,
            None,
            true,
            true,
            String::new(),
            String::new(),
            String::new(),
            None,
            false,
            true,
            false,
            None,
        );

        assert_eq!(config.base.host, "localhost");
        assert_eq!(config.base.port, 11000);
        assert_eq!(config.base.keepAliveTimeout, 10);
        assert_eq!(config.metadataDBPath, "/custom/metadata.json");
        assert_eq!(config.extentDBPath, "/custom/extent.json");
        assert!(config.base.loose);
        assert!(config.base.skipApiVersionCheck);
        assert!(config.bugForBugCompatibility);
    }

    #[test]
    fn test_blob_environment_parse_defaults() {
        let args = vec!["azurite-blob".to_string()];
        let env = BlobEnvironment::new(args).expect("Failed to parse args");

        // Validate getter methods return default values
        assert_eq!(
            <BlobEnvironment as IBlobEnvironment>::blobHost(&env),
            Some(DEFAULT_BLOB_SERVER_HOST_NAME.to_string())
        );
        assert_eq!(
            <BlobEnvironment as IBlobEnvironment>::blobPort(&env),
            Some(DEFAULT_BLOB_LISTENING_PORT)
        );
    }

    #[test]
    fn test_blob_environment_parse_custom_host_port() {
        let args = vec![
            "azurite-blob".to_string(),
            "--blobHost".to_string(),
            "0.0.0.0".to_string(),
            "--blobPort".to_string(),
            "11000".to_string(),
        ];
        let env = BlobEnvironment::new(args).expect("Failed to parse args");

        assert_eq!(
            <BlobEnvironment as IBlobEnvironment>::blobHost(&env),
            Some("0.0.0.0".to_string())
        );
        assert_eq!(
            <BlobEnvironment as IBlobEnvironment>::blobPort(&env),
            Some(11000)
        );
    }

    #[test]
    fn test_blob_environment_parse_boolean_flags() {
        let args = vec![
            "azurite-blob".to_string(),
            "--silent".to_string(),
            "--loose".to_string(),
            "--skipApiVersionCheck".to_string(),
            "--inMemoryPersistence".to_string(),
        ];
        let env = BlobEnvironment::new(args).expect("Failed to parse args");

        // Use non-async methods from IBlobEnvironment
        assert!(<BlobEnvironment as IBlobEnvironment>::silent(&env));
        assert!(<BlobEnvironment as IBlobEnvironment>::loose(&env));
        assert!(<BlobEnvironment as IBlobEnvironment>::skipApiVersionCheck(
            &env
        ));
        assert!(<BlobEnvironment as IBlobEnvironment>::bugForBugCompatibility(&env));
        assert!(<BlobEnvironment as IBlobEnvironment>::inMemoryPersistence(
            &env
        ));
    }

    #[test]
    fn test_blob_environment_bug_for_bug_compatibility_can_be_disabled() {
        let defaults = BlobEnvironment::new(vec!["azurite-blob".to_string()])
            .expect("Failed to parse default args");
        let disabled = BlobEnvironment::new(vec![
            "azurite-blob".to_string(),
            "--disableBugForBugCompatibility".to_string(),
        ])
        .expect("Failed to parse compat args");

        assert!(<BlobEnvironment as IBlobEnvironment>::bugForBugCompatibility(&defaults));
        assert!(!<BlobEnvironment as IBlobEnvironment>::bugForBugCompatibility(&disabled));
    }

    #[tokio::test]
    async fn test_blob_environment_location_path() {
        let args = vec![
            "azurite-blob".to_string(),
            "--location".to_string(),
            "/tmp/azurite".to_string(),
        ];
        let env = BlobEnvironment::new(args).expect("Failed to parse args");

        // location() is async, so use tokio::test
        let location = <BlobEnvironment as IBlobEnvironment>::location(&env).await;
        assert!(location.is_ok());
        assert_eq!(location.unwrap(), "/tmp/azurite".to_string());
    }

    #[test]
    fn test_constants_values() {
        // Verify critical constants match TS values
        assert_eq!(VERSION, "3.35.0");
        assert_eq!(BLOB_API_VERSION, "2025-11-05");
        assert_eq!(DEFAULT_BLOB_SERVER_HOST_NAME, "127.0.0.1");
        assert_eq!(DEFAULT_BLOB_LISTENING_PORT, 10000);
        assert_eq!(DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT, 5);
        assert_eq!(DEFAULT_LIST_BLOBS_MAX_RESULTS, 5000);
        assert_eq!(DEFAULT_LIST_CONTAINERS_MAX_RESULTS, 5000);
        assert_eq!(DEFAULT_BLOB_LOKI_DB_PATH, "__azurite_db_blob__.json");
        assert_eq!(
            DEFAULT_BLOB_EXTENT_LOKI_DB_PATH,
            "__azurite_db_blob_extent__.json"
        );
        assert_eq!(DEFAULT_BLOB_PERSISTENCE_PATH, "__blobstorage__");
        assert_eq!(DEFAULT_GC_INTERVAL_MS, 10 * 60 * 1000);
        assert_eq!(EMULATOR_ACCOUNT_NAME, "devstoreaccount1");
        assert_eq!(
            EMULATOR_ACCOUNT_KEY_STR,
            "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
        );
    }

    #[test]
    fn test_header_constants() {
        // Verify header constant names match TS
        assert_eq!(HeaderConstants::AUTHORIZATION, "authorization");
        assert_eq!(HeaderConstants::CONTENT_TYPE, "content-type");
        assert_eq!(HeaderConstants::CONTENT_LENGTH, "content-length");
        assert_eq!(HeaderConstants::ORIGIN, "origin");
        assert_eq!(
            HeaderConstants::ACCESS_CONTROL_ALLOW_ORIGIN,
            "Access-Control-Allow-Origin"
        );
        assert_eq!(
            HeaderConstants::ACCESS_CONTROL_ALLOW_METHODS,
            "Access-Control-Allow-Methods"
        );
        assert_eq!(
            HeaderConstants::ACCESS_CONTROL_ALLOW_HEADERS,
            "Access-Control-Allow-Headers"
        );
        assert_eq!(
            HeaderConstants::ACCESS_CONTROL_MAX_AGE,
            "Access-Control-Max-Age"
        );
        assert_eq!(
            HeaderConstants::ACCESS_CONTROL_REQUEST_METHOD,
            "access-control-request-method"
        );
    }

    #[test]
    fn test_method_constants() {
        assert_eq!(MethodConstants::OPTIONS, "OPTIONS");
    }

    #[test]
    fn test_valid_api_versions() {
        // Verify ValidAPIVersions array contains expected versions
        assert!(ValidAPIVersions.contains(&"2025-11-05"));
        assert!(ValidAPIVersions.contains(&"2024-11-04"));
        assert!(ValidAPIVersions.contains(&"2023-11-03"));
        assert!(ValidAPIVersions.contains(&"2021-12-02"));
        assert!(ValidAPIVersions.contains(&"2020-10-02"));
        assert_eq!(ValidAPIVersions.len(), 45);
    }
}
