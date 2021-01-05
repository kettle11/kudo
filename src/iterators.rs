use crate::World;
use std::iter::Zip;
// GetIter is pretty much the standard library IntoIterator trait, but it uses a lifetime
// instead of taking ownership.
// But maybe there's a way to use the standard library IntoIterator instead?
pub trait GetIter<'iter> {
    type Iter: Iterator;

    // Named get_iter to disambiguate from into_iter
    // But will be renamed because it's annoying.
    fn get_iter(&'iter mut self, world: &'iter World) -> Self::Iter;
}

impl<'iter> GetIter<'iter> for () {
    type Iter = std::iter::Empty<()>;
    fn get_iter(&'iter mut self, _world: &'iter World) -> Self::Iter {
        std::iter::empty()
    }
}

// This first iterator wraps the standard library `Zip` iterator and flattens nested tuples
// of values returned to a flat list.
// Through experimentation I found this to be the a reasonable way to wrap the the standard library `Zip
// without dramatically hurting the optimizations the compiler can make..
macro_rules! impl_zip {
    ($name: ident, $zip_type: ty, $m_stuff: expr, $($T: ident),*) => {
        pub struct $name<A: Iterator, $($T: Iterator,)*> {
            inner: $zip_type,
        }

        impl<A: Iterator, $($T: Iterator,)*> $name<A, $($T,)*> {
            #[allow(non_snake_case)]
            pub fn new (A: A, $($T: $T,)*) -> Self {
                Self {
                    inner: A$(.zip($T))*
                }
            }
        }

        impl<A: Iterator, $($T: Iterator,)*> Iterator for $name<A, $($T,)*> {
            type Item = (A::Item, $($T::Item,)*);

            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next().map($m_stuff)
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }

    };
}

// I am not good at writing recursive macros.
// So instead just the parts that would need recursion are passed in. :)
impl_zip! {Zip3, Zip<Zip<A, B>, C>, |((a, b), c)| {(a, b, c)}, B, C}
impl_zip! {Zip4, Zip<Zip<Zip<A, B>, C>, D>, |(((a, b), c), d)| {(a, b, c, d)}, B, C, D}
impl_zip! {Zip5, Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, |((((a, b), c), d), e)| {(a, b, c, d, e)}, B, C, D, E}
impl_zip! {Zip6, Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, |(((((a, b), c), d), e), f)| {(a, b, c, d, e, f)}, B, C, D, E, F}
impl_zip! {Zip7, Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, |((((((a, b), c), d), e), f), g)| {(a, b, c, d, e, f, g)}, B, C, D, E, F, G}
impl_zip! {Zip8, Zip<Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, H>, |(((((((a, b), c), d), e), f), g), h)| {(a, b, c, d, e, f, g, h)}, B, C, D, E, F, G, H}

#[doc(hidden)]
/// A series of iterators of the same type that are traversed in a row.
pub struct ChainedIterator<I: Iterator> {
    current_iter: Option<I>,
    iterators: Vec<I>,
}

impl<I: Iterator> ChainedIterator<I> {
    #[doc(hidden)]
    pub fn new(mut iterators: Vec<I>) -> Self {
        let current_iter = iterators.pop();
        Self {
            current_iter,
            iterators,
        }
    }
}

impl<I: Iterator> Iterator for ChainedIterator<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Chain the iterators together.
        // If the end of one iterator is reached go to the next.

        match self.current_iter {
            Some(ref mut iter) => match iter.next() {
                None => {
                    self.current_iter = self.iterators.pop();
                    if let Some(ref mut iter) = self.current_iter {
                        iter.next()
                    } else {
                        None
                    }
                }
                item => item,
            },
            None => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut min = 0;
        let mut max = 0;

        if let Some(current_iter) = &self.current_iter {
            let (i_min, i_max) = current_iter.size_hint();
            min += i_min;
            max += i_max.unwrap();
        }

        for i in self.iterators.iter() {
            let (i_min, i_max) = i.size_hint();
            min += i_min;
            // This function is designed under the assumption that all
            // iterators passed in implement size_hint, which works fine
            // for kudo's purposes.
            max += i_max.unwrap();
        }
        (min, Some(max))
    }
}

macro_rules! get_iter_impl {
    ($zip_type: ident, $($name: ident),*) => {
        #[allow(non_snake_case)]
        impl<'iter, $($name: GetIter<'iter>),*> GetIter<'iter> for ($($name,)*){
            type Iter = $zip_type<$($name::Iter,)*>;
            fn get_iter(&'iter mut self, world: &'iter World) -> Self::Iter {
                let ($(ref mut $name,)*) = self;
                $zip_type::new($($name.get_iter(world),)*)
            }
        }
    }
}

// Non-macro implementations of GetIter that just wraps the inner Iter type.
// Is a unique 'GetIter' trait really needed or could something in the standard
// library be used?
impl<'iter, A: GetIter<'iter>> GetIter<'iter> for (A,) {
    type Iter = A::Iter;
    fn get_iter(&'iter mut self, world: &'iter World) -> Self::Iter {
        self.0.get_iter(world)
    }
}

// Implementing this for all tuples that implement GetIter is a pretty strong
// assumption for a somewhat generic seeming trait.
impl<'iter, A: GetIter<'iter>, B: GetIter<'iter>> GetIter<'iter> for (A, B) {
    type Iter = Zip<A::Iter, B::Iter>;
    fn get_iter(&'iter mut self, world: &'iter World) -> Self::Iter {
        self.0.get_iter(world).zip(self.1.get_iter(world))
    }
}

get_iter_impl! {Zip3, A, B, C}
get_iter_impl! {Zip4, A, B, C, D}
get_iter_impl! {Zip5, A, B, C, D, E}
get_iter_impl! {Zip6, A, B, C, D, E, F}
get_iter_impl! {Zip7, A, B, C, D, E, F, G}
get_iter_impl! {Zip8, A, B, C, D, E, F, G, H}
