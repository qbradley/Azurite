use azurite_common::i_environment::IEnvironment;

#[allow(non_snake_case)]
pub trait IQueueEnvironment: IEnvironment + Send + Sync {}
