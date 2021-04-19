mod archetype;
mod component_bundle;
mod entities;
mod error;
mod function_system;
mod iterators;
mod queries;
mod world;

use archetype::*;
use entities::*;

pub use component_bundle::*;
pub use error::*;
pub use function_system::*;
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

pub trait ComponentTrait: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ComponentTrait for T {}
