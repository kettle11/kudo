//mod multi_queries;
mod exclusive_world_query;
mod multi_query;
mod single_query;

pub use exclusive_world_query::*;
pub use multi_query::*;
pub use single_query::*;

use crate::{Error, World, WorldTrait};

#[derive(Clone)]
pub enum ReadOrWrite {
    Read,
    Write,
}

pub struct ResourceBorrows {
    pub(crate) writes: Vec<usize>,
    pub(crate) reads: Vec<usize>,
}

impl ResourceBorrows {
    pub(crate) fn new() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub(crate) fn extend(&mut self, resource_borrows: &ResourceBorrows) {
        self.reads.extend_from_slice(&resource_borrows.reads);
        self.writes.extend_from_slice(&resource_borrows.writes);
    }
}

// To be used for recreating and later for scheduling the query.
pub trait QueryInfoTrait {
    fn borrows(&self) -> ResourceBorrows {
        ResourceBorrows {
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }
}

pub trait GetQueryInfoTrait<WORLD: WorldTrait> {
    type QueryInfo: QueryInfoTrait;
    fn query_info(world: &WORLD) -> Result<Self::QueryInfo, Error>;
}

pub trait QueryTrait<'a, WORLD: WorldTrait>: GetQueryInfoTrait<WORLD> {
    type Result: for<'b> AsSystemArg<'b>;

    /// This is used to actually construct the query.
    fn get_query(world: &'a WORLD, query_info: &Self::QueryInfo) -> Result<Self::Result, Error>;

    /// Some queries may need exclusive access to the World, this is used to construct those queries.
    /// But most queries will just work the same if they have exclusive access.
    fn get_query_exclusive(
        world: &'a mut WORLD,
        query_info: &Self::QueryInfo,
    ) -> Result<Self::Result, Error> {
        Self::get_query(world, query_info)
    }

    /// If this trait requires exclusive access.
    fn exclusive() -> bool {
        false
    }
}

/// The Result of `QueryTrait` must implement this trait.
/// This trait specifies how the `QueryTrait::Result` is passed into the system.
pub trait AsSystemArg<'a> {
    type Arg;
    fn as_system_arg(&'a mut self) -> Self::Arg;
}
