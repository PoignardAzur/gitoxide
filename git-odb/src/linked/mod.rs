//! An object database representing a list of [compound databases][compound::Db] commonly created using _git alternates_.
use crate::compound;

/// A database with a list of [compound databases][compound::Db] created by traversing git `alternates` files.
///
/// It does not contain any objects itself.
pub struct Db {
    /// The compound databases containing the actual objects.
    pub dbs: Vec<compound::Db>,
}

///
pub mod init;

///
pub mod find;

///
mod write;

///
mod iter;
