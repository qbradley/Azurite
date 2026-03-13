use std::{
    collections::HashMap,
    env, fs,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::{i_logger::ILogger, logger::logger};

const DEFAULT_BLOB_SERVER_HOST_NAME: &str = "127.0.0.1";
const DEFAULT_BLOB_LISTENING_PORT: u16 = 10000;
const DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT: u64 = 5;
const DEFAULT_QUEUE_LISTENING_PORT: u16 = 10001;
const DEFAULT_TABLE_LISTENING_PORT: u16 = 10002;

#[derive(Clone, Debug, Default)]
pub struct WorkspaceConfiguration {
    pub values: HashMap<String, Value>,
}

impl WorkspaceConfiguration {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }
}

#[derive(Clone, Debug)]
pub enum TelemetryEnvironment {
    WorkspaceConfiguration(WorkspaceConfiguration),
    Other,
}

impl Default for TelemetryEnvironment {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TelemetryServiceType {
    Blob,
    Queue,
    Table,
    #[default]
    Unknown,
}

impl TelemetryServiceType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Blob => "Blob",
            Self::Queue => "Queue",
            Self::Table => "Table",
            Self::Unknown => "",
        }
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct TelemetryRequest {
    pub headers: HashMap<String, String>,
    pub queries: HashMap<String, String>,
    pub method: String,
    pub endpoint: String,
}

