use std::iter::Zip;

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
