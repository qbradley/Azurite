use crate::generated::artifacts::specifications::specification;
use crate::generated::context::Context;
use crate::generated::errors::operation_mismatch_error::OperationMismatchError;
use crate::generated::i_response::IResponse;
use crate::generated::utils::i_logger::ILogger;
use crate::generated::utils::serializer::serialize;

pub async fn serializer_middleware<R: IResponse, L: ILogger + ?Sized>(
    context: &Context,
    res: &mut R,
    logger: &L,
) -> crate::generated::GeneratedResult<()> {
    logger.verbose(
        "SerializerMiddleware: Start serializing...",
        context.contextId().as_deref(),
    );

    let operation = if let Some(operation) = context.operation() {
        operation
    } else {
        let handlerError = OperationMismatchError::new();
        logger.error(
            &format!("SerializerMiddleware: {}", handlerError.message),
            context.contextId().as_deref(),
        );
        return Err(Box::new(handlerError));
    };

    let spec = if let Some(spec) = specification(operation) {
        spec
    } else {
        logger.warn(
            &format!(
                "SerializerMiddleware: Cannot find serializer for operation {}",
                operation
            ),
            context.contextId().as_deref(),
        );
        return Ok(());
    };

    if let Some(handlerResponses) = context.handlerResponses() {
        serialize(context, res, spec, &handlerResponses, logger).await?;
    }

    Ok(())
}