impl TelemetryRequest {
    pub fn getHeader(&self, header: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(header))
            .map(|(_, value)| value.as_str())
    }

    pub fn getQuery(&self, name: &str) -> Option<&str> {
        self.queries.get(name).map(String::as_str)
    }

    pub fn getMethod(&self) -> &str {
        &self.method
    }

    pub fn getEndpoint(&self) -> &str {
        &self.endpoint
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct TelemetryResponse {
    pub headers: HashMap<String, String>,
    pub statusCode: Option<u16>,
}

impl TelemetryResponse {
    pub fn getHeader(&self, header: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(header))
            .map(|(_, value)| value.as_str())
    }

    pub fn getStatusCode(&self) -> Option<u16> {
        self.statusCode
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct TelemetryContext {
    pub serviceType: TelemetryServiceType,
    pub operation: Option<String>,
    pub request: Option<TelemetryRequest>,
    pub response: Option<TelemetryResponse>,
    pub startTime: Option<DateTime<Utc>>,
    pub contextId: Option<String>,
    pub contextID: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct EnvelopeTelemetry {
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub struct TelemetryClientContext {
    pub cloudRole: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct TelemetryClientConfig {
    pub samplingPercentage: f64,
    pub maxBatchSize: usize,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct TrackedRequest {
    pub name: String,
    pub url: String,
    pub duration: i64,
    pub resultCode: u16,
    pub success: bool,
    pub id: Option<String>,
    pub source: Option<String>,
    pub properties: HashMap<String, String>,
    pub contextObjects: HashMap<String, String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct TrackedEvent {
    pub name: String,
    pub properties: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct TelemetryClient {
    pub connectionString: String,
    pub context: TelemetryClientContext,
    pub config: TelemetryClientConfig,
    processors: Vec<fn(&mut EnvelopeTelemetry) -> bool>,
}

impl TelemetryClient {
    fn new(connectionString: &str) -> Self {
        Self {
            connectionString: connectionString.to_string(),
            context: TelemetryClientContext::default(),
            config: TelemetryClientConfig::default(),
            processors: Vec::new(),
        }
    }

    pub fn addTelemetryProcessor(&mut self, processor: fn(&mut EnvelopeTelemetry) -> bool) {
        self.processors.push(processor);
    }

    pub fn trackRequest(&self, request: TrackedRequest) {
        let mut envelope = EnvelopeTelemetry {
            tags: HashMap::from([
                (
                    "ai.cloud.roleInstance".to_string(),
                    self.context.cloudRole.clone(),
                ),
                ("ai.operation.name".to_string(), request.name),
            ]),
        };
        for processor in &self.processors {
            if !processor(&mut envelope) {
                break;
            }
        }
    }

    pub fn trackEvent(&self, event: TrackedEvent) {
        let mut envelope = EnvelopeTelemetry {
            tags: HashMap::from([
                (
                    "ai.cloud.roleInstance".to_string(),
                    self.context.cloudRole.clone(),
                ),
                ("ai.operation.name".to_string(), event.name),
            ]),
        };
        for processor in &self.processors {
            if !processor(&mut envelope) {
                break;
            }
        }
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
struct TelemetryState {
    eventClient: Option<TelemetryClient>,
    requestClient: Option<TelemetryClient>,
    enableTelemetry: bool,
    location: String,
    configFileName: String,
    _totalIngressSize: u64,
    _totalEgressSize: u64,
    _totalBlobRequestCount: u64,
    _totalQueueRequestCount: u64,
    _totalTableRequestCount: u64,
    sessionID: String,
    instanceID: String,
    initialized: bool,
    env: Option<TelemetryEnvironment>,
    isVSC: bool,
    requestCollectPercentage: f64,
    cloudRole: String,
    requestMaxBatchSize: usize,
}

impl Default for TelemetryState {
    fn default() -> Self {
        Self {
            eventClient: None,
            requestClient: None,
            enableTelemetry: true,
            location: String::new(),
            configFileName: "AzuriteConfig".to_string(),
            _totalIngressSize: 0,
            _totalEgressSize: 0,
            _totalBlobRequestCount: 0,
            _totalQueueRequestCount: 0,
            _totalTableRequestCount: 0,
            sessionID: Uuid::new_v4().to_string(),
            instanceID: String::new(),
            initialized: false,
            env: None,
            isVSC: false,
            requestCollectPercentage: 1f64,
            cloudRole: "Azurite_V1.0".to_string(),
            requestMaxBatchSize: 0,
        }
    }
}

static TELEMETRY_STATE: LazyLock<Mutex<TelemetryState>> =
    LazyLock::new(|| Mutex::new(TelemetryState::default()));

pub struct AzuriteTelemetryClient;

impl AzuriteTelemetryClient {
    pub fn init(
        location: String,
        enableTelemetry: bool,
        env: Option<TelemetryEnvironment>,
        isVSC: bool,
    ) {
        let mut state = TELEMETRY_STATE.lock().unwrap();
        state.enableTelemetry = enableTelemetry;

        if enableTelemetry != false && state.initialized != true {
            state.isVSC = isVSC;
            state.location = location;
            state.instanceID =
                Self::GetInstanceID(state.location.clone(), state.configFileName.clone(), false);
            logger.info(
                &format!(
                    "InstaceID {}, SessionID {}.",
                    state.instanceID, state.sessionID
                ),
                None,
            );

            state.env = env;
            if state.enableTelemetry && state.eventClient.is_none() {
                state.eventClient = Some(Self::createAppInsigntClient(
                    state.cloudRole.clone(),
                    Some(100f64),
                    Some(0),
                ));
            }
            if state.enableTelemetry && state.requestClient.is_none() {
                state.requestClient = Some(Self::createAppInsigntClient(
                    state.cloudRole.clone(),
                    Some(state.requestCollectPercentage),
                    Some(state.requestMaxBatchSize),
                ));
            }

            state.initialized = true;
            logger.info("Telemetry initialize successfully.", None);
        } else {
            logger.info(
                &format!(
                    "Don't need initialize Telemetry. enableTelemetry: {enableTelemetry}, initialized: {}",
                    state.initialized
                ),
                None,
            );
        }
    }

    #[allow(non_snake_case)]
    fn removeRoleInstance(envelope: &mut EnvelopeTelemetry) -> bool {
        if let Some(role_instance) = envelope.tags.get("ai.cloud.roleInstance").cloned() {
            let mut hash = Sha256::new();
            hash.update(role_instance.as_bytes());
            envelope.tags.insert(
                "ai.cloud.roleInstance".to_string(),
                hash.finalize()
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect(),
            );
        }
        envelope
            .tags
            .insert("ai.operation.name".to_string(), String::new());
        true
    }

    #[allow(non_snake_case)]
    pub fn createAppInsigntClient(
        cloudRole: String,
        samplingPercentage: Option<f64>,
        maxBatchSize: Option<usize>,
    ) -> TelemetryClient {
        let ConnectionString = "InstrumentationKey=feb4ae36-1db7-4808-abaa-e0b94996d665;IngestionEndpoint=https://eastus2-3.in.applicationinsights.azure.com/;LiveEndpoint=https://eastus2.livediagnostics.monitor.azure.com/;ApplicationId=9af871a3-75b5-417c-8a2f-7f2eb1ba6a6c";
        let mut telemetryClient = TelemetryClient::new(ConnectionString);
        telemetryClient.addTelemetryProcessor(Self::removeRoleInstance);
        telemetryClient.context.cloudRole = cloudRole;
        telemetryClient.config.samplingPercentage = samplingPercentage.unwrap_or(1f64);
        if let Some(maxBatchSize) = maxBatchSize {
            telemetryClient.config.maxBatchSize = maxBatchSize;
        }
        telemetryClient
    }

    #[allow(non_snake_case)]
    pub fn TraceRequest(context: TelemetryContext) {
        let mut state = TELEMETRY_STATE.lock().unwrap();
        if !state.enableTelemetry {
            return;
        }
        let Some(client) = state.requestClient.clone() else {
            return;
        };

        let serviceType = context.serviceType.as_str().to_string();
        let mut totalReqs = 0u64;
        let mut reqName = String::new();
        match context.serviceType {
            TelemetryServiceType::Blob => {
                state._totalBlobRequestCount += 1;
                totalReqs = state._totalBlobRequestCount;
                reqName = format!(
                    "B_{}",
                    context.operation.clone().unwrap_or_else(|| "0".to_string())
                );
            }
            TelemetryServiceType::Queue => {
                state._totalQueueRequestCount += 1;
                totalReqs = state._totalQueueRequestCount;
                reqName = format!(
                    "Q_{}",
                    context.operation.clone().unwrap_or_else(|| "0".to_string())
                );
            }
            TelemetryServiceType::Table => {
                state._totalTableRequestCount += 1;
                totalReqs = state._totalTableRequestCount;
                reqName = format!(
                    "T_{}",
                    context.operation.clone().unwrap_or_else(|| "0".to_string())
                );
            }
            TelemetryServiceType::Unknown => {}
        }

        let mut requestProperties = HashMap::from([
            (
                "apiVersion".to_string(),
                format!(
                    "v{}",
                    context
                        .request
                        .as_ref()
                        .and_then(|request| request.getHeader("x-ms-version"))
                        .unwrap_or("undefined")
                ),
            ),
            (
                "authorization".to_string(),
                context
                    .request
                    .as_ref()
                    .map(|request| {
                        Self::GetRequestAuthentication(
                            request.getHeader("authorization"),
                            request.getQuery("sig"),
                        )
                    })
                    .unwrap_or_default(),
            ),
            ("instanceID".to_string(), state.instanceID.clone()),
            ("sessionID".to_string(), state.sessionID.clone()),
            ("ReqNo".to_string(), totalReqs.to_string()),
        ]);

        if let Some(ingress) = context
            .request
            .as_ref()
            .and_then(|request| request.getHeader("content-length"))
        {
            if ingress.parse::<u64>().unwrap_or(0) > 0 {
                requestProperties.insert("ingress".to_string(), ingress.to_string());
                state._totalIngressSize += ingress.parse::<u64>().unwrap_or(0);
            }
        }

        if context
            .request
            .as_ref()
            .map(|request| request.getMethod() != "HEAD")
            .unwrap_or(true)
        {
            if let Some(egress) = context
                .response
                .as_ref()
                .and_then(|response| response.getHeader("content-length"))
            {
                if egress.parse::<u64>().unwrap_or(0) > 0 {
                    requestProperties.insert("egress".to_string(), egress.to_string());
                    state._totalEgressSize += egress.parse::<u64>().unwrap_or(0);
                }
            }
        }

        let resultCode = context
            .response
            .as_ref()
            .and_then(TelemetryResponse::getStatusCode)
            .unwrap_or(0);
        let success = context
            .response
            .as_ref()
            .and_then(TelemetryResponse::getStatusCode)
            .unwrap_or(500)
            <= 399;
        let duration = context
            .startTime
            .map(|startTime| (Utc::now() - startTime).num_milliseconds())
            .unwrap_or(0);

        let trackedRequest = TrackedRequest {
            name: reqName.clone(),
            url: context
                .request
                .as_ref()
                .map(|request| Self::GetRequestUri(request.getEndpoint()))
                .unwrap_or_default(),
            duration,
            resultCode,
            success,
            id: context.contextId.clone(),
            source: context
                .request
                .as_ref()
                .and_then(|request| request.getHeader("user-agent"))
                .map(str::to_string),
            properties: requestProperties,
            contextObjects: HashMap::from([
                ("operationId".to_string(), String::new()),
                ("operationParentId".to_string(), String::new()),
                ("operationName".to_string(), "test".to_string()),
                ("operation_Name".to_string(), "test".to_string()),
                ("appName".to_string(), String::new()),
            ]),
        };
        drop(state);
        client.trackRequest(trackedRequest);
        logger.verbose(
            &format!("Send {serviceType} telemetry: {reqName}"),
            context
                .contextId
                .as_deref()
                .or(context.contextID.as_deref()),
        );
    }

    #[allow(non_snake_case)]
    pub async fn TraceStartEvent(serviceType: &str) {
        let (enabled, eventClient, instanceID, sessionID) = {
            let state = TELEMETRY_STATE.lock().unwrap();
            (
                state.enableTelemetry,
                state.eventClient.clone(),
                state.instanceID.clone(),
                state.sessionID.clone(),
            )
        };

        if enabled {
            if let Some(eventClient) = eventClient {
                eventClient.trackEvent(TrackedEvent {
                    name: format!(
                        "Azurite Start{}",
                        if serviceType.is_empty() {
                            String::new()
                        } else {
                            format!(": {serviceType}")
                        }
                    ),
                    properties: HashMap::from([
                        ("instanceID".to_string(), instanceID),
                        ("sessionID".to_string(), sessionID),
                        (
                            "parameters".to_string(),
                            Self::GetAllParameterString().await,
                        ),
                    ]),
                });
                logger.verbose("Send start telemetry", None);
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn TraceStopEvent(serviceType: &str) {
        let state = TELEMETRY_STATE.lock().unwrap().clone();
        if state.enableTelemetry {
            if let Some(eventClient) = state.eventClient {
                eventClient.trackEvent(TrackedEvent {
                    name: format!(
                        "Azurite Stop{}",
                        if serviceType.is_empty() {
                            String::new()
                        } else {
                            format!(": {serviceType}")
                        }
                    ),
                    properties: HashMap::from([
                        ("instanceID".to_string(), state.instanceID),
                        ("sessionID".to_string(), state.sessionID),
                        (
                            "blobRequest".to_string(),
                            state._totalBlobRequestCount.to_string(),
                        ),
                        (
                            "queueRequest".to_string(),
                            state._totalQueueRequestCount.to_string(),
                        ),
                        (
                            "tableRequest".to_string(),
                            state._totalTableRequestCount.to_string(),
                        ),
                        (
                            "totalIngress".to_string(),
                            state._totalIngressSize.to_string(),
                        ),
                        (
                            "totalEgress".to_string(),
                            state._totalEgressSize.to_string(),
                        ),
                    ]),
                });
                logger.verbose("Send stop telemetry", None);
            }
        }
    }

    #[allow(non_snake_case)]
    fn GetRequestUri(endpoint: &str) -> String {
        let Ok(uri) = Url::parse(endpoint) else {
            return endpoint.to_string();
        };
        let knownHosts = ["127.0.0.1", "localhost", "host.docker.internal"];
        if uri
            .host_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .parse::<usize>()
            .map(|index| index < knownHosts.len())
            .unwrap_or(false)
        {
            endpoint.replace(uri.host_str().unwrap_or_default(), "[hidden]")
        } else {
            endpoint.to_string()
        }
    }

    #[allow(non_snake_case)]
    fn GetInstanceID(
        location: String,
        configFileName: String,
        inMemoryPersistence: bool,
    ) -> String {
        let configFilePath = PathBuf::from(location).join(configFileName);
        let mut instaceID = String::new();
        if inMemoryPersistence {
            return Uuid::new_v4().to_string();
        }

        if !configFilePath.exists() {
            instaceID = Uuid::new_v4().to_string();
            let _ = fs::write(
                &configFilePath,
                format!("{{\"instaceID\":\"{instaceID}\"}}"),
            );
        } else if let Ok(data) = fs::read_to_string(&configFilePath) {
            instaceID = serde_json::from_str::<HashMap<String, String>>(&data)
                .ok()
                .and_then(|json| json.get("instaceID").cloned())
                .unwrap_or_default();
            if instaceID.is_empty() {
                instaceID = Uuid::new_v4().to_string();
                let _ = fs::write(
                    &configFilePath,
                    format!("{{\"instaceID\":\"{instaceID}\"}}"),
                );
            }
        }

        instaceID
    }

    #[allow(non_snake_case)]
    fn GetRequestAuthentication(
        authorizationHeader: Option<&str>,
        sigQuery: Option<&str>,
    ) -> String {
        let mut auth = authorizationHeader
            .and_then(|header| header.split(' ').next())
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        if let Some(current) = auth.clone() {
            if sigQuery.is_some() {
                auth = Some(format!("{current},Sas"));
            }
        } else if sigQuery.is_some() {
            auth = Some("Sas".to_string());
        } else {
            auth = Some("Anonymous".to_string());
        }
        auth.unwrap_or_default()
    }

    #[allow(non_snake_case)]
    async fn GetAllParameterString() -> String {
        let mut parameters = String::new();
        if env::var("AZURITE_ACCOUNTS").is_ok() {
            parameters.push_str("AZURITE_ACCOUNTS,");
        }
        if env::var("AZURITE_DB").is_ok() {
            parameters.push_str("AZURITE_DB,");
        }
        let longParameters = [
            "blobHost",
            "queueHost",
            "tableHost",
            "blobPort",
            "queuePort",
            "tablePort",
            "blobKeepAliveTimeout",
            "queueKeepAliveTimeout",
            "tableKeepAliveTimeout",
            "location",
            "cert",
            "key",
            "pwd",
            "oauth",
            "extentMemoryLimit",
            "debug",
            "silent",
            "loose",
            "skipApiVersionCheck",
            "disableProductStyleUrl",
            "inMemoryPersistence",
            "disableTelemetry",
        ];
        let shortParameters = HashMap::from([
            ("d", "debug"),
            ("l", "location"),
            ("L", "loose"),
            ("s", "silent"),
        ]);

        let (isVSC, telemetryEnv) = {
            let state = TELEMETRY_STATE.lock().unwrap();
            (state.isVSC, state.env.clone())
        };

        if isVSC {
            let Some(TelemetryEnvironment::WorkspaceConfiguration(workspaceConfiguration)) =
                telemetryEnv
            else {
                return parameters.trim_end_matches(',').to_string();
            };

            for flag in longParameters {
                let Some(value) = workspaceConfiguration.get(flag) else {
                    continue;
                };
                if value.is_null()
                    || value == &Value::Bool(false)
                    || value == &Value::String(String::new())
                {
                    continue;
                }
                if flag.ends_with("Host")
                    && value == &Value::String(DEFAULT_BLOB_SERVER_HOST_NAME.to_string())
                {
                    continue;
                }
                if flag.ends_with("KeepAliveTimeout")
                    && value == &Value::Number(DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT.into())
                {
                    continue;
                }
                if (flag == "blobPort"
                    && value == &Value::Number(DEFAULT_BLOB_LISTENING_PORT.into()))
                    || (flag == "queuePort"
                        && value == &Value::Number(DEFAULT_QUEUE_LISTENING_PORT.into()))
                    || (flag == "tablePort"
                        && value == &Value::Number(DEFAULT_TABLE_LISTENING_PORT.into()))
                {
                    continue;
                }
                parameters.push_str(flag);
                parameters.push(',');
            }
        } else {
            for value in env::args() {
                if value.starts_with("--") {
                    for flag in longParameters {
                        if value.eq_ignore_ascii_case(&format!("--{flag}")) {
                            parameters.push_str(flag);
                            parameters.push(',');
                        }
                    }
                } else if value.starts_with('-') {
                    if let Some(flag) = shortParameters.get(&value[1..]) {
                        parameters.push_str(flag);
                        parameters.push(',');
                    }
                }
            }
        }

        parameters.trim_end_matches(',').to_string()
    }
}
