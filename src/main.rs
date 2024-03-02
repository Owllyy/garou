// This is the most basic use example from the readme.md

use bevy_ecs_ldtk::prelude::*;
use bevy::{
    math::vec3, prelude::*, sprite::Anchor
};
use bevy_asepritesheet::prelude::*;
use bevy_inspector_egui::{prelude::*, quick::{ResourceInspectorPlugin, WorldInspectorPlugin}};
use bevy_xpbd_2d::{
    components::{LinearVelocity, RigidBody, Collider},
    plugins::{PhysicsPlugins, PhysicsDebugPlugin}, resources::Gravity,
};

#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Configuration {
    acceleration: f32,
    decceleration: f32,
    max_speed: f32,
    dash_speed: f32,
    dash_deceleration: f32,
    dash_duration: f32,
    dash_cooldown: f32,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            acceleration: 3000.0,
            decceleration: 2000.0,
            max_speed: 400.0,
            dash_speed: 1000.0,
            dash_deceleration: 10000.0,
            dash_duration: 0.1,
            dash_cooldown: 1.0,
        }
    }
}


#[derive(Component, Reflect)]
struct Attack {
    duration: f32,
}

impl Default for Attack {
    fn default() -> Self {
        Self { duration: 0.45 }
    }
}

#[derive(Default, Component, Reflect)]
struct Player {
    speed: Vec2,
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    direction: Direction,
    rigid_body: RigidBody,
    collider: Collider,
    animated_sprite_bundle: AnimatedSpriteBundle,
}

impl PlayerBundle {
    fn new(spritesheet: Handle<Spritesheet>) -> Self {
        Self {
            player: Player { speed: Vec2::ZERO },
            direction: Direction(Vec2::ZERO),
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(70.0, 70.0),
            animated_sprite_bundle: AnimatedSpriteBundle {
                animator: SpriteAnimator::from_anim(AnimHandle::from_index(1)),
                sprite_bundle: SpriteSheetBundle{
                    transform: Transform::from_translation(vec3(1., 1., 10.)),
                    ..Default::default()
                },
                spritesheet,
                ..Default::default()
            },
        }
    }
}

#[derive(Default, Component, Reflect)]
struct Enemy;

#[derive(Bundle)]
struct EnemyBundle {
    enemy: Enemy,
    rigid_body: RigidBody,
    collider: Collider,
    animated_sprite_bundle: AnimatedSpriteBundle,
}

impl EnemyBundle {
    fn new(spritesheet_handle: Handle<Spritesheet>) -> Self {
        Self {
            enemy: Enemy,
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(70.0, 70.0),
            animated_sprite_bundle: AnimatedSpriteBundle {
                animator: SpriteAnimator::from_anim(AnimHandle::from_index(5)),
                spritesheet: spritesheet_handle,
                ..Default::default()
            },
        }
    }
}

#[derive(Resource, Reflect, Deref, DerefMut)]
struct EnemySpawnTimer(Timer);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            AsepritesheetPlugin::new(&["json"]),
        ))
        .add_systems(Startup, setup)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
        .register_type::<EnemySpawnTimer>()
        .register_type::<Configuration>() // you need to register your type to display it
        .insert_resource(EnemySpawnTimer(Timer::from_seconds(3.0, TimerMode::Once)))
        .init_resource::<Configuration>() // `ResourceInspectorPlugin` won't initialize the resource
        .insert_resource(Gravity::ZERO)
        // .add_systems(Startup, setup_aesprites)
        .add_systems(Update, (keyboard_input, dash, apply_force, following_cam))
        .add_systems(Update, spawn_enemies)
        .add_plugins(LdtkPlugin)
        .insert_resource(LevelSelection::index(0))
        .add_systems(Update, (keyboard_input, dash, apply_force, following_cam, animate, attack))
        .run();
}

