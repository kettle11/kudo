//! This file contains a number of workarounds to get around the lack of generic associated types (GATs)
//! so expect some weirdness and convolutedness.
//!
//! A `Query` has `QueryParameters` that is a tuple of `QueryParameter`s
//! A `QueryParameter` has code to filter archetypes from the world.
//! A `QueryParameter implements `QueryParameterFetch` which borrows from the `World`.
//! `QueryParameterFetch` has a `FetchItem` which is a borrow from the world.
//! `FetchItem` has `Item` which is the final value passed to a system.
//!
//! `FetchItem` exists so that RwLocks can be held in the scope that calls the user system.
//! but the user system receives a simple &T or &mut T.

use crate::{FetchError, World};

/// Fetch returns a FetchItem.
/// FetchItem must implement 'inner' to borrow the inner data.
/// The inner item is retrieved by kudo and passed to the user code.
pub trait Fetch<'world_borrow> {
    type Item: for<'a> FetchItem<'a>;
    fn fetch(world: &'world_borrow World) -> Result<Self::Item, FetchError>;
}

pub trait FetchItem<'a> {
    type InnerItem;
    fn inner(&'a mut self) -> Self::InnerItem;
}
