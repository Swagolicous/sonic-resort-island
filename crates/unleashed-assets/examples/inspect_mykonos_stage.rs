use anyhow::Result;
use unleashed_assets::stage::load_actd_mykonos_act1;

fn main() -> Result<()> {
    let root = std::env::args().nth(1).unwrap_or_else(|| "..".to_owned());
    let stage = load_actd_mykonos_act1(&root)?;

    println!("game_dir={}", stage.game_dir.display());
    println!(
        "counts control={} pfi={} pfd={} groups_total={} groups_loaded={} groups_empty={} groups_skipped={} models={} instances={} resources_skipped={}",
        stage.counts.control_entries,
        stage.counts.pfi_entries,
        stage.counts.pfd_entries,
        stage.counts.terrain_groups_total,
        stage.counts.terrain_groups_loaded,
        stage.counts.terrain_groups_empty,
        stage.counts.terrain_groups_skipped,
        stage.counts.terrain_models,
        stage.counts.terrain_instances,
        stage.counts.resources_skipped,
    );

    println!("=== MODELS ===");
    for model in &stage.models {
        println!(
            "{} | {} | vertices={} indices={} stride={}",
            model.group_name,
            model.resource_name,
            model.mesh.positions.len(),
            model.mesh.indices.len(),
            model.mesh.vertex_stride
        );
    }

    println!("=== INSTANCES ===");
    for instance in &stage.instances {
        let m = &instance.transform.matrix;
        println!(
            "{} | {} | rows=[{:?},{:?},{:?},{:?}]",
            instance.group_name, instance.resource_name, m[0], m[1], m[2], m[3]
        );
    }

    println!("=== DIAGNOSTICS ===");
    for diagnostic in &stage.diagnostics {
        println!(
            "{:?} | {} | {}",
            diagnostic.severity, diagnostic.scope, diagnostic.message
        );
    }

    Ok(())
}
