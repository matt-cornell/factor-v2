use bevy_egui::egui;
use bevy_gui::asset::RenderAssetUsages;
use bevy_gui::ecs as bevy_ecs;
use bevy_gui::prelude::*;
use bevy_gui::render::mesh::PrimitiveTopology;
use bevy_gui::render::render_resource::Extent3d;
use bevy_gui::utils::synccell::SyncCell;
use libnoise::Generator;
use std::f32::consts::FRAC_1_SQRT_2;
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Resource, PartialEq)]
enum SelectedTexture {
    Axes,
    Noise,
    Oceans,
}

#[derive(Component)]
struct Planet;

#[derive(Component)]
struct Axes;

#[derive(Component)]
struct Icosahedron;

#[derive(Debug, Resource)]
struct Textures {
    axes_image: Handle<Image>,
    axes_texture: Handle<StandardMaterial>,

    noise_image: Handle<Image>,
    noise_texture: Handle<StandardMaterial>,
    noise_egui: egui::TextureId,

    ocean_image: Handle<Image>,
    ocean_texture: Handle<StandardMaterial>,
    ocean_egui: egui::TextureId,
}

#[derive(Debug)]
struct Requests {
    axes: Mutex<Option<u32>>,
    noise: Mutex<Option<([u8; 32], u32)>>,
    ocean: Mutex<Option<(Vec<u8>, u32, u8)>>,
}

#[derive(Resource)]
struct RequestRes(Arc<Requests>);

#[derive(Resource)]
struct OceanLevel(u8);

struct RenderedTexture {
    selection: SelectedTexture,
    resolution: u32,
    data: Vec<u8>,
}

fn main() {
    let requests = Arc::new(Requests {
        axes: Mutex::new(Some(256)),
        noise: Mutex::new(Some(([0; 32], 256))),
        ocean: Mutex::new(None),
    });
    let (tx, rx) = mpsc::sync_channel(8);

    let req_clone = Arc::clone(&requests);
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        'main: loop {
            'render: {
                if let Some(resolution) = req_clone.axes.lock().ok().and_then(|mut g| g.take()) {
                    let res = resolution as usize;
                    buf.resize(res * res * 8, 0);
                    let scale = PI / res as f64;
                    for (y, row) in buf.chunks_mut(res * 8).enumerate() {
                        for (x, px) in row.chunks_mut(4).enumerate() {
                            let [r, g, b, a] = px else { unreachable!() };
                            let lon = x as f64 * scale;
                            let lat = (y as f64).mul_add(-scale, FRAC_PI_2);
                            let (so, co) = lon.sin_cos();
                            let (sa, ca) = lat.sin_cos();
                            let [x, y, z] = [co * ca, so * ca, sa]
                                .map(|v| v.mul_add(128.0, 127.0).clamp(0.0, 255.0) as u8);
                            *r = x;
                            *g = y;
                            *b = z;
                            *a = 255;
                        }
                        if req_clone.axes.try_lock().is_ok_and(|g| g.is_some()) {
                            break 'render;
                        }
                    }
                    let res = tx.send(RenderedTexture {
                        selection: SelectedTexture::Axes,
                        resolution,
                        data: buf.clone(),
                    });
                    if res.is_err() {
                        break 'main;
                    }
                }
            }
            'render: {
                if let Some((seed, resolution)) =
                    req_clone.noise.lock().ok().and_then(|mut g| g.take())
                {
                    let res = resolution as usize;
                    buf.resize(res * res * 8, 0);
                    let noise = factor_terrain::init::noise_source(seed);
                    let scale = PI / res as f64;
                    for (y, row) in buf.chunks_mut(res * 8).enumerate() {
                        for (x, px) in row.chunks_mut(4).enumerate() {
                            let [r, g, b, a] = px else { unreachable!() };
                            let arr = factor_terrain::init::to_coords(
                                x as f64 * scale,
                                (y as f64).mul_add(-scale, FRAC_PI_2),
                            );
                            let v = noise.sample(arr).mul_add(128.0, 127.0).clamp(0.0, 255.0) as u8;
                            *r = v;
                            *g = v;
                            *b = v;
                            *a = 255;
                        }
                        if req_clone.noise.try_lock().is_ok_and(|g| g.is_some()) {
                            break 'render;
                        }
                    }
                    let res = tx.send(RenderedTexture {
                        selection: SelectedTexture::Noise,
                        resolution,
                        data: buf.clone(),
                    });
                    if res.is_err() {
                        break 'main;
                    }
                }
            }
            'render: {
                if let Some((height, resolution, level)) =
                    req_clone.ocean.lock().ok().and_then(|mut g| g.take())
                {
                    let res = resolution as usize;
                    buf.resize(res * res * 8, 0);
                    for (srow, drow) in height.chunks(res * 8).zip(buf.chunks_mut(res * 8)) {
                        for (from, to) in srow.chunks(4).zip(drow.chunks_mut(4)) {
                            if from[0] > level {
                                to.fill(255);
                            } else {
                                to.copy_from_slice(&[0, 0, 255, 255]);
                            }
                        }
                        if req_clone.noise.try_lock().is_ok_and(|g| g.is_some()) {
                            break 'render;
                        }
                    }
                    let res = tx.send(RenderedTexture {
                        selection: SelectedTexture::Oceans,
                        resolution,
                        data: buf.clone(),
                    });
                    if res.is_err() {
                        break 'main;
                    }
                }
            }
        }
    });

    App::new()
        .add_plugins((
            DefaultPlugins,
            bevy_panorbit_camera::PanOrbitCameraPlugin,
            bevy_egui::EguiPlugin::default(),
            bevy_gui::pbr::wireframe::WireframePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, poll_textures(rx))
        .add_systems(bevy_egui::EguiPrimaryContextPass, ui_system)
        .insert_resource(AmbientLight {
            brightness: 1000.0,
            ..default()
        })
        .insert_resource(SelectedTexture::Noise)
        .insert_resource(RequestRes(requests))
        .insert_resource(OceanLevel(128))
        .run();
}

