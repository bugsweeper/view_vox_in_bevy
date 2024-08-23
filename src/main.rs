use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;

#[derive(Resource)]
pub struct Scene {
    pub grandparent: Entity,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "MagicaVox viewier using bevy".to_string(),
                ..default()
            }),
            ..default()
        })
        .set(AssetPlugin {
            file_path: std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
            ..default()
        }),))
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, file_drop)
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin);

    app.run();
}

fn load_vox(
    vox_path: &str,
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Vec3 {
    info!("Loading {}", vox_path);

    let vox = dot_vox::load(vox_path).unwrap();
    if vox.models.is_empty() || vox.models[0].voxels.is_empty() {
        return Vec3::ZERO;
    }

    let cube_mesh = meshes.add(Cuboid::from_length(1.0));

    let mut make_bundle = |voxel: dot_vox::Voxel| {
        let palette_index = voxel.i as usize;
        let color: &dot_vox::Color = vox
            .palette
            .get(palette_index)
            .or(dot_vox::DEFAULT_PALETTE.get(palette_index))
            .unwrap();
        let color: Color = Srgba::from_u8_array(color.into()).into();
        let mut mesh_material = StandardMaterial {
            base_color: color,
            alpha_mode: if color.alpha() < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..default()
        };
        if let Some(material) = vox
            .materials
            .iter()
            .find(|material| material.id == u32::from(voxel.i))
        {
            if let Some(metalness) = material.metalness() {
                mesh_material.metallic = metalness;
            }
            if let Some(roughness) = material.roughness() {
                mesh_material.perceptual_roughness = roughness;
            }
            if let Some(specular) = material.specular() {
                mesh_material.specular_transmission = specular;
            }
            if let Some(index_of_refraction) = material.refractive_index() {
                mesh_material.ior = index_of_refraction;
            }
            if let Some(attenuation) = material.attenuation() {
                mesh_material.attenuation_distance = attenuation;
            }
        }

        PbrBundle {
            mesh: cube_mesh.clone(),
            material: materials.add(mesh_material),
            transform: Transform::from_xyz(
                f32::from(voxel.x),
                f32::from(voxel.y),
                f32::from(voxel.z),
            ),
            ..default()
        }
    };

    //commands.spawn(make_bundle(vox.models[0].voxels[0]));
    let mut dimensions = Vec3::default();

    let grandparent = commands
        .spawn(SpatialBundle::default())
        .with_children(|grand_parent| {
            for model in vox.models {
                grand_parent
                    .spawn(SpatialBundle::default())
                    .with_children(|parent| {
                        for voxel in model.voxels {
                            parent.spawn(make_bundle(voxel));
                        }
                    });
                dimensions = dimensions.max(Vec3::new(
                    model.size.x as f32,
                    model.size.y as f32,
                    model.size.z as f32,
                ));
            }
        })
        .id();

    commands.insert_resource(Scene { grandparent });

    dimensions
}

fn clear_vox(commands: &mut Commands, scene: &Res<Scene>) {
    commands.entity(scene.grandparent).despawn_recursive();
    commands.remove_resource::<Scene>();
}

fn setup(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    let vox_path = std::env::args().nth(1);
    let dimensions = load_vox(
        vox_path.as_deref().unwrap_or("assets/snow.vox"),
        &mut commands,
        meshes,
        materials,
    );

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(
                dimensions / 2.0 + Vec3::ZERO.with_z(3.0 * dimensions.z),
            )
            .looking_at(dimensions / 2.0, Dir3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
    ));

    commands.spawn((
        PerfUiRoot {
            display_labels: false,
            layout_horizontal: true,
            ..default()
        },
        PerfUiEntryFPS::default(),
    ));
}

fn file_drop(
    mut evr_dnd: EventReader<FileDragAndDrop>,
    mut commands: Commands,
    scene: Res<Scene>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    mut camera_transform: Query<&mut Transform, With<Camera>>,
) {
    for ev in evr_dnd.read() {
        if let FileDragAndDrop::DroppedFile {
            window: _,
            path_buf,
        } = ev
        {
            if path_buf
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("vox"))
            {
                if let Some(path) = path_buf.as_os_str().to_str() {
                    clear_vox(&mut commands, &scene);

                    let dimensions = load_vox(path, &mut commands, meshes, materials);

                    for mut transform in &mut camera_transform {
                        transform.translation =
                            dimensions / 2.0 + Vec3::ZERO.with_z(3.0 * dimensions.z);
                        transform.look_at(dimensions / 2.0, Dir3::Y);
                    }
                    // Loads only first file
                    return;
                }
                warn!("could not read path {path_buf:?}");
            } else {
                warn!("File {path_buf:?} is not MagicaVox file");
            }
        }
    }
}
