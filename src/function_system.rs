use crate::{AsSystemArg, GetQueryInfoTrait, QueryInfoTrait, QueryTrait, World, WorldBorrow};

pub trait FunctionSystem<'world_borrow, RETURN: 'world_borrow, Params> {
    type Thing;
    /// Borrow the system and run it.
    // Maybe there's a way to unify `run_borrow` and `run`?
    fn run_borrow(&mut self, world: &'world_borrow World) -> Option<RETURN>;

    /// Run a system once.
    /// This function exists to allow for slightly nicer syntax in the common case.
    fn run(self, world: &'world_borrow World) -> Option<RETURN>;

    fn borrows(&self, world: &World) -> Vec<WorldBorrow>;
}

pub trait IntoSystem<P, R> {
    fn box_system(self) -> Box<dyn FnMut(&World) -> Option<R> + Send + Sync>;
}

impl<P, R, S: for<'a> FunctionSystem<'a, R, P> + Sync + Send + 'static + Copy> IntoSystem<P, R>
    for S
{
    fn box_system(self) -> Box<dyn FnMut(&World) -> Option<R> + Send + Sync> {
        Box::new(move |world| self.run(world))
    }
}

impl<'world_borrow, FUNC, RETURN: 'world_borrow> FunctionSystem<'world_borrow, RETURN, ()> for FUNC
where
    FUNC: FnMut() -> RETURN,
{
    type Thing = ();
    fn run_borrow(&mut self, _world: &'world_borrow World) -> Option<RETURN> {
        Some(self())
    }

    #[allow(non_snake_case)]
    fn run(mut self, _world: &'world_borrow World) -> Option<RETURN> {
        Some(self())
    }

    fn borrows(&self, _world: &World) -> Vec<WorldBorrow> {
        Vec::new()
    }
}

macro_rules! system_impl {
    ($($name: ident),*) => {
        impl<'world_borrow, FUNC, R: 'world_borrow, $($name: QueryTrait<'world_borrow>),*> FunctionSystem< 'world_borrow, R, ($($name,)*)> for FUNC
        where
        FUNC: FnMut($($name,)*) -> R + FnMut($(<<$name as QueryTrait<'world_borrow>>::Result as AsSystemArg>::Arg,)*) -> R,
        {
            type Thing = ();

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn run_borrow(&mut self, world: &'world_borrow World) -> Option<R> {
                $(let $name = <$name as GetQueryInfoTrait>::query_info(world)?;)*
                $(let mut $name = <$name as QueryTrait<'world_borrow>>::get_query(world, &$name)?;)*
                Some(self($($name.as_system_arg(),)*))
            }

            #[allow(non_snake_case)]
            fn run(mut self, world: &'world_borrow World) -> Option<R> {
                $(let $name = <$name as GetQueryInfoTrait>::query_info(world)?;)*
                $(let mut $name = <$name as QueryTrait<'world_borrow>>::get_query(world, &$name)?;)*
                Some(self($($name.as_system_arg(),)*))
            }

            // This could definitely be improved.
            // The borrows should not have to be requested again
            // to run later.
            #[allow(non_snake_case)]
            fn borrows(&self, world: &World) -> Vec<WorldBorrow> {
                let mut v = Vec::new();
                $(let $name = <$name as GetQueryInfoTrait>::query_info(world).unwrap();)*
                $(v.extend_from_slice($name.borrows());)*
                v
            }
        }
    };
}

system_impl! {A}
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

#[test]
fn async_query() {
    use crate::*;

    let mut world = World::new();
    world.spawn((3 as i32,));

    async fn f(_: Query<'_, (&f32,)>) -> f32 {
        10.
    }

    // This returns something that borrows world.
    (f).run(&world);
}

#[test]
fn system_with_result() {
    use crate::*;

    let mut world = World::new();
    world.spawn((1 as i32,));
    world.spawn((2 as i32,));
    world.spawn((3 as i32,));

    // This returns something that borrows world.
    let result = (|q: Query<(&i32,)>| -> i32 { q.iter().sum() })
        .run(&world)
        .unwrap();

    assert!(result == 6);
}

#[test]
fn system_single_query() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));

    (|i: &i32| assert!(*i == 3)).run(&world).unwrap()
}

#[test]
fn system_multiple_parameters() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    (|_i: &i32, _q: Query<(&i32,)>| {}).run(&world).unwrap()
}

/// This test intentionally creates overlapping borrows
#[test]
fn conflicting_queries() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    assert!((|_: &mut i32, _: &i32| {}).run(&world).is_none())
}

#[test]
fn mutable_closure() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));

    let mut internal_data = 0;
    let mut closure = |num: &i32| {
        internal_data += num;
    };

    (&mut closure).run(&world);
    (&mut closure).run(&world);
}

#[test]
fn box_system() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));

    let mut boxed_system = (|i: &i32| assert!(*i == 3)).box_system();
    (boxed_system)(&world);
}

#[test]
fn three_parameters() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    (|_q: Query<(&i32, &bool, &f32)>| {}).run(&world).unwrap()
}

#[test]
fn four_parameters() {
    use crate::*;
    let mut world = World::new();

    struct Commands {}

    fn test(commands: &mut Commands, _renderables: Query<(&bool,)>) {}
    fn test0(c_renderables: Query<(&bool,)>) {}
    // test0.box_system();
    // test.box_system();
    // test.box_system();

    /*
    println!(
        "TYPE: {:?}",
        std::any::type_name::<<fn(&f32) as FunctionSystem<(), (&f32,)>>::Thing>()
    );
    */

    <&fn(&mut Commands, Query<(&bool,)>) as FunctionSystem<(), (&mut Commands, Query<(&bool,)>)>>::run(&(test as fn(&mut Commands, Query<(&bool,)>)) , &world);
    // <|commands: &mut Commands, _renderables: Query<(&bool,)>| {} as FunctionSystem>).box_system();
    //(|commands: &mut Commands, _renderables: Query<(&bool,)>| {}).run(&world);
}
