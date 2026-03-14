use std::collections::BTreeSet;
use std::future::Future;

use super::i_blob_metadata_store::BlobPrefixModel;

/// Mirrors TypeScript `PageWithDelimiter<BlobType>`.
///
/// Implements a page of blob results taking delimiters into account.
/// When a delimiter is passed to list blobs, items must be squashed into BlobPrefix items.
pub struct PageWithDelimiter<BlobType: Clone> {
    pub delimiter: Option<String>,
    pub max_results: usize,
    pub prefix: Option<String>,
    pub prefix_length: usize,

    pub blob_items: Vec<BlobType>,
    /// Use BTreeSet to preserve insertion-order semantics like TS Set.
    /// TS Set preserves insertion order; BTreeSet gives sorted order, but since
    /// input is sorted this is functionally equivalent.
    pub blob_prefixes: BTreeSet<String>,
    /// Tracks insertion-order explicitly for fidelity with TS Set iteration.
    blob_prefix_order: Vec<String>,
    pub latest_marker: String,

    is_full: bool,
    is_exhausted: bool,
}

impl<BlobType: Clone> PageWithDelimiter<BlobType> {
    pub fn new(max_results: usize, delimiter: Option<String>, prefix: Option<String>) -> Self {
        let prefix_length = prefix.as_ref().map(|p| p.len()).unwrap_or(0);
        Self {
            delimiter,
            max_results,
            prefix,
            prefix_length,
            blob_items: Vec::new(),
            blob_prefixes: BTreeSet::new(),
            blob_prefix_order: Vec::new(),
            latest_marker: String::new(),
            is_full: false,
            is_exhausted: false,
        }
    }

    /// Empty the page (useful in unit tests).
    pub fn reset(&mut self) {
        self.blob_items.clear();
        self.blob_prefixes.clear();
        self.blob_prefix_order.clear();
        self.is_full = false;
        self.is_exhausted = false;
        self.latest_marker = String::new();
    }

    fn update_full(&mut self) {
        self.is_full = self.blob_items.len() + self.blob_prefixes.len() == self.max_results;
    }

    /// addItem will add to the blob list if possible and update the full/exhausted state.
    fn add_item(&mut self, item: BlobType) -> bool {
        if self.is_exhausted {
            return false;
        }
        let mut added = false;
        if !self.is_full {
            self.blob_items.push(item);
            added = true;
        }
        self.update_full();
        // if a blob causes fullness the next item read cannot be squashed only duplicate prefixes can
        self.is_exhausted = self.is_full;
        added
    }

    /// addPrefix will add to the prefix set if possible and update the full/exhausted state.
    fn add_prefix(&mut self, prefix: String) -> bool {
        if self.is_exhausted {
            return false;
        }
        let added;
        if self.is_full {
            // the page is exhausted if this prefix is new, only matching prefixes may be 'added'
            self.is_exhausted = !self.blob_prefixes.contains(&prefix);
            added = !self.is_exhausted;
        } else {
            if !self.blob_prefixes.contains(&prefix) {
                self.blob_prefix_order.push(prefix.clone());
            }
            self.blob_prefixes.insert(prefix);
            added = true;
        }
        self.update_full();
        added
    }

    /// Add a BlobType item to the appropriate collection, update the marker.
    fn add(&mut self, name: &str, item: BlobType) -> bool {
        if self.is_exhausted {
            return false;
        }
        if name < self.latest_marker.as_str() {
            panic!("add received unsorted item. add must be called on sorted data");
        }
        let marker = if name > self.latest_marker.as_str() {
            name.to_string()
        } else {
            self.latest_marker.clone()
        };

        let added;
        if let Some(ref delimiter) = self.delimiter {
            let delimiter_pos_after_prefix = name[self.prefix_length..].find(delimiter.as_str());

            if let Some(pos) = delimiter_pos_after_prefix {
                let prefix = name[..self.prefix_length + pos + delimiter.len()].to_string();
                added = self.add_prefix(prefix);
            } else {
                added = self.add_item(item);
            }
        } else {
            added = self.add_item(item);
        }

        if added {
            self.latest_marker = marker;
        }
        added
    }

    /// Iterate over an array of blobs and add them until the page cannot accept new items.
    fn process_list<F>(&mut self, docs: &[BlobType], name_fn: &F) -> usize
    where
        F: Fn(&BlobType) -> String,
    {
        let mut added: usize = 0;
        for item in docs {
            if self.add(&name_fn(item), item.clone()) {
                added += 1;
            }
            if self.is_exhausted {
                break;
            }
        }
        added
    }

    /// Fill the page by using the provided reader function.
    ///
    /// Returns `(blob_items, blob_prefixes, continuation_token)`.
    /// Token is empty string when no more items.
    pub async fn fill<F, Fut>(
        &mut self,
        reader: F,
        namer: impl Fn(&BlobType) -> String,
    ) -> (Vec<BlobType>, Vec<BlobPrefixModel>, String)
    where
        F: Fn(usize) -> Fut,
        Fut: Future<Output = Vec<BlobType>>,
    {
        let mut offset: usize = 0;
        let mut docs = reader(offset).await;
        let mut added: usize = 0;
        while !docs.is_empty() {
            added = self.process_list(&docs, &namer);
            offset += added;
            if added < self.max_results {
                break;
            }
            docs = reader(offset).await;
        }

        let continuation = if added < docs.len() {
            self.latest_marker.clone()
        } else {
            String::new()
        };

        (self.blob_items.clone(), self.prefixes(), continuation)
    }

    /// Materializes prefixes in insertion order, mirroring TS `Set` iteration.
    fn prefixes(&self) -> Vec<BlobPrefixModel> {
        self.blob_prefix_order
            .iter()
            .map(|name| BlobPrefixModel {
                name: name.clone(),
                persistency: None,
            })
            .collect()
    }
}
