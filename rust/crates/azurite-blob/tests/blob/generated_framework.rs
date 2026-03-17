#![allow(non_snake_case)]

use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use azurite_blob::errors::StorageErrorFactory;
use azurite_blob::generated::artifacts::mappers::{Mapper, MapperType};
use azurite_blob::generated::artifacts::models::{
    self, GeneratedBody, GeneratedResponse, GeneratedValue,
};
use azurite_blob::generated::artifacts::operation::{Operation, ALL_OPERATIONS};
use azurite_blob::generated::artifacts::parameters::{OperationParameter, ParameterPath};
use azurite_blob::generated::artifacts::specifications::{
    OperationSpec, RequestBodySpec, ResponseSpec,
};
use azurite_blob::generated::context::Context;
use azurite_blob::generated::errors::middleware_error::MiddlewareError;
use azurite_blob::generated::express_middleware_factory::ExpressMiddlewareFactory;
use azurite_blob::generated::express_request_adapter::ExpressRequestAdapter;
use azurite_blob::generated::express_response_adapter::ExpressResponseAdapter;
use azurite_blob::generated::handlers::{
    getHandlerByOperation, IAppendBlobHandler, IBlobHandler, IBlockBlobHandler, IContainerHandler,
    IHandlers, IPageBlobHandler, IServiceHandler,
};
use azurite_blob::generated::i_request::{
    GeneratedHttpRequest, GeneratedReadableStream, HttpMethod, IRequest, RequestHeaderValue,
};
use azurite_blob::generated::i_response::{GeneratedHttpResponse, IResponse, ResponseHeaderValue};
use azurite_blob::generated::middleware::dispatch::dispatch_middleware;
use azurite_blob::generated::middleware::end::end_middleware;
use azurite_blob::generated::middleware::error::error_middleware;
use azurite_blob::generated::middleware_factory::GENERATED_MIDDLEWARE_ORDER;
use azurite_blob::generated::utils::i_logger::ILogger;
use azurite_blob::generated::utils::serializer::{deserialize, serialize};
use chrono::Utc;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Default)]
struct CallRecorder {
    entries: Arc<Mutex<Vec<String>>>,
}

impl CallRecorder {
    fn record(&self, value: impl Into<String>) {
        self.entries
            .lock()
            .expect("call recorder mutex poisoned")
            .push(value.into());
    }

    fn entries(&self) -> Vec<String> {
        self.entries
            .lock()
            .expect("call recorder mutex poisoned")
            .clone()
    }
}

#[derive(Clone, Default)]
struct RecordingLogger {
    entries: Arc<Mutex<Vec<String>>>,
}

impl RecordingLogger {
    fn entries(&self) -> Vec<String> {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .clone()
    }

    fn push(&self, level: &str, message: &str, context_id: Option<&str>) {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .push(format!(
                "{level}:{message}:{}",
                context_id.unwrap_or("<none>")
            ));
    }
}

impl ILogger for RecordingLogger {
    fn error(&self, message: &str, contextID: Option<&str>) {
        self.push("error", message, contextID);
    }

    fn warn(&self, message: &str, contextID: Option<&str>) {
        self.push("warn", message, contextID);
    }

    fn info(&self, message: &str, contextID: Option<&str>) {
        self.push("info", message, contextID);
    }

    fn verbose(&self, message: &str, contextID: Option<&str>) {
        self.push("verbose", message, contextID);
    }

    fn debug(&self, message: &str, contextID: Option<&str>) {
        self.push("debug", message, contextID);
    }
}

#[derive(Clone)]
struct ServiceHandlerHarness {
    recorder: CallRecorder,
}

#[derive(Clone)]
struct ContainerHandlerHarness {
    recorder: CallRecorder,
}

#[derive(Clone)]
struct BlobHandlerHarness {
    recorder: CallRecorder,
}

#[derive(Clone)]
struct PageBlobHandlerHarness {
    recorder: CallRecorder,
}

#[derive(Clone)]
struct AppendBlobHandlerHarness {
    recorder: CallRecorder,
}

#[derive(Clone)]
struct BlockBlobHandlerHarness {
    recorder: CallRecorder,
}

fn recorded_response(method: &str) -> GeneratedResponse {
    let mut response = GeneratedResponse::new(200);
    response.body = Some(GeneratedBody::Value(GeneratedValue::Object(
        BTreeMap::from([(
            String::from("handlerMethod"),
            GeneratedValue::String(method.to_owned()),
        )]),
    )));
    response
}

