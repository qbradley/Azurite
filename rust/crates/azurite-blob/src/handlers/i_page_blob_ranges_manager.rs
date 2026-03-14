use crate::generated::artifacts::models::PageRange;
use crate::persistence::PersistencyPageRange;

pub trait IPageBlobRangesManager {
    fn merge_range(&self, ranges: &mut Vec<PersistencyPageRange>, range: PersistencyPageRange);
    fn clear_range(&self, ranges: &mut Vec<PersistencyPageRange>, range: PageRange);
    fn cut_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        range: PageRange,
    ) -> Vec<PersistencyPageRange>;
    fn fill_zero_ranges(
        &self,
        ranges: &[PersistencyPageRange],
        range: PageRange,
    ) -> Vec<PersistencyPageRange>;
}
