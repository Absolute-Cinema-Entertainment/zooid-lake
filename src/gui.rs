use avian2d::{parry::shape::SharedShape, prelude::*};
use bevy::{
    asset::RenderAssetUsages,
    ecs::relationship::Relationship,
    mesh::{AnnulusMeshBuilder, CircleMeshBuilder},
    picking::hover::PickingInteraction,
    prelude::*,
};

use crate::{
    GameState,
    consts::{
        CAMERA_PAUSED_Z, CREATURE_Z, GUI_ANIM_SPEED, GUI_Z, WINDOW_TITLE_PAUSED, WINDOW_TITLE_ROOT,
    },
    creature::{Creature, CreatureOfHead, Player, part::HeadOfCreature},
    session::{self, SessionId},
    shared_assets::{MaterialId, SharedMaterials, SharedMeshes, SharedShapes},
};

const SAVE_BUTTON_GAP: f32 = 3.0;

/// Plugin handling GUI behavior.
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct GuiPlugin;
impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sys_tick.run_if(in_state(GameState::Paused)))
            .add_systems(OnEnter(GameState::Paused), sys_spawn);
    }
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(Visibility::Visible)]
struct GuiRoot;

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(Visibility::Visible, Transform)]
struct SaveRoot;

#[derive(Clone, Component, Copy, Eq, PartialEq, Hash)]
#[require(Visibility::Visible, PhysicsPickable, Sensor)]
struct SaveSlot {
    id: u8,
    exists: bool,
}
impl SaveSlot {
    fn make_hovered(
        event_target: Entity,
        mut buttons: Query<&mut MeshMaterial3d<StandardMaterial>, With<Self>>,
        shared_materials: Res<SharedMaterials>,
    ) {
        buttons.get_mut(event_target).unwrap().0 = shared_materials.get(MaterialId::GuiHovered);
    }

    fn on_enter(
        event: On<Pointer<Enter>>,
        buttons: Query<&mut MeshMaterial3d<StandardMaterial>, With<Self>>,
        shared_materials: Res<SharedMaterials>,
    ) {
        Self::make_hovered(event.event_target(), buttons, shared_materials);
    }

    fn on_release(
        event: On<Pointer<Release>>,
        buttons: Query<&mut MeshMaterial3d<StandardMaterial>, With<Self>>,
        shared_materials: Res<SharedMaterials>,
    ) {
        Self::make_hovered(event.event_target(), buttons, shared_materials);
    }

    fn on_leave(
        event: On<Pointer<Leave>>,
        mut buttons: Query<&mut MeshMaterial3d<StandardMaterial>, With<Self>>,
        shared_materials: Res<SharedMaterials>,
    ) {
        buttons.get_mut(event.event_target()).unwrap().0 =
            shared_materials.get(MaterialId::GuiDefault);
    }

    fn on_press(
        event: On<Pointer<Press>>,
        mut commands: Commands,
        mut window: Single<&mut Window>,
        mut buttons: Query<(&mut Self, &mut MeshMaterial3d<StandardMaterial>)>,
        creatures: Query<Entity, With<Creature>>,
        mut session_id: ResMut<SessionId>,
        shared_meshes: Res<SharedMeshes>,
        shared_materials: Res<SharedMaterials>,
        shared_shapes: Res<SharedShapes>,
    ) {
        let (mut slot, mut material) = buttons.get_mut(event.event_target()).unwrap();

        material.0 = shared_materials.get(MaterialId::GuiPressed);

        if session_id.0 != slot.id {
            session_id.0 = slot.id;

            let window_title = format!("{WINDOW_TITLE_ROOT} ({}) {WINDOW_TITLE_PAUSED}", slot.id);
            if window.title != window_title {
                window.title = window_title;
            }

            session::clear(&mut commands, &creatures);

            let exists = session::exists(slot.id);
            if slot.exists != exists {
                slot.exists = exists;
            }

            if exists {
                session::load(
                    commands,
                    shared_meshes,
                    shared_materials,
                    shared_shapes,
                    slot.id,
                );
            } else {
                session::load_default(commands, shared_meshes, shared_materials, shared_shapes);
            }
        }
    }
}

