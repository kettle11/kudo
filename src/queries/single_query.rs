//! This module implements queries that query for a single component.
//! Querying for `Option<&T>` or `Option<&mut T>` are fallible, but
//! a query for just `&T` is fallible and will produce an error when
//! a system with that argument tries to run

use super::*;
use crate::*;
use std::{
    any::TypeId,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

pub struct SingleQueryInfo {
    archetype_index: usize,
    channel_index: usize,
    resource_index: usize,
    write: bool,
}

impl<'a> QueryInfoTrait for SingleQueryInfo {
    fn borrows(&self) -> ResourceBorrows {
        if self.write {
            ResourceBorrows {
                reads: Vec::new(),
                writes: vec![self.resource_index],
            }
        } else {
            ResourceBorrows {
                reads: vec![self.resource_index],
                writes: Vec::new(),
            }
        }
    }
}

fn get_query_info<WORLD: WorldTrait, T: 'static>(
    world: &WORLD,
    write: bool,
) -> Result<SingleQueryInfo, Error> {
    let type_ids = [Filter {
        filter_type: FilterType::With,
        type_id: TypeId::of::<T>(),
    }];
    // This allocation could probably be avoided in the future.
    let archetypes = world
        .storage_lookup()
        .get_matching_archetypes(&type_ids, &[]);

    let (archetype_index, channel_index, resource_index) = archetypes.iter().next().map_or_else(
        || Err(Error::MissingComponent(std::any::type_name::<T>())),
        |a| {
            Ok((
                a.archetype_index,
                a.channels[0].unwrap(),
                0, // a.resource_indices[0].unwrap(),
                   // TODO: resource_indices aren't setup yet.
            ))
        },
    )?;

    Ok(SingleQueryInfo {
        archetype_index,
        channel_index,
        resource_index,
        write,
    })
}

impl<T: 'static, WORLD: WorldTrait> GetQueryInfoTrait<WORLD> for &T {
    type QueryInfo = SingleQueryInfo;
    fn query_info(world: &WORLD) -> Result<Self::QueryInfo, Error> {
        get_query_info::<WORLD, T>(world, false)
    }
}

impl<'a, WORLD: WorldTrait, T: 'static> QueryTrait<'a, WORLD> for &T {
    type Result = RwLockReadGuard<'a, Vec<T>>;

    fn get_query(world: &'a WORLD, query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
        let borrow = world
            .borrow_archetype(query_info.archetype_index)
            .borrow_channel::<T>(query_info.channel_index)?;
        Ok(borrow)
    }
}

impl<WORLD: WorldTrait, T: 'static> GetQueryInfoTrait<WORLD> for &mut T {
    type QueryInfo = SingleQueryInfo;
    fn query_info(world: &WORLD) -> Result<Self::QueryInfo, Error> {
        get_query_info::<WORLD, T>(world, true)
    }
}

impl<'a, WORLD: WorldTrait, T: 'static> QueryTrait<'a, WORLD> for &mut T {
    type Result = RwLockWriteGuard<'a, Vec<T>>;

    fn get_query(world: &'a WORLD, query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
        let borrow = world
            .borrow_archetype(query_info.archetype_index)
            .borrow_channel_mut::<T>(query_info.channel_index)?;
        Ok(borrow)
    }
}

impl<'a, T: 'static> AsSystemArg<'a> for RwLockReadGuard<'_, Vec<T>> {
    type Arg = &'a T;
    fn as_system_arg(&'a mut self) -> Self::Arg {
        &self[0]
    }
}

impl<'a, T: 'static> AsSystemArg<'a> for RwLockWriteGuard<'_, Vec<T>> {
    type Arg = &'a mut T;
    fn as_system_arg(&'a mut self) -> Self::Arg {
        &mut self[0]
    }
}

impl<'a, T: AsSystemArg<'a>> AsSystemArg<'a> for Option<T> {
    type Arg = Option<T::Arg>;
    fn as_system_arg(&'a mut self) -> Self::Arg {
        self.as_mut().map(|t| t.as_system_arg())
    }
}
