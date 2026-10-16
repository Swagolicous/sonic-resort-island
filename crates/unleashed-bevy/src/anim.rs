//! Sonic's skinned model driven by his original Havok animations.
//!
//! The skeleton (`SonicRoot.skl.hkx`) becomes a hierarchy of joint entities;
//! each `SonicRoot.model` submesh is skinned through its bone palette (model
//! bones are matched to skeleton bones by name, the model stores inverse bind
//! matrices). Every frame the active clip is sampled, cross-faded from the
//! previous pose, and written into the joint transforms.

use std::collections::HashMap;

use bevy::{
    camera::visibility::NoFrustumCulling,
    mesh::{
        VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    prelude::*,
};
use unleashed_assets::{
    animation::{Animation, QsTransform, Skeleton, parse_animation, parse_skeleton},
    mirage::mesh::parse_model,
};

use crate::{
    Run, Stage,
    convert::{self, Shading},
    player::{PlayerSet, Sonic, SonicBody},
    world::Converted,
};

pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, build_rig)
            .add_systems(Update, animate.after(PlayerSet))
            .add_systems(Last, log_head);
    }
}

const CLIPS: &[&str] = &[
    "sonic_idle",
    "sonic_walk",
    "sonic_jog",
    "sonic_run",
    "sonic_dash",
    "sonic_jump_U_loop",
    "sonic_jump_D_loop",
    "sonic_jump_spring",
    "sonic_stomping_loop",
    "sonic_run_quickstep_L",
    "sonic_run_quickstep_R",
    "sonic_damage_far",
    "sonic_result_reaction_A",
    "sonic_brake_M_L",
];

const BLEND_TIME: f32 = 0.15;

#[derive(Resource)]
pub struct SonicRig {
    skeleton: Skeleton,
    joints: Vec<Entity>,
    clips: HashMap<&'static str, Animation>,
    clip: &'static str,
    time: f32,
    pose: Vec<QsTransform>,
    blend_from: Vec<QsTransform>,
    blend: f32,
    scratch: Vec<QsTransform>,
    /// SonicRoot.model's bones (name, inverse bind) in model order: the order of the
    /// model-space matrices the game computes for Sonic each frame.
    model_bones: Vec<(String, Mat4)>,
}

