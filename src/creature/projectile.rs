use std::time::Duration;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    FixedTimer, PhysicsLayers,
    creature::{
        connect::{Connected, DisconnectSocket},
        part::{self, SocketIds},
    },
    shared_assets::{MaterialId, MeshId, ShapeId, SharedMaterials, SharedMeshes, SharedShapes},
};

/// Marker component attached to all projectiles.
#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[expect(clippy::duplicated_attributes, reason = "False positive")]
#[require(
    RigidBody::Dynamic,
    CollisionLayers::new(PhysicsLayers::Damage, PhysicsLayers::Damage),
    CollisionEventsEnabled,
    AngularInertia(1.0),
    CenterOfMass::ZERO,
    Mass(1.0)
)]
#[component(immutable)]
pub struct Projectile;
impl Projectile {
    /// Despawning of projectiles which have existed longer than their [`ProjectileKind::LIFESPAN`].
    pub(super) fn sys_despawn(
        mut commands: Commands,
        projectiles: Query<(Entity, &FixedTimer), With<Self>>,
    ) {
        projectiles
            .contiguous_iter_inner()
            .unwrap()
            .for_each(|(entities, timers)| {
                entities.iter().zip(timers).for_each(|(&entity, timer)| {
                    if timer.0.is_finished() {
                        commands.entity(entity).try_despawn();
                    }
                });
            });
    }
}

// TODO: Just use delayed commands instead of timers.

/// Trait implemented by marker components attached to specific types of sockets.
pub(super) trait ProjectileKind: Component + Default {
    const MESH: MeshId;
    const MATERIAL: MaterialId;
    const SHAPE: ShapeId;
    const LIFESPAN: f32;
}

/// Returns a bundle for spawning a projectile with the marker component `T`.
pub(super) fn bundle<T: ProjectileKind>(
    pos: Position,
    rot: Rotation,
    shared_meshes: &Res<SharedMeshes>,
    shared_materials: &Res<SharedMaterials>,
    shared_shapes: &Res<SharedShapes>,
) -> (
    T,
    Projectile,
    Mesh3d,
    MeshMaterial3d<StandardMaterial>,
    Collider,
    Position,
    Rotation,
    FixedTimer,
) {
    (
        T::default(),
        Projectile,
        Mesh3d(shared_meshes.get(T::MESH)),
        MeshMaterial3d(shared_materials.get(T::MATERIAL)),
        Collider::from(shared_shapes.get(T::SHAPE)),
        pos,
        rot,
        FixedTimer(Timer::new(
            Duration::from_secs_f32(T::LIFESPAN),
            TimerMode::Once,
        )),
    )
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
pub(super) struct Needle;
impl ProjectileKind for Needle {
    const MESH: MeshId = MeshId::ProjectileNeedle;
    const MATERIAL: MaterialId = MaterialId::Socket;
    const SHAPE: ShapeId = ShapeId::ProjectileNeedle;
    const LIFESPAN: f32 = 0.05;
}
impl Needle {
    pub(super) fn on_collision_start(
        event: On<CollisionStart>,
        mut commands: Commands,
        hearts: Query<&SocketIds, With<part::heart::Heart>>,
        connected_sockets: Query<(), With<Connected>>,
    ) {
        info!("e");
        // TODO: Maybe we want to do this in [`Projectile`]?
        if let Ok(heart) = hearts.get_inner(event.collider2) {
            let socket = *heart.0.first().unwrap();

            if connected_sockets.contains(socket) {
                commands.queue(DisconnectSocket {
                    connected_socket: socket,
                });
            }
        }
    }
}
