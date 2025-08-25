use bevy_gui::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy_gui::prelude::*;
use factor_mesh::builder::{Hexahedron, MeshBuilder, Octahedron};
use factor_mesh::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            WireframePlugin::default(),
            bevy_flycam::PlayerPlugin,
        ))
        .add_systems(Startup, startup)
        .add_systems(Update, (sync_meshes, handle_keypresses))
        .add_observer(recompute_normals)
        .run();
}

fn startup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut mesh = DefaultPackedMesh::<u8>::new();
    Hexahedron::CENTERED_CUBE
        .translate(Vec3::X * 2.0)
        .append_to(&mut mesh);
    Octahedron::CENTERED
        .translate(Vec3::NEG_X * 2.0)
        .append_to(&mut mesh);
    println!("{mesh:#?}");
    commands.spawn(PointLight {
        color: Color::WHITE,
        intensity: 100.0,
        ..default()
    });
    commands.spawn((
        DynMesh::from(mesh),
        SurfaceSync::default(),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::BLACK,
            cull_mode: None,
            ..default()
        })),
    ));
}

fn recompute_normals(
    trig: Trigger<factor_mesh::ecs::UpdatedMesh>,
    query: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if let Ok(handle) = query.get(trig.target())
        && let Some(mesh) = meshes.get_mut(handle)
    {
        mesh.compute_normals();
    }
}

fn handle_keypresses(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut wireframe: ResMut<WireframeConfig>,
    mut query: Query<&mut SurfaceSync>,
) {
    if keyboard.just_pressed(KeyCode::KeyF) {
        wireframe.global = !wireframe.global;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        for mut s in query.iter_mut() {
            s.internal = !s.internal;
        }
    }
}
