use async_trait::async_trait;

pub const GENERATED_MIDDLEWARE_ORDER: [&str; 6] = [
    "DispatchMiddleware",
    "DeserializerMiddleware",
    "HandlerMiddleware",
    "SerializerMiddleware",
    "ErrorMiddleware",
    "EndMiddleware",
];

#[async_trait]
pub trait MiddlewareFactory {
    type DispatchMiddleware;
    type DeserializerMiddleware;
    type HandlerMiddleware;
    type SerializerMiddleware;
    type ErrorMiddleware;
    type EndMiddleware;

    fn createDispatchMiddleware(&self) -> Self::DispatchMiddleware;
    fn createDeserializerMiddleware(&self) -> Self::DeserializerMiddleware;
    fn createHandlerMiddleware(&self) -> Self::HandlerMiddleware;
    fn createSerializerMiddleware(&self) -> Self::SerializerMiddleware;
    fn createErrorMiddleware(&self) -> Self::ErrorMiddleware;
    fn createEndMiddleware(&self) -> Self::EndMiddleware;
}