fn build_rig(
    mut commands: Commands,
    stage: Res<Stage>,
    mut converted: ResMut<Converted>,
    body: Query<Entity, With<SonicBody>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    guest: Option<Res<crate::guest::GuestMode>>,
    mut chr_materials: ResMut<Assets<crate::chr::ChrMaterial>>,
) {
    let db = &stage.0.db;
    let Ok(body) = body.single() else { return };
    let (skeleton, model) = match (
        db.get("SonicRoot.skl.hkx").map(parse_skeleton),
        db.get("SonicRoot.model").map(parse_model),
    ) {
        (Some(Ok(s)), Some(Ok(m))) => (s, m),
        other => {
            warn!("Sonic rig unavailable: skeleton ok={} model ok={}",
                matches!(other.0, Some(Ok(_))), matches!(other.1, Some(Ok(_))));
            return;
        }
    };

    // Joint hierarchy in skeleton order (parents precede children).
    let mut joints: Vec<Entity> = Vec::with_capacity(skeleton.bones.len());
    for (i, name) in skeleton.bones.iter().enumerate() {
        let joint = commands
            .spawn((Name::new(name.clone()), to_transform(&skeleton.reference_pose[i]), Visibility::default()))
            .id();
        let parent = skeleton.parents[i];
        let parent = if parent >= 0 && (parent as usize) < joints.len() {
            joints[parent as usize]
        } else {
            body
        };
        commands.entity(parent).add_child(joint);
        joints.push(joint);
    }
    let by_name: HashMap<&str, usize> = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();

    let mut skinned = 0;
    let mut shadow_material: Option<Handle<StandardMaterial>> = None;
    for sub in &model.submeshes {
        if sub.triangles.is_empty() || sub.bone_palette.is_empty() {
            continue;
        }
        let palette: Vec<(Entity, Mat4)> = sub
            .bone_palette
            .iter()
            .map(|&b| {
                let bone = &model.bones[b as usize];
                let joint = by_name.get(bone.name.as_str()).map(|&i| joints[i]).unwrap_or(body);
                (joint, convert::row_major_to_mat4(&bone.matrix))
            })
            .collect();
        // Guest mode: Sonic's own shaders, skinned by the game's matrices; the Bevy-skinned
        // copy below is then only his shadow caster (chr::SHADOW_CASTER_LAYER).
        let mut shadow_only = false;
        if guest.is_some() {
            if let Some(material) = crate::chr::material_for(db, &mut converted.0, &mut images, &sub.material, sub.slot.as_ref()) {
                let handle = chr_materials.add(material);
                let mesh_entity = commands
                    .spawn((
                        Mesh3d(meshes.add(crate::chr::chr_mesh(sub))),
                        MeshMaterial3d(handle),
                        crate::chr::ChrPart { palette: sub.bone_palette.clone() },
                        NoFrustumCulling,
                        Transform::default(),
                    ))
                    .id();
                commands.entity(body).add_child(mesh_entity);
                skinned += 1;
                shadow_only = true;
            } else {
                warn!("guest: {} has no Sonic shader program; drawn with the Bevy material", sub.material);
            }
        }
        if shadow_only && matches!(sub.slot, Some(unleashed_assets::mirage::mesh::MeshSlot::Transparent | unleashed_assets::mirage::mesh::MeshSlot::Special(_))) {
            continue; // blended parts cast no shadow
        }
        let mut mesh = convert::submesh_mesh(sub, false);
        let count = sub.positions.len();
        let (indices, weights): (Vec<[u16; 4]>, Vec<[f32; 4]>) = (0..count)
            .map(|v| {
                let j = sub.joints.get(v).copied().unwrap_or([0, 255, 255, 255]);
                let w = sub.weights.get(v).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
                let mut idx = [0u16; 4];
                let mut wt = [0f32; 4];
                for k in 0..4 {
                    if (j[k] as usize) < palette.len() && w[k] > 0.0 {
                        idx[k] = j[k] as u16;
                        wt[k] = w[k];
                    }
                }
                let sum: f32 = wt.iter().sum();
                if sum <= 1e-6 {
                    ([0; 4], [1.0, 0.0, 0.0, 0.0])
                } else {
                    (idx, wt.map(|x| x / sum))
                }
            })
            .unzip();
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
        let material = if shadow_only {
            shadow_material.get_or_insert_with(|| materials.add(StandardMaterial::default())).clone()
        } else {
            convert::material(db, &mut converted.0, &mut images, &mut materials, &sub.material, sub.slot.as_ref(), Shading::Lit)
        };
        let inverse = bindposes.add(SkinnedMeshInverseBindposes::from(
            palette.iter().map(|(_, m)| *m).collect::<Vec<_>>(),
        ));
        let mesh_entity = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                SkinnedMesh {
                    inverse_bindposes: inverse,
                    joints: palette.iter().map(|(e, _)| *e).collect(),
                },
                NoFrustumCulling,
                Transform::default(),
            ))
            .id();
        if shadow_only {
            commands.entity(mesh_entity).insert(bevy::camera::visibility::RenderLayers::layer(crate::chr::SHADOW_CASTER_LAYER));
        }
        commands.entity(body).add_child(mesh_entity);
        skinned += 1;
    }

    let mut clips = HashMap::new();
    for &name in CLIPS {
        match db.get(&format!("{name}.anm.hkx")).map(parse_animation) {
            Some(Ok(clip)) => {
                clips.insert(name, clip);
            }
            Some(Err(e)) => warn!("animation {name}: {e:#}"),
            None => warn!("animation {name}: missing"),
        }
    }
    info!(
        "Sonic rig: {} joints, {skinned} skinned meshes, {} clips",
        joints.len(),
        clips.len()
    );
    let pose = skeleton.reference_pose.clone();
    commands.insert_resource(SonicRig {
        skeleton,
        joints,
        clips,
        clip: "sonic_idle",
        time: 0.0,
        blend_from: pose.clone(),
        pose,
        blend: 1.0,
        scratch: Vec::new(),
        model_bones: model.bones.iter().map(|b| (b.name.clone(), convert::row_major_to_mat4(&b.matrix))).collect(),
    });
}

/// Guest mode: where the game's bone matrices are and which rig joint each drives.
pub struct GuestPose {
    array: Option<u32>,
    joint_of: Vec<Option<usize>>,
}

fn to_transform(q: &QsTransform) -> Transform {
    Transform {
        translation: Vec3::from(q.translation),
        rotation: Quat::from_array(q.rotation).normalize(),
        scale: Vec3::from(q.scale),
    }
}

