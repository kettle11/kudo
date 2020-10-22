use std::iter::Zip;

// This first iterator wraps the standard library `Zip` iterator and flattens nested tuples 
// of values returned to a flat list.
// Through experimentation I found this to be the a reasonable way to wrap the the standard library `Zip
// without dramatically hurting the optimizations the compiler can make..
macro_rules! impl_zip {
    ($name: ident, $zip_type: ty, $m_stuff: expr, $($T: ident),*) => {
        pub struct $name<$($T: Iterator,)*> {
            inner: $zip_type,
        }

        impl<$($T: Iterator,)*> Iterator for $name<$($T,)*> {
            type Item = ($($T::Item,)*);

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
impl_zip! {Zip3, Zip<Zip<A, B>, C>, |((a, b), c)| {(a, b, c)}, A, B, C}
impl_zip! {Zip4, Zip<Zip<Zip<A, B>, C>, D>, |(((a, b), c), d)| {(a, b, c, d)}, A, B, C, D}
impl_zip! {Zip5, Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, |((((a, b), c), d), e)| {(a, b, c, d, e)}, A, B, C, D, E}
impl_zip! {Zip6, Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, |(((((a, b), c), d), e), f)| {(a, b, c, d, e, f)}, A, B, C, D, E, F}
impl_zip! {Zip7, Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, |((((((a, b), c), d), e), f), g)| {(a, b, c, d, e, f, g)}, A, B, C, D, E, F, G}
impl_zip! {Zip8, Zip<Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, H>, |(((((((a, b), c), d), e), f), g), h)| {(a, b, c, d, e, f, g, h)}, A, B, C, D, E, F, G, H}

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