fn sys_spawn(
    mut commands: Commands,
    camera: Single<&Transform, With<Camera>>,
    shared_materials: Res<SharedMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut save_button_observers = [
        Observer::new(SaveSlot::on_enter),
        Observer::new(SaveSlot::on_leave),
        Observer::new(SaveSlot::on_press),
        Observer::new(SaveSlot::on_release),
    ];

    let camera = **camera;

    commands
        .spawn((
            DespawnOnExit(GameState::Paused),
            GuiRoot,
            camera.with_translation(
                camera
                    .translation
                    .with_z(camera.translation.z + CAMERA_PAUSED_Z),
            ),
        ))
        .with_children(|gui_root| {
            let save_mesh = Mesh3d(meshes.add({
                let mut mesh = AnnulusMeshBuilder::new(1.0, 1.1, 4).build();
                mesh.merge(&CircleMeshBuilder::new(0.8, 4).build()).unwrap();
                mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;

                mesh
            }));
            let save_default_material =
                MeshMaterial3d(shared_materials.get(MaterialId::GuiDefault));

            /*
                // TODO: (?) Button to open the save directory in the default program.
                #[cfg(not(target_family = "wasm"))]
                parent.spawn((
                    Transform {
                        translation: Vec3::new(0.0, -24.0, 0.0),
                        rotation: Quat::from_axis_angle(Vec3::Z, PI / 4.0),
                        scale: Vec2::splat(0.5).extend(0.0),
                    },
                    save_default_material.clone(),
                    save_mesh.clone(),
                ));
            */

            gui_root.spawn((
                Transform {
                    translation: vec3(0.0, -2.0, 0.0),
                    ..default()
                },
                save_default_material.clone(),
                Mesh3d(meshes.add({
                    let mut mesh: Mesh = Polyline2d::new([
                        vec2(-2.0, 0.0),
                        vec2(-1.0, 0.0),
                        vec2(0.0, -1.0),
                        vec2(1.0, 0.0),
                        vec2(2.0, 0.0),
                    ])
                    .into();

                    mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;

                    mesh
                })),
            ));

            gui_root.spawn(SaveRoot).with_children(|save_root| {
                let save_collider = Collider::from(SharedShape::ball(1.1));

                (0..=u8::MAX).for_each(|id| {
                    let exists = session::exists(id);

                    let button = save_root
                        .spawn((
                            SaveSlot { id, exists },
                            Transform::from_xyz(SAVE_BUTTON_GAP * id as f32, 0.0, 0.0).with_scale(
                                Vec2::splat(if exists { 1.0 } else { 0.25 }).extend(1.0),
                            ),
                            save_mesh.clone(),
                            save_default_material.clone(),
                            save_collider.clone(),
                        ))
                        .id();

                    save_button_observers.iter_mut().for_each(|obs| {
                        obs.watch_entity(button);
                    });
                });
            });
        });

    commands.spawn_batch(save_button_observers);
}

fn sys_tick(
    mut gui_root: Single<&mut Transform, (With<GuiRoot>, Without<SaveSlot>)>,
    mut save_root: Single<&mut Transform, (With<SaveRoot>, Without<GuiRoot>, Without<SaveSlot>)>,
    save_slots: Query<(&SaveSlot, &PickingInteraction, &mut Transform)>,
    mut physics_picking_settings: ResMut<PhysicsPickingSettings>,
    session_id: ResMut<SessionId>,
    player: Single<&CreatureOfHead, With<Player>>,
    heads: Query<&GlobalTransform, With<HeadOfCreature>>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();

    let player_head = heads.get(player.get()).unwrap().translation();
    gui_root.translation.smooth_nudge(
        &player_head.with_z(player_head.z - CREATURE_Z + GUI_Z),
        GUI_ANIM_SPEED * 0.1,
        delta,
    );
    physics_picking_settings.z_plane = gui_root.translation.z;

    let mut save_root_x = save_root.translation.x;
    save_root_x.smooth_nudge(
        &(-SAVE_BUTTON_GAP * session_id.0 as f32),
        GUI_ANIM_SPEED * 0.5,
        delta,
    );
    save_root.translation.x = save_root_x;

    save_slots
        .into_iter()
        .for_each(|(slot, interaction, mut transform)| {
            let mut xy_scale = transform.scale.xy();
            xy_scale.smooth_nudge(
                &Vec2::splat({
                    let mut scale = match interaction {
                        PickingInteraction::None => 1.0,
                        PickingInteraction::Hovered => 1.1,
                        PickingInteraction::Pressed => 0.9,
                    };

                    if !slot.exists {
                        scale *= 0.25;
                    }

                    scale
                }),
                GUI_ANIM_SPEED,
                delta,
            );
            transform.scale = transform.scale.with_xy(xy_scale);
        });
}
