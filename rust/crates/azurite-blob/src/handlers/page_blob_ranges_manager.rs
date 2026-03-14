use crate::generated::artifacts::models::{GeneratedValue, PageRange};
use crate::handlers::i_page_blob_ranges_manager::IPageBlobRangesManager;
use crate::persistence::{IExtentChunk, PersistencyPageRange, ZERO_EXTENT_ID};

#[derive(Debug, Clone, Default)]
pub struct PageBlobRangesManager;

impl PageBlobRangesManager {
    pub fn new() -> Self {
        Self
    }

    pub fn merge_range(&self, ranges: &mut Vec<PersistencyPageRange>, range: PersistencyPageRange) {
        let start = Self::page_range_start(&range.range);
        let end = Self::page_range_end(&range.range);
        let persistency = range.persistency;

        let (impacted_start_index, impacted_end_index) =
            self.select_impacted_ranges(ranges, start, end);
        let impacted_ranges_count = if impacted_start_index == usize::MAX
            || impacted_end_index < 0
            || impacted_start_index as isize > impacted_end_index
        {
            0
        } else {
            impacted_end_index as usize - impacted_start_index + 1
        };

        let new_range = Self::persistency_page_range(start, end, persistency);

        if impacted_ranges_count == 0 {
            if impacted_start_index == usize::MAX {
                ranges.push(new_range);
            } else {
                ranges.insert(impacted_start_index, new_range);
            }
            return;
        }

        let first_impacted_range = ranges[impacted_start_index].clone();
        let last_impacted_range = ranges[impacted_end_index as usize].clone();
        let mut new_ranges = Vec::new();

        let first_start = Self::page_range_start(&first_impacted_range.range);
        let first_end = Self::page_range_end(&first_impacted_range.range);
        if first_end >= start && first_start < start {
            new_ranges.push(Self::persistency_page_range(
                first_start,
                start - 1,
                IExtentChunk {
                    id: first_impacted_range.persistency.id.clone(),
                    offset: first_impacted_range.persistency.offset,
                    count: start - first_start,
                },
            ));
        }

        new_ranges.push(new_range);

        let last_start = Self::page_range_start(&last_impacted_range.range);
        let last_end = Self::page_range_end(&last_impacted_range.range);
        if end >= last_start && end < last_end {
            new_ranges.push(Self::persistency_page_range(
                end + 1,
                last_end,
                IExtentChunk {
                    id: last_impacted_range.persistency.id.clone(),
                    offset: last_impacted_range.persistency.offset + (end + 1 - last_start),
                    count: last_end - end,
                },
            ));
        }

        ranges.splice(
            impacted_start_index..(impacted_start_index + impacted_ranges_count),
            new_ranges,
        );
    }

    pub fn clear_range(&self, ranges: &mut Vec<PersistencyPageRange>, range: PageRange) {
        let start = Self::page_range_start(&range);
        let end = Self::page_range_end(&range);

        let (impacted_start_index, impacted_end_index) =
            self.select_impacted_ranges(ranges, start, end);
        let impacted_ranges_count = if impacted_start_index == usize::MAX
            || impacted_end_index < 0
            || impacted_start_index as isize > impacted_end_index
        {
            0
        } else {
            impacted_end_index as usize - impacted_start_index + 1
        };

        if impacted_ranges_count == 0 {
            return;
        }

        let first_impacted_range = ranges[impacted_start_index].clone();
        let last_impacted_range = ranges[impacted_end_index as usize].clone();
        let mut new_ranges = Vec::new();

        let first_start = Self::page_range_start(&first_impacted_range.range);
        let first_end = Self::page_range_end(&first_impacted_range.range);
        if first_end >= start && first_start < start {
            new_ranges.push(Self::persistency_page_range(
                first_start,
                start - 1,
                IExtentChunk {
                    id: first_impacted_range.persistency.id.clone(),
                    offset: first_impacted_range.persistency.offset,
                    count: start - first_start,
                },
            ));
        }

        let last_start = Self::page_range_start(&last_impacted_range.range);
        let last_end = Self::page_range_end(&last_impacted_range.range);
        if end >= last_start && end < last_end {
            new_ranges.push(Self::persistency_page_range(
                end + 1,
                last_end,
                IExtentChunk {
                    id: last_impacted_range.persistency.id.clone(),
                    offset: last_impacted_range.persistency.offset + (end + 1 - last_start),
                    count: last_end - end,
                },
            ));
        }

        ranges.splice(
            impacted_start_index..(impacted_start_index + impacted_ranges_count),
            new_ranges,
        );
    }

