# Porting Record — `src/blob/generated/middleware/error.middleware.ts`

## File info
- Source path: `src/blob/generated/middleware/error.middleware.ts`
- Source lines: `134`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware/error.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware::error`
- Phase: `5.25`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default function errorMiddleware(
  context: Context,
  err: MiddlewareError | Error,
  req: IRequest,
  res: IResponse,
  next: NextFunction,
  logger: ILogger
): void {
  if (res.headersSent()) {
    logger.warn(
      `Error middleware received an error, but response.headersSent is true, pass error to next middleware`,
      context.contextId
    );
    return next(err);
  }

  // Only handle ServerError, for other customized error types hand over to
  // other error handlers.
  if (err instanceof MiddlewareError) {
    logger.error(
      `ErrorMiddleware: Received a MiddlewareError, fill error information to HTTP response`,
      context.contextId
    );

    logger.error(
      `ErrorMiddleware: ErrorName=${err.name} ErrorMessage=${
        err.message
      }  ErrorHTTPStatusCode=${err.statusCode} ErrorHTTPStatusMessage=${
        err.statusMessage
      } ErrorHTTPHeaders=${JSON.stringify(
        err.headers
      )} ErrorHTTPBody=${JSON.stringify(err.body)} ErrorStack=${JSON.stringify(
        err.stack
      )}`,
      context.contextId
    );

    logger.error(
      `ErrorMiddleware: Set HTTP code: ${err.statusCode}`,
      context.contextId
    );

    res.setStatusCode(err.statusCode);
    if (err.statusMessage) {
      logger.error(
        `ErrorMiddleware: Set HTTP status message: ${err.statusMessage}`,
        context.contextId
      );
      res.setStatusMessage(err.statusMessage);
    }

    if (err.headers) {
      for (const key in err.headers) {
        if (err.headers.hasOwnProperty(key)) {
          const value = err.headers[key];
          if (value) {
            logger.error(
              `ErrorMiddleware: Set HTTP Header: ${key}=${value}`,
              context.contextId
            );
            res.setHeader(key, value);
          }
        }
      }
    }

    if (err.contentType && req.getMethod() !== "HEAD") {
      logger.error(
        `ErrorMiddleware: Set content type: ${err.contentType}`,
        context.contextId
      );
      res.setContentType(err.contentType);
    }

    logger.error(
      `ErrorMiddleware: Set HTTP body: ${JSON.stringify(err.body)}`,
      context.contextId
    );
    if (err.body && req.getMethod() !== "HEAD") {
      res.getBodyStream().write(err.body);
    }
  } else if (err instanceof Error) {
    logger.error(
      `ErrorMiddleware: Received an error, fill error information to HTTP response`,
      context.contextId
    );
    logger.error(
      `ErrorMiddleware: ErrorName=${err.name} ErrorMessage=${
        err.message
      } ErrorStack=${JSON.stringify(err.stack)}`,
      context.contextId
    );
    logger.error(`ErrorMiddleware: Set HTTP code: ${500}`, context.contextId);
    res.setStatusCode(500);

    // logger.error(
    //   `ErrorMiddleware: Set error message: ${err.message}`,
    //   context.contextID
    // );
    // res.getBodyStream().write(err.message);
  } else {
    logger.warn(
      `ErrorMiddleware: Received unhandled error object`,
      context.contextId
    );
  }

  next();
}
```

## Dependencies
- Internal imports:
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `../errors/MiddlewareError` → `src/blob/generated/errors/MiddlewareError.ts` — Phase 5 — analyzed in this pass
  - `../IRequest` → `src/blob/generated/IRequest.ts` — Phase 5 — analyzed in this pass
  - `../IResponse` → `src/blob/generated/IResponse.ts` — Phase 5 — analyzed in this pass
  - `../MiddlewareFactory` → `src/blob/generated/MiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `../utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `MiddlewareError | Error` | error enum with generated/non-generated branches | Generated middleware distinguishes structured HTTP errors from generic 500s. |
| `OutgoingHttpHeaders` iteration | `HeaderMap` iteration preserving repeated/string values | Structured errors may inject headers before end middleware runs. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- If `res.headersSent()` is already true, the middleware logs and re-throws to the next error middleware by calling `next(err)`.
- `MiddlewareError` instances populate status, optional status message, optional headers, optional content type, and optional body. HEAD requests suppress content type/body writes.
- Plain `Error` instances become status 500 only; the commented-out body write stays disabled.
- The stage always finishes with `next()` (no error arg) so `end.middleware.ts` still finalizes the response.

## Middleware chain ordering
- Stage 5 of 6: handles structured or generic failures after serializer/handler/deserializer stages and before final `end()` of the response stream.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If generated error classes gain fields, update this serializer path field-for-field instead of collapsing them into a generic error string.
