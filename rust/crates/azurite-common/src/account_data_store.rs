use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use base64::Engine as _;
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{interval, Duration},
};

use crate::{
    i_account_data_store::{IAccountDataStore, IAccountProperties},
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_logger::ILogger,
    logger::Logger,
    storage_error::StorageError,
    utils::constants::{
        AZURITE_ACCOUNTS_ENV, DEFAULT_ACCOUNTS_REFRESH_INTERVAL, EMULATOR_ACCOUNT_KEY,
        EMULATOR_ACCOUNT_NAME,
    },
};

/// Decode a base64 string leniently, matching Node.js `Buffer.from(str, "base64")`.
/// Node.js accepts both standard base64 (`+`, `/`) and URL-safe base64 (`-`, `_`)
/// simultaneously, treating `-` as `+` (value 62) and `_` as `/` (value 63).
/// It also silently ignores any remaining characters outside the base64 alphabet,
/// tolerates missing padding, and allows non-zero trailing bits.
fn lenient_base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::alphabet::STANDARD as STANDARD_ALPHA;
    use base64::engine::{GeneralPurpose, GeneralPurposeConfig};

    let mut cleaned: String = input
        .chars()
        .filter_map(|c| match c {
            '-' => Some('+'),
            '_' => Some('/'),
            c if c.is_ascii_alphanumeric() || c == '+' || c == '/' => Some(c),
            '=' => Some('='),
            _ => None,
        })
        .collect();

    // Strip any trailing '=' and re-add the correct amount of padding.
    let trimmed = cleaned.trim_end_matches('=');
    let pad_needed = (4 - trimmed.len() % 4) % 4;
    cleaned = format!("{}{}", trimmed, "=".repeat(pad_needed));

    // Node.js tolerates non-zero trailing bits in the last symbol, so we must
    // use `with_decode_allow_trailing_bits(true)`.
    let lenient_engine = GeneralPurpose::new(
        &STANDARD_ALPHA,
        GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true),
    );
    lenient_engine.decode(&cleaned)
}

#[allow(dead_code)]
enum Status {
    Initializing,
    Initialized,
    Closing,
    Closed,
}

type IAccounts = HashMap<String, IAccountProperties>;

fn default_emulator_accounts() -> IAccounts {
    HashMap::from([(
        EMULATOR_ACCOUNT_NAME.to_string(),
        IAccountProperties {
            name: EMULATOR_ACCOUNT_NAME.to_string(),
            key1: EMULATOR_ACCOUNT_KEY.clone(),
            key2: None,
        },
    )])
}

#[allow(non_snake_case)]
pub struct AccountDataStore {
    status: Status,
    timer: Option<JoinHandle<()>>,
    timerShutdown: Option<oneshot::Sender<()>>,
    accounts: Arc<RwLock<IAccounts>>,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl Default for AccountDataStore {
    fn default() -> Self {
        Self::new(Arc::new(Logger::default()))
    }
}

impl AccountDataStore {
    pub fn new(logger: Arc<dyn ILogger + Send + Sync>) -> Self {
        Self {
            status: Status::Closed,
            timer: None,
            timerShutdown: None,
            accounts: Arc::new(RwLock::new(default_emulator_accounts())),
            logger,
        }
    }

    fn refresh(&self) {
        let env = std::env::var(AZURITE_ACCOUNTS_ENV).ok();
        let masked = env
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|_| "*****")
            .unwrap_or("undefined");
        self.logger.info(
            &format!(
                "AccountDataStore:init() Refresh accounts from environment variable {AZURITE_ACCOUNTS_ENV} with value {masked}"
            ),
            None,
        );

        if let Some(env) = env.filter(|value| !value.is_empty()) {
            match self.parserAccountsEnvironmentString(&env) {
                Ok(accounts) => {
                    *self.accounts.write().unwrap() = accounts;
                }
                Err(error) => {
                    self.logger.error(
                        &format!(
                            "AccountDataStore:init() Fallback to default emulator account {EMULATOR_ACCOUNT_NAME}. Refresh accounts from environment variable {AZURITE_ACCOUNTS_ENV} failed. \"{}\"",
                            error.message
                        ),
                        None,
                    );
                    *self.accounts.write().unwrap() = default_emulator_accounts();
                }
            }
        } else {
            self.logger.info(
                &format!(
                    "AccountDataStore:init() Fallback to default emulator account {EMULATOR_ACCOUNT_NAME}."
                ),
                None,
            );
            *self.accounts.write().unwrap() = default_emulator_accounts();
        }
    }

    fn parserAccountsEnvironmentString(&self, accounts: &str) -> Result<IAccounts, StorageError> {
        let mut results = HashMap::new();
        for accountAndKeys in accounts.trim().split(';') {
            if !accountAndKeys.is_empty() {
                let parts: Vec<&str> = accountAndKeys.split(':').collect();
                if parts.len() < 2 || parts.len() > 3 {
                    return Err(StorageError::new(format!(
                        "AccountDataStore:parserAccountsEnvironmentString() Invalid environment string format for {accounts}"
                    )));
                }
                let account = parts[0];
                let key1 = lenient_base64_decode(parts[1])
                    .map_err(|error| StorageError::new(error.to_string()))?;
                let key2 = if parts.len() > 2 {
                    Some(
                        lenient_base64_decode(parts[2])
                            .map_err(|error| StorageError::new(error.to_string()))?,
                    )
                } else {
                    None
                };
                results.insert(
                    account.to_string(),
                    IAccountProperties {
                        name: account.to_string(),
                        key1,
                        key2,
                    },
                );
            }
        }
        Ok(results)
    }
}

#[async_trait]
impl IDataStore for AccountDataStore {
    async fn init(&mut self) -> Result<(), StorageError> {
        self.refresh();

        let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();
        let accounts = Arc::clone(&self.accounts);
        let logger = Arc::clone(&self.logger);
        self.timerShutdown = Some(shutdown_sender);
        self.timer = Some(tokio::spawn(async move {
            let mut timer = interval(Duration::from_millis(DEFAULT_ACCOUNTS_REFRESH_INTERVAL));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let store = AccountDataStore {
                            status: Status::Initialized,
                            timer: None,
                            timerShutdown: None,
                            accounts: Arc::clone(&accounts),
                            logger: Arc::clone(&logger),
                        };
                        store.refresh();
                    }
                    _ = &mut shutdown_receiver => {
                        break;
                    }
                }
            }
        }));
        self.status = Status::Initialized;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        matches!(self.status, Status::Initialized)
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        if let Some(shutdown) = self.timerShutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(timer) = self.timer.take() {
            let _ = timer.await;
        }
        self.status = Status::Closed;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        matches!(self.status, Status::Closed)
    }
}

#[async_trait]
impl ICleaner for AccountDataStore {
    async fn clean(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

impl IAccountDataStore for AccountDataStore {
    fn getAccount(&self, name: &str) -> Option<IAccountProperties> {
        self.accounts.read().unwrap().get(name).cloned()
    }
}
