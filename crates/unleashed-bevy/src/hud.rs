//! Minimal HUD: rings, time, speed, boost gauge, controls and stage clear.

use bevy::prelude::*;

use crate::{Run, player::Sonic};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
            .add_systems(Update, update_hud);
    }
}

#[derive(Component)]
struct Banner;

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(
            "WASD/stick move  Space/A jump (again: homing)  Shift/X boost  Ctrl/B stomp  \
             Q/E quickstep  R respawn  click: mouse look  Esc: free cursor",
        ),
        TextFont::from_font_size(15.0),
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            bottom: Val::Px(20.0),
            ..default()
        },
    ));
    commands.spawn((
        Banner,
        Text::new(""),
        TextFont::from_font_size(64.0),
        TextColor(Color::srgb(1.0, 0.9, 0.3)),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(30.0),
            top: Val::Percent(35.0),
            ..default()
        },
    ));
}

fn update_hud(run: Res<Run>, sonic: Query<&Sonic>, mut banner: Query<&mut Text, With<Banner>>) {
    let Ok(s) = sonic.single() else { return };
    if let Ok(mut text) = banner.single_mut() {
        // The original CSD clear banner and results screen cover the goal.
        text.0 = if run.cleared {
            String::new()
        } else if s.dead > 0.0 {
            "MISS".to_owned()
        } else {
            String::new()
        };
    }
}
