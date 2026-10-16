//! Data-driven loader for Sonic Unleashed daytime action stages (`ActD_*`).
//!
//! Resolution follows the stage descriptor exactly like the engine does:
//! `Stage.stg.xml` names the spawn, collision container, sky model and set
//! layers; terrain groups stream from `Packed/<stage>/Stage.pfd`; set layers
//! in `update/work/<stage>/` override the disc copies.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    collision::havok::{CollisionBody, parse_collision},
    mirage::{
        material::{TerrainInstance, parse_terrain_instance},
        mesh::{MirageModel, parse_terrain_model},
    },
    placement::{PlacedObject, parse_set_file},
    resources::{ResourceDb, read_maybe_compressed},
};

/// Language overlay mounted over each archive (`game/Languages/<LANGUAGE>`).
pub const LANGUAGE: &str = "English";

#[derive(Debug, Clone)]
pub struct Spawn {
    pub position: [f32; 3],
    pub yaw_degrees: f32,
}

/// One placed set object. Multi-set objects are expanded into one entry per
/// element, all sharing the parent's parameters.
#[derive(Debug, Clone)]
pub struct SetObject {
    pub kind: String,
    pub layer: String,
    pub id: u32,
    pub position: [f32; 3],
    /// Quaternion as (x, y, z, w).
    pub rotation: [f32; 4],
    pub params: HashMap<String, String>,
}

impl SetObject {
    pub fn f32(&self, key: &str) -> Option<f32> {
        self.params.get(key).and_then(|v| v.trim().parse().ok())
    }

    pub fn flag(&self, key: &str) -> bool {
        self.params.get(key).is_some_and(|v| v.trim() == "true")
    }
}

pub struct DayStage {
    pub name: String,
    pub root: PathBuf,
    pub db: ResourceDb,
    pub spawn: Spawn,
    pub sky_model: Option<String>,
    pub instances: Vec<TerrainInstance>,
    pub models: HashMap<String, MirageModel>,
    pub collision: Vec<CollisionBody>,
    pub objects: Vec<SetObject>,
    pub diagnostics: Vec<String>,
}