#[async_trait]
impl IServiceHandler for ServiceHandlerHarness {
    async fn setProperties(
        &self,
        storageServiceProperties: models::StorageServiceProperties,
        options: models::ServiceSetPropertiesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceSetPropertiesResponse> {
        let _ = (storageServiceProperties, options, context);
        self.recorder.record("serviceHandler.setProperties");
        Ok(recorded_response("serviceHandler.setProperties"))
    }

    async fn getProperties(
        &self,
        options: models::ServiceGetPropertiesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceGetPropertiesResponse> {
        let _ = (options, context);
        self.recorder.record("serviceHandler.getProperties");
        Ok(recorded_response("serviceHandler.getProperties"))
    }

    async fn getStatistics(
        &self,
        options: models::ServiceGetStatisticsOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceGetStatisticsResponse> {
        let _ = (options, context);
        self.recorder.record("serviceHandler.getStatistics");
        Ok(recorded_response("serviceHandler.getStatistics"))
    }

    async fn listContainersSegment(
        &self,
        options: models::ServiceListContainersSegmentOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceListContainersSegmentResponse>
    {
        let _ = (options, context);
        self.recorder.record("serviceHandler.listContainersSegment");
        Ok(recorded_response("serviceHandler.listContainersSegment"))
    }

    async fn getUserDelegationKey(
        &self,
        keyInfo: models::KeyInfo,
        options: models::ServiceGetUserDelegationKeyOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceGetUserDelegationKeyResponse> {
        let _ = (keyInfo, options, context);
        self.recorder.record("serviceHandler.getUserDelegationKey");
        Ok(recorded_response("serviceHandler.getUserDelegationKey"))
    }

    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceGetAccountInfoResponse> {
        let _ = (context,);
        self.recorder.record("serviceHandler.getAccountInfo");
        Ok(recorded_response("serviceHandler.getAccountInfo"))
    }

    async fn submitBatch(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        multipartContentType: String,
        options: models::ServiceSubmitBatchOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceSubmitBatchResponse> {
        let _ = (body, contentLength, multipartContentType, options, context);
        self.recorder.record("serviceHandler.submitBatch");
        Ok(recorded_response("serviceHandler.submitBatch"))
    }

    async fn filterBlobs(
        &self,
        options: models::ServiceFilterBlobsOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ServiceFilterBlobsResponse> {
        let _ = (options, context);
        self.recorder.record("serviceHandler.filterBlobs");
        Ok(recorded_response("serviceHandler.filterBlobs"))
    }
}

#[async_trait]
impl IContainerHandler for ContainerHandlerHarness {
    async fn create(
        &self,
        options: models::ContainerCreateOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerCreateResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.create");
        Ok(recorded_response("containerHandler.create"))
    }

    async fn getProperties(
        &self,
        options: models::ContainerGetPropertiesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerGetPropertiesResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.getProperties");
        Ok(recorded_response("containerHandler.getProperties"))
    }

    async fn getPropertiesWithHead(
        &self,
        options: models::ContainerGetPropertiesWithHeadOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerGetPropertiesWithHeadResponse>
    {
        let _ = (options, context);
        self.recorder
            .record("containerHandler.getPropertiesWithHead");
        Ok(recorded_response("containerHandler.getPropertiesWithHead"))
    }

    async fn delete(
        &self,
        options: models::ContainerDeleteMethodOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerDeleteResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.delete");
        Ok(recorded_response("containerHandler.delete"))
    }

    async fn setMetadata(
        &self,
        options: models::ContainerSetMetadataOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerSetMetadataResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.setMetadata");
        Ok(recorded_response("containerHandler.setMetadata"))
    }

    async fn getAccessPolicy(
        &self,
        options: models::ContainerGetAccessPolicyOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerGetAccessPolicyResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.getAccessPolicy");
        Ok(recorded_response("containerHandler.getAccessPolicy"))
    }

    async fn setAccessPolicy(
        &self,
        options: models::ContainerSetAccessPolicyOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerSetAccessPolicyResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.setAccessPolicy");
        Ok(recorded_response("containerHandler.setAccessPolicy"))
    }

    async fn restore(
        &self,
        options: models::ContainerRestoreOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerRestoreResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.restore");
        Ok(recorded_response("containerHandler.restore"))
    }

    async fn submitBatch(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        multipartContentType: String,
        options: models::ContainerSubmitBatchOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerSubmitBatchResponse> {
        let _ = (body, contentLength, multipartContentType, options, context);
        self.recorder.record("containerHandler.submitBatch");
        Ok(recorded_response("containerHandler.submitBatch"))
    }

    async fn filterBlobs(
        &self,
        options: models::ContainerFilterBlobsOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerFilterBlobsResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.filterBlobs");
        Ok(recorded_response("containerHandler.filterBlobs"))
    }

    async fn acquireLease(
        &self,
        options: models::ContainerAcquireLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerAcquireLeaseResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.acquireLease");
        Ok(recorded_response("containerHandler.acquireLease"))
    }

    async fn releaseLease(
        &self,
        leaseId: String,
        options: models::ContainerReleaseLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerReleaseLeaseResponse> {
        let _ = (leaseId, options, context);
        self.recorder.record("containerHandler.releaseLease");
        Ok(recorded_response("containerHandler.releaseLease"))
    }

    async fn renewLease(
        &self,
        leaseId: String,
        options: models::ContainerRenewLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerRenewLeaseResponse> {
        let _ = (leaseId, options, context);
        self.recorder.record("containerHandler.renewLease");
        Ok(recorded_response("containerHandler.renewLease"))
    }

    async fn breakLease(
        &self,
        options: models::ContainerBreakLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerBreakLeaseResponse> {
        let _ = (options, context);
        self.recorder.record("containerHandler.breakLease");
        Ok(recorded_response("containerHandler.breakLease"))
    }

    async fn changeLease(
        &self,
        leaseId: String,
        proposedLeaseId: String,
        options: models::ContainerChangeLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerChangeLeaseResponse> {
        let _ = (leaseId, proposedLeaseId, options, context);
        self.recorder.record("containerHandler.changeLease");
        Ok(recorded_response("containerHandler.changeLease"))
    }

    async fn listBlobFlatSegment(
        &self,
        options: models::ContainerListBlobFlatSegmentOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerListBlobFlatSegmentResponse>
    {
        let _ = (options, context);
        self.recorder.record("containerHandler.listBlobFlatSegment");
        Ok(recorded_response("containerHandler.listBlobFlatSegment"))
    }

    async fn listBlobHierarchySegment(
        &self,
        delimiter: String,
        options: models::ContainerListBlobHierarchySegmentOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerListBlobHierarchySegmentResponse>
    {
        let _ = (delimiter, options, context);
        self.recorder
            .record("containerHandler.listBlobHierarchySegment");
        Ok(recorded_response(
            "containerHandler.listBlobHierarchySegment",
        ))
    }

    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::ContainerGetAccountInfoResponse> {
        let _ = (context,);
        self.recorder.record("containerHandler.getAccountInfo");
        Ok(recorded_response("containerHandler.getAccountInfo"))
    }
}

#[async_trait]
impl IBlobHandler for BlobHandlerHarness {
    async fn download(
        &self,
        options: models::BlobDownloadOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobDownloadResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.download");
        Ok(recorded_response("blobHandler.download"))
    }

    async fn getProperties(
        &self,
        options: models::BlobGetPropertiesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobGetPropertiesResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.getProperties");
        Ok(recorded_response("blobHandler.getProperties"))
    }

    async fn delete(
        &self,
        options: models::BlobDeleteMethodOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobDeleteResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.delete");
        Ok(recorded_response("blobHandler.delete"))
    }

    async fn undelete(
        &self,
        options: models::BlobUndeleteOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobUndeleteResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.undelete");
        Ok(recorded_response("blobHandler.undelete"))
    }

    async fn setExpiry(
        &self,
        expiryOptions: models::BlobExpiryOptions,
        options: models::BlobSetExpiryOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetExpiryResponse> {
        let _ = (expiryOptions, options, context);
        self.recorder.record("blobHandler.setExpiry");
        Ok(recorded_response("blobHandler.setExpiry"))
    }

    async fn setHTTPHeaders(
        &self,
        options: models::BlobSetHTTPHeadersOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetHTTPHeadersResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.setHTTPHeaders");
        Ok(recorded_response("blobHandler.setHTTPHeaders"))
    }

    async fn setImmutabilityPolicy(
        &self,
        options: models::BlobSetImmutabilityPolicyOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetImmutabilityPolicyResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.setImmutabilityPolicy");
        Ok(recorded_response("blobHandler.setImmutabilityPolicy"))
    }

    async fn deleteImmutabilityPolicy(
        &self,
        options: models::BlobDeleteImmutabilityPolicyOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobDeleteImmutabilityPolicyResponse>
    {
        let _ = (options, context);
        self.recorder.record("blobHandler.deleteImmutabilityPolicy");
        Ok(recorded_response("blobHandler.deleteImmutabilityPolicy"))
    }

    async fn setLegalHold(
        &self,
        legalHold: bool,
        options: models::BlobSetLegalHoldOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetLegalHoldResponse> {
        let _ = (legalHold, options, context);
        self.recorder.record("blobHandler.setLegalHold");
        Ok(recorded_response("blobHandler.setLegalHold"))
    }

    async fn setMetadata(
        &self,
        options: models::BlobSetMetadataOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetMetadataResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.setMetadata");
        Ok(recorded_response("blobHandler.setMetadata"))
    }

    async fn acquireLease(
        &self,
        options: models::BlobAcquireLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobAcquireLeaseResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.acquireLease");
        Ok(recorded_response("blobHandler.acquireLease"))
    }

    async fn releaseLease(
        &self,
        leaseId: String,
        options: models::BlobReleaseLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobReleaseLeaseResponse> {
        let _ = (leaseId, options, context);
        self.recorder.record("blobHandler.releaseLease");
        Ok(recorded_response("blobHandler.releaseLease"))
    }

    async fn renewLease(
        &self,
        leaseId: String,
        options: models::BlobRenewLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobRenewLeaseResponse> {
        let _ = (leaseId, options, context);
        self.recorder.record("blobHandler.renewLease");
        Ok(recorded_response("blobHandler.renewLease"))
    }

    async fn changeLease(
        &self,
        leaseId: String,
        proposedLeaseId: String,
        options: models::BlobChangeLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobChangeLeaseResponse> {
        let _ = (leaseId, proposedLeaseId, options, context);
        self.recorder.record("blobHandler.changeLease");
        Ok(recorded_response("blobHandler.changeLease"))
    }

    async fn breakLease(
        &self,
        options: models::BlobBreakLeaseOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobBreakLeaseResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.breakLease");
        Ok(recorded_response("blobHandler.breakLease"))
    }

    async fn createSnapshot(
        &self,
        options: models::BlobCreateSnapshotOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobCreateSnapshotResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.createSnapshot");
        Ok(recorded_response("blobHandler.createSnapshot"))
    }

    async fn startCopyFromURL(
        &self,
        copySource: String,
        options: models::BlobStartCopyFromURLOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobStartCopyFromURLResponse> {
        let _ = (copySource, options, context);
        self.recorder.record("blobHandler.startCopyFromURL");
        Ok(recorded_response("blobHandler.startCopyFromURL"))
    }

    async fn copyFromURL(
        &self,
        copySource: String,
        options: models::BlobCopyFromURLOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobCopyFromURLResponse> {
        let _ = (copySource, options, context);
        self.recorder.record("blobHandler.copyFromURL");
        Ok(recorded_response("blobHandler.copyFromURL"))
    }

    async fn abortCopyFromURL(
        &self,
        copyId: String,
        options: models::BlobAbortCopyFromURLOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobAbortCopyFromURLResponse> {
        let _ = (copyId, options, context);
        self.recorder.record("blobHandler.abortCopyFromURL");
        Ok(recorded_response("blobHandler.abortCopyFromURL"))
    }

    async fn setTier(
        &self,
        tier: models::AccessTier,
        options: models::BlobSetTierOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetTierResponse> {
        let _ = (tier, options, context);
        self.recorder.record("blobHandler.setTier");
        Ok(recorded_response("blobHandler.setTier"))
    }

    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobGetAccountInfoResponse> {
        let _ = (context,);
        self.recorder.record("blobHandler.getAccountInfo");
        Ok(recorded_response("blobHandler.getAccountInfo"))
    }

    async fn query(
        &self,
        options: models::BlobQueryOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobQueryResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.query");
        Ok(recorded_response("blobHandler.query"))
    }

    async fn getTags(
        &self,
        options: models::BlobGetTagsOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobGetTagsResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.getTags");
        Ok(recorded_response("blobHandler.getTags"))
    }

    async fn setTags(
        &self,
        options: models::BlobSetTagsOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlobSetTagsResponse> {
        let _ = (options, context);
        self.recorder.record("blobHandler.setTags");
        Ok(recorded_response("blobHandler.setTags"))
    }
}

#[async_trait]
impl IPageBlobHandler for PageBlobHandlerHarness {
    async fn create(
        &self,
        contentLength: f64,
        blobContentLength: f64,
        options: models::PageBlobCreateOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobCreateResponse> {
        let _ = (contentLength, blobContentLength, options, context);
        self.recorder.record("pageBlobHandler.create");
        Ok(recorded_response("pageBlobHandler.create"))
    }

    async fn uploadPages(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::PageBlobUploadPagesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobUploadPagesResponse> {
        let _ = (body, contentLength, options, context);
        self.recorder.record("pageBlobHandler.uploadPages");
        Ok(recorded_response("pageBlobHandler.uploadPages"))
    }

    async fn clearPages(
        &self,
        contentLength: f64,
        options: models::PageBlobClearPagesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobClearPagesResponse> {
        let _ = (contentLength, options, context);
        self.recorder.record("pageBlobHandler.clearPages");
        Ok(recorded_response("pageBlobHandler.clearPages"))
    }

    async fn uploadPagesFromURL(
        &self,
        sourceUrl: String,
        sourceRange: String,
        contentLength: f64,
        range: String,
        options: models::PageBlobUploadPagesFromURLOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobUploadPagesFromURLResponse> {
        let _ = (
            sourceUrl,
            sourceRange,
            contentLength,
            range,
            options,
            context,
        );
        self.recorder.record("pageBlobHandler.uploadPagesFromURL");
        Ok(recorded_response("pageBlobHandler.uploadPagesFromURL"))
    }

    async fn getPageRanges(
        &self,
        options: models::PageBlobGetPageRangesOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobGetPageRangesResponse> {
        let _ = (options, context);
        self.recorder.record("pageBlobHandler.getPageRanges");
        Ok(recorded_response("pageBlobHandler.getPageRanges"))
    }

    async fn getPageRangesDiff(
        &self,
        options: models::PageBlobGetPageRangesDiffOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobGetPageRangesDiffResponse> {
        let _ = (options, context);
        self.recorder.record("pageBlobHandler.getPageRangesDiff");
        Ok(recorded_response("pageBlobHandler.getPageRangesDiff"))
    }

    async fn resize(
        &self,
        blobContentLength: f64,
        options: models::PageBlobResizeOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobResizeResponse> {
        let _ = (blobContentLength, options, context);
        self.recorder.record("pageBlobHandler.resize");
        Ok(recorded_response("pageBlobHandler.resize"))
    }

    async fn updateSequenceNumber(
        &self,
        sequenceNumberAction: models::SequenceNumberActionType,
        options: models::PageBlobUpdateSequenceNumberOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobUpdateSequenceNumberResponse>
    {
        let _ = (sequenceNumberAction, options, context);
        self.recorder.record("pageBlobHandler.updateSequenceNumber");
        Ok(recorded_response("pageBlobHandler.updateSequenceNumber"))
    }

    async fn copyIncremental(
        &self,
        copySource: String,
        options: models::PageBlobCopyIncrementalOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::PageBlobCopyIncrementalResponse> {
        let _ = (copySource, options, context);
        self.recorder.record("pageBlobHandler.copyIncremental");
        Ok(recorded_response("pageBlobHandler.copyIncremental"))
    }
}

#[async_trait]
impl IAppendBlobHandler for AppendBlobHandlerHarness {
    async fn create(
        &self,
        contentLength: f64,
        options: models::AppendBlobCreateOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::AppendBlobCreateResponse> {
        let _ = (contentLength, options, context);
        self.recorder.record("appendBlobHandler.create");
        Ok(recorded_response("appendBlobHandler.create"))
    }

    async fn appendBlock(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::AppendBlobAppendBlockOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::AppendBlobAppendBlockResponse> {
        let _ = (body, contentLength, options, context);
        self.recorder.record("appendBlobHandler.appendBlock");
        Ok(recorded_response("appendBlobHandler.appendBlock"))
    }

    async fn appendBlockFromUrl(
        &self,
        sourceUrl: String,
        contentLength: f64,
        options: models::AppendBlobAppendBlockFromUrlOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::AppendBlobAppendBlockFromUrlResponse>
    {
        let _ = (sourceUrl, contentLength, options, context);
        self.recorder.record("appendBlobHandler.appendBlockFromUrl");
        Ok(recorded_response("appendBlobHandler.appendBlockFromUrl"))
    }

    async fn seal(
        &self,
        options: models::AppendBlobSealOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::AppendBlobSealResponse> {
        let _ = (options, context);
        self.recorder.record("appendBlobHandler.seal");
        Ok(recorded_response("appendBlobHandler.seal"))
    }
}

#[async_trait]
impl IBlockBlobHandler for BlockBlobHandlerHarness {
    async fn upload(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::BlockBlobUploadOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlockBlobUploadResponse> {
        let _ = (body, contentLength, options, context);
        self.recorder.record("blockBlobHandler.upload");
        Ok(recorded_response("blockBlobHandler.upload"))
    }

    async fn putBlobFromUrl(
        &self,
        contentLength: f64,
        copySource: String,
        options: models::BlockBlobPutBlobFromUrlOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlockBlobPutBlobFromUrlResponse> {
        let _ = (contentLength, copySource, options, context);
        self.recorder.record("blockBlobHandler.putBlobFromUrl");
        Ok(recorded_response("blockBlobHandler.putBlobFromUrl"))
    }

    async fn stageBlock(
        &self,
        blockId: String,
        contentLength: f64,
        body: GeneratedReadableStream,
        options: models::BlockBlobStageBlockOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlockBlobStageBlockResponse> {
        let _ = (blockId, contentLength, body, options, context);
        self.recorder.record("blockBlobHandler.stageBlock");
        Ok(recorded_response("blockBlobHandler.stageBlock"))
    }

    async fn stageBlockFromURL(
        &self,
        blockId: String,
        contentLength: f64,
        sourceUrl: String,
        options: models::BlockBlobStageBlockFromURLOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlockBlobStageBlockFromURLResponse> {
        let _ = (blockId, contentLength, sourceUrl, options, context);
        self.recorder.record("blockBlobHandler.stageBlockFromURL");
        Ok(recorded_response("blockBlobHandler.stageBlockFromURL"))
    }

    async fn commitBlockList(
        &self,
        blocks: models::BlockLookupList,
        options: models::BlockBlobCommitBlockListOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlockBlobCommitBlockListResponse> {
        let _ = (blocks, options, context);
        self.recorder.record("blockBlobHandler.commitBlockList");
        Ok(recorded_response("blockBlobHandler.commitBlockList"))
    }

    async fn getBlockList(
        &self,
        options: models::BlockBlobGetBlockListOptionalParams,
        context: Context,
    ) -> azurite_blob::generated::GeneratedResult<models::BlockBlobGetBlockListResponse> {
        let _ = (options, context);
        self.recorder.record("blockBlobHandler.getBlockList");
        Ok(recorded_response("blockBlobHandler.getBlockList"))
    }
}

struct ContractHandlers {
    recorder: CallRecorder,
    service: ServiceHandlerHarness,
    container: ContainerHandlerHarness,
    blob: BlobHandlerHarness,
    page_blob: PageBlobHandlerHarness,
    append_blob: AppendBlobHandlerHarness,
    block_blob: BlockBlobHandlerHarness,
}

impl Default for ContractHandlers {
    fn default() -> Self {
        let recorder = CallRecorder::default();
        Self {
            recorder: recorder.clone(),
            service: ServiceHandlerHarness {
                recorder: recorder.clone(),
            },
            container: ContainerHandlerHarness {
                recorder: recorder.clone(),
            },
            blob: BlobHandlerHarness {
                recorder: recorder.clone(),
            },
            page_blob: PageBlobHandlerHarness {
                recorder: recorder.clone(),
            },
            append_blob: AppendBlobHandlerHarness {
                recorder: recorder.clone(),
            },
            block_blob: BlockBlobHandlerHarness { recorder },
        }
    }
}

impl ContractHandlers {
    fn calls(&self) -> Vec<String> {
        self.recorder.entries()
    }
}

impl IHandlers for ContractHandlers {
    fn serviceHandler(&self) -> &(dyn IServiceHandler + Send + Sync) {
        &self.service
    }

    fn containerHandler(&self) -> &(dyn IContainerHandler + Send + Sync) {
        &self.container
    }

    fn blobHandler(&self) -> &(dyn IBlobHandler + Send + Sync) {
        &self.blob
    }

    fn pageBlobHandler(&self) -> &(dyn IPageBlobHandler + Send + Sync) {
        &self.page_blob
    }

    fn appendBlobHandler(&self) -> &(dyn IAppendBlobHandler + Send + Sync) {
        &self.append_blob
    }

    fn blockBlobHandler(&self) -> &(dyn IBlockBlobHandler + Send + Sync) {
        &self.block_blob
    }
}

#[derive(Debug, Deserialize)]
struct HandlerMappingMetadata {
    operation: String,
    handler: String,
    method: String,
    arguments: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HandlerInterfaceMetadata {
    #[serde(rename = "ifaceName")]
    iface_name: String,
    #[serde(rename = "handlerName")]
    handler_name: String,
    methods: Vec<HandlerMethodMetadata>,
}

#[derive(Debug, Deserialize)]
struct HandlerMethodMetadata {
    #[serde(rename = "methodName")]
    method_name: String,
    params: Vec<HandlerParamMetadata>,
    #[serde(rename = "responseType")]
    response_type: String,
}

#[derive(Debug, Deserialize)]
struct HandlerParamMetadata {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
}

fn empty_context() -> Context {
    Context::from_holder(Context::new_holder(), "phase5", None, None)
}

fn mapper(serialized_name: &str, type_name: &str) -> Mapper {
    Mapper {
        serializedName: Some(serialized_name.to_owned()),
        r#type: MapperType {
            name: type_name.to_owned(),
            ..MapperType::default()
        },
        ..Mapper::default()
    }
}

fn header_collection_mapper(prefix: &str) -> Mapper {
    Mapper {
        headerCollectionPrefix: Some(prefix.to_owned()),
        r#type: MapperType {
            name: String::from("Dictionary"),
            ..MapperType::default()
        },
        ..Mapper::default()
    }
}

fn composite_body_mapper(serialized_name: &str) -> Mapper {
    Mapper {
        serializedName: Some(serialized_name.to_owned()),
        r#type: MapperType {
            name: String::from("Composite"),
            ..MapperType::default()
        },
        ..Mapper::default()
    }
}

fn operation_parameter(parameter_path: ParameterPath, mapper: Mapper) -> OperationParameter {
    OperationParameter {
        __name: None,
        parameterPath: parameter_path,
        collectionFormat: None,
        mapper,
    }
}

fn stage_index(entries: &[String], needle: &str) -> usize {
    entries
        .iter()
        .position(|entry| entry.contains(needle))
        .expect("expected log entry to exist")
}

#[test]
fn middleware_order_matches_typescript_contract() {
    assert_eq!(
        GENERATED_MIDDLEWARE_ORDER,
        [
            "DispatchMiddleware",
            "DeserializerMiddleware",
            "HandlerMiddleware",
            "SerializerMiddleware",
            "ErrorMiddleware",
            "EndMiddleware",
        ]
    );
}

#[test]
fn context_holder_shares_state_by_path_and_isolates_other_paths() {
    let holder = Context::new_holder();
    let request = GeneratedHttpRequest::new(
        HttpMethod::GET,
        "http://127.0.0.1/",
        "http://127.0.0.1",
        "/",
    );
    let response = GeneratedHttpResponse::default();
    let context_a = Context::from_holder(
        holder.clone(),
        "generated",
        Some(request.clone()),
        Some(response.clone()),
    );
    let context_b = Context::from_holder(holder.clone(), "generated", None, None);
    let other_context = Context::from_holder(holder, "other", None, None);

    context_a.setOperation(Some(Operation::Blob_GetTags));
    context_a.setDispatchPattern(Some(String::from("/demo")));
    context_a.setContextId(Some(String::from("ctx-1")));
    context_a.setStartTime(Some(Utc::now()));
    context_a.setHandlerParameters(Some(BTreeMap::from([(
        String::from("flag"),
        GeneratedValue::Bool(true),
    )])));
    context_a.setHandlerResponses(Some(recorded_response("serviceHandler.getAccountInfo")));
    context_a.insertExtra("trace", GeneratedValue::String(String::from("value")));

    assert_eq!(context_b.path, "generated");
    assert_eq!(context_b.operation(), Some(Operation::Blob_GetTags));
    assert_eq!(context_b.dispatchPattern().as_deref(), Some("/demo"));
    assert_eq!(context_b.contextId().as_deref(), Some("ctx-1"));
    assert!(context_b.startTime().is_some());
    assert!(context_b.request().is_some());
    assert!(context_b.response().is_some());
    assert!(matches!(
        context_b
            .handlerParameters()
            .and_then(|params| params.get("flag").cloned()),
        Some(GeneratedValue::Bool(true))
    ));
    assert_eq!(
        context_b
            .extras()
            .get("trace")
            .and_then(GeneratedValue::as_string)
            .as_deref(),
        Some("value")
    );
    assert_eq!(other_context.operation(), None);
    assert!(other_context.extras().is_empty());
}

#[test]
fn operation_enum_and_handler_mapping_metadata_stay_in_lockstep() {
    let operations: Vec<String> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/generated/artifacts/metadata/operations.generated.json"
    )))
    .expect("operation metadata should deserialize");
    let mappings: Vec<HandlerMappingMetadata> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/generated/artifacts/metadata/handler_mappers.generated.json"
    )))
    .expect("handler mapping metadata should deserialize");

    assert_eq!(operations.len(), ALL_OPERATIONS.len());
    assert_eq!(mappings.len(), ALL_OPERATIONS.len());

    for (index, operation) in ALL_OPERATIONS.iter().copied().enumerate() {
        let route = getHandlerByOperation(operation);
        assert_eq!(operation.as_str(), operations[index]);
        assert_eq!(Operation::from_usize(index), Some(operation));
        assert_eq!(mappings[index].operation, operation.as_str());
        assert_eq!(route.handler, mappings[index].handler);
        assert_eq!(route.method, mappings[index].method);
        assert_eq!(route.arguments, mappings[index].arguments.as_slice());
    }

    assert_eq!(
        getHandlerByOperation(Operation::Service_GetAccountInfoWithHead).method,
        "getAccountInfo"
    );
    assert_eq!(
        getHandlerByOperation(Operation::Container_GetAccountInfoWithHead).method,
        "getAccountInfo"
    );
    assert_eq!(
        getHandlerByOperation(Operation::Blob_GetAccountInfoWithHead).method,
        "getAccountInfo"
    );
}

#[test]
fn handler_traits_cover_every_generated_interface_method() {
    let interfaces: BTreeMap<String, HandlerInterfaceMetadata> =
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/generated/artifacts/metadata/handler_interfaces.generated.json"
        )))
        .expect("handler interface metadata should deserialize");

    assert_eq!(interfaces["IServiceHandler"].iface_name, "IServiceHandler");
    assert_eq!(interfaces["IServiceHandler"].handler_name, "ServiceHandler");
    assert_eq!(interfaces["IServiceHandler"].methods.len(), 8);
    assert_eq!(interfaces["IContainerHandler"].methods.len(), 18);
    assert_eq!(interfaces["IBlobHandler"].methods.len(), 24);
    assert_eq!(interfaces["IPageBlobHandler"].methods.len(), 9);
    assert_eq!(interfaces["IAppendBlobHandler"].methods.len(), 4);
    assert_eq!(interfaces["IBlockBlobHandler"].methods.len(), 6);
    assert_eq!(
        interfaces
            .values()
            .map(|iface| iface.methods.len())
            .sum::<usize>(),
        69
    );

    let first_service_method = &interfaces["IServiceHandler"].methods[0];
    assert_eq!(first_service_method.method_name, "setProperties");
    assert_eq!(
        first_service_method
            .params
            .iter()
            .map(|param| (param.name.as_str(), param.type_name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (
                "storageServiceProperties",
                "Models.StorageServiceProperties"
            ),
            ("options", "Models.ServiceSetPropertiesOptionalParams"),
            ("context", "Context"),
        ]
    );
    assert_eq!(
        first_service_method.response_type,
        "ServiceSetPropertiesResponse"
    );

    let _ = ContractHandlers::default();
}

#[test]
fn dispatch_routes_service_account_info_requests_to_the_service_handler() {
    let logger = RecordingLogger::default();
    let mut request = GeneratedHttpRequest::new(
        HttpMethod::GET,
        "http://127.0.0.1/?restype=account&comp=properties",
        "http://127.0.0.1",
        "/",
    );
    request
        .query
        .insert(String::from("restype"), String::from("account"));
    request
        .query
        .insert(String::from("comp"), String::from("properties"));
    let context = Context::from_holder(
        Context::new_holder(),
        "generated",
        Some(request.clone()),
        Some(GeneratedHttpResponse::default()),
    );

    dispatch_middleware(&context, &request, &logger)
        .expect("dispatch should recognize the generated service account-info route");

    assert_eq!(context.operation(), Some(Operation::Service_GetAccountInfo));
}

#[tokio::test]
async fn express_factory_stages_share_context_across_the_generated_pipeline() {
    let logger = Arc::new(RecordingLogger::default());
    let handlers = Arc::new(ContractHandlers::default());
    let factory = ExpressMiddlewareFactory::new(logger.clone(), handlers.clone(), "phase5_context");

    let mut request = GeneratedHttpRequest::new(
        HttpMethod::GET,
        "http://127.0.0.1/?restype=account&comp=properties",
        "http://127.0.0.1",
        "/",
    );
    request
        .query
        .insert(String::from("restype"), String::from("account"));
    request
        .query
        .insert(String::from("comp"), String::from("properties"));

    let request = ExpressRequestAdapter::new(request);
    let response = ExpressResponseAdapter::new(GeneratedHttpResponse::default());
    let context = factory.createContext(Context::new_holder(), request.clone(), response.clone());
    context.setContextId(Some(String::from("ctx-42")));
    context.setStartTime(Some(Utc::now()));

    let mut request_for_deserialize = request.clone();
    let mut response_for_serialize = response.clone();

    factory
        .dispatch(&context, &request)
        .expect("dispatch stage should succeed");
    factory
        .deserialize(&context, &mut request_for_deserialize)
        .await
        .expect("deserialize stage should succeed");
    factory
        .handle(&context)
        .await
        .expect("handler stage should succeed");
    factory
        .serialize(&context, &mut response_for_serialize)
        .await
        .expect("serializer stage should succeed");
    factory.end(&context, &mut response_for_serialize);

    assert_eq!(context.operation(), Some(Operation::Service_GetAccountInfo));
    assert_eq!(
        handlers.calls(),
        vec![String::from("serviceHandler.getAccountInfo")]
    );
    assert_eq!(response_for_serialize.getStatusCode(), 200);
    assert!(response_for_serialize.getBodyStream().is_ended());

    let log_entries = logger.entries();
    let dispatch_index = stage_index(&log_entries, "DispatchMiddleware: Dispatching request");
    let deserialize_index =
        stage_index(&log_entries, "DeserializerMiddleware: Start deserializing");
    let handler_index = stage_index(&log_entries, "HandlerMiddleware: DeserializedParameters");
    let serializer_index = stage_index(&log_entries, "SerializerMiddleware: Start serializing");
    let end_index = stage_index(&log_entries, "EndMiddleware: End response");
    assert!(dispatch_index < deserialize_index);
    assert!(deserialize_index < handler_index);
    assert!(handler_index < serializer_index);
    assert!(serializer_index < end_index);
}

#[tokio::test]
async fn deserializer_narrows_queries_headers_and_body_into_concrete_values() {
    let logger = RecordingLogger::default();
    let context = empty_context();
    let body = r#"{"name":"example","size":3,"active":true}"#;
    let mut request = GeneratedHttpRequest::new(
        HttpMethod::PUT,
        "http://127.0.0.1/demo?enabled=true&count=42.5&ids=a,b,c",
        "http://127.0.0.1",
        "/demo",
    );
    request.bodyStream = GeneratedReadableStream::from_string(body);
    request
        .query
        .insert(String::from("enabled"), String::from("true"));
    request
        .query
        .insert(String::from("count"), String::from("42.5"));
    request
        .query
        .insert(String::from("ids"), String::from("a,b,c"));
    request.headers.insert(
        String::from("x-ms-meta-owner"),
        RequestHeaderValue::Single(String::from("alice")),
    );
    request.headers.insert(
        String::from("X-MS-META-KIND"),
        RequestHeaderValue::Single(String::from("gold")),
    );
    request.headers.insert(
        String::from("x-mode"),
        RequestHeaderValue::Single(String::from("false")),
    );
    request.headers.insert(
        String::from("content-type"),
        RequestHeaderValue::Single(String::from("application/json")),
    );

    let spec = OperationSpec {
        operation: String::from("CustomDeserialize"),
        httpMethod: String::from("PUT"),
        path: Some(String::from("/demo")),
        urlParameters: vec![],
        queryParameters: vec![
            operation_parameter(
                ParameterPath::Single(String::from("enabled")),
                mapper("enabled", "Boolean"),
            ),
            operation_parameter(
                ParameterPath::Single(String::from("count")),
                mapper("count", "Number"),
            ),
            operation_parameter(
                ParameterPath::Single(String::from("ids")),
                mapper("ids", "Sequence"),
            ),
        ],
        headerParameters: vec![
            operation_parameter(
                ParameterPath::Single(String::from("metadata")),
                header_collection_mapper("x-ms-meta-"),
            ),
            operation_parameter(
                ParameterPath::Single(String::from("mode")),
                mapper("x-mode", "Boolean"),
            ),
        ],
        requestBody: Some(RequestBodySpec {
            parameterPath: ParameterPath::Single(String::from("payload")),
            mapper: composite_body_mapper("payload"),
        }),
        contentType: None,
        responses: BTreeMap::new(),
        isXML: false,
    };

    let parameters = deserialize(&context, &mut request, &spec, &logger)
        .await
        .expect("deserialization should succeed");

    assert_eq!(
        parameters.get("enabled").and_then(GeneratedValue::as_bool),
        Some(true)
    );
    assert_eq!(
        parameters.get("count").and_then(GeneratedValue::as_number),
        Some(42.5)
    );
    match parameters.get("ids") {
        Some(GeneratedValue::Array(values)) => {
            assert_eq!(
                values
                    .iter()
                    .map(GeneratedValue::as_string)
                    .collect::<Option<Vec<_>>>()
                    .expect("sequence values should all be strings"),
                vec![String::from("a"), String::from("b"), String::from("c")]
            );
        }
        other => panic!("expected array for ids, got {other:?}"),
    }
    match parameters.get("metadata") {
        Some(GeneratedValue::Object(values)) => {
            assert_eq!(
                values
                    .get("owner")
                    .and_then(GeneratedValue::as_string)
                    .as_deref(),
                Some("alice")
            );
            assert_eq!(
                values
                    .get("KIND")
                    .and_then(GeneratedValue::as_string)
                    .as_deref(),
                Some("gold")
            );
        }
        other => panic!("expected object for metadata, got {other:?}"),
    }
    assert_eq!(
        parameters.get("mode").and_then(GeneratedValue::as_bool),
        Some(false)
    );
    match parameters.get("payload") {
        Some(GeneratedValue::Object(values)) => {
            assert_eq!(
                values
                    .get("name")
                    .and_then(GeneratedValue::as_string)
                    .as_deref(),
                Some("example")
            );
            assert_eq!(
                values.get("size").and_then(GeneratedValue::as_number),
                Some(3.0)
            );
            assert_eq!(
                values.get("active").and_then(GeneratedValue::as_bool),
                Some(true)
            );
        }
        other => panic!("expected object body payload, got {other:?}"),
    }
    assert_eq!(request.getBody().as_deref(), Some(body));
    assert_eq!(
        parameters
            .get("body")
            .and_then(GeneratedValue::as_string)
            .as_deref(),
        Some(body)
    );
}

#[tokio::test]
async fn serializer_emits_json_headers_and_body_in_wire_format() {
    let logger = RecordingLogger::default();
    let context = empty_context();
    let mut response = GeneratedHttpResponse::default();

    let header_mapper = Mapper {
        r#type: MapperType {
            name: String::from("Composite"),
            modelProperties: IndexMap::from([
                (String::from("etag"), mapper("etag", "String")),
                (
                    String::from("metadata"),
                    Mapper {
                        headerCollectionPrefix: Some(String::from("x-ms-meta-")),
                        r#type: MapperType {
                            name: String::from("Dictionary"),
                            ..MapperType::default()
                        },
                        ..Mapper::default()
                    },
                ),
            ]),
            ..MapperType::default()
        },
        ..Mapper::default()
    };
    let spec = OperationSpec {
        operation: String::from("CustomSerializeJson"),
        httpMethod: String::from("GET"),
        path: Some(String::from("/demo")),
        urlParameters: vec![],
        queryParameters: vec![],
        headerParameters: vec![],
        requestBody: None,
        contentType: None,
        responses: BTreeMap::from([(
            String::from("201"),
            ResponseSpec {
                bodyMapper: Some(composite_body_mapper("DemoResponse")),
                headersMapper: Some(header_mapper),
            },
        )]),
        isXML: false,
    };

    let mut handler_response = GeneratedResponse::new(201);
    handler_response.statusMessage = Some(String::from("Created"));
    handler_response.insert_field("etag", GeneratedValue::String(String::from("\"etag-1\"")));
    handler_response.insert_field(
        "metadata",
        GeneratedValue::Object(BTreeMap::from([
            (
                String::from("owner"),
                GeneratedValue::String(String::from("alice")),
            ),
            (String::from("attempts"), GeneratedValue::Number(2.0)),
        ])),
    );
    handler_response.body = Some(GeneratedBody::Value(GeneratedValue::Object(
        BTreeMap::from([
            (
                String::from("name"),
                GeneratedValue::String(String::from("demo")),
            ),
            (String::from("sealed"), GeneratedValue::Bool(true)),
        ]),
    )));

    serialize(&context, &mut response, &spec, &handler_response, &logger)
        .await
        .expect("serialization should succeed");

    assert_eq!(response.getStatusCode(), 201);
    assert_eq!(response.getStatusMessage(), "Created");
    assert_eq!(
        response
            .getHeader("etag")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("\"etag-1\"")
    );
    assert_eq!(
        response
            .getHeader("x-ms-meta-owner")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("alice")
    );
    assert_eq!(
        response
            .getHeader("x-ms-meta-attempts")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        response
            .getHeader("content-type")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("application/json")
    );
    let body: Value = serde_json::from_str(&response.getBodyStream().text())
        .expect("serialized JSON should be valid");
    assert_eq!(body, json!({"name": "demo", "sealed": true}));
}

#[tokio::test]
async fn serializer_emits_xml_and_stream_bodies() {
    let logger = RecordingLogger::default();
    let context = empty_context();

    let xml_spec = OperationSpec {
        operation: String::from("CustomSerializeXml"),
        httpMethod: String::from("GET"),
        path: Some(String::from("/xml")),
        urlParameters: vec![],
        queryParameters: vec![],
        headerParameters: vec![],
        requestBody: None,
        contentType: None,
        responses: BTreeMap::from([(
            String::from("200"),
            ResponseSpec {
                bodyMapper: Some(Mapper {
                    serializedName: Some(String::from("DemoResponse")),
                    xmlName: Some(String::from("DemoResponse")),
                    r#type: MapperType {
                        name: String::from("String"),
                        ..MapperType::default()
                    },
                    ..Mapper::default()
                }),
                headersMapper: None,
            },
        )]),
        isXML: true,
    };
    let mut xml_response = GeneratedHttpResponse::default();
    let mut handler_response = GeneratedResponse::new(200);
    handler_response.body = Some(GeneratedBody::Value(GeneratedValue::String(String::from(
        "demo",
    ))));

    serialize(
        &context,
        &mut xml_response,
        &xml_spec,
        &handler_response,
        &logger,
    )
    .await
    .expect("xml serialization should succeed");

    assert_eq!(
        xml_response
            .getHeader("content-type")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("application/xml")
    );
    let xml_body = xml_response.getBodyStream().text();
    assert!(xml_body.contains("<DemoResponse>demo</DemoResponse>"));

    let stream_spec = OperationSpec {
        operation: String::from("CustomSerializeStream"),
        httpMethod: String::from("GET"),
        path: Some(String::from("/stream")),
        urlParameters: vec![],
        queryParameters: vec![],
        headerParameters: vec![],
        requestBody: None,
        contentType: None,
        responses: BTreeMap::from([(
            String::from("206"),
            ResponseSpec {
                bodyMapper: Some(Mapper {
                    serializedName: Some(String::from("body")),
                    r#type: MapperType {
                        name: String::from("Stream"),
                        ..MapperType::default()
                    },
                    ..Mapper::default()
                }),
                headersMapper: None,
            },
        )]),
        isXML: false,
    };
    let mut stream_response = GeneratedHttpResponse::default();
    let mut streamed = GeneratedResponse::new(206);
    streamed.body = Some(GeneratedBody::Stream(GeneratedReadableStream::from_string(
        "chunk-1",
    )));

    serialize(
        &context,
        &mut stream_response,
        &stream_spec,
        &streamed,
        &logger,
    )
    .await
    .expect("stream serialization should succeed");

    assert_eq!(stream_response.getStatusCode(), 206);
    assert_eq!(stream_response.getBodyStream().text(), "chunk-1");
}

#[tokio::test]
async fn stream_request_bodies_remain_streams_during_deserialization() {
    let logger = RecordingLogger::default();
    let context = empty_context();
    let mut request = GeneratedHttpRequest::new(
        HttpMethod::PUT,
        "http://127.0.0.1/upload",
        "http://127.0.0.1",
        "/upload",
    );
    request.bodyStream = GeneratedReadableStream::from_string("stream-payload");

    let spec = OperationSpec {
        operation: String::from("CustomDeserializeStream"),
        httpMethod: String::from("PUT"),
        path: Some(String::from("/upload")),
        urlParameters: vec![],
        queryParameters: vec![],
        headerParameters: vec![],
        requestBody: Some(RequestBodySpec {
            parameterPath: ParameterPath::Single(String::from("payload")),
            mapper: Mapper {
                serializedName: Some(String::from("payload")),
                r#type: MapperType {
                    name: String::from("Stream"),
                    ..MapperType::default()
                },
                ..Mapper::default()
            },
        }),
        contentType: None,
        responses: BTreeMap::new(),
        isXML: false,
    };

    let parameters = deserialize(&context, &mut request, &spec, &logger)
        .await
        .expect("stream deserialization should succeed");

    assert_eq!(
        parameters
            .get("body")
            .and_then(GeneratedValue::as_stream)
            .map(|stream| stream.read_to_string())
            .as_deref(),
        Some("stream-payload")
    );
}

#[test]
fn error_middleware_writes_storage_errors_without_collapsing_them_to_500() {
    let logger = RecordingLogger::default();
    let request = GeneratedHttpRequest::new(
        HttpMethod::GET,
        "http://127.0.0.1/",
        "http://127.0.0.1",
        "/",
    );
    let context = Context::from_holder(
        Context::new_holder(),
        "generated",
        Some(request.clone()),
        Some(GeneratedHttpResponse::default()),
    );
    let mut response = GeneratedHttpResponse::default();
    let error = StorageErrorFactory::getAuthorizationFailure("ctx-403");

    error_middleware(&context, &error, &request, &mut response, &logger)
        .expect("storage error should convert into an HTTP response");
    end_middleware(&context, &mut response, &logger);

    assert_eq!(response.getStatusCode(), 403);
    assert_eq!(
        response
            .getHeader("x-ms-error-code")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("AuthorizationFailure")
    );
    assert_eq!(
        response
            .getHeader("content-type")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("application/xml")
    );
    let body = response.getBodyStream().text();
    assert!(body.contains("<Code>AuthorizationFailure</Code>"));
    assert!(body.contains("<Message>Server failed to authenticate the request."));
    assert!(response.getBodyStream().is_ended());
}

#[test]
fn error_middleware_writes_structured_wire_errors_and_end_finishes_the_response() {
    let logger = RecordingLogger::default();
    let request = GeneratedHttpRequest::new(
        HttpMethod::GET,
        "http://127.0.0.1/",
        "http://127.0.0.1",
        "/",
    );
    let context = Context::from_holder(
        Context::new_holder(),
        "generated",
        Some(request.clone()),
        Some(GeneratedHttpResponse::default()),
    );
    let mut response = GeneratedHttpResponse::default();

    let mut error = MiddlewareError::new(409, "Conflict");
    error.statusMessage = Some(String::from("Conflict"));
    error.headers = Some(BTreeMap::from([(
        String::from("x-ms-error-code"),
        ResponseHeaderValue::from("BlobAlreadyExists"),
    )]));
    error.contentType = Some(String::from("application/json"));
    error.body = Some(GeneratedValue::Object(BTreeMap::from([(
        String::from("code"),
        GeneratedValue::String(String::from("BlobAlreadyExists")),
    )])));

    error_middleware(&context, &error, &request, &mut response, &logger)
        .expect("middleware error should convert into an HTTP response");
    end_middleware(&context, &mut response, &logger);

    assert_eq!(response.getStatusCode(), 409);
    assert_eq!(response.getStatusMessage(), "Conflict");
    assert_eq!(
        response
            .getHeader("x-ms-error-code")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("BlobAlreadyExists")
    );
    assert_eq!(
        response
            .getHeader("content-type")
            .and_then(|value| value.as_single())
            .as_deref(),
        Some("application/json")
    );
    let body: Value = serde_json::from_str(&response.getBodyStream().text())
        .expect("error body should be valid JSON");
    assert_eq!(body, json!({"code": "BlobAlreadyExists"}));
    assert!(response.getBodyStream().is_ended());
}
