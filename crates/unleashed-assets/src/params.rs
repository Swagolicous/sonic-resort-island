//! Hedgehog Engine parameter files (`*.prm.xml`).
//!
//! The engine registers every tunable with a default in code; these XML files
//! override a subset per mode. Layout:
//!
//! ```xml
//! <Sonic.prm.xml>
//!   <ForwardView>
//!     <Category><Velocity><Param><MaxVelocityBasis>60</MaxVelocityBasis></Param></Velocity></Category>
//!   </ForwardView>
//! </Sonic.prm.xml>
//! ```
//!
//! `Category` and `Param` are structural wrappers; everything else is a path
//! segment, so the example flattens to `ForwardView/Velocity/MaxVelocityBasis`.

use std::collections::BTreeMap;

use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct ParamFile {
    pub values: BTreeMap<String, String>,
}

impl ParamFile {
    pub fn f32(&self, path: &str) -> Option<f32> {
        self.values.get(path).and_then(|v| v.trim().parse().ok())
    }

    pub fn bool(&self, path: &str) -> Option<bool> {
        self.values.get(path).map(|v| v.trim() == "true")
    }
}

pub fn parse_param_xml(bytes: &[u8]) -> Result<ParamFile> {
    let text = std::str::from_utf8(bytes)
        .context("parameter file is not UTF-8")?
        .trim_start_matches('\u{feff}');
    let doc = roxmltree::Document::parse(text).context("parse parameter XML")?;
    let mut out = ParamFile::default();
    walk(doc.root_element(), &mut Vec::new(), &mut out, true);
    Ok(out)
}

fn walk(node: roxmltree::Node<'_, '_>, path: &mut Vec<String>, out: &mut ParamFile, is_root: bool) {
    let elements: Vec<_> = node.children().filter(|c| c.is_element()).collect();
    if elements.is_empty() && !is_root {
        if let Some(text) = node.text() {
            out.values.insert(path.join("/"), text.trim().to_owned());
        }
        return;
    }
    for child in elements {
        let name = child.tag_name().name();
        let structural = name == "Category" || name == "Param";
        if !structural {
            path.push(name.to_owned());
        }
        walk(child, path, out, false);
        if !structural {
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_categories() {
        let xml = br#"<?xml version="1.0"?><Sonic.prm.xml><ForwardView><Category><Velocity><Param>
            <MaxVelocityBasis>60</MaxVelocityBasis></Param></Velocity></Category></ForwardView></Sonic.prm.xml>"#;
        let p = parse_param_xml(xml).unwrap();
        assert_eq!(p.f32("ForwardView/Velocity/MaxVelocityBasis"), Some(60.0));
    }
}
