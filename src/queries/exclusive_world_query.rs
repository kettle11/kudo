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
    fn query_info(_world: &World) -> Result<Self::QueryInfo, Error> {
        Ok(ExclusiveWorldQueryInfo {})
    }
}

impl QueryInfoTrait for ExclusiveWorldQueryInfo {
    fn borrows(&self) -> ResourceBorrows {
        const R: ResourceBorrows = ResourceBorrows {
            writes: Vec::new(),
            reads: Vec::new(),
        };
        R
    }
}

// Is the 'static here too limiting?
impl<'a> QueryTrait<'a> for ExclusiveWorld<'_> {
    type Result = &'a mut World;

    fn get_query(_world: &'a World, _query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
        Err(Error::MustRunExclusively)
    }

    fn get_query_exclusive(
        world: &'a mut World,
        _query_info: &Self::QueryInfo,
    ) -> Result<Self::Result, Error> {
        Ok(world)
    }

    fn exclusive() -> bool {
        true
    }
}

impl<'b> AsSystemArg<'b> for &'_ mut World {
    type Arg = ExclusiveWorld<'b>;
    fn as_system_arg(&'b mut self) -> Self::Arg {
        ExclusiveWorld(&mut *self)
    }
}