fn setup(
    mut commands: Commands,
    mut contexts: bevy_egui::EguiContexts,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let placeholder = Image::new_uninit(
        Extent3d {
            width: 512,
            height: 256,
            depth_or_array_layers: 1,
        },
        bevy_gui::render::render_resource::TextureDimension::D2,
        bevy_gui::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let axes_image = images.add(placeholder.clone());
    let axes_texture = materials.add(axes_image.clone());

    let noise_image = images.add(placeholder.clone());
    let noise_texture = materials.add(noise_image.clone());
    let noise_egui = contexts.add_image(noise_image.clone());

    let ocean_image = images.add(placeholder.clone());
    let ocean_texture = materials.add(ocean_image.clone());
    let ocean_egui = contexts.add_image(ocean_image.clone());

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(32, 18))),
        MeshMaterial3d(noise_texture.clone()),
        Transform::IDENTITY,
        Planet,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.5).mesh().ico(0).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(1.0, 1.0, 1.0, 0.2),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::IDENTITY,
        Visibility::Hidden,
        bevy_gui::pbr::wireframe::Wireframe,
        Icosahedron,
    ));

    commands.insert_resource(Textures {
        noise_image,
        noise_texture,
        noise_egui,
        axes_image,
        axes_texture,
        ocean_image,
        ocean_texture,
        ocean_egui,
    });

    commands.spawn((
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        bevy_panorbit_camera::PanOrbitCamera::default(),
    ));

    let line = Mesh3d(
        meshes.add(
            Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
                )
                .with_inserted_indices(bevy_gui::render::mesh::Indices::U16(vec![0, 1])),
        ),
    );
    commands.spawn((
        Visibility::Visible,
        Transform::IDENTITY,
        Axes,
        children![
            (
                line.clone(),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::linear_rgb(1.0, 0.0, 0.0),
                    unlit: true,
                    ..default()
                })),
                Transform::IDENTITY,
            ),
            (
                line.clone(),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::linear_rgb(0.0, 1.0, 0.0),
                    unlit: true,
                    ..default()
                })),
                Transform::from_rotation(Quat::from_xyzw(0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
            ),
            (
                line,
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::linear_rgb(0.0, 0.0, 1.0),
                    unlit: true,
                    ..default()
                })),
                Transform::from_rotation(Quat::from_xyzw(0.0, -FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2)),
            )
        ],
    ));
}

