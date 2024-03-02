// This is the most basic use example from the readme.md

use bevy::{math::vec3, prelude::*, sprite::Anchor};
use bevy_asepritesheet::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_inspector_egui::{
    prelude::*,
    quick::{ResourceInspectorPlugin, WorldInspectorPlugin},
};
use bevy_xpbd_2d::{
    components::{Collider, LinearVelocity, LockedAxes, RigidBody, Sensor},
    plugins::{collision::contact_reporting::CollisionStarted, PhysicsDebugPlugin, PhysicsPlugins},
    resources::Gravity,
    PhysicsSchedule, PhysicsStepSet,
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
    pawn_speed: f32,
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
            pawn_speed: 300.0,
        }
    }
}

#[derive(Component, Reflect)]
struct Attack;

#[derive(Component, Reflect)]
struct Duration(f32);

#[derive(Bundle)]
struct AttackBundle {
    attack: Attack,
    transform: Transform,
    collider: Collider,
    sensor: Sensor,
    duration: Duration,
}

impl Default for AttackBundle {
    fn default() -> Self {
        Self {
            attack: Attack,
            transform: Transform::default(),
            collider: Collider::cuboid(70.0, 70.0),
            sensor: Sensor,
            duration: Duration(0.45),
        }
    }
}

impl AttackBundle {
    fn new(position: Vec3) -> Self {
        Self {
            attack: Attack,
            transform: Transform::from_translation(position),
            ..Default::default()
        }
    }
}

#[derive(Default, Component, Reflect)]
struct Life(u64);

#[derive(Default, Component, Reflect)]
struct Player {
    speed: Vec2,
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    life: Life,
    locked_axes: LockedAxes,
    direction: Direction,
    rigid_body: RigidBody,
    collider: Collider,
    animated_sprite_bundle: AnimatedSpriteBundle,
}

impl PlayerBundle {
    fn new(spritesheet: Handle<Spritesheet>) -> Self {
        Self {
            player: Player { speed: Vec2::ZERO },
            life: Life(5),
            direction: Direction(Vec2::ZERO),
            rigid_body: RigidBody::Dynamic,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collider: Collider::capsule(15.0, 20.0),
            animated_sprite_bundle: AnimatedSpriteBundle {
                animator: SpriteAnimator::from_anim(AnimHandle::from_index(1)),
                sprite_bundle: SpriteSheetBundle {
                    transform: Transform::from_translation(vec3(700., 600., 20.)),
                    ..Default::default()
                },
                spritesheet,
                ..Default::default()
            },
        }
    }
}

#[derive(Default, Component, Reflect)]
struct Dead;

#[derive(Bundle)]
struct DeadBundle {
    dead: Dead,
    duration: Duration,
    animated_sprite_bundle: AnimatedSpriteBundle,
}

