use futures::stream::BoxStream;

use crate::i_data_store::IDataStore;

#[allow(non_snake_case)]
pub trait IGCExtentProvider: IDataStore + Send + Sync {
    fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>>;
}
