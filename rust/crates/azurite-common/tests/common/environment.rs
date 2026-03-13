use async_trait::async_trait;
use azurite_common::{i_environment::IEnvironment, storage_error::StorageError};
use pretty_assertions::assert_eq;

#[derive(Clone, Default)]
struct EnvironmentFixture {
    blob_host: Option<String>,
    blob_port: Option<u16>,
    blob_keep_alive_timeout: Option<u64>,
    queue_host: Option<String>,
    queue_port: Option<u16>,
    queue_keep_alive_timeout: Option<u64>,
    table_host: Option<String>,
    table_port: Option<u16>,
    table_keep_alive_timeout: Option<u64>,
    location: String,
    silent: bool,
    loose: bool,
    skip_api_version_check: bool,
    disable_product_style_url: bool,
    cert: Option<String>,
    key: Option<String>,
    pwd: Option<String>,
    debug: Option<String>,
    oauth: Option<String>,
    in_memory_persistence: bool,
    extent_memory_limit: Option<f64>,
    disable_telemetry: bool,
}

#[allow(non_snake_case)]
#[async_trait]
impl IEnvironment for EnvironmentFixture {
    fn blobHost(&self) -> Option<String> {
        self.blob_host.clone()
    }

    fn blobPort(&self) -> Option<u16> {
        self.blob_port
    }

    fn blobKeepAliveTimeout(&self) -> Option<u64> {
        self.blob_keep_alive_timeout
    }

    fn queueHost(&self) -> Option<String> {
        self.queue_host.clone()
    }

    fn queuePort(&self) -> Option<u16> {
        self.queue_port
    }

    fn queueKeepAliveTimeout(&self) -> Option<u64> {
        self.queue_keep_alive_timeout
    }

    fn tableHost(&self) -> Option<String> {
        self.table_host.clone()
    }

    fn tablePort(&self) -> Option<u16> {
        self.table_port
    }

    fn tableKeepAliveTimeout(&self) -> Option<u64> {
        self.table_keep_alive_timeout
    }

    async fn location(&self) -> Result<String, StorageError> {
        Ok(self.location.clone())
    }

    fn silent(&self) -> bool {
        self.silent
    }

    fn loose(&self) -> bool {
        self.loose
    }

    fn skipApiVersionCheck(&self) -> bool {
        self.skip_api_version_check
    }

    fn disableProductStyleUrl(&self) -> bool {
        self.disable_product_style_url
    }

    fn cert(&self) -> Option<String> {
        self.cert.clone()
    }

    fn key(&self) -> Option<String> {
        self.key.clone()
    }

    fn pwd(&self) -> Option<String> {
        self.pwd.clone()
    }

    async fn debug(&self) -> Result<Option<String>, StorageError> {
        Ok(self.debug.clone())
    }

    fn oauth(&self) -> Option<String> {
        self.oauth.clone()
    }

    fn inMemoryPersistence(&self) -> bool {
        self.in_memory_persistence
    }

    fn extentMemoryLimit(&self) -> Option<f64> {
        self.extent_memory_limit
    }

    fn disableTelemetry(&self) -> bool {
        self.disable_telemetry
    }
}

fn assert_environment_contract<T: IEnvironment>(_environment: &T) {}

#[tokio::test]
async fn environment_contract_exposes_blob_queue_and_table_settings() {
    let environment = EnvironmentFixture {
        blob_host: Some(String::new()),
        blob_port: Some(0),
        blob_keep_alive_timeout: Some(0),
        queue_host: Some("queue-host".to_owned()),
        queue_port: Some(10001),
        queue_keep_alive_timeout: Some(5),
        table_host: Some("table-host".to_owned()),
        table_port: Some(10002),
        table_keep_alive_timeout: Some(5),
        location: "workspace".to_owned(),
        silent: true,
        loose: false,
        skip_api_version_check: true,
        disable_product_style_url: true,
        cert: Some(String::new()),
        key: Some(String::new()),
        pwd: None,
        debug: Some(String::new()),
        oauth: Some(String::new()),
        in_memory_persistence: true,
        extent_memory_limit: Some(0.0),
        disable_telemetry: true,
    };

    assert_environment_contract(&environment);
    assert_eq!(environment.blobHost(), Some(String::new()));
    assert_eq!(environment.blobPort(), Some(0));
    assert_eq!(environment.blobKeepAliveTimeout(), Some(0));
    assert_eq!(environment.queueHost(), Some("queue-host".to_owned()));
    assert_eq!(environment.queuePort(), Some(10001));
    assert_eq!(environment.queueKeepAliveTimeout(), Some(5));
    assert_eq!(environment.tableHost(), Some("table-host".to_owned()));
    assert_eq!(environment.tablePort(), Some(10002));
    assert_eq!(environment.tableKeepAliveTimeout(), Some(5));
    assert_eq!(environment.location().await.unwrap(), "workspace");
    assert!(environment.silent());
    assert!(!environment.loose());
    assert!(environment.skipApiVersionCheck());
    assert!(environment.disableProductStyleUrl());
    assert_eq!(environment.cert(), Some(String::new()));
    assert_eq!(environment.key(), Some(String::new()));
    assert_eq!(environment.pwd(), None);
    assert_eq!(environment.debug().await.unwrap(), Some(String::new()));
    assert_eq!(environment.oauth(), Some(String::new()));
    assert!(environment.inMemoryPersistence());
    assert_eq!(environment.extentMemoryLimit(), Some(0.0));
    assert!(environment.disableTelemetry());
}

#[tokio::test]
async fn environment_debug_value_supports_string_and_none_variants() {
    let string_environment = EnvironmentFixture {
        location: "/tmp/azurite".to_owned(),
        debug: Some("debug.log".to_owned()),
        ..Default::default()
    };
    let none_environment = EnvironmentFixture {
        location: "/tmp/azurite".to_owned(),
        ..Default::default()
    };

    assert_eq!(
        string_environment.debug().await.unwrap(),
        Some("debug.log".to_owned())
    );
    assert_eq!(none_environment.debug().await.unwrap(), None);
}
