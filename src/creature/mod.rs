//! Creature-related functionality.
//!
//! Example structure of a creature:
//!
//! ```text
//! top level:                  Creature
//!                            /   |    \
//! children:              Part⇐⇐Joint⇒⇒Part
//!                       / |    ⇙  ⇘    | \
//!                      /  |  ⇙      ⇘  |  \
//! grandchildren: Socket Socket      Socket Socket
//! ```
//!
//! The creature is the parent of all its parts and joints,
//! and every part is the parent of its sockets.
//!
//! Joints are not parents,
//! but have [`Entity`] references to their connected parts and anchors at the positions of their chosen sockets.

use avian2d::prelude::*;
use bevy::{ecs::relationship::Relationship, prelude::*};
use serde::{Deserialize, Serialize};
use strum::{EnumCount, FromRepr};

use crate::{
    GameState,
    consts::{
        CREATURE_Z, DASH_MUL, DEPTH_LEVEL_POWER, DEPTH_LEVEL_SPEED, DEPTH_LEVEL_STEP,
        TURN_SHARPNESS,
    },
    creature::{
        connect::ConnectSocketsUnchecked,
        event::{CreatureModified, PlayerCreatureModified, RelativeDepthLayerChanged},
        npc::NpcSpawner,
        part::{CreaturePartData, CreaturePartKindId, Reactables, SocketIds},
        projectile::Projectile,
    },
    shared_assets::{SharedMaterials, SharedMeshes, SharedShapes},
};

pub mod connect;
pub mod event;
pub mod generic_socket;
pub mod npc;
pub mod part;
pub mod projectile;
mod sockets;

/// Plugin handing creature functionality.
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct CreaturePlugin;
impl Plugin for CreaturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                sys_nervous,
                Projectile::sys_despawn,
                part::weapon::Spear::sys_nervous,
                npc::PlayerFriendly::sys_control,
                npc::Wandering::sys_control,
                npc::Hostile::sys_control,
                npc::HeartOnly::sys_control,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            ((
                Player::sys_z_from_depth,
                Reactables::sys_tick,
                NpcSpawner::sys_try_spawn.run_if(NpcSpawner::cond_timer_finished),
            )
                .run_if(in_state(GameState::Playing)),),
        )
        .add_observer(CreatureModified::on)
        .add_observer(generic_socket::on_parent_changed)
        .add_observer(generic_socket::ParentChanged::on)
        .add_observer(PlayerCreatureModified::on)
        .add_observer(RelativeDepthLayerChanged::on)
        .init_resource::<NpcSpawner>();
    }
}

/// Marker component for the singular [`Creature`] controlled by the player.
#[derive(Clone, Component, Copy, Default, Eq, Hash, PartialEq)]
#[require(Creature, CreatureKindId::Player, Transform)]
pub struct Player {
    /// Maximum depth that the creature can travel to.
    ///
    /// Should be set to the return value of [`max_depth_from_power`] when the power level of the player creature changes.
    pub max_depth: u16,
}
impl Player {
    /// Smoothly moves the player creature between world space Z levels based on its depth.
    fn sys_z_from_depth(
        mut creature: Single<(&mut Transform, &Creature), With<Self>>,
        time: Res<Time>,
        mut physics_picking_settings: ResMut<PhysicsPickingSettings>,
    ) {
        let target_z = creature.1.z_from_depth();

        if (creature.0.translation.z - target_z).abs() < 0.5 {
            if creature.0.translation.z != target_z {
                // Snap to exactly the target so we can stop mutating the components when it's not visible to the eye.
                creature.0.translation.z = target_z;
            }
        } else {
            creature
                .0
                .translation
                .z
                .smooth_nudge(&target_z, DEPTH_LEVEL_SPEED, time.delta_secs());
        }

        physics_picking_settings.z_plane = creature.0.translation.z;
    }
}

/// Component attached to the parent entity of a creature, storing its global state.
///
/// When spawning a value of this component with non-default `depth`,
/// you likely want to override the required [`Transform`] to one at [`Self::z_from_depth`].
///
/// Otherwise, `Self::default().z_from_depth()` is used.
#[derive(Clone, Component, Copy, Default, PartialEq)]
#[require(
    Transform::from_xyz(0.0, 0.0, Self::default().z_from_depth()),
    Visibility::Visible
)]
pub struct Creature {
    /// No movement, or a movement direction.
    pub desired_movement: Option<Dir2>,

