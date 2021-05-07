use crate::*;

pub struct SystemInfo {
    pub borrows: ResourceBorrows,
    pub exclusive: bool,
}

pub trait FunctionSystem<'world_borrow, WORLD: WorldTrait, RETURN: 'world_borrow, Params>:
    Sized
{
    type Thing;
    /// Borrow the system and run it.
    // Maybe there's a way to unify `run_borrow` and `run`?
    fn run_borrow(&mut self, world: &'world_borrow WORLD) -> Result<RETURN, Error>;

    /// Run a system once.
    /// This function exists to allow for slightly nicer syntax in the common case.
    fn run(self, world: &'world_borrow WORLD) -> Result<RETURN, Error>;

    /// Run a system with exclusive access to the World
    fn run_exclusive(self, world: &'world_borrow mut WORLD) -> Result<RETURN, Error> {
        self.run(world)
    }

    fn exclusive(&self) -> bool;
    fn system_info(&self, world: &WORLD) -> Result<SystemInfo, Error>;
}

pub trait IntoSystem<WORLD: WorldTrait, P, R> {
    fn box_system(self) -> Box<dyn FnMut(&WORLD) -> Result<R, Error> + Send + Sync>;
}

impl<
        WORLD: WorldTrait,
        P,
        R,
        S: for<'a> FunctionSystem<'a, WORLD, R, P> + Sync + Send + 'static + Copy,
    > IntoSystem<WORLD, P, R> for S
{
    fn box_system(self) -> Box<dyn FnMut(&WORLD) -> Result<R, Error> + Send + Sync> {
        Box::new(move |world| self.run(world))
    }
}

impl<'world_borrow, WORLD: WorldTrait, FUNC, RETURN: 'world_borrow>
    FunctionSystem<'world_borrow, WORLD, RETURN, ()> for FUNC
where
    FUNC: FnMut() -> RETURN,
{
    type Thing = ();
    fn run_borrow(&mut self, _world: &'world_borrow WORLD) -> Result<RETURN, Error> {
        Ok(self())
    }

    fn run(mut self, _world: &'world_borrow WORLD) -> Result<RETURN, Error> {
        Ok(self())
    }

    fn run_exclusive(mut self, _world: &'world_borrow mut WORLD) -> Result<RETURN, Error> {
        Ok(self())
    }

    fn exclusive(&self) -> bool {
        false
    }

    fn system_info(&self, _world: &WORLD) -> Result<SystemInfo, Error> {
        Ok(SystemInfo {
            exclusive: false,
            borrows: ResourceBorrows {
                reads: Vec::new(),
                writes: Vec::new(),
            },
        })
    }
}

impl<
        'world_borrow,
        WORLD: WorldTrait,
        FUNC,
        R: 'world_borrow,
        A: QueryTrait<'world_borrow, WORLD>,
    > FunctionSystem<'world_borrow, WORLD, R, (A,)> for FUNC
where
    FUNC: FnMut(A) -> R
        + FnMut(<<A as QueryTrait<'world_borrow, WORLD>>::Result as AsSystemArg>::Arg) -> R,
{
    type Thing = ();

    #[allow(non_snake_case)]
    #[allow(unused_variables)]
    fn run_borrow(&mut self, world: &'world_borrow WORLD) -> Result<R, Error> {
        let a = <A as GetQueryInfoTrait<WORLD>>::query_info(world)?;
        let mut a = <A as QueryTrait<'world_borrow, WORLD>>::get_query(world, &a)?;
        Ok(self(a.as_system_arg()))
    }

    #[allow(non_snake_case)]
    fn run(mut self, world: &'world_borrow WORLD) -> Result<R, Error> {
        let a = <A as GetQueryInfoTrait<WORLD>>::query_info(world)?;
        let mut a = <A as QueryTrait<'world_borrow, WORLD>>::get_query(world, &a)?;
        Ok(self(a.as_system_arg()))
    }

    // This could definitely be improved.
    // The borrows should not have to be requested again
    // to run later.
    #[allow(non_snake_case)]
    fn system_info(&self, world: &WORLD) -> Result<SystemInfo, Error> {
        let a = <A as GetQueryInfoTrait<WORLD>>::query_info(world)?;

        let exclusive = A::exclusive();
        Ok(SystemInfo {
            borrows: a.borrows(),
            exclusive,
        })
    }

    fn exclusive(&self) -> bool {
        A::exclusive()
    }

    #[allow(non_snake_case)]
    fn run_exclusive(mut self, world: &'world_borrow mut WORLD) -> Result<R, Error> {
        let a = <A as GetQueryInfoTrait<WORLD>>::query_info(world)?;
        let mut a = <A as QueryTrait<'world_borrow, WORLD>>::get_query_exclusive(world, &a)?;
        Ok(self(a.as_system_arg()))
    }
}

macro_rules! system_impl {
    ($($name: ident),*) => {
        impl<'world_borrow, WORLD: WorldTrait, FUNC, R: 'world_borrow, $($name: QueryTrait<'world_borrow, WORLD>),*> FunctionSystem< 'world_borrow, WORLD, R, ($($name,)*)> for FUNC
        where
        FUNC: FnMut($($name,)*) -> R + FnMut($(<<$name as QueryTrait<'world_borrow, WORLD>>::Result as AsSystemArg>::Arg,)*) -> R,
        {

            type Thing = ();

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn run_borrow(&mut self, world: &'world_borrow WORLD) -> Result<R, Error> {
                $(let $name = <$name as GetQueryInfoTrait<WORLD>>::query_info(world)?;)*
                $(let mut $name = <$name as QueryTrait<'world_borrow, WORLD>>::get_query(world, &$name)?;)*
                Ok(self($($name.as_system_arg(),)*))
            }

            #[allow(non_snake_case)]
            fn run(mut self, world: &'world_borrow WORLD) -> Result<R, Error>{
                $(let $name = <$name as GetQueryInfoTrait<WORLD>>::query_info(world)?;)*
                $(let mut $name = <$name as QueryTrait<'world_borrow, WORLD>>::get_query(world, &$name)?;)*
                Ok(self($($name.as_system_arg(),)*))
            }

            fn exclusive(&self) -> bool {
                false $(|| $name::exclusive())*
            }
            // This could definitely be improved.
            // The borrows should not have to be requested again
            // to run later.
            #[allow(non_snake_case)]
            fn system_info(&self, world: &WORLD) -> Result<SystemInfo, Error> {
                let mut borrows = ResourceBorrows::new();
                $(let $name = <$name as GetQueryInfoTrait<WORLD>>::query_info(world)?;)*

                $(borrows.extend(&$name.borrows());)*

                let exclusive = false $(|| $name::exclusive())*;
                Ok(SystemInfo {
                    borrows,
                    exclusive
                })
            }
        }
    };
}

//system_impl! {A}
system_impl! {A, B}
system_impl! {A, B, C}
system_impl! {A, B, C, D}
system_impl! {A, B, C, D, E}
system_impl! {A, B, C, D, E, F}
system_impl! {A, B, C, D, E, F, G}
system_impl! {A, B, C, D, E, F, G, H}
system_impl! {A, B, C, D, E, F, G, H, I}
system_impl! {A, B, C, D, E, F, G, H, I, J}
system_impl! {A, B, C, D, E, F, G, H, I, J, K}
system_impl! {A, B, C, D, E, F, G, H, I, J, K, L}
