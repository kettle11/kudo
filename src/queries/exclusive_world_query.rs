use std::ops::{Deref, DerefMut};

use crate::*;

/// This query is unique in that it gets exclusive access to the world.
/// Systems that use this query cannot use any other queries at the same time.
/// Systems using this query will block all other queries from running at the same time.
pub struct ExclusiveWorld<'a>(&'a mut World);

impl<'a> Deref for ExclusiveWorld<'a> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for ExclusiveWorld<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

pub struct ExclusiveWorldQueryInfo {}

impl GetQueryInfoTrait for ExclusiveWorld<'_> {
    type QueryInfo = ExclusiveWorldQueryInfo;
    fn query_info(_world: &World) -> Option<Self::QueryInfo> {
        Some(ExclusiveWorldQueryInfo {})
    }
}

impl QueryInfoTrait for ExclusiveWorldQueryInfo {
    fn borrows(&self) -> &[WorldBorrow] {
        &[]
    }

    fn exclusive(&self) -> bool {
        true
    }
}

impl<'a> QueryTrait<'a> for ExclusiveWorld<'_> {
    type Result = &'a mut World;

    fn get_query(_world: &'a World, _query_info: &Self::QueryInfo) -> Option<Self::Result> {
        None
    }

    fn get_query_exclusive(
        world: &'a mut World,
        _query_info: &Self::QueryInfo,
    ) -> Option<Self::Result> {
        Some(world)
    }
}

impl<'b> AsSystemArg<'b> for &'_ mut World {
    type Arg = ExclusiveWorld<'b>;
    fn as_system_arg(&'b mut self) -> Self::Arg {
        ExclusiveWorld(&mut *self)
    }
}
