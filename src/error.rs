#[derive(Debug)]
pub enum Error {
    NoMatchingEntities,
    MissingComponent(&'static str),
    MustRunExclusively,
    CouldNotBorrowComponent(&'static str),
}