    pub fn cut_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        range: PageRange,
    ) -> Vec<PersistencyPageRange> {
        let start = Self::page_range_start(&range);
        let end = Self::page_range_end(&range);

        let (impacted_start_index, impacted_end_index) =
            self.select_impacted_ranges(ranges, start, end);
        let impacted_ranges_count = if impacted_start_index == usize::MAX
            || impacted_end_index < 0
            || impacted_start_index as isize > impacted_end_index
        {
            0
        } else {
            impacted_end_index as usize - impacted_start_index + 1
        };

        if impacted_ranges_count == 0 {
            return Vec::new();
        }

        let mut impacted_ranges =
            ranges[impacted_start_index..=impacted_end_index as usize].to_vec();

        if let Some(first_impacted_range) = impacted_ranges.first().cloned() {
            let first_start = Self::page_range_start(&first_impacted_range.range);
            let first_end = Self::page_range_end(&first_impacted_range.range);
            if first_end >= start && first_start < start {
                impacted_ranges[0] = Self::persistency_page_range(
                    start,
                    first_end,
                    IExtentChunk {
                        id: first_impacted_range.persistency.id,
                        offset: first_impacted_range.persistency.offset + (start - first_start),
                        count: first_impacted_range.persistency.count - (start - first_start),
                    },
                );
            }
        }

        if let Some(last_impacted_range) = impacted_ranges.last().cloned() {
            let last_start = Self::page_range_start(&last_impacted_range.range);
            let last_end = Self::page_range_end(&last_impacted_range.range);
            if end >= last_start && end < last_end {
                let last_index = impacted_ranges.len() - 1;
                impacted_ranges[last_index] = Self::persistency_page_range(
                    last_start,
                    end,
                    IExtentChunk {
                        id: last_impacted_range.persistency.id,
                        offset: last_impacted_range.persistency.offset,
                        count: last_impacted_range.persistency.count - (last_end - end),
                    },
                );
            }
        }

        impacted_ranges
    }

    pub fn fill_zero_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        range: PageRange,
    ) -> Vec<PersistencyPageRange> {
        let ranges = self.cut_ranges(ranges, range.clone());
        let start = Self::page_range_start(&range);
        let end = Self::page_range_end(&range);
        let mut filled_ranges = Vec::new();

        if ranges.is_empty() {
            filled_ranges.push(Self::persistency_page_range(
                start,
                end,
                IExtentChunk {
                    id: ZERO_EXTENT_ID.to_string(),
                    offset: 0,
                    count: end + 1 - start,
                },
            ));
            return filled_ranges;
        }

        let first_start = Self::page_range_start(&ranges[0].range);
        let last_end = Self::page_range_end(&ranges[ranges.len() - 1].range);

        if start < first_start {
            filled_ranges.push(Self::persistency_page_range(
                start,
                first_start - 1,
                IExtentChunk {
                    id: ZERO_EXTENT_ID.to_string(),
                    offset: 0,
                    count: first_start - start,
                },
            ));
        }

        for window in ranges.windows(2) {
            let current = window[0].clone();
            let next = window[1].clone();
            let current_end = Self::page_range_end(&current.range);
            let next_start = Self::page_range_start(&next.range);
            filled_ranges.push(current);

            let gap = next_start - 1 - current_end;
            if gap > 0 {
                filled_ranges.push(Self::persistency_page_range(
                    current_end + 1,
                    next_start - 1,
                    IExtentChunk {
                        id: ZERO_EXTENT_ID.to_string(),
                        offset: 0,
                        count: gap,
                    },
                ));
            }
        }

        filled_ranges.push(ranges[ranges.len() - 1].clone());

        if last_end < end {
            filled_ranges.push(Self::persistency_page_range(
                last_end + 1,
                end,
                IExtentChunk {
                    id: ZERO_EXTENT_ID.to_string(),
                    offset: 0,
                    count: end - last_end,
                },
            ));
        }

        filled_ranges
    }

    fn select_impacted_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        start: i64,
        end: i64,
    ) -> (usize, isize) {
        if ranges.is_empty() {
            return (usize::MAX, -1);
        }

        assert!(
            !(start > end || start < 0),
            "PageBlobRangesManager:selectImpactedRanges() start must less equal than end parameter, start must larger equal than 0."
        );

        (
            self.locate_first_impacted_range(ranges, 0, ranges.len(), start),
            self.locate_last_impacted_range(ranges, 0, ranges.len(), end),
        )
    }

    fn locate_first_impacted_range(
        &self,
        ranges: &[PersistencyPageRange],
        search_start: usize,
        search_end: usize,
        position: i64,
    ) -> usize {
        if ranges.is_empty() || search_start >= search_end {
            return usize::MAX;
        }

        if search_start == search_end - 1 {
            let range = &ranges[search_start];
            return if self.position_in_range(range, position)
                || position < Self::page_range_start(&range.range)
            {
                search_start
            } else {
                usize::MAX
            };
        }

        let search_mid = (search_start + search_end) / 2;
        let index_in_left =
            self.locate_first_impacted_range(ranges, search_start, search_mid, position);
        if index_in_left != usize::MAX {
            return index_in_left;
        }

        if self.position_in_range(&ranges[search_mid], position)
            || position < Self::page_range_start(&ranges[search_mid].range)
        {
            search_mid
        } else {
            self.locate_first_impacted_range(ranges, search_mid + 1, search_end, position)
        }
    }

    fn locate_last_impacted_range(
        &self,
        ranges: &[PersistencyPageRange],
        search_start: usize,
        search_end: usize,
        position: i64,
    ) -> isize {
        if ranges.is_empty() || search_start >= search_end {
            return -1;
        }

        if search_start == search_end - 1 {
            let range = &ranges[search_start];
            return if self.position_in_range(range, position)
                || position > Self::page_range_end(&range.range)
            {
                search_start as isize
            } else {
                -1
            };
        }

        let search_mid = (search_start + search_end) / 2;
        let index_in_right =
            self.locate_last_impacted_range(ranges, search_mid + 1, search_end, position);
        if index_in_right > -1 {
            return index_in_right;
        }

        if self.position_in_range(&ranges[search_mid], position)
            || position > Self::page_range_end(&ranges[search_mid].range)
        {
            search_mid as isize
        } else {
            self.locate_last_impacted_range(ranges, search_start, search_mid, position)
        }
    }

    fn position_in_range(&self, range: &PersistencyPageRange, position: i64) -> bool {
        let start = Self::page_range_start(&range.range);
        let end = Self::page_range_end(&range.range);
        position >= start && position <= end
    }

    fn page_range_start(range: &PageRange) -> i64 {
        range
            .get("start")
            .and_then(|value| value.as_number())
            .unwrap_or_default() as i64
    }

    fn page_range_end(range: &PageRange) -> i64 {
        range
            .get("end")
            .and_then(|value| value.as_number())
            .unwrap_or_default() as i64
    }

    fn persistency_page_range(
        start: i64,
        end: i64,
        persistency: IExtentChunk,
    ) -> PersistencyPageRange {
        let mut range = PageRange::default();
        range.insert("start".to_string(), GeneratedValue::Number(start as f64));
        range.insert("end".to_string(), GeneratedValue::Number(end as f64));
        PersistencyPageRange { range, persistency }
    }
}

impl IPageBlobRangesManager for PageBlobRangesManager {
    fn merge_range(&self, ranges: &mut Vec<PersistencyPageRange>, range: PersistencyPageRange) {
        PageBlobRangesManager::merge_range(self, ranges, range)
    }

    fn clear_range(&self, ranges: &mut Vec<PersistencyPageRange>, range: PageRange) {
        PageBlobRangesManager::clear_range(self, ranges, range)
    }

    fn cut_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        range: PageRange,
    ) -> Vec<PersistencyPageRange> {
        PageBlobRangesManager::cut_ranges(self, ranges, range)
    }

    fn fill_zero_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        range: PageRange,
    ) -> Vec<PersistencyPageRange> {
        PageBlobRangesManager::fill_zero_ranges(self, ranges, range)
    }
}
