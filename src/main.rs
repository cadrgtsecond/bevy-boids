use std::{
    ops::{Add, Div},
    time::Duration,
};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_spatial::{kdtree::KDTree3, AutomaticUpdate, SpatialAccess};
use egui::Slider;
use rand::{distributions::Uniform, Rng};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(AutomaticUpdate::<Boid>::new().with_frequency(Duration::from_millis(300)))
        .insert_resource(BoidArgs {
            cohesion: 1.0,
            alignment: 1.0,
            seperation: 1.0,
            range: 100.0,
        })
        .add_systems(Update, draw_ui)
        .add_systems(Startup, setup)
        .add_systems(Update, update_pos)
        .add_event::<UpdateVelocity>()
        .add_systems(Update, update_velocity)
        .add_systems(Update, boid_rules)
        .add_systems(Update, avoid_edges)
        .run();
}

const WIDTH: f32 = 5.0;
const HEIGHT: f32 = 10.0;

#[derive(Component)]
#[require(Velocity)]
struct Boid;

type SpatialTree = KDTree3<Boid>;

struct BoidMeshBuilder;
impl MeshBuilder for BoidMeshBuilder {
    fn build(&self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [0.0, HEIGHT / 2.0, 0.0],
                [0.0, 0.0, 0.0],
                [WIDTH / 2.0, -HEIGHT / 2.0, 0.0],
                [-WIDTH / 2.0, -HEIGHT / 2.0, 0.0],
            ],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![[0.0, 1.0], [0.5, 0.0], [1.0, 0.0], [0.5, 1.0]],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
        )
        .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 1, 3]))
    }
}

#[derive(Component, Clone, Default, Debug)]
#[require(Transform)]
struct Velocity(pub Vec3);

#[derive(Debug, Resource)]
struct BoidArgs {
    cohesion: f32,
    alignment: f32,
    seperation: f32,
    range: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Query<&Window>,
) {
    commands.spawn(Camera2d);
    let circle = meshes.add(BoidMeshBuilder);
    let color = materials.add(ColorMaterial::from_color(Color::srgba(1.0, 0.0, 1.0, 1.0)));

    let window = window.single();
    let xrange = Uniform::new(-window.width() / 2.0, window.width() / 2.0);
    let yrange = Uniform::new(-window.height() / 2.0, window.height() / 2.0);
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        commands.spawn((
            Boid,
            Mesh2d(circle.clone()),
            MeshMaterial2d(color.clone()),
            Transform::from_xyz(rng.sample(xrange), rng.sample(yrange), 0.0),
            Velocity(Vec3::new(10.0, 10.0, 0.0)),
        ));
    }

    info!("Starting!");
}

fn update_pos(time: Res<Time>, mut objects: Query<(&Velocity, &mut Transform)>) {
    for (velocity, mut transform) in &mut objects {
        transform.translation += velocity.0 * 0.5 * time.delta_secs();
    }
}
fn update_velocity(
    time: Res<Time>,
    mut ev: EventReader<UpdateVelocity>,
    mut birds: Query<(&mut Velocity, &mut Transform)>,
) {
    for UpdateVelocity(entity, vel) in ev.read() {
        let Ok(mut bird) = birds.get_mut(*entity) else {
            return;
        };
        bird.0 .0 += vel;

        // Add some friction
        let friction = bird.0.0 * 0.1;
        bird.0.0 -= friction * time.delta_secs();

        const MAX_VELOCITY: f32 = 500.0;
        const MIN_VELOCITY: f32 = 50.0;
        if bird.0 .0.length() > MAX_VELOCITY {
            bird.0 .0 = bird.0 .0.normalize() * MAX_VELOCITY;
        }
        if bird.0 .0.length() < MIN_VELOCITY {
            bird.0 .0 = bird.0 .0.normalize() * MIN_VELOCITY;
        }

        if let Some(norm) = bird.0 .0.try_normalize() {
            bird.1.rotation = Quat::from_rotation_arc(Vec3::Y, norm)
        }
    }
}

