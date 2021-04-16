#[derive(Debug)]
pub enum Error {
    MissingComponent(&'static str),
    MustRunExclusively,
    CouldNotBorrowComponent(&'static str),
}
