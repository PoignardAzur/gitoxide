use crate::pack;
use std::{
    convert::TryFrom,
    path::{Path, PathBuf},
};

mod find;
///
pub mod write;

mod verify {
    use crate::pack;
    use git_features::progress::Progress;

    impl super::Bundle {
        /// Similar to [`pack::index::File::verify_integrity()`] but more convenient to call as the presence of the
        /// pack file is a given.
        pub fn verify_integrity<C, P>(
            &self,
            verify_mode: pack::index::verify::Mode,
            traversal: pack::index::traverse::Algorithm,
            make_pack_lookup_cache: impl Fn() -> C + Send + Sync,
            thread_limit: Option<usize>,
            progress: Option<P>,
        ) -> Result<
            (git_hash::ObjectId, Option<pack::index::traverse::Outcome>, Option<P>),
            pack::index::traverse::Error<pack::index::verify::Error>,
        >
        where
            P: Progress,
            C: pack::cache::DecodeEntry,
        {
            self.index.verify_integrity(
                Some((&self.pack, verify_mode, traversal, make_pack_lookup_cache)),
                thread_limit,
                progress,
            )
        }
    }
}

/// Returned by [`Bundle::at()`]
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("An 'idx' extension is expected of an index file: '{0}'")]
    InvalidPath(PathBuf),
    #[error(transparent)]
    Pack(#[from] pack::data::header::decode::Error),
    #[error(transparent)]
    Index(#[from] pack::index::init::Error),
}

/// A way to uniquely identify the location of an object within a pack bundle
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub(crate) struct Location {
    /// The id of the pack containing the object
    pub(crate) pack_id: u32,
    /// The index at which the object can be fonud in the index file
    pub(crate) index_file_id: u32,
    /// The size of the entry of disk
    pub(crate) entry_size: usize,
}

impl Location {
    pub(crate) fn entry_range(&self, pack_offset: u64) -> pack::data::EntryRange {
        pack_offset..pack_offset + self.entry_size as u64
    }
}

/// A bundle of pack data and the corresponding pack index
pub struct Bundle {
    /// The pack file corresponding to `index`
    pub pack: pack::data::File,
    /// The index file corresponding to `pack`
    pub index: pack::index::File,
}

/// Initialization
impl Bundle {
    /// Create a `Bundle` from `path`, which is either a pack file _(*.pack)_ or an index file _(*.idx)_.
    ///
    /// The corresponding complementary file is expected to be present.
    /// Also available via [`Bundle::try_from()`].
    pub fn at(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::try_from(path.as_ref())
    }
}

impl TryFrom<&Path> for Bundle {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| Error::InvalidPath(path.to_owned()))?;
        Ok(match ext {
            "idx" => Self {
                index: pack::index::File::at(path)?,
                pack: pack::data::File::at(path.with_extension("pack"))?,
            },
            "pack" => Self {
                pack: pack::data::File::at(path)?,
                index: pack::index::File::at(path.with_extension("idx"))?,
            },
            _ => return Err(Error::InvalidPath(path.to_owned())),
        })
    }
}
