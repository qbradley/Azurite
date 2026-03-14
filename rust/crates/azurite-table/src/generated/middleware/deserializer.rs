use crate::generated::artifacts::specifications::specification;
use crate::generated::context::Context;
use crate::generated::errors::deserialization_error::DeserializationError;
use crate::generated::errors::operation_mismatch_error::OperationMismatchError;
use crate::generated::i_request::IRequest;
use crate::generated::utils::i_logger::ILogger;
use crate::generated::utils::serializer::deserialize;

pub async fn deserializer_middleware<R: IRequest, L: ILogger + ?Sized>(
    context: &Context,
    req: &mut R,
    logger: &L,
) -> crate::generated::GeneratedResult<()> {
    logger.verbose(
        "DeserializerMiddleware: Start deserializing...",
        context.contextId().as_deref(),
    );

    let operation = if let Some(operation) = context.operation() {
        operation
    } else {
        let handlerError = OperationMismatchError::new();
        logger.error(
            &format!("DeserializerMiddleware: {}", handlerError.message),
            context.contextId().as_deref(),
        );
        return Err(Box::new(handlerError));
    };

    let spec = if let Some(spec) = specification(operation) {
        spec
    } else {
        logger.warn(
            &format!(
                "DeserializerMiddleware: Cannot find deserializer for operation {}",
                operation
            ),
            context.contextId().as_deref(),
        );
        context.setHandlerParameters(Some(Default::default()));
        return Ok(());
    };

    match deserialize(context, req, spec, logger).await {
        Ok(parameters) => {
            context.setHandlerParameters(Some(parameters));
            Ok(())
        }
        Err(error) => {
            let deserializationError = DeserializationError::new(error.to_string());
            Err(Box::new(deserializationError))
        }
    }
}