impl DeadBundle {
    fn new(spritesheet_handle: Handle<Spritesheet>, position: Vec3) -> Self {
        Self {
            dead: Dead,
            duration: Duration(1.6),
            animated_sprite_bundle: AnimatedSpriteBundle {
                animator: SpriteAnimator::from_anim(AnimHandle::from_index(0)),
                spritesheet: spritesheet_handle,
                sprite_bundle: SpriteSheetBundle {
                    transform: Transform::from_translation(position),
                    ..Default::default()
                },
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
    locked_axes: LockedAxes,
    collider: Collider,
    animated_sprite_bundle: AnimatedSpriteBundle,
}

impl EnemyBundle {
    fn new(spritesheet_handle: Handle<Spritesheet>, position: Vec3) -> Self {
        Self {
            enemy: Enemy,
            rigid_body: RigidBody::Dynamic,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collider: Collider::capsule(15.0, 20.0),
            animated_sprite_bundle: AnimatedSpriteBundle {
                animator: SpriteAnimator::from_anim(AnimHandle::from_index(5)),
                spritesheet: spritesheet_handle,
                sprite_bundle: SpriteSheetBundle {
                    transform: Transform::from_translation(position),
                    ..Default::default()
                },
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
        // Inspect World
        .add_plugins(WorldInspectorPlugin::default())
        // Inspect Configuration
        .register_type::<Configuration>()
        .init_resource::<Configuration>()
        .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
        // Setup Physics
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity::ZERO)
        // Debug Physics
        .add_plugins(PhysicsDebugPlugin::default())
        // Spawn Enemies
        .register_type::<EnemySpawnTimer>()
        .insert_resource(EnemySpawnTimer(Timer::from_seconds(
            3.0,
            TimerMode::Repeating,
        )))

        // Load map
        .add_plugins(LdtkPlugin)
        .insert_resource(LevelSelection::index(0))
        // Setup game
        .add_systems(Startup, setup)
        // Game logic systems
        .add_systems(
            Update,
            (
                spawn_enemies,
                animate,
                damage_enemies,
                death,
            )
                .chain(),
        )
        // Update player position
        .add_systems(
            PhysicsSchedule,
            (keyboard_input, dash, apply_force, attack)
                .chain()
                .before(PhysicsStepSet::BroadPhase),
        )
        // Follow player position
        .add_systems(
            PhysicsSchedule,
            (
                enemies_follow_player,
                following_cam,
                camera_fit_inside_current_level,
            )
            .chain()
            .in_set(PhysicsStepSet::SpatialQuery),
        )
        .run();
}

fn enemies_follow_player(
    configuration: Res<Configuration>,
    player: Query<&mut Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&mut LinearVelocity, &Transform), With<Enemy>>,
) {
    let player = player.single();
    for (mut velocity, transform) in &mut enemies {
        let gap = (player.translation - transform.translation).xy();
        if gap.length() > 10.0 {
            velocity.0 = gap.normalize() * configuration.pawn_speed;
        } else {
            velocity.0 = Vec2::ZERO;
        }
    }
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
        commands.spawn(EnemyBundle::new(enemy_spritesheet, vec3(350., 970., 10.)));
    }
}

fn death(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Duration), With<Dead>>,
) {
    for (entity, mut dead) in query.iter_mut() {
        if dead.0 > 0. {
            dead.0 -= time.delta_seconds();
        } else {
            commands.entity(entity).despawn();
        }
    }
}

fn damage_enemies(
    mut commands: Commands,
    enemy: Query<(Entity, &Transform), With<Enemy>>,
    attack: Query<Entity, With<Attack>>,
    asset: Res<AssetServer>,
    mut collision_event_reader: EventReader<CollisionStarted>,
) {
    for &CollisionStarted(e1, e2) in collision_event_reader.read() {
        if enemy.get(e1).is_ok() && attack.get(e2).is_ok() {
            let dead_spritesheet = load_spritesheet(
                &mut commands,
                &asset,
                "Tiny Swords/Factions/Knights/Troops/Dead/Dead.json",
                Anchor::Center,
            );
            let (_entity, position) = enemy.get(e1).unwrap();
            commands.spawn(DeadBundle::new(dead_spritesheet, position.translation));
            commands.entity(e1).despawn();
        } else if enemy.get(e2).is_ok() && attack.get(e1).is_ok() {
            let dead_spritesheet = load_spritesheet(
                &mut commands,
                &asset,
                "Tiny Swords/Factions/Knights/Troops/Dead/Dead.json",
                Anchor::Center,
            );
            let (_entity, position) = enemy.get(e2).unwrap();
            commands.spawn(DeadBundle::new(dead_spritesheet, position.translation));
            commands.entity(e2).despawn();
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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
        transform: Transform::from_translation(vec3(0., 0., -1.)),
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
                if dash.speed.length() > 0. {
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
    mut cam: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    cam.single_mut().translation = player.single().translation;
}
 #[allow(clippy::type_complexity)]
fn camera_fit_inside_current_level(
    mut camera_query: Query<
        (
            &mut bevy::render::camera::OrthographicProjection,
            &mut Transform,
        ),
        Without<Player>,
    >,
    window_query: Query<&Window>,
    player_query: Query<&Transform, With<Player>>,
    level_query: Query<(&Transform, &LevelIid), (Without<OrthographicProjection>, Without<Player>)>,
    ldtk_projects: Query<&Handle<LdtkProject>>,
    level_selection: Res<LevelSelection>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
) {
    let Ok(Transform {
        translation: player_translation,
        ..
    }) = player_query.get_single() else {
        return;
    };

    let window = window_query.single();
    let aspect_ratio = window.resolution.width() / window.resolution.height();

    let player_translation = *player_translation;

    let (mut orthographic_projection, mut camera_transform) = camera_query.single_mut();

    for (level_transform, level_iid) in &level_query {
        let ldtk_project = ldtk_project_assets
            .get(ldtk_projects.single())
            .expect("Project should be loaded if level has spawned");

        let level = ldtk_project
            .get_raw_level_by_iid(&level_iid.to_string())
            .expect("Spawned level should exist in LDtk project");

        if level_selection.is_match(&LevelIndices::default(), level) {
            let level_ratio = level.px_wid as f32 / level.px_hei as f32;
            orthographic_projection.viewport_origin = Vec2::ZERO;
            if level_ratio > aspect_ratio {
                // level is wider than the screen
                let height = (level.px_hei as f32 / 9.).round() * 9.;
                let width = height * aspect_ratio;
                orthographic_projection.scaling_mode =
                    bevy::render::camera::ScalingMode::Fixed { width, height };
                camera_transform.translation.x =
                    (player_translation.x - level_transform.translation.x - width / 2.)
                        .clamp(0., level.px_wid as f32 - width);
                camera_transform.translation.y = 0.;
            } else {
                // level is taller than the screen
                let width = (level.px_wid as f32 / 16.).round() * 16.;
                let height = width / aspect_ratio;
                orthographic_projection.scaling_mode =
                    bevy::render::camera::ScalingMode::Fixed { width, height };
                camera_transform.translation.y =
                    (player_translation.y - level_transform.translation.y - height / 2.)
                        .clamp(0., level.px_hei as f32 - height);
                camera_transform.translation.x = 0.;
            }

            camera_transform.translation.x += level_transform.translation.x;
            camera_transform.translation.y += level_transform.translation.y;
        }
    }
}

fn keyboard_input(
    time: Res<Time>,
    configurition: Res<Configuration>,
    mut command: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(Entity, &mut Player, &mut Direction)>,
    existing_attack: Query<&Attack>,
) {
    for (entity, mut player, mut direction) in &mut query {
        direction.0 = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::Left) {
            direction.0.x += -1.0;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            direction.0.x += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            direction.0.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            direction.0.y += -1.0;
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

        if keyboard_input.pressed(KeyCode::K) && existing_attack.get_single().is_err() {
            let new_attack = command
                .spawn(AttackBundle::new(direction.0.extend(0.) * 50.))
                .id();
            command.entity(entity).add_child(new_attack);
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

fn attack(
    mut query: Query<(Entity, Option<&mut LinearVelocity>, &Children), With<Player>>,
    mut attack: Query<&mut Duration, (With<Attack>, Without<Player>)>,
    mut command: Commands,
    time: Res<Time>,
) {
    let Ok((entity, velocity, children)) = query.get_single_mut() else {
        return;
    };
    if let Some(&child) = children.get(0) {
        let mut attack_child = attack.get_mut(child).unwrap();
        if attack_child.0 > 0. {
            if let Some(mut velocity) = velocity {
                velocity.0 /= 2.;
            }
            attack_child.0 -= time.delta_seconds();
        } else {
            command.entity(entity).despawn_descendants();
        }
    }
}

fn animate(
    mut query: Query<(&mut SpriteAnimator, &Direction, Has<Dash>), With<Player>>,
    attack: Query<(), With<Attack>>,
) {
    let (mut animator, direction, _dash) = query.single_mut();
    if !attack.is_empty() {
        if direction.0.y > 0. {
            animator.set_anim_index(7);
        } else if direction.0.y < 0. {
            animator.set_anim_index(6);
        } else if direction.0.x > 0.0 {
            animator.set_anim_index(5);
        } else {
            animator.set_anim_index(8);
        }
    } else if direction.0.length() > 0. {
        if direction.0.x > 0.0 {
            animator.set_anim_index(4);
        } else {
            animator.set_anim_index(2);
        }
    } else {
        if direction.0.x > 0.0 {
            animator.set_anim_index(3);
        } else {
            animator.set_anim_index(1);
        }
    }
}
