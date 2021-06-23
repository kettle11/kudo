mod archetype;
mod component_bundle;
mod entities;
mod error;
mod function_system;
mod hierarchy;
mod iterators;
mod queries;
mod world;

use archetype::*;
pub use archetype::{EntityMigrator, WorldClone};
pub use component_bundle::*;
use entities::*;
pub use error::*;
pub use function_system::*;
pub use hierarchy::*;
pub use iterators::*;
pub use queries::*;
pub use world::*;

mod sparse_set;
mod storage_lookup;
use storage_lookup::*;

// #[cfg(feature = "scheduler")]
// mod scheduler;
//
// #[cfg(feature = "scheduler")]
// pub use scheduler::*;
//
// mod scheduler1;

mod tests;
