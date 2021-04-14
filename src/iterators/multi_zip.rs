use std::iter::Zip;

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
impl_zip! {Zip9, Zip<Zip<Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, H>, I>, |((((((((a, b), c), d), e), f), g), h), i)| {(a, b, c, d, e, f, g, h, i)}, B, C, D, E, F, G, H, I}
impl_zip! {Zip10, Zip<Zip<Zip<Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, H>, I>, J>, |(((((((((a, b), c), d), e), f), g), h), i), j)| {(a, b, c, d, e, f, g, h, i, j)}, B, C, D, E, F, G, H, I, J}
impl_zip! {Zip11, Zip<Zip<Zip<Zip<Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, H>, I>, J>, K>, |((((((((((a, b), c), d), e), f), g), h), i), j), k)| {(a, b, c, d, e, f, g, h, i, j, k)}, B, C, D, E, F, G, H, I, J, K}
