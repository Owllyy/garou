

#[derive(Default, Component, Reflect)]
struct Dash {
    speed: Vec2,
    duration: f32,
}

#[derive(Default, Component)]
struct Direction(Vec2);

#[derive(Default, Bundle)]
struct WallBundle {
    rigid_body: RigidBody,
    collider: Collider,
    material: MaterialMesh2dBundle<ColorMaterial>,
}

impl WallBundle {
    fn new(
        rectangle: Rectangle,
        position: Vec3,
        color: Handle<ColorMaterial>,
        meshes: &mut Assets<Mesh>,
    ) -> WallBundle {
        WallBundle {
            rigid_body: RigidBody::Static,
            collider: Collider::rectangle(1., 1.),
            material: MaterialMesh2dBundle {
                mesh: meshes.add(Rectangle::default()).into(),
                transform: Transform::from_translation(position)
                    .with_scale(rectangle.size().extend(1.)),
                material: color,
                ..default()
            },
        }
    }
}

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

fn keyboard_input_system(
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

fn wall_e(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((WallBundle::new(
        Rectangle::new(20., 850.),
        Vec3 {
            x: -600.,
            y: 0.,
            z: 0.0,
        },
        materials.add(Color::PURPLE),
        &mut meshes,
    ),));
    commands.spawn((WallBundle::new(
        Rectangle::new(20., 850.),
        Vec3 {
            x: 600.,
            y: 0.,
            z: 0.0,
        },
        materials.add(Color::PURPLE),
        &mut meshes,
    ),));
    commands.spawn((WallBundle::new(
        Rectangle::new(1200., 20.),
        Vec3 {
            x: 0.,
            y: -350.,
            z: 0.0,
        },
        materials.add(Color::PURPLE),
        &mut meshes,
    ),));
}
