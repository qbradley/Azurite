use std::{fmt, sync::Arc};

use azurite_common::{
    i_request_listener_factory::IRequestListenerFactory, server_base::RequestListener,
};

use crate::blob_configuration::BlobConfiguration;

#[allow(non_snake_case)]
#[derive(Default)]
pub struct BlobServer {
    pub configuration: BlobConfiguration,
    requestListenerFactory: Option<Arc<dyn IRequestListenerFactory>>,
    requestListener: Option<RequestListener>,
}

impl BlobServer {
    pub fn new() -> Self {
        Self::withConfiguration(BlobConfiguration::default())
    }

    #[allow(non_snake_case)]
    pub fn withConfiguration(configuration: BlobConfiguration) -> Self {
        Self {
            configuration,
            requestListenerFactory: None,
            requestListener: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withRequestListenerFactory(
        configuration: BlobConfiguration,
        requestListenerFactory: Arc<dyn IRequestListenerFactory>,
    ) -> Self {
        Self {
            configuration,
            requestListenerFactory: Some(requestListenerFactory),
            requestListener: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn createRequestListener(&mut self) -> RequestListener {
        let listener = self
            .requestListenerFactory
            .as_ref()
            .map(|factory| factory.createRequestListener())
            .unwrap_or_default();
        self.requestListener = Some(listener.clone());
        listener
    }

    #[allow(non_snake_case)]
    pub fn requestListener(&self) -> Option<RequestListener> {
        self.requestListener.clone()
    }

    pub fn configuration(&self) -> &BlobConfiguration {
        &self.configuration
    }
}

impl fmt::Debug for BlobServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BlobServer")
            .field("configuration", &self.configuration)
            .field(
                "hasRequestListenerFactory",
                &self
                    .requestListenerFactory
                    .as_ref()
                    .map(|_| true)
                    .unwrap_or(false),
            )
            .field(
                "hasRequestListener",
                &self.requestListener.as_ref().map(|_| true).unwrap_or(false),
            )
            .finish()
    }
}
