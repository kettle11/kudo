//! This module implements queries that query for a single component.
//! Querying for `Option<&T>` or `Option<&mut T>` are fallible, but
//! a query for just `&T` is fallible and will produce an error when
//! a system with that argument tries to run

use super::*;
use crate::Requirement;
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

fn get_query_info<T: 'static>(world: &World, write: bool) -> Option<SingleQueryInfo> {
    let type_ids = [Requirement::with_(0, TypeId::of::<T>())];
    let mut archetype_channel_and_resource_index = None;
    // Intentionally ignore error here because error is used to early out
    world
        .storage_graph
        .iterate_matching_storage(&type_ids, |i, channels| -> Result<(), ()> {
            archetype_channel_and_resource_index = Some((
                i,
                channels[0],
                world.archetypes[i].channels[channels[0]].channel_id,
            ));
            Err(())
        })
        .ok();
    let (archetype_index, channel_index, resource_index) = archetype_channel_and_resource_index?;

    Some(SingleQueryInfo {
        archetype_index,
        channel_index,
        resource_index,
        write,
    })
}

impl<T: 'static> GetQueryInfoTrait for &T {
    type QueryInfo = SingleQueryInfo;
    fn query_info(world: &World) -> Option<Self::QueryInfo> {
        get_query_info::<T>(world, false)
    }
}

impl<'a, T: 'static> QueryTrait<'a> for &T {
    type Result = RwLockReadGuard<'a, Vec<T>>;

    fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Option<Self::Result> {
        let borrow = world.archetypes[query_info.archetype_index]
            .borrow_channel::<T>(query_info.channel_index)?;
        Some(borrow)
    }
}

impl<T: 'static> GetQueryInfoTrait for &mut T {
    type QueryInfo = SingleQueryInfo;
    fn query_info(world: &World) -> Option<Self::QueryInfo> {
        get_query_info::<T>(world, true)
    }
}

impl<'a, T: 'static> QueryTrait<'a> for &mut T {
    type Result = RwLockWriteGuard<'a, Vec<T>>;

    fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Option<Self::Result> {
        let borrow = world.archetypes[query_info.archetype_index]
            .borrow_channel_mut::<T>(query_info.channel_index)?;
        Some(borrow)
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
