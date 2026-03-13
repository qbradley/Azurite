use azurite_common::{
    account_data_store::AccountDataStore,
    configuration_base::{CertOptions, ConfigurationBase},
    environment::Environment,
    i_environment::IEnvironment,
    server_base::{ServerBase, ServerStatus},
};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn configuration_base_defaults_match_phase4_translation() {
    let config = ConfigurationBase::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 10000);
    assert_eq!(config.hasCert(), CertOptions::Default);
    assert_eq!(config.getOAuthLevel(), None);
    assert_eq!(config.getHttpServerAddress(), "http://127.0.0.1:10000");
}

#[tokio::test]
async fn environment_constructor_preserves_cli_arguments() {
    let args = vec!["--blobHost".to_owned(), "127.0.0.1".to_owned()];
    let environment = Environment::new(args.clone());

    assert_eq!(environment.args, args);
    assert_eq!(environment.blobHost(), Some("127.0.0.1".to_owned()));
    assert_eq!(Environment::default().args, Vec::<String>::new());
}

#[test]
fn server_base_new_starts_closed() {
    let server = ServerBase::default();

    assert_eq!(server.getStatus(), ServerStatus::Closed);
    assert_eq!(server.host, "127.0.0.1");
    assert_eq!(server.port, 10000);
}

#[test]
fn phase4_structs_remain_constructible() {
    let _account_store = AccountDataStore::default();
}