/// Chooses the clip and playback rate for Sonic's current state.
fn choose(s: &Sonic, run: &Run) -> (&'static str, f32, bool) {
    let speed = s.speed();
    if s.dead > 0.0 {
        return ("sonic_damage_far", 1.0, false);
    }
    if run.cleared && speed < 3.0 {
        return ("sonic_result_reaction_A", 1.0, false);
    }
    if !s.grounded {
        if s.stomping {
            return ("sonic_stomping_loop", 1.0, true);
        }
        return if s.vel.y > 2.0 {
            ("sonic_jump_U_loop", 1.0, true)
        } else {
            ("sonic_jump_D_loop", 1.0, true)
        };
    }
    if s.quickstep > 0.0 {
        let right = s.up.cross(s.facing).dot(s.quickstep_dir) < 0.0;
        return (if right { "sonic_run_quickstep_R" } else { "sonic_run_quickstep_L" }, 2.0, false);
    }
    if s.braking() {
        return ("sonic_brake_M_L", 1.0, true);
    }
    // Motion* thresholds from the parameter table (walk-fast and jet share clips).
    let p = &s.ctl.params;
    match speed {
        v if v < p.motion_stand_to_walk.max(0.6) => ("sonic_idle", 1.0, true),
        v if v < p.motion_walk_fast_to_jog => ("sonic_walk", (v / 3.0).clamp(0.5, 1.8), true),
        v if v < p.motion_jog_to_run => ("sonic_jog", (v / 10.0).clamp(0.6, 1.6), true),
        v if v < p.motion_run_to_dash && !s.boosting => ("sonic_run", (v / 22.0).clamp(0.7, 1.6), true),
        v => ("sonic_dash", (v / 60.0).clamp(0.8, 1.5), true),
    }
}