/// `root` is the install root containing `game/` (and optionally `update/`).
/// `common_archives` are loaded first so stage-local resources win.
pub fn load_day_stage(root: &Path, stage: &str, common_archives: &[&str]) -> Result<DayStage> {
    let game = root.join("game");
    if !game.join(format!("#{stage}.arl")).is_file() {
        bail!("{} has no game/#{stage}.arl", root.display());
    }
    let mut diagnostics = Vec::new();
    let mut db = ResourceDb::new();

    let region = stage_region(stage);
    let mut archives: Vec<String> = common_archives.iter().map(|s| s.to_string()).collect();
    for derived in [
        format!("Cmn{region}"),
        format!("CmnActD_Terrain_{region}"),
        format!("SonicActionCommon_{region}"),
        format!("#SonicActionCommon_{region}"),
    ] {
        if game.join(format!("{derived}.arl")).is_file() {
            archives.push(derived);
        }
    }
    archives.push(stage.to_owned());
    archives.push(format!("#{stage}"));
    let language_dir = game.join("Languages").join(LANGUAGE);
    for stem in &archives {
        match db.load_arl(&game, stem) {
            Ok(n) => diagnostics.push(format!("archive {stem}: {n} entries")),
            Err(e) if common_archives.contains(&stem.as_str()) => {
                diagnostics.push(format!("archive {stem} skipped: {e:#}"))
            }
            Err(e) => return Err(e.context(format!("load archive {stem}"))),
        }
        // Localised overlay (UI text textures, voice, subtitles) wins over the base.
        if language_dir.join(format!("{stem}.arl")).is_file() {
            match db.load_arl(&language_dir, stem) {
                Ok(n) => diagnostics.push(format!("archive {LANGUAGE}/{stem}: {n} entries")),
                Err(e) => diagnostics.push(format!("archive {LANGUAGE}/{stem} skipped: {e:#}")),
            }
        }
    }

    let pfd = game.join("Packed").join(stage).join("Stage.pfd");
    let groups = db
        .load_pfd(&pfd, |name| name.starts_with("tg-"))
        .with_context(|| format!("load {}", pfd.display()))?;
    diagnostics.push(format!("terrain groups: {}", groups.len()));

    let stg_bytes = db.get("Stage.stg.xml").context("Stage.stg.xml missing")?;
    let stg = std::str::from_utf8(stg_bytes)
        .context("Stage.stg.xml not UTF-8")?
        .trim_start_matches('\u{feff}');
    let doc = roxmltree::Document::parse(stg).context("parse Stage.stg.xml")?;
    let root_el = doc.root_element();

    let num = |path: &[&str]| -> Option<f32> { text_at(root_el, path)?.trim().parse().ok() };
    let spawn = Spawn {
        position: [
            num(&["Sonic", "Position", "x"]).context("spawn x")?,
            num(&["Sonic", "Position", "y"]).context("spawn y")?,
            num(&["Sonic", "Position", "z"]).context("spawn z")?,
        ],
        yaw_degrees: num(&["Sonic", "Yaw"]).unwrap_or(0.0),
    };
    let sky_model = text_at(root_el, &["Sky", "Model"]).map(str::to_owned);

    // Terrain instances and the models they reference.
    let mut instances = Vec::new();
    let mut names: Vec<&str> = db
        .names()
        .filter(|n| n.ends_with(".terrain-instanceinfo"))
        .collect();
    names.sort_unstable();
    for name in names {
        match parse_terrain_instance(db.get(name).unwrap()) {
            Ok(instance) => instances.push(instance),
            Err(e) => diagnostics.push(format!("{name}: {e:#}")),
        }
    }
    let mut models = HashMap::new();
    let wanted: HashSet<&str> = instances.iter().map(|i| i.model.as_str()).collect();
    for model in wanted {
        let file = format!("{model}.terrain-model");
        match db.get(&file).map(parse_terrain_model) {
            Some(Ok(m)) => {
                models.insert(model.to_owned(), m);
            }
            Some(Err(e)) => diagnostics.push(format!("{file}: {e:#}")),
            None => diagnostics.push(format!("{file}: referenced but missing")),
        }
    }
    let unreferenced = db
        .names()
        .filter_map(|n| n.strip_suffix(".terrain-model"))
        .filter(|n| !models.contains_key(*n))
        .count();
    diagnostics.push(format!(
        "terrain: {} instances, {} models ({} models never instanced)",
        instances.len(),
        models.len(),
        unreferenced
    ));

    // Collision.
    let mut collision = Vec::new();
    for container in descendants_text(root_el, "RigidBodyContainer") {
        let file = format!("{container}.phy.hkx");
        let bytes = db
            .get(&file)
            .with_context(|| format!("collision {file} missing"))?;
        collision.extend(parse_collision(bytes).with_context(|| format!("parse {file}"))?);
    }
    diagnostics.push(format!("collision bodies: {}", collision.len()));

    // Set layers.
    let mut objects = Vec::new();
    let update_dir = root.join("update").join("work").join(stage);
    for layer in root_el
        .descendants()
        .filter(|n| n.has_tag_name("Layer"))
    {
        let Some(file) = child_text(layer, "FileName") else {
            continue;
        };
        let active = child_text(layer, "IsGameActive").is_none_or(|v| v.trim() == "true");
        if !active {
            continue;
        }
        let layer_name = child_text(layer, "Name").unwrap_or(file).to_owned();
        let override_path = update_dir.join(file);
        let bytes = if override_path.is_file() {
            diagnostics.push(format!("set layer {file}: update override"));
            read_maybe_compressed(&override_path)?
        } else if let Some(bytes) = db.get(file) {
            bytes.to_vec()
        } else {
            diagnostics.push(format!("set layer {file}: missing"));
            continue;
        };
        let set = parse_set_file(&bytes).map_err(|e| anyhow::anyhow!("{file}: {e}"))?;
        for object in &set.objects {
            expand_set_object(object, &layer_name, &mut objects);
        }
    }
    diagnostics.push(format!("set objects: {}", objects.len()));

    Ok(DayStage {
        name: stage.to_owned(),
        root: root.to_path_buf(),
        db,
        spawn,
        sky_model,
        instances,
        models,
        collision,
        objects,
        diagnostics,
    })
}

