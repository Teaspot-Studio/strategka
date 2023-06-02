use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;

/// Each simulation that global state that implements the trait.
pub trait World {
    /// External input to simulation. Typically that is player input.
    type Input: Debug + Clone + PartialEq + Serialize + DeserializeOwned;

    /// Specific magic bytes for a game that is build on the library.
    /// That bytes help to prevent opening save files or replays with wrong
    /// simulation code.
    fn magic_bytes() -> [u8; 4];

    /// Version of game the current world implements. The number is written to
    /// files to make backward compatible parsers.
    fn current_version() -> u32;

    /// Check if the world parser can handle the given version
    fn guard_version(version: u32) -> bool {
        version == Self::current_version()
    }
}