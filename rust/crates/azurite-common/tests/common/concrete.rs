use azurite_common::{
    account_data_store::AccountDataStore, configuration_base::ConfigurationBase,
    environment::Environment, server_base::ServerBase,
};
use pretty_assertions::assert_eq;

#[test]
fn configuration_base_defaults_match_current_phase1_stub() {
    let config = ConfigurationBase::new();
    let defaulted = ConfigurationBase::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 10000);
    assert_eq!(defaulted.host, config.host);
    assert_eq!(defaulted.port, config.port);
}

#[test]
fn environment_constructor_preserves_cli_arguments() {
    let args = vec!["--blobHost".to_owned(), "127.0.0.1".to_owned()];
    let environment = Environment::new(args.clone());

    assert_eq!(environment.args, args);
    assert_eq!(Environment::default().args, Vec::<String>::new());
}

#[test]
fn server_base_new_starts_stopped() {
    let server = ServerBase::new();

    assert!(!server.is_started);
    assert!(!ServerBase::default().is_started);
}

#[test]
fn placeholder_phase1_structs_remain_constructible() {
    let _account_store = AccountDataStore::new().clone();
}
