// This is the most basic use example from the readme.md

use bevy::{
    prelude::*,
    sprite::Anchor,
};
use bevy_asepritesheet::prelude::*;
use bevy_cursor::{CursorLocation, TrackCursorPlugin};
use bevy_inspector_egui::{prelude::*, quick::ResourceInspectorPlugin};
use bevy_xpbd_2d::{
    components::{AngularDamping, LinearVelocity, RigidBody},
    plugins::{collision::Collider, PhysicsPlugins},
};

#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Configuration {
    acceleration: f32,
    decceleration: f32,
    max_speed: f32,
    dash_speed: f32,
    dash_decelration: f32,
    dash_duration: f32,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            acceleration: 6000.0,
            decceleration: 4000.0,
            max_speed: 800.0,
            dash_speed: 2000.0,
            dash_decelration: 200000.0,
            dash_duration: 0.1,
        }
    }
}

#[derive(Default, Component, Reflect)]
struct Player {
    speed: Vec2,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            TrackCursorPlugin,
            AsepritesheetPlugin::new(&["json"]),
        ))
        .add_systems(Startup, setup)
        .init_resource::<Configuration>() // `ResourceInspectorPlugin` won't initialize the resource
        .register_type::<Configuration>() // you need to register your type to display it
        .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
        .add_plugins(PhysicsPlugins::default())
        // .add_systems(Startup, setup_aesprites)
        .add_systems(Update, (keyboard_input, look_cursor, dash, apply_force))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // spawn the camera so we can see the sprite
    commands.spawn(Camera2dBundle::default());

    // load the spritesheet and get it's handle
    let sheet_handle = load_spritesheet(
        &mut commands,
        &asset_server,
        "Tiny Swords/Factions/Goblins/Troops/Torch/Red/Torch_Red.json",
        Anchor::Center,
    );

    // spawn the animated sprite
    commands.spawn((
        Player { speed: Vec2::ZERO },
        Direction(Vec2::default()),
        AngularDamping(100.),
        RigidBody::Dynamic,
        Collider::triangle(
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, 0.0),
            Vec2::new(-0.5, 0.5),
        ),
        AnimatedSpriteBundle {
            animator: SpriteAnimator::from_anim(AnimHandle::from_index(1)),
            spritesheet: sheet_handle,
            ..Default::default()
        },
    ));
}

#[derive(Default, Component, Reflect)]
struct Dash {
    speed: Vec2,
    duration: f32,
}

#[derive(Default, Component)]
struct Direction(Vec2);

fn look_cursor(cursor: Res<CursorLocation>, mut query: Query<&mut Transform, With<Player>>) {
    for mut transform in &mut query {
        if let Some(cursor_pos) = cursor.world_position() {
            let dir = cursor_pos - transform.translation.xy();
            transform.rotation = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), dir.to_angle())
        }
    }
}

fn dash(
    time: Res<Time>,
    configurition: Res<Configuration>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut command: Commands,
    mut query: Query<(Entity, &Direction, Option<&mut Dash>)>,
) {
    for (id, dir, dash) in &mut query {
        if keyboard_input.just_pressed(KeyCode::Space) && dash.is_none() {
            command.entity(id).insert(Dash {
                speed: dir.0.normalize_or_zero() * configurition.dash_speed,
                duration: configurition.dash_duration,
            });
        } else if let Some(mut dash) = dash {
            if dash.duration > 0.0 {
                dash.duration -= time.delta_seconds();
            } else {
                let force = dash.speed.normalize_or_zero()
                    * configurition.dash_decelration
                    * time.delta_seconds();
                if force.length() >= dash.speed.length() {
                    dash.speed = Vec2::ZERO;
                    command.entity(id).remove::<Dash>();
                } else {
                    dash.speed -= force;
                }
            }
        }
    }
}

fn keyboard_input(
    time: Res<Time>,
    configurition: Res<Configuration>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Player, &mut Direction)>,
) {
    for (mut player, mut direction) in &mut query {
        direction.0 = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction.0.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction.0.x += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyW) {
            direction.0.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction.0.y -= 1.0;
        }

        if direction.0 != Vec2::ZERO {
            player.speed +=
                direction.0.normalize_or_zero() * configurition.acceleration * time.delta_seconds();
        } else {
            let force = player.speed.normalize_or_zero()
                * configurition.decceleration
                * time.delta_seconds();
            if force.length() > player.speed.length() {
                player.speed = Vec2::ZERO;
            } else {
                player.speed -= force;
            }
        }

        if player.speed.length() > configurition.max_speed {
            player.speed = player.speed.normalize_or_zero() * configurition.max_speed;
        }
    }
}

fn apply_force(mut query: Query<(&mut Player, &mut LinearVelocity, Option<&Dash>)>) {
    for (player, mut velocity, dash) in &mut query {
        velocity.0 = player.speed;
        if let Some(dash) = dash {
            velocity.0 += dash.speed;
        }
    }
}
