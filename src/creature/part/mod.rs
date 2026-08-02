//! Generic items used by all creature parts.

use std::num::NonZero;

use avian2d::{
    data_structures::{graph::NodeIndex, stable_graph::StableUnGraph},
    dynamics::solver::joint_graph::{JointGraph, JointGraphEdge},
    prelude::*,
};
use bevy::{
    ecs::{entity::UniqueEntitySlice, relationship::RelatedSpawnerCommands},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, FromRepr};
use tinyvec::ArrayVec;

use crate::{
    PhysicsLayers,
    consts::{MAX_HEART_DIST, PART_ANGULAR_DAMPING},
    creature::{
        CreatureKindId, SharedShapes, connect::DisconnectSocket, generic_socket::SocketKindId, part,
    },
    shared_assets::{MaterialId, MeshId, ShapeId, SharedMaterials, SharedMeshes},
};

pub mod attachment;
pub mod heart;
pub mod regular;
pub mod weapon;

/// Component storing a list of [`Reactable`] that a creature part will react to,
/// and in how long each reaction will happen.
#[derive(Clone, Copy, Component, Default, PartialEq)]
pub(super) struct Reactables {
    /// The currently active reactables applying to this part, sorted from oldest to newest.
    pub active: ArrayVec<[(f32, Reactable); 4]>,
    /// Whether this part currently using a dedicated, non-shared material.
    ///
    /// If false, a shared material is being used instead.
    dedicated_material: bool,
}
impl Reactables {
    pub(super) fn sys_tick(
        mut commands: Commands,
        parts: Query<(
            &mut Self,
            &mut MeshMaterial3d<StandardMaterial>,
            &SocketIds,
            &ChildOf,
            Entity,
            Has<HeadOfCreature>,
            Has<part::heart::Heart>,
        )>,
        hearts: Query<(), With<part::heart::Heart>>,
        creatures: Query<&CreatureKindId>,
        shared_materials: Res<SharedMaterials>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        joint_graph: Res<JointGraph>,
        time: Res<Time>,
    ) {
        let delta = time.delta_secs();

        parts.into_iter().for_each(
            |(
                mut reactables,
                mut material_handle,
                sockets,
                child_of,
                entity,
                is_head,
                is_heart,
            )| {
                let creature = creatures.get(child_of.parent()).unwrap();

                let base_material_id = if is_head {
                    match creature {
                        CreatureKindId::Player => MaterialId::PartHeadPlayer,
                        CreatureKindId::PlayerFriendly => MaterialId::PartHeadPlayerFriendly,
                        CreatureKindId::Wandering => MaterialId::PartHeadWandering,
                        CreatureKindId::Hostile => MaterialId::PartHeadHostile,
                        CreatureKindId::HeartOnly => MaterialId::PartHeadHeartOnly,
                    }
                } else {
                    match creature {
                        CreatureKindId::Player => MaterialId::PartPlayer,
                        CreatureKindId::PlayerFriendly => MaterialId::PartPlayerFriendly,
                        CreatureKindId::Wandering => MaterialId::PartWandering,
                        CreatureKindId::Hostile => MaterialId::PartHostile,
                        CreatureKindId::HeartOnly => unreachable!(
                            "Heart-only creatures should never contain a non-head part"
                        ),
                    }
                };

                if reactables.active.is_empty() {
                    if reactables.dedicated_material {
                        // Remove the dedicated material and switch to the shared one.

                        reactables.dedicated_material = false;
                        material_handle
                            .set_if_neq(MeshMaterial3d(shared_materials.get(base_material_id)));
                    }
                } else {
                    let mut modifier = Vec3::ONE;

                    reactables.active.retain_mut(|reactable| {
                        // See [`Reactable::FALLOFF`].
                        let intensity = (-(reactable.0.abs())).exp2()
                            * (1.0 - reactable.0).max(0.0)
                            * reactable
                                .0
                                .mul_add(Reactable::FALLOFF.recip(), 1.0)
                                .max(0.0);

                        match reactable.1 {
                            Reactable::Connect => {
                                modifier += vec3(0.0, 3.0, 3.0) * intensity;
                            }
                            Reactable::Disconnect => {
                                modifier += vec3(0.5, -0.5, -0.5) * intensity;
                            }
                            Reactable::Death => {
                                modifier += vec3(5.0, -5.0, -5.0) * intensity;
                            }
                            Reactable::Spawn => {
                                modifier += vec3(-1.0, -1.0, -1.0) * intensity;
                            }
                        }

                        reactable.0 = delta.mul_add(-Reactable::SPEED, reactable.0);

                        // Check the distance to the nearest heart regardless of what the event is,
                        // to prevent spawning or building invalid creatures.
                        //
                        // If the heart is too far away, disconnect all sockets on this part.
                        if !is_heart && reactable.0 <= 0.0 {
                            let mut dead = true;

                            traverse_connected(entity, &joint_graph, &mut |other_entity, depth| {
                                if depth <= MAX_HEART_DIST && hearts.contains(other_entity) {
                                    dead = false;
                                }
                            });

                            if dead {
                                sockets.0.iter().for_each(|&connected_socket| {
                                    commands.queue(DisconnectSocket { connected_socket });
                                });

                                reactable.1 = Reactable::Death; // Convert this reaction into a death, because this part died.
                            }
                        }

                        reactable.0 > -Reactable::FALLOFF
                    });

                    let mut base_material = base_material_id.create();
                    let new_emissive =
                        base_material.emissive.to_vec3() * modifier.max(Vec3::splat(0.0));

                    if reactables.dedicated_material {
                        // Re-use the existing dedicated material.

                        let material = materials.get_mut(material_handle.id()).unwrap();

                        if material.emissive.to_vec3() != new_emissive {
                            let material = material.into_inner();

                            material.emissive = LinearRgba::from_vec3(new_emissive)
                                .with_alpha(material.emissive.alpha);
                        }
                    } else {
                        // Create a dedicated material and switch to it.

                        reactables.dedicated_material = true;

                        base_material.emissive = LinearRgba::from_vec3(new_emissive)
                            .with_alpha(base_material.emissive.alpha);

                        material_handle.0 = materials.add(base_material);
                    }
                }
            },
        );
    }
}

