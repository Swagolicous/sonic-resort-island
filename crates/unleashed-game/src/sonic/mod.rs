//! Daytime Sonic, ported from the decompiled executable.

pub mod constants;
pub mod context;
pub mod controller;
pub mod crt_math;
pub mod crt_tables;
pub mod look;
pub mod param_fields;
pub mod param_table;
pub mod params;
pub mod posture;
pub mod vmx;
pub mod vmx_gen;
pub mod xenon_math;

pub use controller::{Body, Controller, Events, GroundState, Input, JumpPhase};
pub use params::{ParamSet, SonicParams, View};