#[allow(clippy::too_many_arguments)]
fn ui_system(
    mut contexts: bevy_egui::EguiContexts,
    mut tex: ResMut<SelectedTexture>,
    textures: Res<Textures>,
    requests: Res<RequestRes>,
    images: Res<Assets<Image>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    mut ocean_level: ResMut<OceanLevel>,
    mut mat: Single<&mut MeshMaterial3d<StandardMaterial>, With<Planet>>,
    mut axes: Single<&mut Visibility, With<Axes>>,
    mut ico: Single<&mut Visibility, (With<Icosahedron>, Without<Axes>)>,
    mut resolution: Local<Option<u32>>,
    mut noise_seed: Local<factor_terrain::init::Seed>,
) -> Result {
    let mut regen_noise = false;
    let ctx = contexts.ctx_mut()?;
    let resolution = resolution.get_or_insert(256);
    egui::Window::new("Misc.").show(ctx, |ui| {
        if ui
            .add(egui::Slider::new(resolution, 1..=1024).text("Resolution"))
            .changed()
        {
            regen_noise = true;
            if let Ok(mut g) = requests.0.axes.lock() {
                *g = Some(*resolution);
            }
        }
        {
            let mut t_val = *tex == SelectedTexture::Axes;
            let old = t_val;
            ui.horizontal(|ui| {
                ui.label("Rainbow Texture: ");
                ui.add(toggle(&mut t_val, true));
            });
            if !old && t_val {
                *tex = SelectedTexture::Axes;
                mat.0 = textures.axes_texture.clone();
            }
        }
        {
            let mut v = **axes == Visibility::Visible;
            let changed = ui
                .horizontal(|ui| {
                    ui.label("Coordinate Axes: ");
                    ui.add(toggle(&mut v, false)).changed()
                })
                .inner;
            if changed {
                if v {
                    **axes = Visibility::Visible;
                } else {
                    **axes = Visibility::Hidden;
                }
            }
        }
        {
            let mut v = **ico == Visibility::Visible;
            let changed = ui
                .horizontal(|ui| {
                    ui.label("Icosahedron: ");
                    ui.add(toggle(&mut v, false)).changed()
                })
                .inner;
            if changed {
                if v {
                    **ico = Visibility::Visible;
                } else {
                    **ico = Visibility::Hidden;
                }
            }
        }
    });
    egui::Window::new("Noise").show(ctx, |ui| {
        let mut t_val = *tex == SelectedTexture::Noise;
        let old = t_val;
        ui.horizontal(|ui| {
            ui.label("Selected: ");
            ui.add(toggle(&mut t_val, true));
        });
        if !old && t_val {
            *tex = SelectedTexture::Noise;
            mat.0 = textures.noise_texture.clone();
        }
        if ui.button("Reroll").clicked() {
            *noise_seed = rand::random();
            regen_noise = true;
        }
        ui.image((textures.noise_egui, egui::vec2(256.0, 128.0)));
    });
    egui::Window::new("Oceans").show(ctx, |ui| {
        let mut t_val = *tex == SelectedTexture::Oceans;
        let old = t_val;
        ui.horizontal(|ui| {
            ui.label("Selected: ");
            ui.add(toggle(&mut t_val, true));
        });
        if !old && t_val {
            *tex = SelectedTexture::Oceans;
            mat.0 = textures.ocean_texture.clone();
        }
        if ui
            .add(egui::Slider::new(
                &mut ocean_level.bypass_change_detection().0,
                0..=255,
            ))
            .changed()
        {
            let _ = &mut *ocean_level;
            let data = images
                .get(&textures.noise_image)
                .unwrap()
                .data
                .clone()
                .unwrap();
            if let Ok(mut g) = requests.0.ocean.lock() {
                *g = Some((data, *resolution, ocean_level.0));
            }
        }
        ui.image((textures.ocean_egui, egui::vec2(256.0, 128.0)));
    });
    if regen_noise && let Ok(mut g) = requests.0.noise.lock() {
        *g = Some((*noise_seed, *resolution));
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
fn poll_textures(
    rx: mpsc::Receiver<RenderedTexture>,
) -> impl FnMut(
    ResMut<Assets<Image>>,
    ResMut<Assets<StandardMaterial>>,
    Res<Textures>,
    Res<RequestRes>,
    Res<OceanLevel>,
) + Send
+ Sync
+ 'static {
    let mut rx = SyncCell::new(rx);
    move |mut images, mut materials, textures, requests, ocean_level| {
        for tex in rx.get().try_iter() {
            let (ihandle, mhandle) = match tex.selection {
                SelectedTexture::Axes => (&textures.axes_image, &textures.axes_texture),
                SelectedTexture::Noise => (&textures.noise_image, &textures.noise_texture),
                SelectedTexture::Oceans => (&textures.ocean_image, &textures.ocean_texture),
            };
            #[allow(clippy::collapsible_if)]
            if tex.selection == SelectedTexture::Noise {
                if let Ok(mut g) = requests.0.ocean.lock() {
                    *g = Some((tex.data.clone(), tex.resolution, ocean_level.0));
                }
            }
            let image = images.get_mut(ihandle).unwrap();
            image.data = Some(tex.data);
            image.resize(Extent3d {
                width: tex.resolution * 2,
                height: tex.resolution,
                depth_or_array_layers: 1,
            });
            materials.insert(mhandle, ihandle.clone().into());
        }
    }
}

fn toggle(on: &mut bool, force_true: bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {
        let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        if response.clicked() {
            *on = force_true || !*on;
            response.mark_changed();
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
        });

        if ui.is_rect_visible(rect) {
            let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
            let visuals = ui.style().interact_selectable(&response, *on);
            let rect = rect.expand(visuals.expansion);
            let radius = 0.5 * rect.height();
            ui.painter().rect(
                rect,
                radius,
                visuals.bg_fill,
                visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );
            let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
            let center = egui::pos2(circle_x, rect.center().y);
            ui.painter()
                .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
        }
        response
    }
}
