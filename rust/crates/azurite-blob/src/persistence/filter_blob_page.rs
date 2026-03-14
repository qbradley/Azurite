use std::future::Future;

/// Mirrors TypeScript `FilterBlobPage<FilterBlobType>`.
///
/// Implements a page of filtered blob results.
/// When maxResults is smaller than the number of items in the metadata source,
/// multiple reads may be necessary.
pub struct FilterBlobPage<FilterBlobType: Clone> {
    pub max_results: usize,
    pub filter_blob_items: Vec<FilterBlobType>,
    pub latest_marker: String,
    is_full: bool,
    is_exhausted: bool,
}

impl<FilterBlobType: Clone> FilterBlobPage<FilterBlobType> {
    pub fn new(max_results: usize) -> Self {
        Self {
            max_results,
            filter_blob_items: Vec::new(),
            latest_marker: String::new(),
            is_full: false,
            is_exhausted: false,
        }
    }

    /// Empty the page (useful in unit tests).
    pub fn reset(&mut self) {
        self.filter_blob_items.clear();
        self.is_full = false;
        self.is_exhausted = false;
        self.latest_marker = String::new();
    }

    fn update_full(&mut self) {
        self.is_full = self.filter_blob_items.len() == self.max_results;
    }

    /// addItem will add to the blob list if possible and update the full/exhausted state.
    fn add_item(&mut self, item: FilterBlobType) -> bool {
        if self.is_exhausted {
            return false;
        }
        let mut added = false;
        if !self.is_full {
            self.filter_blob_items.push(item);
            added = true;
        }
        self.update_full();
        // if a blob causes fullness the next item read cannot be squashed
        self.is_exhausted = self.is_full;
        added
    }

    /// Add a FilterBlobType item, update the marker.
    fn add(&mut self, name: &str, item: FilterBlobType) -> bool {
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
        let added = self.add_item(item);
        if added {
            self.latest_marker = marker;
        }
        added
    }

    /// Iterate over an array of items and add them until the page cannot accept new items.
    fn process_list<F>(&mut self, docs: &[FilterBlobType], name_fn: &F) -> usize
    where
        F: Fn(&FilterBlobType) -> String,
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
    /// Returns `(items, continuation_token)`. Token is empty string when no more items.
    pub async fn fill<F, Fut>(
        &mut self,
        reader: F,
        namer: impl Fn(&FilterBlobType) -> String,
    ) -> (Vec<FilterBlobType>, String)
    where
        F: Fn(usize) -> Fut,
        Fut: Future<Output = Vec<FilterBlobType>>,
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
        (self.filter_blob_items.clone(), continuation)
    }
}
