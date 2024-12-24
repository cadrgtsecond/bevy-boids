use std::{any::Any, borrow::BorrowMut};

use bevy::{
    asset::RenderAssetUsages,
    math::bounding::Aabb2d,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_resource::PolygonMode,
    },
};
use bevy_egui::{EguiContext, EguiContexts, EguiPlugin};
use egui::Slider;
use rand::{distributions::Uniform, Rng};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .insert_resource(BoidArgs {
            cohesion: 1.0,
            alignment: 1.0,
            seperation: 1.0,
            avoid: 10.0,
        })
        .add_systems(Update, draw_ui)
        .add_systems(Startup, setup)
        .add_systems(Update, update_vel)
        .add_systems(Update, boid_rules)
        .run();
}

const WIDTH: f32 = 5.0;
const HEIGHT: f32 = 10.0;

#[derive(Component)]
#[require(Velocity)]
struct Boid;

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

#[derive(Component, Default, Debug)]
#[require(Transform)]
struct Velocity(pub Vec3);

#[derive(Debug, Resource)]
struct BoidArgs {
    cohesion: f32,
    alignment: f32,
    seperation: f32,
    avoid: f32,
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
            Velocity(Vec3::new(100.0, 100.0, 0.0)),
        ));
    }

    info!("Starting!");
}

fn update_vel(time: Res<Time>, mut objects: Query<(&Velocity, &mut Transform)>) {
    for (velocity, mut transform) in &mut objects {
        transform.translation += velocity.0 * time.delta_secs();
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, velocity.0.normalize());
    }
}

const BORDER: f32 = 40.0;
fn boid_rules(
    time: Res<Time>,
    boidargs: Res<BoidArgs>,
    window: Query<&Window>,
    mut birds: Query<(&mut Velocity, &Transform)>,
    others: Query<&Transform>,
) {
    let BoidArgs {
        cohesion,
        alignment,
        seperation,
        avoid,
    } = *boidargs;
    let window = window.single();

    // TODO: Use spacial queries to avoid nested loops
    for (mut velocity, my_pos) in &mut birds {
        let others: Vec<_> = others
            .iter()
            .filter(|pos| pos.translation.distance(my_pos.translation) < 300.0)
            .collect();
        let len = others.len();
        let my_dir = velocity.0.normalize();
        let my_rot = my_dir.angle_between(Vec3::X);

        // Fly towards center
        let sum: Vec3 = others.iter().map(|t| t.translation).sum();
        let target = sum / len as f32;
        let cohesion_angle =
            my_dir.angle_between((target - my_pos.translation).normalize_or(my_dir));

        // Align with others
        let sum: f32 = others.iter().map(|r| r.rotation.to_axis_angle().1).sum();
        let alignment_angle = (sum / len as f32) - my_rot;

        // Avoid others
        let sum: Vec3 = others.iter().map(|t| -(t.translation - my_pos.translation)).sum();
        let target = sum / len as f32;
        let seperation_angle =
            my_dir.angle_between(target.normalize_or(my_dir));

        // Avoid edges by rotating toward center
        let Vec3 { x, y, .. } = my_pos.translation;
        let distance_to_edge =
            (window.width() / 2.0 - x.abs()).min(window.height() / 2.0 - y.abs());
        let avoid_angle = if distance_to_edge < BORDER {
            my_dir.angle_between((Vec3::ZERO - my_pos.translation).normalize_or(my_dir))
        } else {
            0.0
        };

        let rot = (cohesion_angle * cohesion + alignment_angle * alignment + seperation * seperation_angle) / 2.0;
        // The closer we are to the edge, the more we should avoid
        let rot = f32::lerp(rot, avoid_angle * avoid, (BORDER - distance_to_edge).max(0.0) / BORDER);
        velocity.0 = Quat::from_rotation_z(rot * time.delta_secs()) * velocity.0;
    }
}

fn draw_ui(mut boidargs: ResMut<BoidArgs>, mut contexts: EguiContexts) {
    egui::Window::new("Boids").show(contexts.ctx_mut(), |ui| {
        ui.add(Slider::new(&mut boidargs.cohesion, 0.0..=10.0).text("Cohesion"));
        ui.add(Slider::new(&mut boidargs.alignment, 0.0..=10.0).text("Alignment"));
        ui.add(Slider::new(&mut boidargs.seperation, 0.0..=10.0).text("Separation"));
        ui.add(Slider::new(&mut boidargs.avoid, 0.0..=10.0).text("Avoidance"));
    });
}
