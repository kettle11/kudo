//! A simple and predictable Entity Component System.
//!
//! An entity-component-system (ECS) is a data structure and organizational pattern often
//! used by game-like code.
//!
//! An ECS is made up of three parts:
//!
//! * **Entities**: IDs associated with various components
//! * **Components**: Individual units of data associated with an entity.
//! * **Systems**: Code that iterates over components
//!
//! ```
//! # use kudo::*;
//! // First we create the world.
//! let mut world = World::new();
//!
//! // Let's create a new entity with a name and a health component.
//! // Components are just plain structs.
//!
//! // This will be our health component
//! struct Health(i32);
//!
//! // Spawn the entity with a String component we'll use for the name and a Health component.
//! // Within the call to spawn we pass in a tuple that can have multiple components.
//! world.spawn(("Medusa".to_string(), Health(-2)));
//!
//! // Query the world for entities that have a String component and a Health component.
//! // The '&' before each component requests read-only access to the component.
//! // Using '&mut' would request write/read access for that component.
//! let mut query = world.query::<(&String, &Health)>().unwrap();
//!
//! // Iterate over all the components we found and check if their health is less than 0.
//! for (name, health) in query.iter() {
//!     if health.0 < 0 {
//!         println!("{} has perished!", name);
//!     }
//! }
//! ```

mod iterators;
//mod query;
mod errors;
mod other_queries;
mod query;
mod query_infrastructure;
mod system;
mod world;

pub use errors::*;
pub use iterators::*;
pub use other_queries::*;
pub use query::*;
pub use query_infrastructure::*;
//pub use query::*;
pub use system::*;
pub use world::*;
