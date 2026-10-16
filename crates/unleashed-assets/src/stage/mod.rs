//! Stage-level asset assembly.
//!
//! This module is intentionally narrow: it wires already-evidenced asset
//! parsers together for specific stages instead of growing a second set of
//! archive/resource parsers.

pub mod day;
mod mykonos;

pub use mykonos::{
    ActDMykonosAct1, DiagnosticSeverity, StageCounts, StageDiagnostic, TerrainInstanceResource,
    TerrainModelResource, load_actd_mykonos_act1,
};