    /// No attack, attack with no target, or attack with a target.
    pub attack: Attack,

    /// Not dashing, or dashing.
    pub dash: bool,

    /// Current depth level.
    pub depth: u16,
}
impl Creature {
    /// Returns the world space Z coordinate that the creature is at or moving towards based on its depth level.
    #[must_use]
    #[inline]
    const fn z_from_depth(&self) -> f32 {
        (self.depth as f32).mul_add(const { -DEPTH_LEVEL_STEP }, CREATURE_Z)
    }

    /// Spawns a complete creature at depth `depth` containing the parts described by `parts`,
    /// connected by the joints described by `joints` as 2D indices into `parts` and then their [`SocketIds`]/[`part::CreaturePartKind::SOCKETS`],
    /// returning the [`Entity`] of the spawned creature and a list of the [`Entity`] and [`SocketIds`] of all spawned parts.
    ///
    /// If `is_player` is set, the spawned creature will contain a [`Player`].
    ///
    /// # Examples
    ///
    /// ```
    /// spawn_creature(
    ///     &mut commands,
    ///     &shared_meshes,
    ///     &shared_materials,
    ///     &shared_shapes,
    ///     &[first_part, second_part],
    ///     &[( // One joint connects the two parts:
    ///         (
    ///             0, // First part.
    ///             0, // First socket on first part.
    ///         ),
    ///         (
    ///             1, // Second part.
    ///             2, // Third socket on second part.
    ///         ),
    ///     )], // The joint kind depends on the kinds of the sockets. They must be equal.
    ///     0, // First part is head.
    ///     0, // Depth is zero.
    ///     0, // Player depth is zero.
    ///     CreatureKindId::Wandering, // The creature is of the wandering kind.
    /// )
    /// ```
    pub fn spawn(
        commands: &mut Commands,
        shared_meshes: &Res<SharedMeshes>,
        shared_materials: &Res<SharedMaterials>,
        shared_shapes: &Res<SharedShapes>,
        parts: impl IntoIterator<Item = CreaturePartData>,
        joints: impl IntoIterator<Item = ((u16, u8), (u16, u8))>,
        head: u16,
        depth: u16,
        player_depth: u16,
        kind: CreatureKindId,
    ) -> (Entity, Box<[(Entity, SocketIds)]>) {
        let mut spawned_parts = None;

        let creature_component = Self { depth, ..default() };
        let mut creature = commands.spawn((
            creature_component,
            Transform::from_xyz(0.0, 0.0, creature_component.z_from_depth()),
        ));
        creature.with_children(|child_spawner| {
            spawned_parts = Some(
                parts
                    .into_iter()
                    .map(|part_data| {
                        part_data.spawn(
                            child_spawner,
                            shared_meshes,
                            shared_materials,
                            shared_shapes,
                        )
                    })
                    .collect::<Box<[(Entity, SocketIds)]>>(),
            );
        });

        match kind {
            CreatureKindId::Player => creature.insert_if_new(Player::default()), // The max depth will be set properly by [`PlayerCreatureModified`],
            CreatureKindId::HeartOnly => creature.insert_if_new(npc::HeartOnly),
            CreatureKindId::PlayerFriendly => creature.insert_if_new(npc::PlayerFriendly),
            CreatureKindId::Wandering => creature.insert_if_new(npc::Wandering),
            CreatureKindId::Hostile => creature.insert_if_new(npc::Hostile),
        };

        let spawned_parts = spawned_parts.unwrap();
        let creature_id = creature.id();

        let mut head = commands.entity((*spawned_parts)[head as usize].0);
        head.add_one_related::<CreatureOfHead>(creature_id);
        let head_id = head.id();

        joints
            .into_iter()
            .for_each(|((part0_i, socket0_i), (part1_i, socket1_i))| {
                commands.queue(ConnectSocketsUnchecked {
                    first_socket: spawned_parts[part0_i as usize].1.0[socket0_i as usize],
                    second_socket: spawned_parts[part1_i as usize].1.0[socket1_i as usize],
                });
            });

        commands.trigger(CreatureModified {
            entity: head_id,
            event: part::Reactable::Spawn,
        });

        if kind == CreatureKindId::Player {
            commands.trigger(PlayerCreatureModified);
        } else {
            commands.trigger(RelativeDepthLayerChanged {
                affected_creature: creature_id,
                player_depth,
            });
        }

        (creature_id, spawned_parts)
    }
}

