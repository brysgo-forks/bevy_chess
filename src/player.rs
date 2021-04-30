use bevy::{input::system::exit_on_esc_system, prelude::*};
use bevy_prototype_character_controller::{
    controller::{BodyTag, CameraTag, CharacterController, HeadTag, YawTag},
    look::{LookDirection, LookEntity},
    rapier::*,
};
use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierConfiguration, RapierPhysicsPlugin},
    rapier::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder},
};
use rand::Rng;

pub struct CharacterSettings {
    pub scale: Vec3,
    pub head_scale: f32,
    pub head_yaw: f32,
    pub follow_offset: Vec3,
    pub focal_point: Vec3,
}
impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            scale: Vec3::new(0.5, 1.9, 0.3),
            head_scale: 0.3,
            head_yaw: 0.0,
            follow_offset: Vec3::new(0.0, 4.0, 8.0), // Relative to head
            focal_point: Vec3::ZERO,                 // Relative to head
        }
    }
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Generic
        app.insert_resource(ClearColor(Color::hex("101010").unwrap()))
            .add_system(exit_on_esc_system.system())
            // Rapier
            .add_plugin(RapierPhysicsPlugin)
            .insert_resource(RapierConfiguration {
                time_dependent_number_of_timesteps: true,
                ..Default::default()
            });

        // IMPORTANT: The impulse/force systems MUST run before the physics simulation step, so they
        // either need to be added to the end of PRE_UPDATE or the beginning of UPDATE
        app.add_plugin(RapierDynamicImpulseCharacterControllerPlugin);

        // Specific to this demo
        app.init_resource::<CharacterSettings>()
            .add_startup_system(spawn_world.system())
            .add_startup_system(spawn_character.system())
            .run();
    }
}

pub fn spawn_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let cube = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    let box_y = 0.0;

    // Cubes for some kind of reference in the scene to make it easy to see
    // what is happening
    let teal = materials.add(Color::hex("008080").unwrap().into());
    let cube_scale = 1.0;
    let mut rng = rand::thread_rng();
    for _ in 0..20 {
        let x = rng.gen_range(-10.0..10.0);
        let z = rng.gen_range(-10.0..10.0);
        let translation = Vec3::new(x, 0.5 * (cube_scale - box_y), z);
        commands
            .spawn_bundle(PbrBundle {
                material: teal.clone(),
                mesh: cube.clone(),
                transform: Transform::from_matrix(Mat4::from_scale_rotation_translation(
                    Vec3::splat(cube_scale),
                    Quat::IDENTITY,
                    translation,
                )),
                ..Default::default()
            })
            .insert_bundle((
                RigidBodyBuilder::new_dynamic().translation(x, 0.5 * (cube_scale - box_y), z),
                ColliderBuilder::cuboid(0.5 * cube_scale, 0.5 * cube_scale, 0.5 * cube_scale),
                PhysicsInterpolationComponent::new(translation, Quat::IDENTITY),
            ));
    }
}

pub fn spawn_character(
    mut commands: Commands,
    character_settings: Res<CharacterSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let box_y = 1.0;
    let cube = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let red = materials.add(Color::hex("800000").unwrap().into());
    let body = commands
        .spawn_bundle((
            GlobalTransform::identity(),
            Transform::identity(),
            CharacterController::default(),
            RigidBodyBuilder::new_dynamic()
                .translation(0.0, 0.5 * (box_y + character_settings.scale.y), 0.0)
                .lock_rotations(),
            ColliderBuilder::capsule_y(
                0.5 * character_settings.scale.y,
                0.5 * character_settings.scale.x.max(character_settings.scale.z),
            )
            .density(200.0),
            PhysicsInterpolationComponent::new(
                0.5 * (box_y + character_settings.scale.y) * Vec3::Y,
                Quat::IDENTITY,
            ),
            BodyTag,
        ))
        .id();
    let yaw = commands
        .spawn_bundle((GlobalTransform::identity(), Transform::identity(), YawTag))
        .id();
    let body_model = commands
        .spawn_bundle(PbrBundle {
            material: red.clone(),
            mesh: cube.clone(),
            transform: Transform::from_matrix(Mat4::from_scale_rotation_translation(
                character_settings.scale - character_settings.head_scale * Vec3::Y,
                Quat::IDENTITY,
                Vec3::new(
                    0.0,
                    0.5 * (box_y + character_settings.scale.y - character_settings.head_scale)
                        - 1.695,
                    0.0,
                ),
            )),
            ..Default::default()
        })
        .id();
    let head = commands
        .spawn_bundle((
            GlobalTransform::identity(),
            Transform::from_matrix(Mat4::from_scale_rotation_translation(
                Vec3::ONE,
                Quat::from_rotation_y(character_settings.head_yaw),
                Vec3::new(
                    0.0,
                    0.5 * (box_y - character_settings.head_scale) + character_settings.scale.y
                        - 1.695,
                    0.0,
                ),
            )),
            HeadTag,
        ))
        .id();
    let head_model = commands
        .spawn_bundle(PbrBundle {
            material: red,
            mesh: cube,
            transform: Transform::from_scale(Vec3::splat(character_settings.head_scale)),
            ..Default::default()
        })
        .id();
    let camera = commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                character_settings.follow_offset,
                character_settings.focal_point,
                Vec3::Y,
            )),
            ..Default::default()
        })
        .insert_bundle((LookDirection::default(), CameraTag))
        .id();
    commands
        .entity(body)
        .insert(LookEntity(camera))
        .push_children(&[yaw]);
    commands.entity(yaw).push_children(&[body_model, head]);
    commands.entity(head).push_children(&[head_model, camera]);
}
