//! Generic items used by all creature joints and sockets.

use std::time::Duration;

use avian2d::{
    collision::{
        collider::{Collider, CollisionLayers, Sensor},
        collision_events::CollisionEventsEnabled,
    },
    picking::PhysicsPickable,
};
use bevy::{ecs::lifecycle, prelude::*};

use crate::{
    GameState, PhysicsLayers,
    building::{self, BuildingState},
    creature::{
        Player,
        npc::{HeartOnly, PlayerFriendly},
        part::SocketIds,
        sockets,
    },
    shared_assets::{MaterialId, MeshId, ShapeId, SharedMaterials, SharedMeshes, SharedShapes},
};
use serde::{Deserialize, Serialize};

/// Enum responsible for keeping track of the different types of sockets.
///
/// This component determines what type of Avian joint is spawned by the [`super::connect::ConnectSockets`] as well as the sockets material, mesh and shape.
#[derive(Component, Deserialize, Serialize, PartialEq, Eq, Copy, Clone)]
#[component(immutable)]
pub enum SocketKindId {
    Fixed,
    Rotating,
    Attachment,
    /// Sockets specifically made for [`super::part::heart::Heart`].
    Heart,
}

impl SocketKindId {
    /// Spawns a socket matching the [`SocketKindId`] (self) and returns the spawned entity.
    /// The spawned entity has a transform, as well as everything from the [`bundle`].
    ///
    /// Also spawns observers to watch the socket in case it is clicked, dragged, dropped, right clicked or collides with another socket,
    /// all of which only work in building mode.
    #[must_use]
    pub(super) fn spawn(
        self,
        parent: &mut ChildSpawnerCommands,
        pos: Vec2,
        meshes: &Res<SharedMeshes>,
        materials: &Res<SharedMaterials>,
    ) -> Entity {
        // Spawns a socket of the kind corresponding to `self` and returns the spawned socket's [`Entity`].
        let mut spawned = parent.spawn(Transform::from_translation(pos.extend(0.0)));

        match self {
            Self::Fixed => spawned.insert_if_new(bundle::<sockets::Fixed>(meshes, materials)),
            Self::Rotating => spawned.insert_if_new(bundle::<sockets::Rotating>(meshes, materials)),
            Self::Attachment => {
                spawned.insert_if_new(bundle::<sockets::Attachment>(meshes, materials))
            }
            Self::Heart => spawned.insert_if_new(bundle::<sockets::Heart>(meshes, materials)),
        }
        .id()
    }

    /// Returns constant metadata about the [`SocketKind`]-implementing component associated with the value of `self`,
    /// in the form of a tuple of ([`SocketKind::MESH`], [`SocketKind::MATERIAL`], [`SocketKind::SHAPE`]).
    #[inline]
    #[must_use]
    pub(super) const fn metadata(self) -> (MeshId, ShapeId) {
        #[inline]
        #[must_use]
        const fn metadata_inner<T: SocketKind>() -> (MeshId, ShapeId) {
            (T::MESH, T::SHAPE)
        }

        match self {
            Self::Fixed => const { metadata_inner::<sockets::Fixed>() },
            Self::Rotating => const { metadata_inner::<sockets::Rotating>() },
            Self::Attachment => const { metadata_inner::<sockets::Attachment>() },
            Self::Heart => const { metadata_inner::<sockets::Heart>() },
        }
    }
}

/// Trait implemented by marker components attached to specific types of sockets.
///
/// A component implementing this trait must require a [`SocketKindId`].
pub(super) trait SocketKind: Component + Default {
    /// Identifier of the mesh shared by all sockets of this type.
    const MESH: MeshId;

    /// Identifier of the collider shape shared by all sockets of this type.
    const SHAPE: ShapeId;
}

