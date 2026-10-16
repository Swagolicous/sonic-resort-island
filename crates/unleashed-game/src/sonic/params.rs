//! Parameter stores, resolved the way the game resolves them.
//!
//! The executable registers every tunable into a tree of stores (common ->
//! ForwardView / SideView -> special modes); a lookup walks up to the parent
//! when a store does not set a value. `Sonic.prm.xml` (in #ActionCommon) then
//! overrides a handful of values per view.

use std::collections::HashMap;

use super::param_table::{PARAMS, STORE_PARENTS};

// Generated from sheets/stores.csv and sheets/sonic_fields.csv.
pub use super::param_fields::SonicParams;
pub use super::param_table::{STORE_COMMON, STORE_FORWARD_VIEW, STORE_SIDE_VIEW};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Forward,
    Side,
}

impl View {
    pub fn store(self) -> u32 {
        match self {
            View::Forward => STORE_FORWARD_VIEW,
            View::Side => STORE_SIDE_VIEW,
        }
    }

    /// Section name in `Sonic.prm.xml`.
    pub fn xml_section(self) -> &'static str {
        match self {
            View::Forward => "ForwardView",
            View::Side => "SideView",
        }
    }
}

/// Every parameter's value for one store, after inheritance and overrides.
#[derive(Clone, Debug, Default)]
pub struct ParamSet {
    values: HashMap<&'static str, f32>,
}

impl ParamSet {
    pub fn resolve(store: u32) -> Self {
        let parent = |s: u32| STORE_PARENTS.iter().find(|(c, _)| *c == s).map(|(_, p)| *p);
        let mut values = HashMap::new();
        for def in PARAMS {
            let mut s = Some(store);
            while let Some(cur) = s {
                if let Some((_, v)) = def.values.iter().find(|(st, _)| *st == cur) {
                    values.insert(def.name, *v);
                    break;
                }
                s = parent(cur);
            }
        }
        Self { values }
    }

    /// Applies `Section/Category/Name` overrides from a parameter file; returns
    /// how many matched.
    pub fn apply_overrides<'a>(&mut self, section: &str, entries: impl IntoIterator<Item = (&'a str, f32)>) -> usize {
        let mut n = 0;
        for (path, value) in entries {
            let mut parts = path.split('/');
            if parts.next() != Some(section) {
                continue;
            }
            let Some(name) = parts.last() else { continue };
            if let Some(def) = PARAMS.iter().find(|d| d.name == name) {
                self.values.insert(def.name, value);
                n += 1;
            }
        }
        n
    }

    pub fn get(&self, name: &str) -> Option<f32> {
        self.values.get(name).copied()
    }
}

impl SonicParams {
    /// Parameters for `view` with `Sonic.prm.xml` entries (flattened
    /// `Section/Category/Name` paths) applied on top.
    pub fn load<'a>(view: View, xml: impl IntoIterator<Item = (&'a str, f32)>) -> (Self, usize) {
        let mut set = ParamSet::resolve(view.store());
        let n = set.apply_overrides(view.xml_section(), xml);
        (Self::from_set(&set), n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_view_inherits_common() {
        let set = ParamSet::resolve(STORE_FORWARD_VIEW);
        // Set on the view itself.
        assert_eq!(set.get("AccelerationForce"), Some(25.0));
        assert_eq!(set.get("JumpPower"), Some(17.0));
        // Only set on the common parent.
        assert_eq!(set.get("RotationForce0"), Some(100.0));
        assert_eq!(set.get("BoostTimeDecrease"), Some(6.0));
        let side = ParamSet::resolve(STORE_SIDE_VIEW);
        assert_eq!(side.get("AccelerationForce"), Some(12.5));
    }

    #[test]
    fn xml_overrides_apply_to_matching_section_only() {
        let (p, n) = SonicParams::load(
            View::Forward,
            [("ForwardView/Velocity/MaxVelocityBasis", 60.0), ("SideView/Velocity/MaxVelocityBasis", 30.0)],
        );
        assert_eq!(n, 1);
        assert_eq!(p.max_velocity_basis, 60.0);
    }
}
