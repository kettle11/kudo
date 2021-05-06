use std::ops::{Deref, DerefMut};

use crate::*;

/// This query is unique in that it gets exclusive access to the world.
/// Systems that use this query cannot use any other queries at the same time.
/// Systems using this query will block all other queries from running at the same time.
pub struct ExclusiveWorld<'a, WORLD: WorldTrait>(&'a mut WORLD);

impl<'a, WORLD: WorldTrait> Deref for ExclusiveWorld<'a, WORLD> {
    type Target = WORLD;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, WORLD: WorldTrait> DerefMut for ExclusiveWorld<'a, WORLD> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

pub struct ExclusiveWorldQueryInfo {}

impl<WORLD: WorldTrait> GetQueryInfoTrait<WORLD> for ExclusiveWorld<'_, WORLD> {
    type QueryInfo = ExclusiveWorldQueryInfo;
    fn query_info(_world: &WORLD) -> Result<Self::QueryInfo, Error> {
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
impl<'a, WORLD: WorldTrait + 'static> QueryTrait<'a, WORLD> for ExclusiveWorld<'_, WORLD> {
    type Result = &'a mut WORLD;

    fn get_query(_world: &'a WORLD, _query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
        Err(Error::MustRunExclusively)
    }

    fn get_query_exclusive(
        world: &'a mut WORLD,
        _query_info: &Self::QueryInfo,
    ) -> Result<Self::Result, Error> {
        Ok(world)
    }

    fn exclusive() -> bool {
        true
    }
}

impl<'b, WORLD: WorldTrait + 'b> AsSystemArg<'b> for &'_ mut WORLD {
    type Arg = ExclusiveWorld<'b, WORLD>;
    fn as_system_arg(&'b mut self) -> Self::Arg {
        ExclusiveWorld(&mut *self)
    }
}
