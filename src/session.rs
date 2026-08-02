//! Session save & load.

use avian2d::physics_transform::{Position, Rotation};
use bevy::{prelude::*, window::WindowClosing};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    VirtualTimer,
    creature::{
        Creature, CreatureKindId,
        connect::CreatureJoint,
        part::{CreaturePartData, CreaturePartKindId, HeadOfCreature},
    },
    shared_assets::{SharedMaterials, SharedMeshes, SharedShapes},
};

/// Plugin handling session save & load.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub struct SessionPlugin;
impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, sys_startup)
            .add_systems(Update, sys_save.run_if(Autosave::cond_should_save))
            .add_systems(Last, sys_save.run_if(on_message::<WindowClosing>))
            .init_resource::<SessionId>()
            .init_resource::<Autosave>();
    }
}

/// Loading of the first or default session.
fn sys_startup(
    commands: Commands,
    shared_meshes: Res<SharedMeshes>,
    shared_materials: Res<SharedMaterials>,
    shared_shapes: Res<SharedShapes>,
) {
    if exists(0) {
        load(commands, shared_meshes, shared_materials, shared_shapes, 0);
    } else {
        load_default(commands, shared_meshes, shared_materials, shared_shapes);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CreatureSave {
    /// Player's last depth.
    pub depth: u16,

    /// The kind of the creature.
    pub kind: CreatureKindId,

    /// Player head part id.
    pub head: u16,

    /// Lists parts of Creatures.
    pub parts: Vec<CreaturePartData>,

    /// Lists joints of Creatures.
    pub joints: Vec<JointSave>,
}

/// [`CreatureJoint`] but with own ids for parts to be saved in ``SavedFile``
#[derive(Clone, Serialize, Deserialize)]
pub struct JointSave {
    pub part_a: u16,
    pub socket_a: u8,

    pub part_b: u16,
    pub socket_b: u8,
}

/// Timer for periodically saving the current session.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Resource)]
#[component(immutable)]
#[require(VirtualTimer(Timer::from_seconds(Self::DELAY, TimerMode::Repeating)))]
pub struct Autosave;
impl Autosave {
    /// Saves automatically after a period of time for web.
    #[must_use]
    fn cond_should_save(timer: Single<&VirtualTimer, With<Self>>) -> bool {
        timer.0.just_finished()
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Resource)]
pub struct SessionId(pub u8);

#[derive(Clone, Serialize, Deserialize)]
struct WorldSave(pub Vec<CreatureSave>);

/// Saves the current session.
pub fn sys_save(
    creature_query: Query<(&Children, &Creature, &CreatureKindId)>,
    joint_query: Query<&CreatureJoint>,
    part_query: Query<(
        Entity,
        &CreaturePartKindId,
        &Position,
        &Rotation,
        Has<HeadOfCreature>,
    )>,
    id: Res<SessionId>,
) {
    let save = postcard::to_stdvec(&WorldSave(
        creature_query
            .into_iter()
            .map(|(creature_children, creature, creature_kind)| {
                let mut head = None;
                let mut entity_to_part_id = HashMap::<Entity, u16>::new();

                CreatureSave {
                    depth: creature.depth,
                    kind: *creature_kind,
                    parts: part_query
                        .iter_many_inner(creature_children)
                        .enumerate()
                        .map(|(part_id, (part_entity, &kind, pos, rot, is_head))| {
                            entity_to_part_id.insert(part_entity, part_id as u16);

                            if is_head {
                                head = Some(part_id as u16);
                            }

                            CreaturePartData {
                                pos: pos.0,
                                rot: rot.as_radians(),
                                kind,
                            }
                        })
                        .collect(),
                    joints: joint_query
                        .into_iter()
                        .filter_map(|joint| {
                            let part_a_id = *entity_to_part_id.get(&joint.part_a)?;
                            let part_b_id = *entity_to_part_id.get(&joint.part_b)?;

                            Some(JointSave {
                                part_a: part_a_id,
                                socket_a: joint.socket_a,

                                part_b: part_b_id,
                                socket_b: joint.socket_b,
                            })
                        })
                        .collect(),
                    head: head.unwrap(),
                }
            })
            .collect(),
    ))
    .unwrap();

    let mut save_name = "zlsave-".to_owned();
    save_name.push_str(&id.0.to_string());

    cfg_select! {
        target_family = "wasm" => {
            {
                use web_sys::window;
                use base64::engine::{Engine, general_purpose::STANDARD};

                let storage = window().unwrap().local_storage().unwrap().unwrap();
                storage.set_item(&save_name, &STANDARD.encode(save)).unwrap();
            }
        }
        _ => {
            {
                use std::{fs::OpenOptions, io::Write, path::{Path, PathBuf}};

                use crate::consts::{SAVE_DIRECTORY_PATH, SAVE_FILE_EXTENSION};

                let project_dirs = project_dirs();
                let data_dir = project_dirs.data_dir();
                let _ = std::fs::create_dir(data_dir);

                let save_dir = [data_dir, Path::new(SAVE_DIRECTORY_PATH)].into_iter().collect::<PathBuf>();
                let _ = std::fs::create_dir(&save_dir);

                let mut save_file = save_dir;
                save_file.push(Path::new(&save_name));
                save_file.add_extension(SAVE_FILE_EXTENSION);

                OpenOptions::new().write(true).create(true).truncate(true).open(
                    save_file
                ).unwrap().write_all(&save).unwrap();
            }
        }
    }
}

/// Despawns all current creatures.
pub fn clear(commands: &mut Commands, creatures: &Query<Entity, With<Creature>>) {
    creatures
        .contiguous_iter_inner()
        .unwrap()
        .for_each(|creatures| {
            creatures.iter().for_each(|&creature| {
                commands.entity(creature).try_despawn();
            });
        });
}

/// Returns whether a saved session exists with `id`.
#[must_use]
pub fn exists(id: u8) -> bool {
    let save_name = id.to_string();

    cfg_select! {
        target_family = "wasm" => {
            {
                use web_sys::window;

                let storage = window().unwrap().local_storage().unwrap().unwrap();

                storage.get_item(&save_name).is_ok_and(|content| content.is_some())
            }
        }
        _ => {
            {
                use std::path::{Path, PathBuf};

                let mut save_file = [
                    project_dirs().data_dir(),
                    Path::new(crate::consts::SAVE_DIRECTORY_PATH),
                    Path::new(&save_name)
                ].into_iter().collect::<PathBuf>();

                save_file.add_extension(crate::consts::SAVE_FILE_EXTENSION);

                std::fs::exists(save_file).unwrap_or_default()
            }
        }
    }
}

/// Loads the session with `id`.
///
/// It is possible for the save to stop existing during/before this even if [`exists`] returns `true`,
/// or for it to be corrupt. In that case, we panic.
///
/// TODO: Handle this more cleanly and inform the user somehow of what happened.
pub fn load(
    mut commands: Commands,
    shared_meshes: Res<SharedMeshes>,
    shared_materials: Res<SharedMaterials>,
    shared_shapes: Res<SharedShapes>,
    id: u8,
) {
    let save = {
        let save_name = id.to_string();

        postcard::from_bytes::<WorldSave>(&cfg_select! {
            target_family = "wasm" => {
                {
                    use web_sys::window;
                    use base64::engine::{Engine, general_purpose::STANDARD};

                    let storage = window().unwrap().local_storage().unwrap().unwrap();

                    let raw = storage.get_item(&save_name).unwrap().unwrap();

                    STANDARD.decode(raw).unwrap()
                }
            }
            _ => {
                {
                    use std::path::{Path, PathBuf};

                    let mut save_file = [
                        project_dirs().data_dir(),
                        Path::new(crate::consts::SAVE_DIRECTORY_PATH),
                        Path::new(&save_name)
                    ].into_iter().collect::<PathBuf>();

                    save_file.add_extension(crate::consts::SAVE_FILE_EXTENSION);

                    std::fs::read(save_file).expect("Failed to read save file")
                }
            }
        })
        .unwrap()
    };

    let player_depth = save
        .0
        .iter()
        .find(|save| save.kind == CreatureKindId::Player)
        .unwrap()
        .depth;

    save.0.into_iter().for_each(|creature_save| {
        Creature::spawn(
            &mut commands,
            &shared_meshes,
            &shared_materials,
            &shared_shapes,
            creature_save.parts.clone(),
            creature_save.joints.into_iter().map(|joint_save| {
                (
                    (joint_save.part_a, joint_save.socket_a),
                    (joint_save.part_b, joint_save.socket_b),
                )
            }),
            creature_save.head,
            creature_save.depth,
            player_depth,
            creature_save.kind,
        );
    });
}

/// Loads the default session.
pub fn load_default(
    mut commands: Commands,
    shared_meshes: Res<SharedMeshes>,
    shared_materials: Res<SharedMaterials>,
    shared_shapes: Res<SharedShapes>,
) {
    let heart = CreaturePartData {
        kind: CreaturePartKindId::Heart,
        pos: Vec2::ZERO,
        rot: 0.0,
    };
    let long_leg = CreaturePartData {
        kind: CreaturePartKindId::Leg,
        pos: Vec2::ZERO,
        rot: 0.0,
    };

    Creature::spawn(
        &mut commands,
        &shared_meshes,
        &shared_materials,
        &shared_shapes,
        [
            CreaturePartData {
                kind: CreaturePartKindId::SmallOval,
                pos: Vec2::ZERO,
                rot: 0.0,
            },
            CreaturePartData {
                kind: CreaturePartKindId::TriBlob,
                pos: Vec2::ZERO,
                rot: 0.0,
            },
            CreaturePartData {
                kind: CreaturePartKindId::Spear,
                pos: Vec2::ZERO,
                rot: 0.0,
            },
            long_leg,
            long_leg,
            long_leg,
            long_leg,
            long_leg,
            heart,
            heart,
            heart,
        ],
        [
            ((0, 0), (1, 0)),
            ((0, 1), (2, 0)),
            ((0, 2), (3, 0)),
            ((0, 3), (4, 0)),
            ((1, 1), (5, 0)),
            ((1, 3), (6, 0)),
            ((1, 5), (7, 0)),
            ((0, 4), (8, 0)),
            ((0, 5), (9, 0)),
            ((1, 6), (10, 0)),
        ],
        2,
        0,
        0,
        CreatureKindId::Player,
    );
}

/// Returns the standard project directories of the application.
#[cfg(not(target_family = "wasm"))]
#[inline]
#[must_use]
fn project_dirs() -> directories::ProjectDirs {
    directories::ProjectDirs::from("", "Absolute Cinema Entertainment", "Zooid Lake")
        .expect("Failed to compute project directory paths. It's possible that the home directory path couldn't be found")
}
