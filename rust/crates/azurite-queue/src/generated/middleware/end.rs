use chrono::Utc;

use crate::generated::context::Context;
use crate::generated::i_response::IResponse;
use crate::generated::utils::i_logger::ILogger;

pub fn end_middleware<R: IResponse, L: ILogger + ?Sized>(
    context: &Context,
    res: &mut R,
    logger: &L,
) {
    let totalTimeInMS = context
        .startTime()
        .map(|startTime| (Utc::now() - startTime).num_milliseconds());
    logger.info(
        &format!(
            "EndMiddleware: End response. TotalTimeInMS={:?} StatusCode={} StatusMessage={} Headers={:?}",
            totalTimeInMS,
            res.getStatusCode(),
            res.getStatusMessage(),
            res.getHeaders(),
        ),
        context.contextId().as_deref(),
    );
    res.getBodyStream().end();
}