/// Updates velocity by some delta
#[derive(Event)]
struct UpdateVelocity(pub Entity, pub Vec3);

/// Calculates the average of an iterator of vectors or anything divisible by f32
fn average<T>(first: T, it: impl Iterator<Item = T>) -> T
where
    T: Add<T, Output = T>,
    T: Div<f32, Output = T>,
{
    let (sum, len) = it.fold((first, 0), |(a, count), e| (a + e, count + 1));
    if len == 0 {
        sum
    } else {
        sum / len as f32
    }
}

const BORDER: f32 = 10.0;
fn avoid_edges(
    time: Res<Time>,
    window: Query<&Window>,
    birds: Query<(&Transform, Entity)>,
    mut update_vel: EventWriter<UpdateVelocity>,
) {
    let window = window.single();

    for (transform, entity) in &birds {
        // Avoid edges by rotating toward center
        let Vec3 { x, y, .. } = transform.translation;
        let distance_to_edge =
            (window.width() / 2.0 - x.abs()).min(window.height() / 2.0 - y.abs());

        let avoid_delta = if distance_to_edge < BORDER {
            (Vec3::ZERO - transform.translation) / (distance_to_edge.max(0.01) / 40.0)
        } else {
            Vec3::ZERO
        };
        update_vel.send(UpdateVelocity(entity, avoid_delta * time.delta_secs()));
    }
}

fn boid_rules(
    time: Res<Time>,
    boidargs: Res<BoidArgs>,
    birds: Query<(&Velocity, &Transform, Entity)>,
    tree: Res<SpatialTree>,
    mut update_vel: EventWriter<UpdateVelocity>,
) {
    let BoidArgs {
        cohesion,
        alignment,
        seperation,
        range,
    } = *boidargs;

    for (velocity, my_transform, my_entity) in &birds {
        let Some(my_dir) = velocity.0.try_normalize() else {
            continue;
        };
        let my_pos = my_transform.translation;
        const VIEW_ANGLE: f32 = std::f32::consts::PI / 3.0;

        // Fly towards center
        let target = average(
            Vec3::ZERO,
            tree.within_distance(my_pos, range)
                .iter()
                .map(|(p, _)| *p)
                .filter(|p| (p - my_pos).angle_between(my_dir) < VIEW_ANGLE),
        );
        let cohesion_delta = target - my_pos;

        // Align with others
        let align_delta = average(
            Vec3::ZERO,
            tree.within_distance(my_pos, range)
                .iter()
                .filter_map(|(p, e)| {
                    (((p - my_pos).angle_between(my_dir) < VIEW_ANGLE) && *e != Some(my_entity))
                        .then(|| Some(birds.get((*e)?).ok()?.0))
                        .flatten()
                        .map(|v| v.0)
                }),
        );

        // Avoid others
        let seperation_delta = average(
            Vec3::ZERO,
            tree.within_distance(my_pos, range)
                .iter()
                .map(|(p, _)| my_pos - p)
                .filter(|p| (p - my_pos).angle_between(my_dir) < VIEW_ANGLE)
                .map(|v| v / (v.length().max(0.001) / range))
        );
        let del =
            cohesion * cohesion_delta + alignment * align_delta + seperation * seperation_delta;

        update_vel.send(UpdateVelocity(my_entity, del * time.delta_secs()));
    }
}

fn draw_ui(mut boidargs: ResMut<BoidArgs>, mut contexts: EguiContexts) {
    egui::Window::new("Boids").show(contexts.ctx_mut(), |ui| {
        ui.add(Slider::new(&mut boidargs.cohesion, 0.0..=2.0).text("Cohesion"));
        ui.add(Slider::new(&mut boidargs.alignment, 0.0..=2.0).text("Alignment"));
        ui.add(Slider::new(&mut boidargs.seperation, 0.0..=2.0).text("Separation"));
        ui.add(Slider::new(&mut boidargs.range, 0.0..=400.0).text("View range"));
    });
}