/// `ActD_MykonosAct1` -> `Mykonos`, `ActD_SubEU_01` -> `EU`.
pub fn stage_region(stage: &str) -> String {
    let body = stage
        .trim_start_matches("ActD_")
        .trim_start_matches("ActN_")
        .trim_start_matches("Sub");
    let body = body.split('_').next().unwrap_or(body);
    match body.find("Act") {
        Some(i) if i > 0 => body[..i].to_owned(),
        _ => body.to_owned(),
    }
}

fn expand_set_object(object: &PlacedObject, layer: &str, out: &mut Vec<SetObject>) {
    let mut params = HashMap::new();
    let mut elements: Vec<([f32; 3], [f32; 4])> = Vec::new();
    let mut current: Option<([f32; 3], [f32; 4])> = None;
    for p in &object.parameters {
        if let Some(rest) = p.key.strip_prefix("MultiSetParam/Element/") {
            let value: f32 = p.value.trim().parse().unwrap_or(0.0);
            if rest == "Index" {
                if let Some(e) = current.take() {
                    elements.push(e);
                }
                current = Some(([0.0; 3], [0.0, 0.0, 0.0, 1.0]));
                continue;
            }
            let e = current.get_or_insert(([0.0; 3], [0.0, 0.0, 0.0, 1.0]));
            match rest {
                "Position/x" => e.0[0] = value,
                "Position/y" => e.0[1] = value,
                "Position/z" => e.0[2] = value,
                "Rotation/x" => e.1[0] = value,
                "Rotation/y" => e.1[1] = value,
                "Rotation/z" => e.1[2] = value,
                "Rotation/w" => e.1[3] = value,
                _ => {}
            }
            continue;
        }
        params.entry(p.key.clone()).or_insert_with(|| p.value.clone());
    }
    if let Some(e) = current.take() {
        elements.push(e);
    }
    let id = params
        .get("SetObjectID")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    let base = (
        [object.position.x, object.position.y, object.position.z],
        [
            object.rotation.x,
            object.rotation.y,
            object.rotation.z,
            object.rotation.w,
        ],
    );
    for (position, rotation) in std::iter::once(base).chain(elements) {
        out.push(SetObject {
            kind: object.object_type.clone(),
            layer: layer.to_owned(),
            id,
            position,
            rotation,
            params: params.clone(),
        });
    }
}

fn child_text<'a>(node: roxmltree::Node<'a, '_>, tag: &str) -> Option<&'a str> {
    node.children()
        .find(|c| c.has_tag_name(tag))
        .and_then(|c| c.text())
}

fn text_at<'a>(node: roxmltree::Node<'a, '_>, path: &[&str]) -> Option<&'a str> {
    let mut current = node;
    for tag in path {
        current = current.children().find(|c| c.has_tag_name(*tag))?;
    }
    current.text()
}

fn descendants_text<'a>(node: roxmltree::Node<'a, '_>, tag: &str) -> Vec<&'a str> {
    node.descendants()
        .filter(|n| n.has_tag_name(tag))
        .filter_map(|n| n.text())
        .map(str::trim)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::stage_region;

    #[test]
    fn regions() {
        assert_eq!(stage_region("ActD_MykonosAct1"), "Mykonos");
        assert_eq!(stage_region("ActD_SubEU_01"), "EU");
        assert_eq!(stage_region("ActD_Africa"), "Africa");
    }
}