/// Something that has happened in a creature,
/// that all parts inside should react to.
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum Reactable {
    #[default]
    /// The creature has spawned.
    Spawn,
    /// A part has been connected.
    Connect,
    /// A part has been disconnected.
    Disconnect,
    /// A part has died.
    Death,
}

/// Marker component for parts which are currently in the background depth layer.
#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
pub(super) struct PartInBackground;

/// Component attached to all creature part entities, storing the entities of all their sockets.
///
/// This component is intended to be on every creature part.
#[derive(Clone, Component, Eq, PartialEq, Hash, Default)]
#[component(immutable)]
pub struct SocketIds(pub Box<UniqueEntitySlice>);

/// Trait implemented by marker components attached to specific types of creature parts.
///
/// A component implementing this trait must require a [`CreaturePart`].
pub trait CreaturePartKind: Component + Default {
    /// Contribution from this creature part to the total power of its parent creature.
    ///
    /// Only [`heart::Heart`] should have a power of `0`.
    const POWER: u8;

    /// Identifier of the mesh shared by all creature parts of this type.
    const MESH: MeshId;

    /// Identifier of the collider shape shared by all creature parts of this type.
    const SHAPE: ShapeId;

    const LAYER: PhysicsLayers;

    const SOCKETS: &[(Vec2, SocketKindId)];

    /// The capacity for this part to generate force for creature movement.
    const FORCE: f32;
}

/// Component attached to all creature part entities, defining their unique kind.
///
/// This component is only intended to be required by components implementing [`CreaturePartKind`].
#[derive(
    Clone,
    Component,
    Copy,
    Deserialize,
    Debug,
    EnumCount,
    EnumIter,
    Eq,
    Hash,
    FromRepr,
    PartialEq,
    Serialize,
)]
#[component(immutable)]
#[require(
    RigidBody::Dynamic,
    Reactables,
    ColliderDensity(0.5),
    LinearDamping(4.0),
    AngularDamping(PART_ANGULAR_DAMPING)
)]
pub enum CreaturePartKindId {
    Heart,
    TriBlob,
    Diamond,
    Rectangle,
    HugeBlob,
    Leg,
    SmallOval,
    Spear,
}
impl CreaturePartKindId {
    /// Returns the variant of [`spawn`] for the [`CreaturePartKind`] component associated with the value of `self`.
    #[inline]
    #[must_use]
    pub(super) const fn spawn_fn(
        self,
    ) -> fn(
        &mut RelatedSpawnerCommands<ChildOf>,
        &Res<SharedMeshes>,
        &Res<SharedMaterials>,
        &Res<SharedShapes>,
        Vec2,
        f32,
    ) -> (Entity, SocketIds) {
        match self {
            Self::TriBlob => spawn::<regular::TriBlob>,
            Self::HugeBlob => spawn::<regular::HugeBlob>,
            Self::Diamond => spawn::<regular::Diamond>,
            Self::Heart => spawn::<heart::Heart>,
            Self::Leg => spawn::<attachment::Leg>,
            Self::Rectangle => spawn::<attachment::Rectangle>,
            Self::SmallOval => spawn::<regular::SmallOval>,
            Self::Spear => spawn::<weapon::Spear>,
        }
    }

