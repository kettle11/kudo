use super::GetIter;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

impl<'iter, 'world_borrow, T: 'static> GetIter<'iter> for RwLockReadGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::Iter<'iter, T>;
    fn iter(&'iter mut self) -> Self::Iter {
        <[T]>::iter(self)
    }
}

impl<'iter, 'world_borrow, T: 'static> GetIter<'iter> for RwLockWriteGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::IterMut<'iter, T>;
    fn iter(&'iter mut self) -> Self::Iter {
        <[T]>::iter_mut(self)
    }
}