fn spawn_enemies(
    time: Res<Time>,
    mut commands: Commands,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    asset_server: Res<AssetServer>,
) {
    for _ in 0..spawn_timer.tick(time.delta()).times_finished_this_tick() {
        // load the spritesheet and get it's handle
        let enemy_spritesheet = load_spritesheet(
            &mut commands,
            &asset_server,
            "Tiny Swords/Factions/Knights/Troops/Pawn/Blue/Pawn_Blue.json",
            Anchor::Center,
        );

        // spawn the animated sprite
        commands.spawn(EnemyBundle::new(enemy_spritesheet));
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // spawn the camera so we can see the sprite
    commands.spawn(Camera2dBundle::default());

    // load the spritesheet and get it's handle
    let player_spritesheet = load_spritesheet(
        &mut commands,
        &asset_server,
        "Tiny Swords/Factions/Goblins/Troops/Torch/Red/Torch_Red.json",
        Anchor::Center,
    );

    //spawn map
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("map.ldtk"),
        transform: Transform::default().with_translation(vec3(0., 0., -1.)),
        ..Default::default()
    });

    // spawn the animated sprite
    commands.spawn(PlayerBundle::new(player_spritesheet));
}


#[derive(Default, Component)]
struct Direction(Vec2);

#[derive(Default, Component, Reflect)]
struct Dash {
    speed: Vec2,
    duration: f32,
    cooldown: f32,
}

fn dash(
    time: Res<Time>,
    configuration: Res<Configuration>,
    keyboard_input: Res<Input<KeyCode>>,
    mut command: Commands,
    mut query: Query<(Entity, &Direction, Option<&mut Dash>), With<Player>>,
) {
    for (id, dir, dash) in &mut query {
        if keyboard_input.just_pressed(KeyCode::Space) && dash.is_none() {
            command.entity(id).insert(Dash {
                speed: dir.0.normalize_or_zero() * configuration.dash_speed,
                duration: configuration.dash_duration,
                cooldown: configuration.dash_cooldown,
            });
        } else if let Some(mut dash) = dash {
            if dash.duration > 0.0 {
                dash.duration -= time.delta_seconds();
            } else {
                if dash.speed.length() > 0.{
                    let force = dash.speed.normalize_or_zero()
                        * configuration.dash_deceleration
                        * time.delta_seconds();
                    if force.length() >= dash.speed.length() {
                        dash.speed = Vec2::ZERO;
                    } else {
                        dash.speed -= force;
                    }
                } else {
                    if dash.cooldown > 0.0 {
                        dash.cooldown -= time.delta_seconds();
                    } else {
                        command.entity(id).remove::<Dash>();
                    }
                }
            }
        }
    }
}

fn following_cam(
    player: Query<&Transform, With<Player>>,
    mut cam: Query<&mut Transform, (With<Camera>, Without<Player>)>
) {
    cam.single_mut().translation = player.single().translation;
}

fn keyboard_input(
    time: Res<Time>,
    configurition: Res<Configuration>,
    mut command: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(Entity, &mut Player, &mut Direction, &mut Transform)>,
) {
    for (entity, mut player, mut direction, mut transform) in &mut query {
        direction.0 = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::Left) {
            direction.0.x -= 1.0;
            if transform.scale.x > 0. {
                transform.scale.x *= -1.;
            }
        }
        if keyboard_input.pressed(KeyCode::Right) {
            if transform.scale.x < 0. {
                transform.scale.x *= -1.;
            }
            direction.0.x += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            direction.0.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
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

        if keyboard_input.pressed(KeyCode::K) {
            command.entity(entity).insert(Attack::default());
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

fn attack(mut query: Query<(Entity, Option<&mut Attack>), With<Player>>,
mut command: Commands,
time: Res<Time>) 
{
    let (entity, attack) = query.single_mut();
    if let Some(mut attack) = attack {
        if attack.duration > 0. {
            attack.duration -= time.delta_seconds();
        } else {
            command.entity(entity).remove::<Attack>();
        }
    }
}

fn animate(mut query: Query<(&Player, &mut SpriteAnimator, &Direction ,&Transform, Has<Dash>, Has<Attack>)>) {
    let (_player, mut animator, direction,_transformm, _dash, attack) = query.single_mut();
    if attack {
        animator.set_anim_index(3);
    } else if direction.0.length() > 0. {
        animator.set_anim_index(2);
    } else {
        animator.set_anim_index(1);
    }
}