fn animate(
    time: Res<Time>,
    run: Res<Run>,
    rig: Option<ResMut<SonicRig>>,
    sonic: Query<&Sonic>,
    mut transforms: Query<&mut Transform>,
    guest: Option<NonSendMut<crate::guest::GuestPlayer>>,
    mut guest_map: Local<Option<GuestPose>>,
    mut chr_skin: ResMut<crate::chr::ChrSkin>,
    resort: Option<Res<crate::resort::ResortSettings>>,
) {
    let Some(mut rig) = rig else { return };
    let Ok(s) = sonic.single() else { return };
    let rig = &mut *rig;
    // Guest mode: the pose the game's own animation system computed this frame.
    if let Some(mut guest) = guest {
        let model_count = rig.model_bones.len();
        // Until the game has stepped a frame no run has changed yet: look again.
        if guest_map.as_ref().is_some_and(|m| m.array.is_none()) {
            *guest_map = None;
        }
        let map = guest_map.get_or_insert_with(|| {
            // The game's model-space bone matrices for SonicRoot.model: a run of exactly
            // one matrix per model bone on the guest heap.
            let runs = guest.sonic.animated_matrix_runs(model_count);
            let array = runs.first().copied();
            let joint_of: Vec<Option<usize>> =
                rig.model_bones.iter().map(|(n, _)| rig.skeleton.bones.iter().position(|b| b == n)).collect();
            if array.is_some() {
                // Order check: static 111-matrix arrays against the model's bind pose.
                for (a, n) in guest.sonic.matrix_runs(model_count) {
                    if n != model_count { continue; }
                    let m = guest.sonic.matrices(a, n);
                    let (mut same, mut inv, mut any) = (0, 0, 0);
                    for k in 0..n {
                        let g = Mat4::from_cols_array(&m[k]);
                        let bind = rig.model_bones[k].1.inverse();
                        if (g.w_axis.truncate() - bind.w_axis.truncate()).length() < 1e-3 { same += 1; }
                        if (g.w_axis.truncate() - rig.model_bones[k].1.w_axis.truncate()).length() < 1e-3 { inv += 1; }
                        if rig.model_bones.iter().any(|(_, ib)| (g.w_axis.truncate() - ib.inverse().w_axis.truncate()).length() < 1e-3) { any += 1; }
                    }
                    info!("guest pose order check {a:08X}: bind T same index {same}/{n}, inverse-bind T same index {inv}/{n}, bind T anywhere {any}/{n}");
                }
                info!("guest pose: model {model_count} bones ({} on the skeleton); matrix runs {:08X?}; using {:08X?}",
                joint_of.iter().flatten().count(), runs, array); }
            if let Some(a) = array {
                let m = guest.sonic.matrices(a, model_count);
                for k in [2usize, 3, 5, 6] {
                    if k >= model_count { continue; }
                    let g = Mat4::from_cols_array(&m[k]);
                    let bind = rig.model_bones[k].1.inverse();
                    info!("guest pose: bone {:10} as model-space T {:6.3?} | as skin, model T {:6.3?} | bind T {:6.3?}", rig.model_bones[k].0,
                        g.w_axis.truncate(), (g * bind).w_axis.truncate(), bind.w_axis.truncate());
                }
            }
            GuestPose { array, joint_of }
        });
        if let Some(array) = map.array {
            // The game writes the palette into two buffers on alternate frames (the
            // update and render sides swap): draw the one the last game frame wrote.
            guest.sonic.track_skin(model_count);
            let array = guest.sonic.skin().unwrap_or(array);
            chr_skin.0 = Some(array);
            let mats = if resort.is_some() { guest.rendered_skin(array,model_count) } else { guest.sonic.matrices(array, model_count) };
            let n = rig.skeleton.bones.len();
            let mut model: Vec<Option<Mat4>> = vec![None; n];
            // The game's array holds skinning matrices (model-space bone * inverse bind,
            // column vectors, translation in floats 12-14): model = skin * bind.
            for (k, j) in map.joint_of.iter().enumerate() {
                if let Some(j) = *j {
                    model[j] = Some(Mat4::from_cols_array(&mats[k]) * rig.model_bones[k].1.inverse());
                }
            }
            // Skeleton bones the model has no matrix for follow their parent at rest.
            for j in 0..n {
                if model[j].is_none() {
                    let p = rig.skeleton.parents[j];
                    let parent = if p >= 0 { model[p as usize].unwrap_or(Mat4::IDENTITY) } else { Mat4::IDENTITY };
                    model[j] = Some(parent * to_transform(&rig.skeleton.reference_pose[j]).to_matrix());
                }
            }
            for j in 0..n {
                let p = rig.skeleton.parents[j];
                let parent = if p >= 0 { model[p as usize].unwrap() } else { Mat4::IDENTITY };
                let local = parent.inverse() * model[j].unwrap();
                if let Ok(mut transform) = transforms.get_mut(rig.joints[j]) {
                    *transform = Transform::from_matrix(local);
                }
            }
            return;
        }
    }
    let dt = time.delta_secs();
    let (clip, rate, looping) = choose(s, &run);
    if clip != rig.clip && rig.clips.contains_key(clip) {
        rig.blend_from = rig.pose.clone();
        rig.blend = 0.0;
        rig.clip = clip;
        rig.time = 0.0;
    }
    let Some(anim) = rig.clips.get(rig.clip) else { return };
    rig.time += dt * rate;
    if looping {
        rig.time = rig.time.rem_euclid(anim.duration.max(1e-3));
    } else {
        rig.time = rig.time.min(anim.duration);
    }
    anim.sample(rig.time, &mut rig.scratch);

    let mut pose = rig.skeleton.reference_pose.clone();
    for (track, q) in rig.scratch.iter().enumerate() {
        if let Some(&bone) = anim.track_to_bone.get(track) {
            if bone >= 0 && (bone as usize) < pose.len() {
                pose[bone as usize] = *q;
            }
        }
    }
    rig.blend = (rig.blend + dt / BLEND_TIME).min(1.0);
    if rig.blend < 1.0 {
        let t = rig.blend;
        for (p, from) in pose.iter_mut().zip(&rig.blend_from) {
            let a = Vec3::from(from.translation).lerp(Vec3::from(p.translation), t);
            let r = Quat::from_array(from.rotation).slerp(Quat::from_array(p.rotation), t);
            p.translation = a.into();
            p.rotation = r.to_array();
        }
    }
    for (joint, q) in rig.joints.iter().zip(&pose) {
        if let Ok(mut transform) = transforms.get_mut(*joint) {
            *transform = to_transform(q);
        }
    }
    rig.pose = pose;
}

/// Guest mode diagnostics: where Sonic's head and body are, and where the head projects.
fn log_head(
    time: Res<Time>,
    rig: Option<Res<SonicRig>>,
    globals: Query<&GlobalTransform>,
    body: Query<&GlobalTransform, With<SonicBody>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    guest: Option<NonSend<crate::guest::GuestPlayer>>,
) {
    if guest.is_none() || (time.elapsed_secs() as u32) == ((time.elapsed_secs() - time.delta_secs()) as u32) {
        return;
    }
    let Some(rig) = rig else { return };
    let Some(head) = rig.skeleton.bones.iter().position(|b| b == "Head") else { return };
    let (Ok(h), Ok(b), Ok((cam, ct))) = (globals.get(rig.joints[head]), body.single(), camera.single()) else { return };
    let ndc = cam.world_to_ndc(ct, h.translation());
    info!("guest head: world {:?} body {:?} head ndc {:?}", h.translation(), b.translation(), ndc);
}
