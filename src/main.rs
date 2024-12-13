use bevy::prelude::*;
use rand::{distributions::Uniform, Rng};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_vel)
        .run();
}

#[derive(Component)]
#[require(Velocity)]
struct Boid;

#[derive(Component, Default)]
#[require(Transform)]
struct Velocity(pub Vec3);

const RADIUS: f32 = 20.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Query<&Window>,
) {
    commands.spawn(Camera2d);
    let circle = meshes.add(Circle::new(RADIUS));
    let color = materials.add(ColorMaterial::from_color(Color::srgba(1.0, 0.0, 1.0, 1.0)));

    let window = window.single();
    let xrange = Uniform::new(-window.width()/2.0, window.width()/2.0);
    let yrange = Uniform::new(-window.height()/2.0, window.height()/2.0);
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
    mut circles: Query<(&mut Velocity, &mut Transform)>,
    window: Query<&Window>,
) {
    let window = window.single();
    for (mut velocity, mut transform) in &mut circles {
        transform.translation += velocity.0 * time.delta_secs();

        if transform.translation.y + RADIUS/2.0 > (window.height() / 2.0)
            || transform.translation.y - RADIUS/2.0 < (-window.height() / 2.0)
        {
            velocity.0.y *= -1.0;
        }
        if transform.translation.x + RADIUS/2.0 > (window.width() / 2.0)
            || transform.translation.x - RADIUS/2.0 < (-window.width() / 2.0)
        {
            velocity.0.x *= -1.0;
        }
    }
}