    /// Returns constant metadata about the [`CreaturePartKind`]-implementing component associated with the value of `self`,
    /// in the form of a tuple of ([`CreaturePartKind::POWER`], [`CreaturePartKind::MESH`], [`CreaturePartKind::SHAPE`], [`CreaturePartKind::SOCKETS`], [`CreaturePartKind::create_material`]).
    #[inline]
    #[must_use]
    pub(super) const fn metadata(
        self,
    ) -> (
        u8,
        MeshId,
        ShapeId,
        &'static [(Vec2, SocketKindId)],
        PhysicsLayers,
        f32,
    ) {
        #[inline]
        #[must_use]
        const fn metadata_inner<T: CreaturePartKind>() -> (
            u8,
            MeshId,
            ShapeId,
            &'static [(Vec2, SocketKindId)],
            PhysicsLayers,
            f32,
        ) {
            (T::POWER, T::MESH, T::SHAPE, T::SOCKETS, T::LAYER, T::FORCE)
        }

        match self {
            Self::TriBlob => const { metadata_inner::<regular::TriBlob>() },
            Self::HugeBlob => const { metadata_inner::<regular::HugeBlob>() },
            Self::Diamond => const { metadata_inner::<regular::Diamond>() },
            Self::Heart => const { metadata_inner::<heart::Heart>() },
            Self::Leg => const { metadata_inner::<attachment::Leg>() },
            Self::Rectangle => const { metadata_inner::<attachment::Rectangle>() },
            Self::SmallOval => const { metadata_inner::<regular::SmallOval>() },
            Self::Spear => const { metadata_inner::<weapon::Spear>() },
        }
    }
}

/// Spawns a creature part with the marker component `T` as a child of the creature `commands` was created from,
/// returning the [`Entity`] and [`SocketIds`] of the spawned part.
///
/// The marker component also requires (automatically spawns) other components which are not included here,
/// including a [`CreaturePart`].
fn spawn<T: CreaturePartKind>(
    commands: &mut RelatedSpawnerCommands<ChildOf>,
    shared_meshes: &Res<SharedMeshes>,
    shared_materials: &Res<SharedMaterials>,
    shared_shapes: &Res<SharedShapes>,
    pos: Vec2,
    rot: f32,
) -> (Entity, SocketIds) {
    let mut socket_ids = Box::default();

    (
        commands
            .spawn((
                T::default(),
                Mesh3d(shared_meshes.get(T::MESH)),
                MeshMaterial3d(shared_materials.get(MaterialId::Socket)), // This will be changed almost immediately by the spawning reaction.
                Collider::from(shared_shapes.get(T::SHAPE)),
                Position::new(pos),
                Rotation::radians(rot),
                CollisionLayers::new(T::LAYER, T::LAYER),
            ))
            .with_children(|parent| {
                socket_ids = T::SOCKETS
                    .iter()
                    .map(|(pos, socket_type)| {
                        socket_type.spawn(parent, *pos, shared_meshes, shared_materials)
                    })
                    .collect::<Box<UniqueEntitySlice>>();
            })
            .insert_if_new(SocketIds(socket_ids.clone()))
            .id(),
        SocketIds(socket_ids),
    )
}

/// One-to-one relationship target of [`super::CreatureOfHead`].
#[derive(Clone, Component, Debug, Eq, Hash, PartialEq)]
#[relationship_target(relationship = super::CreatureOfHead)]
pub struct HeadOfCreature(Entity);

/// Runs `f` with the entity of and distance in joints to all parts that the `origin` part is connected to by traversing through all joint connections,
/// starting from **but not including** `origin`.
pub fn traverse_connected(
    origin: Entity,
    joint_graph: &Res<JointGraph>,
    f: &mut impl FnMut(Entity, NonZero<u16>),
) {
    fn walk_graph(
        current: NodeIndex,
        ignored: NodeIndex,
        joint_graph: &StableUnGraph<Entity, JointGraphEdge>,
        depth: NonZero<u16>,
        f: &mut impl FnMut(Entity, NonZero<u16>),
    ) {
        joint_graph.neighbors(current).for_each(|neighbor| {
            if neighbor != ignored {
                let entity = joint_graph.node_weight(neighbor).unwrap();
                f(*entity, depth);

                // Continue the traversal.
                walk_graph(neighbor, current, joint_graph, depth.saturating_add(1), f);
            }
        });
    }

    if let Some(origin_i) = joint_graph.entity_to_body(origin) {
        walk_graph(
            origin_i,
            origin_i,
            joint_graph.graph(),
            const { NonZero::new(1).unwrap() },
            f,
        );
    }
}

/// Description of a spawnable creature part.
#[derive(Clone, Component, Copy, Deserialize, PartialEq, Serialize)]
pub struct CreaturePartData {
    pub kind: CreaturePartKindId,
    pub pos: Vec2,
    pub rot: f32,
}
impl CreaturePartData {
    /// Spawns a creature part described by `self` as a child of the creature `commands` was created from,
    /// returning the [`Entity`] and [`SocketIds`] of the spawned part.
    #[inline]
    pub(super) fn spawn(
        self,
        commands: &mut RelatedSpawnerCommands<ChildOf>,
        shared_meshes: &Res<SharedMeshes>,
        shared_materials: &Res<SharedMaterials>,
        shared_shapes: &Res<SharedShapes>,
    ) -> (Entity, SocketIds) {
        self.kind.spawn_fn()(
            commands,
            shared_meshes,
            shared_materials,
            shared_shapes,
            self.pos,
            self.rot,
        )
    }
}
