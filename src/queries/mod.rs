//mod multi_queries;
mod multi_query;
mod single_query;

pub use multi_query::*;
pub use single_query::*;

use crate::World;

#[derive(Clone)]
pub enum ReadOrWrite {
    Read,
    Write,
}

#[derive(Clone)]
pub enum WorldBorrow {
    Archetype {
        archetype_index: usize,
        channel_index: usize,
        read_or_write: ReadOrWrite,
    },
}

// To be used for recreating and later for scheduling the query.
pub trait QueryInfoTrait {
    fn borrows(&self) -> &[WorldBorrow];
}

pub trait GetQueryInfoTrait {
    type QueryInfo: QueryInfoTrait;
    fn query_info(world: &World) -> Option<Self::QueryInfo>;
}
pub trait QueryTrait<'a>: GetQueryInfoTrait {
    type Result: for<'b> AsSystemArg<'b>;

    /// This is used to actually construct the query.
    fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Option<Self::Result>;
}

/// The Result of `QueryTrait` must implement this trait.
/// This trait specifies how the `QueryTrait::Result` is passed into the system.
pub trait AsSystemArg<'a> {
    type Arg;
    fn as_system_arg(&'a mut self) -> Self::Arg;
}