/// Returns a bundle for spawning a socket with the marker component `T`.
///
/// The marker component also requires (automatically spawns) other components which are not included here,
/// including a [`SocketKindId`] and a [`Transform::default()`]. **You probably want to override the [`Transform`] for most use cases.**
///
/// # Examples
///
/// ```
/// // Spawns a socket.
/// commands.spawn((
///     generic_socket::bundle::<MySocketKind>(&shared_meshes, &shared_shapes, &mut materials),
///     Transform::from_xyz(0.067, 0.420, 0.69),
/// ));
/// ```
#[must_use]
pub(super) fn bundle<T: SocketKind>(
    shared_meshes: &Res<SharedMeshes>,
    shared_materials: &Res<SharedMaterials>,
) -> (T, Mesh3d, MeshMaterial3d<StandardMaterial>) {
    (
        T::default(),
        Mesh3d(shared_meshes.get(T::MESH)),
        MeshMaterial3d(shared_materials.get(MaterialId::Socket)),
    )
}

pub(super) fn on_parent_changed(event: On<lifecycle::Add, ChildOf>, mut commands: Commands) {
    // This indirection and delay is necessary for some reason,
    // or the queries don't work properly.
    //
    // TODO: Ponder.
    commands
        .delayed()
        .duration(Duration::from_nanos(1))
        .trigger(ParentChanged(event.event_target()));
}

#[derive(Clone, Copy, EntityEvent, Eq, Hash, PartialEq)]
pub(super) struct ParentChanged(Entity);
impl ParentChanged {
    pub(super) fn on(
        event: On<Self>,
        mut commands: Commands,
        parts: Query<(&ChildOf, &SocketIds)>,
        pickable_creatures: Query<(), Or<(With<Player>, With<PlayerFriendly>, With<HeartOnly>)>>,
        unpickable_sockets: Query<(Entity, &SocketKindId), Without<PhysicsPickable>>,
        shared_shapes: Res<SharedShapes>,
    ) {
        let event_target = event.event_target();

        if let Ok(part) = parts.get_inner(event_target)
            && pickable_creatures.contains(part.0.parent())
        {
            // TODO: We could have a marker component for parts which are already pickable,
            // to skip the work below.

            let playing_and_building =
                in_state(GameState::Playing).and_then(in_state(BuildingState::Building));

            let playing_and_dragging =
                in_state(GameState::Playing).and_then(in_state(BuildingState::Dragging));

            let mut observers = [
                Observer::new(building::on_leave).run_if(playing_and_building.clone()),
                Observer::new(building::on_enter).run_if(playing_and_building.clone()),
                Observer::new(building::on_press).run_if(playing_and_building.clone()),
                Observer::new(building::on_drag_start).run_if(playing_and_building),
                Observer::new(building::on_drag_enter).run_if(playing_and_dragging.clone()),
                Observer::new(building::on_drag_drop).run_if(playing_and_dragging),
                Observer::new(building::on_collision_start),
            ];

            unpickable_sockets
                .iter_many_unique_inner(part.1.0.iter())
                .for_each(|(id, kind)| {
                    // Make socket pickable.
                    commands.entity(id).insert_if_new((
                        Collider::from(shared_shapes.get(kind.metadata().1)),
                        PhysicsPickable,
                        Sensor,
                        CollisionEventsEnabled,
                        CollisionLayers::new(PhysicsLayers::Socket, PhysicsLayers::Socket),
                    ));

                    observers.iter_mut().for_each(|obs| {
                        obs.watch_entity(id);
                    });

                    /*
                        // There is currently no way for a part to move from a pickable creature to one that isn't pickable.
                        //
                        // If there was, something like this could be done here.
                        /* <if the creature isn't pickable> */ {
                            socket_commands.remove::<(
                                Collider,
                                PhysicsPickable,
                                Sensor,
                                CollisionEventsEnabled,
                                CollisionLayers,
                            )>();

                            // TODO: Despawn all observers watching this socket, maybe using [`ObservedBy`] and `despawn_related`.
                        }
                    */
                });

            commands.spawn_batch(observers);
        }
    }
}
