use std::sync::Arc;

use crate::errors::StorageError;

use super::i_blob_metadata_store::IBlobMetadataStore;

/// Mirrors TypeScript `enum State`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    ListingExtentsInBlobs,
    ListingExtentsInBlocks,
    Done,
}

/// Mirrors TypeScript `BlobReferredExtentsAsyncIterator`.
///
/// An async iterator which enumerates all extents being used.
/// Yields batches of extent IDs (`Vec<String>`) in two phases:
///   1. Walk committed blobs (including snapshots and uncommitted blobs)
///   2. Walk uncommitted block extent chunks
pub struct BlobReferredExtentsAsyncIterator {
    state: State,
    blob_listing_marker: Option<String>,
    block_listing_marker: Option<String>,
    blob_metadata_store: Arc<dyn IBlobMetadataStore>,
}

impl BlobReferredExtentsAsyncIterator {
    pub fn new(blob_metadata_store: Arc<dyn IBlobMetadataStore>) -> Self {
        Self {
            state: State::ListingExtentsInBlobs,
            blob_listing_marker: None,
            block_listing_marker: None,
            blob_metadata_store,
        }
    }

    /// Mirrors TypeScript `next(): Promise<IteratorResult<string[]>>`.
    ///
    /// Returns `Ok((extents, done))` where `done` indicates iteration is complete.
    pub async fn next(&mut self) -> Result<(Vec<String>, bool), StorageError> {
        match self.state {
            State::ListingExtentsInBlobs => {
                let (blobs, marker) = self
                    .blob_metadata_store
                    .listAllBlobs(
                        None, // maxResults
                        self.blob_listing_marker.as_deref(),
                        Some(true), // includeSnapshots
                        Some(true), // includeUncommittedBlobs
                    )
                    .await?;

                self.blob_listing_marker = marker.clone();
                if marker.is_none() {
                    self.state = State::ListingExtentsInBlocks;
                }

                let mut extents = Vec::new();
                for blob in &blobs {
                    // Extract extent IDs from committed blocks
                    if let Some(ref committed_blocks) = blob.committedBlocksInOrder {
                        for block in committed_blocks {
                            extents.push(block.persistency.id.clone());
                        }
                    }
                    // Extract extent IDs from page ranges
                    if let Some(ref page_ranges) = blob.pageRangesInOrder {
                        for range in page_ranges {
                            extents.push(range.persistency.id.clone());
                        }
                    }
                    // Extract blob-level persistency pointer
                    if let Some(ref persistency) = blob.persistency {
                        extents.push(persistency.id.clone());
                    }
                }

                Ok((extents, false))
            }
            State::ListingExtentsInBlocks => {
                let (blocks, marker) = self
                    .blob_metadata_store
                    .listUncommittedBlockPersistencyChunks(
                        self.block_listing_marker.as_deref(),
                        None,
                    )
                    .await?;

                self.block_listing_marker = marker.clone();
                if marker.is_none() {
                    self.state = State::Done;
                }

                let extents: Vec<String> = blocks.iter().map(|block| block.id.clone()).collect();
                Ok((extents, false))
            }
            State::Done => Ok((vec![], true)),
        }
    }
}
