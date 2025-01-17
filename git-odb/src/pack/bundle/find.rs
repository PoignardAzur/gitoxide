use crate::{data, pack};

impl pack::Bundle {
    /// Find an object with the given [`ObjectId`][git_hash::ObjectId] and place its data into `out`.
    ///
    /// [`cache`][pack::cache::DecodeEntry] is used to accelerate the lookup.
    ///
    /// **Note** that ref deltas are automatically resolved within this pack only, which makes this implementation unusable
    /// for thin packs, which by now are expected to be resolved already.
    pub fn find<'a>(
        &self,
        id: impl AsRef<git_hash::oid>,
        out: &'a mut Vec<u8>,
        cache: &mut impl pack::cache::DecodeEntry,
    ) -> Result<Option<data::Object<'a>>, pack::data::decode_entry::Error> {
        let idx = match self.index.lookup(id) {
            Some(idx) => idx,
            None => return Ok(None),
        };
        self.internal_get_object_by_index(idx, out, cache).map(Some)
    }

    /// Internal-use function to look up an object index. Used to avoid double-lookups in
    /// [compound::Db::find()][crate::compound::Db::find()]. (The polonius borrow-checker would support this via the 'find'
    /// function, so this can be [simplified](https://github.com/Byron/gitoxide/blob/0c5f4043da4615820cb180804a81c2d4fe75fe5e/git-odb/src/compound/locate.rs#L47)
    /// once polonius is stable.)
    pub(crate) fn internal_find_pack_index(&self, id: &git_hash::oid) -> Option<u32> {
        self.index.lookup(id)
    }

    /// Internal-use function to get an object given an index previously returned from
    /// internal_find_pack_index.
    pub(crate) fn internal_get_object_by_index<'a>(
        &self,
        idx: u32,
        out: &'a mut Vec<u8>,
        cache: &mut impl pack::cache::DecodeEntry,
    ) -> Result<data::Object<'a>, pack::data::decode_entry::Error> {
        let ofs = self.index.pack_offset_at_index(idx);
        let pack_entry = self.pack.entry(ofs);
        let header_size = pack_entry.header_size();
        self.pack
            .decode_entry(
                pack_entry,
                out,
                |id, _out| {
                    self.index.lookup(id).map(|idx| {
                        pack::data::ResolvedBase::InPack(self.pack.entry(self.index.pack_offset_at_index(idx)))
                    })
                },
                cache,
            )
            .map(move |r| crate::data::Object {
                kind: r.kind,
                data: out.as_slice(),
                pack_location: Some(pack::bundle::Location {
                    pack_id: self.pack.id,
                    index_file_id: idx,
                    entry_size: r.compressed_size + header_size,
                }),
            })
    }
}
