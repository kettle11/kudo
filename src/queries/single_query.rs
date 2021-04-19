//! This module implements queries that query for a single component.
//! Querying for `Option<&T>` or `Option<&mut T>` are fallible, but
//! a query for just `&T` is fallible and will produce an error when
//! a system with that argument tries to run

use super::*;
use crate::storage_lookup::{Filter, FilterType};
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

fn get_query_info<T: 'static>(world: &World, write: bool) -> Result<SingleQueryInfo, Error> {
    let type_ids = [Filter {
        filter_type: FilterType::With,
        type_id: TypeId::of::<T>(),
    }];
    // This allocation could probably be avoided in the future.
    let archetypes = world.storage_lookup.get_matching_archetypes(&type_ids, &[]);

    println!("MATCHES: {:?}", archetypes);
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

impl<T: 'static> GetQueryInfoTrait for &T {
    type QueryInfo = SingleQueryInfo;
    fn query_info(world: &World) -> Result<Self::QueryInfo, Error> {
        get_query_info::<T>(world, false)
    }
}

impl<'a, T: 'static> QueryTrait<'a> for &T {
    type Result = RwLockReadGuard<'a, Vec<T>>;

    fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
        let borrow = world.archetypes[query_info.archetype_index]
            .borrow_channel::<T>(query_info.channel_index)?;
        Ok(borrow)
    }
}

impl<T: 'static> GetQueryInfoTrait for &mut T {
    type QueryInfo = SingleQueryInfo;
    fn query_info(world: &World) -> Result<Self::QueryInfo, Error> {
        get_query_info::<T>(world, true)
    }
}

impl<'a, T: 'static> QueryTrait<'a> for &mut T {
    type Result = RwLockWriteGuard<'a, Vec<T>>;

    fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
        let borrow = world.archetypes[query_info.archetype_index]
            .borrow_channel_mut::<T>(query_info.channel_index)?;
        Ok(borrow)
    }
}

pub trait Single<'world_borrow>: QueryTrait<'world_borrow> {}

impl<'world_borrow, T: 'static> Single<'world_borrow> for &T {}

// impl<'world_borrow, T: 'static> Single<'world_borrow> for &mut T {}

/*
impl<'world_borrow, S: Single<'world_borrow>> QueryTrait<'world_borrow> for Option<S> {
    type Result = Option<<S as QueryTrait<'world_borrow>>::Result>;
    type QueryInfo = ();

    fn query(world: &'world_borrow World) -> Option<Self::Result> {
        Some(<S as QueryTrait<'world_borrow>>::query(world))
    }

    fn query_info() -> Self::QueryInfo {
        todo!()
    }
}
*/

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
/*

#[test]
fn mutable_query() {
    use crate::*;

    let mut world = World::new();
    world.spawn((2 as i32,));

    (|q: &mut i32| {
        *q += 1;
    })
    .run(&world);
    (|q: &i32| assert!(*q == 3)).run(&world);
}

#[test]
fn option_query() {
    use crate::*;

    let world = World::new();
    (|_: Option<&i32>| {}).run(&world);
}
*/
