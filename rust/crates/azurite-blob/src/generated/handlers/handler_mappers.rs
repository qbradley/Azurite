use crate::generated::artifacts::operation::Operation;

#[derive(Debug, Clone, Copy)]
pub struct HandlerPath {
    pub handler: &'static str,
    pub method: &'static str,
    pub arguments: &'static [&'static str],
}

const ARGUMENTS_0: &[&str] = &["storageServiceProperties", "options"];
const ARGUMENTS_1: &[&str] = &["options"];
const ARGUMENTS_2: &[&str] = &["options"];
const ARGUMENTS_3: &[&str] = &["options"];
const ARGUMENTS_4: &[&str] = &["keyInfo", "options"];
const ARGUMENTS_5: &[&str] = &[];
const ARGUMENTS_6: &[&str] = &[];
const ARGUMENTS_7: &[&str] = &["body", "contentLength", "multipartContentType", "options"];
const ARGUMENTS_8: &[&str] = &["options"];
const ARGUMENTS_9: &[&str] = &["options"];
const ARGUMENTS_10: &[&str] = &["options"];
const ARGUMENTS_11: &[&str] = &["options"];
const ARGUMENTS_12: &[&str] = &["options"];
const ARGUMENTS_13: &[&str] = &["options"];
const ARGUMENTS_14: &[&str] = &["options"];
const ARGUMENTS_15: &[&str] = &["options"];
const ARGUMENTS_16: &[&str] = &["options"];
const ARGUMENTS_17: &[&str] = &["body", "contentLength", "multipartContentType", "options"];
const ARGUMENTS_18: &[&str] = &["options"];
const ARGUMENTS_19: &[&str] = &["options"];
const ARGUMENTS_20: &[&str] = &["leaseId", "options"];
const ARGUMENTS_21: &[&str] = &["leaseId", "options"];
const ARGUMENTS_22: &[&str] = &["options"];
const ARGUMENTS_23: &[&str] = &["leaseId", "proposedLeaseId", "options"];
const ARGUMENTS_24: &[&str] = &["options"];
const ARGUMENTS_25: &[&str] = &["delimiter", "options"];
const ARGUMENTS_26: &[&str] = &[];
const ARGUMENTS_27: &[&str] = &[];
const ARGUMENTS_28: &[&str] = &["options"];
const ARGUMENTS_29: &[&str] = &["options"];
const ARGUMENTS_30: &[&str] = &["options"];
const ARGUMENTS_31: &[&str] = &["options"];
const ARGUMENTS_32: &[&str] = &["expiryOptions", "options"];
const ARGUMENTS_33: &[&str] = &["options"];
const ARGUMENTS_34: &[&str] = &["options"];
const ARGUMENTS_35: &[&str] = &["options"];
const ARGUMENTS_36: &[&str] = &["legalHold", "options"];
const ARGUMENTS_37: &[&str] = &["options"];
const ARGUMENTS_38: &[&str] = &["options"];
const ARGUMENTS_39: &[&str] = &["leaseId", "options"];
const ARGUMENTS_40: &[&str] = &["leaseId", "options"];
const ARGUMENTS_41: &[&str] = &["leaseId", "proposedLeaseId", "options"];
const ARGUMENTS_42: &[&str] = &["options"];
const ARGUMENTS_43: &[&str] = &["options"];
const ARGUMENTS_44: &[&str] = &["copySource", "options"];
const ARGUMENTS_45: &[&str] = &["copySource", "options"];
const ARGUMENTS_46: &[&str] = &["copyId", "options"];
const ARGUMENTS_47: &[&str] = &["tier", "options"];
const ARGUMENTS_48: &[&str] = &[];
const ARGUMENTS_49: &[&str] = &[];
const ARGUMENTS_50: &[&str] = &["options"];
const ARGUMENTS_51: &[&str] = &["options"];
const ARGUMENTS_52: &[&str] = &["options"];
const ARGUMENTS_53: &[&str] = &["contentLength", "blobContentLength", "options"];
const ARGUMENTS_54: &[&str] = &["body", "contentLength", "options"];
const ARGUMENTS_55: &[&str] = &["contentLength", "options"];
const ARGUMENTS_56: &[&str] = &[
    "sourceUrl",
    "sourceRange",
    "contentLength",
    "range",
    "options",
];
const ARGUMENTS_57: &[&str] = &["options"];
const ARGUMENTS_58: &[&str] = &["options"];
const ARGUMENTS_59: &[&str] = &["blobContentLength", "options"];
const ARGUMENTS_60: &[&str] = &["sequenceNumberAction", "options"];
const ARGUMENTS_61: &[&str] = &["copySource", "options"];
const ARGUMENTS_62: &[&str] = &["contentLength", "options"];
const ARGUMENTS_63: &[&str] = &["body", "contentLength", "options"];
const ARGUMENTS_64: &[&str] = &["sourceUrl", "contentLength", "options"];
const ARGUMENTS_65: &[&str] = &["options"];
const ARGUMENTS_66: &[&str] = &["body", "contentLength", "options"];
const ARGUMENTS_67: &[&str] = &["contentLength", "copySource", "options"];
const ARGUMENTS_68: &[&str] = &["blockId", "contentLength", "body", "options"];
const ARGUMENTS_69: &[&str] = &["blockId", "contentLength", "sourceUrl", "options"];
const ARGUMENTS_70: &[&str] = &["blocks", "options"];
const ARGUMENTS_71: &[&str] = &["options"];