/// Enum component identifying a kind of creature.
#[derive(
    Clone, Component, Copy, Debug, Deserialize, Eq, EnumCount, FromRepr, Hash, PartialEq, Serialize,
)]
#[component(immutable)]
pub enum CreatureKindId {
    /// Must contain a [`Player`].
    Player,
    /// Must contain a [`npc::HeartOnly`].
    HeartOnly,
    /// Must contain a [`npc::PlayerFriendly`].
    PlayerFriendly,
    /// Must contain a [`npc::Wandering`].
    Wandering,
    /// Must contain a [`npc::Hostile`].
    Hostile,
}

/// Marker component for creatures which are currently in the background depth layer.
#[derive(Clone, Component, Copy, Default, Eq, Hash, PartialEq)]
#[component(immutable)]
struct CreatureInBackground;

/// Marker component for creatures which are currently outside all visible depth layers.
#[derive(Clone, Component, Copy, Default, Eq, Hash, PartialEq)]
#[component(immutable)]
struct CreatureInactive;

/// The current attacking state of a creature.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Attack {
    #[default]
    None,
    WithoutTarget,
    WithTarget(Vec2),
}

/// One-to-one relationship connecting a creature and the creature part currently acting as its "head" in movement.
#[derive(Clone, Component, Copy, Debug, Eq, Hash, PartialEq)]
#[relationship(relationship_target = part::HeadOfCreature)]
pub struct CreatureOfHead(Entity);

/// Creature part movement based on creature intent.
fn sys_nervous(
    mut parts: Query<(Forces, &CreaturePartKindId), Without<RigidBodyDisabled>>,
    creatures: Query<(&Creature, &CreatureOfHead, &Children), Without<CreatureInactive>>,
) {
    creatures.contiguous_iter_inner().unwrap().for_each(
        |(creatures, creature_of_heads, childrens)| {
            creatures
                .iter()
                .zip(creature_of_heads)
                .zip(childrens)
                .for_each(|((creature, creature_of_head), children)| {
                    let head_entity = creature_of_head.get();

                    if let Some(desired_dir) = creature.desired_movement
                        && let Ok((head, ..)) = parts.get(head_entity)
                    {
                        let head = head.position().0;
                        let midpoint = {
                            let mut part_count: u16 = 0;

                            parts
                                .iter_many(children)
                                .map(|(forces, ..)| {
                                    part_count += 1;
                                    forces.position().0
                                })
                                .sum::<Vec2>()
                                / part_count as f32
                        };

                        if let Ok(current_dir) = Dir2::new(head - midpoint) {
                            let required_rotation =
                                current_dir.rotation_to(desired_dir).as_radians();

                            let mut child_parts = parts.iter_many_mut(children);
                            while let Some((mut forces, part)) = child_parts.fetch_next() {
                                forces.apply_force({
                                    let force = if let Ok(right_rotating_force_dir) =
                                        Dir2::new((head - forces.position().0).perp())
                                    {
                                        let rotating_force_dir = if required_rotation >= 0.0 {
                                            -right_rotating_force_dir
                                        } else {
                                            right_rotating_force_dir
                                        };

                                        desired_dir
                                            .slerp(
                                                rotating_force_dir,
                                                (required_rotation.abs() * TURN_SHARPNESS).min(1.0),
                                            )
                                            .as_vec2()
                                    } else {
                                        // The current part is the head of the creature,
                                        // or somehow located at the exact position of it.
                                        desired_dir.as_vec2()
                                    } * part.metadata().5;

                                    if creature.dash {
                                        force * DASH_MUL
                                    } else {
                                        force
                                    }
                                });
                            }
                        } else if let Ok((mut head_forces, head_part)) = parts.get_mut(head_entity)
                        {
                            // The head is exactly the midpoint of the creature.
                            // This very likely means that the creature is just a single part,
                            // so we just apply a force to the head.
                            head_forces.apply_force({
                                let force = desired_dir.as_vec2() * head_part.metadata().5;

                                if creature.dash {
                                    force * DASH_MUL
                                } else {
                                    force
                                }
                            });
                        }
                    }
                });
        },
    );
}

/// Returns the maximum depth level that a creature with a power value of `power` can exist at.
#[must_use]
#[inline]
const fn max_depth_from_power(power: u32) -> u16 {
    (power / DEPTH_LEVEL_POWER.get()) as u16
}
