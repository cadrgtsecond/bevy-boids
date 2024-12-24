use bevy::{
    asset::RenderAssetUsages, math::bounding::Aabb2d, prelude::*, render::{
        mesh::{Indices, PrimitiveTopology},
        render_resource::PolygonMode,
    }
};
use rand::{distributions::Uniform, Rng};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_vel)
        .run();
}

const WIDTH: f32 = 10.0;
const HEIGHT: f32 = 20.0;

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
    for _ in 0..50 {
        commands.spawn((
            Boid,
            Mesh2d(circle.clone()),
            MeshMaterial2d(color.clone()),
            Transform::from_xyz(rng.sample(xrange), rng.sample(yrange), 0.0),
            Velocity(Vec3::new(50.0, 50.0, 0.0)),
        ));
    }
    info!("Starting!");
}

fn update_vel(
    time: Res<Time>,
    mut objects: Query<(&mut Velocity, &mut Transform)>,
    window: Query<&Window>,
) {
    let window = window.single();
    for (mut velocity, mut transform) in &mut objects {
        transform.translation += velocity.0 * time.delta_secs();
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, velocity.0.normalize());

        let Vec3 { x, y, .. } = transform.translation;

        if x - WIDTH / 2.0 < -window.width() / 2.0 {
            velocity.0.x = velocity.0.x.abs()
        }
        if x + WIDTH / 2.0 > window.width() / 2.0 {
            velocity.0.x = -(velocity.0.x.abs());
        }
        if y - HEIGHT / 2.0 < -window.height() / 2.0 {
            velocity.0.y = velocity.0.y.abs()
        }
        if y + HEIGHT / 2.0 > window.height() / 2.0 {
            velocity.0.y = -(velocity.0.y.abs());
        }
    }
}
