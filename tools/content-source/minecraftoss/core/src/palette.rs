//! Section storage: one shared value until a second value is written.
//!
//! Generation writes into flat arrays, so the non-uniform case is a direct
//! array for O(1) access. Read-only chunk snapshots share it; a changed
//! block detaches just its section instead of copying every neighbouring
//! section during features, post-processing, lighting and server snapshots.
//! Packed palettes for saves and the network are
//! produced from this representation when those consumers exist.

use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PalettedContainer<T: Copy + Eq, const N: usize> {
    Single(T),
    Direct(Arc<[T; N]>),
}

impl<T: Copy + Eq, const N: usize> PalettedContainer<T, N> {
    pub fn get(&self, index: usize) -> T {
        match self {
            Self::Single(value) => {
                assert!(index < N, "palette index {index} out of range");
                *value
            }
            Self::Direct(values) => values[index],
        }
    }

    /// Writes one entry and returns the previous value.
    pub fn set(&mut self, index: usize, value: T) -> T {
        match self {
            Self::Single(current) if *current == value => {
                assert!(index < N, "palette index {index} out of range");
                value
            }
            Self::Single(current) => {
                let previous = *current;
                let mut values: Box<[T; N]> = vec![previous; N]
                    .into_boxed_slice()
                    .try_into()
                    .unwrap_or_else(|_| unreachable!());
                values[index] = value;
                *self = Self::Direct(values.into());
                previous
            }
            Self::Direct(values) => {
                let previous=values[index];
                if previous!=value {Arc::make_mut(values)[index]=value;}
                previous
            },
        }
    }

    pub fn fill(&mut self, value: T) {
        *self = Self::Single(value);
    }

    /// Collapses a uniform direct array back to a single value.
    pub fn optimize(&mut self) {
        if let Self::Direct(values) = self {
            let first = values[0];
            if values.iter().all(|&v| v == first) {
                *self = Self::Single(first);
            }
        }
    }

    pub fn single(&self) -> Option<T> {
        match self {
            Self::Single(value) => Some(*value),
            Self::Direct(_) => None,
        }
    }

    /// All entries in index order.
    pub fn to_array(&self) -> Box<[T; N]> {
        match self {
            Self::Single(value) => vec![*value; N]
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            Self::Direct(values) => Box::new(**values),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotes_and_collapses() {
        let mut c: PalettedContainer<u16, 64> = PalettedContainer::Single(0);
        assert_eq!(c.set(5, 0), 0);
        assert!(c.single().is_some());
        assert_eq!(c.set(5, 7), 0);
        assert_eq!((c.get(5), c.get(6)), (7, 0));
        c.set(5, 0);
        c.optimize();
        assert_eq!(c, PalettedContainer::Single(0));
    }
}

#[cfg(test)]mod snapshot_storage_regressions {
    use super::*;
    #[test]fn read_only_neighbours_share_sections_but_edits_detach_only_the_changed_copy(){
        let mut original:PalettedContainer<u32,4096>=PalettedContainer::Single(0);
        original.set(17,31);original.set(4095,4097);
        let mut snapshot=original.clone();
        let pointer=|p:&PalettedContainer<u32,4096>|match p{PalettedContainer::Direct(v)=>Arc::as_ptr(v),_=>panic!("direct section required")};
        assert_eq!(pointer(&original),pointer(&snapshot));
        assert_eq!(snapshot.set(17,31),31);assert_eq!(pointer(&original),pointer(&snapshot),"same-state writes must not allocate/copy a shared section");
        assert_eq!(snapshot.set(17,99),31);assert_ne!(pointer(&original),pointer(&snapshot));
        assert_eq!(original.get(17),31);assert_eq!(snapshot.get(17),99);
        assert_eq!(original.get(4095),4097);assert_eq!(snapshot.get(4095),4097);
        let mut detached=snapshot.to_array();detached[4095]=0;assert_eq!(snapshot.get(4095),4097,"flat export remains independently owned");
    }
    #[test]fn filling_and_collapsing_a_snapshot_preserve_the_loaded_neighbour(){
        let mut original:PalettedContainer<u16,64>=PalettedContainer::Single(2);original.set(3,7);
        let mut a=original.clone();a.fill(9);assert_eq!(original.get(3),7);assert_eq!(a.get(3),9);
        let mut b=original.clone();b.set(3,2);b.optimize();assert_eq!(b.single(),Some(2));assert_eq!(original.get(3),7);
    }
}
