//! Loads a day stage end-to-end and prints what was resolved.
//! usage: inspect_day_stage <install_root> [stage]

use anyhow::Result;
use std::collections::BTreeMap;
use unleashed_assets::{
    collision::havok::CollisionShape,
    mirage::material::{parse_material, parse_texset, parse_texture},
    stage::day::load_day_stage,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let root = args.next().unwrap_or_else(|| "..".into());
    let stage_name = args.next().unwrap_or_else(|| "ActD_MykonosAct1".into());
    let t = std::time::Instant::now();
    let stage = load_day_stage(root.as_ref(), &stage_name, &["Sonic", "#Sonic", "ActionCommon", "SystemCommon"])?;
    println!("loaded in {:?}", t.elapsed());
    for d in &stage.diagnostics {
        println!("  {d}");
    }
    println!("spawn {:?} sky {:?}", stage.spawn, stage.sky_model);

    // Winding check: geometric normal vs authored vertex normals.
    let (mut agree, mut disagree, mut tris, mut verts) = (0usize, 0usize, 0usize, 0usize);
    let mut shaders = BTreeMap::new();
    let mut missing_tex = BTreeMap::new();
    let mut kinds = BTreeMap::new();
    for model in stage.models.values() {
        for mesh in &model.submeshes {
            tris += mesh.triangles.len();
            verts += mesh.positions.len();
            if !mesh.normals.is_empty() {
                for t in &mesh.triangles {
                    let p = t.map(|i| mesh.positions[i as usize]);
                    let e1 = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
                    let e2 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
                    let c = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
                    let n = mesh.normals[t[0] as usize];
                    if c[0] * n[0] + c[1] * n[1] + c[2] * n[2] > 0.0 { agree += 1 } else { disagree += 1 }
                }
            }
            let mat = format!("{}.material", mesh.material);
            match stage.db.get(&mat).map(parse_material) {
                Some(Ok(m)) => {
                    *shaders.entry(m.shader.clone()).or_insert(0) += 1;
                    if let Some(Ok(texs)) = stage.db.get(&format!("{}.texset", m.texset)).map(parse_texset) {
                        for tx in texs {
                            if let Some(Ok(r)) = stage.db.get(&format!("{tx}.texture")).map(parse_texture) {
                                *kinds.entry(format!("{}@uv{}", r.kind, r.texcoord)).or_insert(0) += 1;
                                if !stage.db.contains(&format!("{}.dds", r.dds)) {
                                    *missing_tex.entry(r.dds).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => println!("material {mat}: {e:#}"),
                None => *missing_tex.entry(format!("MATERIAL {mat}")).or_insert(0) += 1,
            }
        }
    }
    println!("terrain tris {tris} verts {verts}; winding agree {agree} disagree {disagree}");
    println!("shaders {shaders:?}");
    println!("texture kinds {kinds:?}");
    println!("missing {missing_tex:?}");

    let mut col_tris = 0;
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for body in &stage.collision {
        if let CollisionShape::Mesh { vertices, triangles } = &body.shape {
            col_tris += triangles.len();
            for v in vertices {
                for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); }
            }
        }
    }
    println!("collision tris {col_tris} bounds {lo:?}..{hi:?}");
    let mut kinds = BTreeMap::new();
    for o in &stage.objects { *kinds.entry(o.kind.as_str()).or_insert(0) += 1; }
    println!("objects {kinds:?}");
    Ok(())
}
