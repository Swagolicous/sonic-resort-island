//! Minimal Mirage resource parsing for the proven Windmill Isle terrain path.
//!
//! This module intentionally implements only byte layouts that are evidenced by
//! the extracted `tg-0068` resources.  It does not try to model every Hedgehog
//! Engine Mirage version.

pub mod material;
pub mod mesh;
pub(crate) mod reader;
mod terrain;

pub use terrain::{
    IndexFormat, InstanceTransform, MeshTopology, TerrainMesh, TerrainParseError,
    parse_terrain_instance_transform, parse_terrain_mesh,
};