pub fn getHandlerByOperation(operation: Operation) -> HandlerPath {
    match operation {
        Operation::Service_SetProperties => HandlerPath {
            handler: "serviceHandler",
            method: "setProperties",
            arguments: ARGUMENTS_0,
        },
        Operation::Service_GetProperties => HandlerPath {
            handler: "serviceHandler",
            method: "getProperties",
            arguments: ARGUMENTS_1,
        },
        Operation::Service_GetStatistics => HandlerPath {
            handler: "serviceHandler",
            method: "getStatistics",
            arguments: ARGUMENTS_2,
        },
        Operation::Service_ListContainersSegment => HandlerPath {
            handler: "serviceHandler",
            method: "listContainersSegment",
            arguments: ARGUMENTS_3,
        },
        Operation::Service_GetUserDelegationKey => HandlerPath {
            handler: "serviceHandler",
            method: "getUserDelegationKey",
            arguments: ARGUMENTS_4,
        },
        Operation::Service_GetAccountInfo => HandlerPath {
            handler: "serviceHandler",
            method: "getAccountInfo",
            arguments: ARGUMENTS_5,
        },
        Operation::Service_GetAccountInfoWithHead => HandlerPath {
            handler: "serviceHandler",
            method: "getAccountInfo",
            arguments: ARGUMENTS_6,
        },
        Operation::Service_SubmitBatch => HandlerPath {
            handler: "serviceHandler",
            method: "submitBatch",
            arguments: ARGUMENTS_7,
        },
        Operation::Service_FilterBlobs => HandlerPath {
            handler: "serviceHandler",
            method: "filterBlobs",
            arguments: ARGUMENTS_8,
        },
        Operation::Container_Create => HandlerPath {
            handler: "containerHandler",
            method: "create",
            arguments: ARGUMENTS_9,
        },
        Operation::Container_GetProperties => HandlerPath {
            handler: "containerHandler",
            method: "getProperties",
            arguments: ARGUMENTS_10,
        },
        Operation::Container_GetPropertiesWithHead => HandlerPath {
            handler: "containerHandler",
            method: "getProperties",
            arguments: ARGUMENTS_11,
        },
        Operation::Container_Delete => HandlerPath {
            handler: "containerHandler",
            method: "delete",
            arguments: ARGUMENTS_12,
        },
        Operation::Container_SetMetadata => HandlerPath {
            handler: "containerHandler",
            method: "setMetadata",
            arguments: ARGUMENTS_13,
        },
        Operation::Container_GetAccessPolicy => HandlerPath {
            handler: "containerHandler",
            method: "getAccessPolicy",
            arguments: ARGUMENTS_14,
        },
        Operation::Container_SetAccessPolicy => HandlerPath {
            handler: "containerHandler",
            method: "setAccessPolicy",
            arguments: ARGUMENTS_15,
        },
        Operation::Container_Restore => HandlerPath {
            handler: "containerHandler",
            method: "restore",
            arguments: ARGUMENTS_16,
        },
        Operation::Container_SubmitBatch => HandlerPath {
            handler: "containerHandler",
            method: "submitBatch",
            arguments: ARGUMENTS_17,
        },
        Operation::Container_FilterBlobs => HandlerPath {
            handler: "containerHandler",
            method: "filterBlobs",
            arguments: ARGUMENTS_18,
        },
        Operation::Container_AcquireLease => HandlerPath {
            handler: "containerHandler",
            method: "acquireLease",
            arguments: ARGUMENTS_19,
        },
        Operation::Container_ReleaseLease => HandlerPath {
            handler: "containerHandler",
            method: "releaseLease",
            arguments: ARGUMENTS_20,
        },
        Operation::Container_RenewLease => HandlerPath {
            handler: "containerHandler",
            method: "renewLease",
            arguments: ARGUMENTS_21,
        },
        Operation::Container_BreakLease => HandlerPath {
            handler: "containerHandler",
            method: "breakLease",
            arguments: ARGUMENTS_22,
        },
        Operation::Container_ChangeLease => HandlerPath {
            handler: "containerHandler",
            method: "changeLease",
            arguments: ARGUMENTS_23,
        },
        Operation::Container_ListBlobFlatSegment => HandlerPath {
            handler: "containerHandler",
            method: "listBlobFlatSegment",
            arguments: ARGUMENTS_24,
        },
        Operation::Container_ListBlobHierarchySegment => HandlerPath {
            handler: "containerHandler",
            method: "listBlobHierarchySegment",
            arguments: ARGUMENTS_25,
        },
        Operation::Container_GetAccountInfo => HandlerPath {
            handler: "containerHandler",
            method: "getAccountInfo",
            arguments: ARGUMENTS_26,
        },
        Operation::Container_GetAccountInfoWithHead => HandlerPath {
            handler: "containerHandler",
            method: "getAccountInfo",
            arguments: ARGUMENTS_27,
        },
        Operation::Blob_Download => HandlerPath {
            handler: "blobHandler",
            method: "download",
            arguments: ARGUMENTS_28,
        },
        Operation::Blob_GetProperties => HandlerPath {
            handler: "blobHandler",
            method: "getProperties",
            arguments: ARGUMENTS_29,
        },
        Operation::Blob_Delete => HandlerPath {
            handler: "blobHandler",
            method: "delete",
            arguments: ARGUMENTS_30,
        },
        Operation::Blob_Undelete => HandlerPath {
            handler: "blobHandler",
            method: "undelete",
            arguments: ARGUMENTS_31,
        },
        Operation::Blob_SetExpiry => HandlerPath {
            handler: "blobHandler",
            method: "setExpiry",
            arguments: ARGUMENTS_32,
        },
        Operation::Blob_SetHTTPHeaders => HandlerPath {
            handler: "blobHandler",
            method: "setHTTPHeaders",
            arguments: ARGUMENTS_33,
        },
        Operation::Blob_SetImmutabilityPolicy => HandlerPath {
            handler: "blobHandler",
            method: "setImmutabilityPolicy",
            arguments: ARGUMENTS_34,
        },
        Operation::Blob_DeleteImmutabilityPolicy => HandlerPath {
            handler: "blobHandler",
            method: "deleteImmutabilityPolicy",
            arguments: ARGUMENTS_35,
        },
        Operation::Blob_SetLegalHold => HandlerPath {
            handler: "blobHandler",
            method: "setLegalHold",
            arguments: ARGUMENTS_36,
        },
        Operation::Blob_SetMetadata => HandlerPath {
            handler: "blobHandler",
            method: "setMetadata",
            arguments: ARGUMENTS_37,
        },
        Operation::Blob_AcquireLease => HandlerPath {
            handler: "blobHandler",
            method: "acquireLease",
            arguments: ARGUMENTS_38,
        },
        Operation::Blob_ReleaseLease => HandlerPath {
            handler: "blobHandler",
            method: "releaseLease",
            arguments: ARGUMENTS_39,
        },
        Operation::Blob_RenewLease => HandlerPath {
            handler: "blobHandler",
            method: "renewLease",
            arguments: ARGUMENTS_40,
        },
        Operation::Blob_ChangeLease => HandlerPath {
            handler: "blobHandler",
            method: "changeLease",
            arguments: ARGUMENTS_41,
        },
        Operation::Blob_BreakLease => HandlerPath {
            handler: "blobHandler",
            method: "breakLease",
            arguments: ARGUMENTS_42,
        },
        Operation::Blob_CreateSnapshot => HandlerPath {
            handler: "blobHandler",
            method: "createSnapshot",
            arguments: ARGUMENTS_43,
        },
        Operation::Blob_StartCopyFromURL => HandlerPath {
            handler: "blobHandler",
            method: "startCopyFromURL",
            arguments: ARGUMENTS_44,
        },
        Operation::Blob_CopyFromURL => HandlerPath {
            handler: "blobHandler",
            method: "copyFromURL",
            arguments: ARGUMENTS_45,
        },
        Operation::Blob_AbortCopyFromURL => HandlerPath {
            handler: "blobHandler",
            method: "abortCopyFromURL",
            arguments: ARGUMENTS_46,
        },
        Operation::Blob_SetTier => HandlerPath {
            handler: "blobHandler",
            method: "setTier",
            arguments: ARGUMENTS_47,
        },
        Operation::Blob_GetAccountInfo => HandlerPath {
            handler: "blobHandler",
            method: "getAccountInfo",
            arguments: ARGUMENTS_48,
        },
        Operation::Blob_GetAccountInfoWithHead => HandlerPath {
            handler: "blobHandler",
            method: "getAccountInfo",
            arguments: ARGUMENTS_49,
        },
        Operation::Blob_Query => HandlerPath {
            handler: "blobHandler",
            method: "query",
            arguments: ARGUMENTS_50,
        },
        Operation::Blob_GetTags => HandlerPath {
            handler: "blobHandler",
            method: "getTags",
            arguments: ARGUMENTS_51,
        },
        Operation::Blob_SetTags => HandlerPath {
            handler: "blobHandler",
            method: "setTags",
            arguments: ARGUMENTS_52,
        },
        Operation::PageBlob_Create => HandlerPath {
            handler: "pageBlobHandler",
            method: "create",
            arguments: ARGUMENTS_53,
        },
        Operation::PageBlob_UploadPages => HandlerPath {
            handler: "pageBlobHandler",
            method: "uploadPages",
            arguments: ARGUMENTS_54,
        },
        Operation::PageBlob_ClearPages => HandlerPath {
            handler: "pageBlobHandler",
            method: "clearPages",
            arguments: ARGUMENTS_55,
        },
        Operation::PageBlob_UploadPagesFromURL => HandlerPath {
            handler: "pageBlobHandler",
            method: "uploadPagesFromURL",
            arguments: ARGUMENTS_56,
        },
        Operation::PageBlob_GetPageRanges => HandlerPath {
            handler: "pageBlobHandler",
            method: "getPageRanges",
            arguments: ARGUMENTS_57,
        },
        Operation::PageBlob_GetPageRangesDiff => HandlerPath {
            handler: "pageBlobHandler",
            method: "getPageRangesDiff",
            arguments: ARGUMENTS_58,
        },
        Operation::PageBlob_Resize => HandlerPath {
            handler: "pageBlobHandler",
            method: "resize",
            arguments: ARGUMENTS_59,
        },
        Operation::PageBlob_UpdateSequenceNumber => HandlerPath {
            handler: "pageBlobHandler",
            method: "updateSequenceNumber",
            arguments: ARGUMENTS_60,
        },
        Operation::PageBlob_CopyIncremental => HandlerPath {
            handler: "pageBlobHandler",
            method: "copyIncremental",
            arguments: ARGUMENTS_61,
        },
        Operation::AppendBlob_Create => HandlerPath {
            handler: "appendBlobHandler",
            method: "create",
            arguments: ARGUMENTS_62,
        },
        Operation::AppendBlob_AppendBlock => HandlerPath {
            handler: "appendBlobHandler",
            method: "appendBlock",
            arguments: ARGUMENTS_63,
        },
        Operation::AppendBlob_AppendBlockFromUrl => HandlerPath {
            handler: "appendBlobHandler",
            method: "appendBlockFromUrl",
            arguments: ARGUMENTS_64,
        },
        Operation::AppendBlob_Seal => HandlerPath {
            handler: "appendBlobHandler",
            method: "seal",
            arguments: ARGUMENTS_65,
        },
        Operation::BlockBlob_Upload => HandlerPath {
            handler: "blockBlobHandler",
            method: "upload",
            arguments: ARGUMENTS_66,
        },
        Operation::BlockBlob_PutBlobFromUrl => HandlerPath {
            handler: "blockBlobHandler",
            method: "putBlobFromUrl",
            arguments: ARGUMENTS_67,
        },
        Operation::BlockBlob_StageBlock => HandlerPath {
            handler: "blockBlobHandler",
            method: "stageBlock",
            arguments: ARGUMENTS_68,
        },
        Operation::BlockBlob_StageBlockFromURL => HandlerPath {
            handler: "blockBlobHandler",
            method: "stageBlockFromURL",
            arguments: ARGUMENTS_69,
        },
        Operation::BlockBlob_CommitBlockList => HandlerPath {
            handler: "blockBlobHandler",
            method: "commitBlockList",
            arguments: ARGUMENTS_70,
        },
        Operation::BlockBlob_GetBlockList => HandlerPath {
            handler: "blockBlobHandler",
            method: "getBlockList",
            arguments: ARGUMENTS_71,
        },
    }
}
