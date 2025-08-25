#[cfg(not(target_family = "wasm"))]
use bevy_gui::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy_gui::prelude::*;
use factor_mesh::builder::{Hexahedron, MeshBuilder, Octahedron};
use factor_mesh::prelude::*;

#[derive(Resource)]
struct ShowingInternals(bool);
#[derive(Resource)]
struct MaterialState {
    transparent: bool,
    culling: bool,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            #[cfg(not(target_family = "wasm"))]
            WireframePlugin::default(),
            bevy_flycam::PlayerPlugin,
        ))
        .insert_resource(AmbientLight {
            brightness: 250.0,
            ..default()
        })
        .insert_resource(ShowingInternals(false))
        .insert_resource(MaterialState {
            transparent: false,
            culling: true,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_keypresses,
                update_internals,
                update_materials,
                update_text,
                sync_meshes,
            ),
        )
        .add_observer(recompute_normals)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut mesh = DefaultPackedMesh::<u8>::new();
    Hexahedron::CENTERED_CUBE
        .translate(Vec3::new(2.0, 1.0, 0.0))
        .append_to(&mut mesh);
    Octahedron::CENTERED
        .translate(Vec3::new(-2.0, 1.0, 0.0))
        .append_to(&mut mesh);
    commands.spawn((
        PointLight {
            color: Color::WHITE,
            intensity: 10000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));
    commands.spawn((
        DynMesh::from(mesh),
        SurfaceSync::default(),
        MeshMaterial3d(materials.add(Color::linear_rgb(1.0, 0.5, 0.5))),
    ));

    let black_material = materials.add(Color::BLACK);
    let white_material = materials.add(Color::WHITE);

    let plane_mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));

    for x in -3..4 {
        for z in -3..4 {
            commands.spawn((
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(if (x + z) % 2 == 0 {
                    black_material.clone()
                } else {
                    white_material.clone()
                }),
                Transform::from_xyz(x as f32 * 2.0, -1.0, z as f32 * 2.0),
            ));
        }
    }

    commands.spawn((
        Text::default(),
        TextFont::default(),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
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
    #[cfg(not(target_family = "wasm"))] mut wireframe: ResMut<WireframeConfig>,
    mut internals: ResMut<ShowingInternals>,
    mut material: ResMut<MaterialState>,
) {
    if keyboard.just_pressed(KeyCode::KeyF) {
        wireframe.global = !wireframe.global;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        internals.0 = !internals.0;
    }
    if keyboard.just_pressed(KeyCode::KeyT) {
        material.transparent = !material.transparent;
    }
    if keyboard.just_pressed(KeyCode::KeyC) {
        material.culling = !material.culling;
    }
}

fn update_internals(show: Res<ShowingInternals>, mut query: Query<&mut SurfaceSync>) {
    if show.is_changed() {
        for mut s in query.iter_mut() {
            s.internal = show.0;
        }
    }
}

fn update_materials(
    state: Res<MaterialState>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<SurfaceSync>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if state.is_changed() {
        for handle in query.iter() {
            if let Some(mat) = materials.get_mut(handle) {
                mat.cull_mode = state
                    .culling
                    .then_some(bevy_render::render_resource::Face::Back);
                if state.transparent {
                    mat.alpha_mode = AlphaMode::Blend;
                    mat.base_color = Color::linear_rgba(1.0, 0.5, 0.5, 0.5);
                } else {
                    mat.alpha_mode = AlphaMode::Opaque;
                    mat.base_color = Color::linear_rgba(1.0, 0.5, 0.5, 1.0);
                }
            }
        }
    }
}

fn update_text(
    mut text: Single<&mut Text>,
    #[cfg(not(target_family = "wasm"))] wireframe: Res<WireframeConfig>,
    internals: Res<ShowingInternals>,
    materials: Res<MaterialState>,
) {
    use std::fmt::Write;
    if wireframe.is_changed() || internals.is_changed() || materials.is_changed() {
        text.0.clear();
        let _ = write!(
            text.0,
            "Use WASD/Space/Shift to fly\nInternal faces (I): {}\nTransparency (T): {}\nBackface culling (C): {}",
            internals.0, materials.transparent, materials.culling,
        );
        #[cfg(not(target_family = "wasm"))]
        let _ = write!(text.0, "\nWireframes (F): {}", wireframe.global);
    }
}
