use std::sync::Arc;

use crate::generated::context::{Context, ContextHolder};
use crate::generated::express_request_adapter::ExpressRequestAdapter;
use crate::generated::express_response_adapter::ExpressResponseAdapter;
use crate::generated::handlers::IHandlers;
use crate::generated::i_request::IRequest;
use crate::generated::i_response::IResponse;
use crate::generated::middleware::deserializer::deserializer_middleware;
use crate::generated::middleware::dispatch::dispatch_middleware;
use crate::generated::middleware::end::end_middleware;
use crate::generated::middleware::error::error_middleware;
use crate::generated::middleware::handler_middleware_factory::HandlerMiddlewareFactory;
use crate::generated::middleware::serializer::serializer_middleware;
use crate::generated::middleware_factory::MiddlewareFactory;
use crate::generated::utils::i_logger::ILogger;

pub struct ExpressMiddlewareFactory<H: IHandlers, L: ILogger + ?Sized> {
    logger: Arc<L>,
    handlers: Arc<H>,
    contextPath: String,
}

impl<H: IHandlers, L: ILogger + ?Sized> ExpressMiddlewareFactory<H, L> {
    pub fn new(logger: Arc<L>, handlers: Arc<H>, contextPath: impl Into<String>) -> Self {
        Self {
            logger,
            handlers,
            contextPath: contextPath.into(),
        }
    }

    pub fn createContext(
        &self,
        holder: ContextHolder,
        request: ExpressRequestAdapter,
        response: ExpressResponseAdapter,
    ) -> Context {
        Context::from_holder(
            holder,
            self.contextPath.clone(),
            Some(request.into_inner()),
            Some(response.into_inner()),
        )
    }

    pub fn dispatch(
        &self,
        context: &Context,
        request: &ExpressRequestAdapter,
    ) -> crate::generated::GeneratedResult<()> {
        dispatch_middleware(context, request, self.logger.as_ref())
    }

    pub async fn deserialize(
        &self,
        context: &Context,
        request: &mut ExpressRequestAdapter,
    ) -> crate::generated::GeneratedResult<()> {
        deserializer_middleware(context, request, self.logger.as_ref()).await
    }

    pub async fn handle(&self, context: &Context) -> crate::generated::GeneratedResult<()> {
        HandlerMiddlewareFactory::new(self.handlers.clone(), self.logger.clone())
            .call(context)
            .await
    }

    pub async fn serialize(
        &self,
        context: &Context,
        response: &mut ExpressResponseAdapter,
    ) -> crate::generated::GeneratedResult<()> {
        serializer_middleware(context, response, self.logger.as_ref()).await
    }

    pub fn error(
        &self,
        context: &Context,
        error: &(dyn std::error::Error + Send + Sync + 'static),
        request: &ExpressRequestAdapter,
        response: &mut ExpressResponseAdapter,
    ) -> crate::generated::GeneratedResult<()> {
        error_middleware(context, error, request, response, self.logger.as_ref())
    }

    pub fn end(&self, context: &Context, response: &mut ExpressResponseAdapter) {
        end_middleware(context, response, self.logger.as_ref())
    }
}

impl<H: IHandlers, L: ILogger + ?Sized> MiddlewareFactory for ExpressMiddlewareFactory<H, L> {
    type DispatchMiddleware = ();
    type DeserializerMiddleware = ();
    type HandlerMiddleware = ();
    type SerializerMiddleware = ();
    type ErrorMiddleware = ();
    type EndMiddleware = ();

    fn createDispatchMiddleware(&self) -> Self::DispatchMiddleware {}
    fn createDeserializerMiddleware(&self) -> Self::DeserializerMiddleware {}
    fn createHandlerMiddleware(&self) -> Self::HandlerMiddleware {}
    fn createSerializerMiddleware(&self) -> Self::SerializerMiddleware {}
    fn createErrorMiddleware(&self) -> Self::ErrorMiddleware {}
    fn createEndMiddleware(&self) -> Self::EndMiddleware {}
}
