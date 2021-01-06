#[derive(Debug)]
pub enum FetchError {
    ComponentAlreadyBorrowed(ComponentAlreadyBorrowed),
    ComponentDoesNotExist(ComponentDoesNotExist),
}

#[derive(Debug)]
pub struct ComponentAlreadyBorrowed(&'static str);

impl ComponentAlreadyBorrowed {
    pub fn new<T>() -> Self {
        Self(std::any::type_name::<T>())
    }
}

impl std::fmt::Display for ComponentAlreadyBorrowed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] is already borrowed from the archetype", self.0)
    }
}

impl std::error::Error for ComponentAlreadyBorrowed {}

#[derive(Debug)]
pub struct ComponentDoesNotExist(&'static str);

impl ComponentDoesNotExist {
    pub fn new<T>() -> Self {
        Self(std::any::type_name::<T>())
    }
}

impl std::fmt::Display for ComponentDoesNotExist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] does not exist", self.0)
    }
}

impl std::error::Error for ComponentDoesNotExist {}
