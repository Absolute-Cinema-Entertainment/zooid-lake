use std::f32::consts::PI;

use avian2d::{
    dynamics::{
        joints::{FixedJoint, RevoluteJoint},
        solver::joint_graph::JointGraph,
    },
    prelude::{ColliderMassProperties, Rotation},
};

use bevy::{ecs::system::SystemState, prelude::*};
use rand::RngExt;

use crate::{
    building::{LineSegment, SocketCollision},
    consts::FREE_COMPLIANCE,
    creature::{
        Player,
        event::{CreatureModified, PlayerCreatureModified, RelativeDepthLayerChanged},
        npc,
        part::{Reactable, SocketIds},
    },
    shared_assets::{AudioId, MaterialId, MeshId, SharedAudio, SharedMaterials, SharedMeshes},
};

use super::{
    Attack, Creature, CreatureOfHead,
    generic_socket::SocketKindId,
    part::{CreaturePartKindId, traverse_connected},
};

/// A joint component for accessing the meta data of joint connections when saving.
#[derive(Component)]
#[component(immutable)]
pub struct CreatureJoint {
    pub part_a: Entity,
    pub socket_a: u8,

    pub part_b: Entity,
    pub socket_b: u8,
}

/// Component attached to a connected socket storing the joint and socket its connected to.
#[derive(Clone, Copy, Component, Hash, PartialEq, Eq)]
#[component(immutable)]
pub struct Connected {
    pub joint: Entity,
    pub socket: Entity,
}

/// Spawns an avian2d joint.
/// The type of joint depends on [`SocketKindId`] provided via `socket_type`.
pub fn spawn_avian_joint(
    world: &mut World,
    socket_type: SocketKindId,
    attached_socket: Entity,
    attached_part: Entity,
    attached_local: Vec2,
    free_socket: Entity,
    free_part: Entity,
    free_local: Vec2,
    point_compliance: f32,
) -> Entity {
    let free_socket_index = {
        let sockets = world.get::<SocketIds>(free_part).unwrap();

        sockets.0.iter().position(|e| *e == free_socket).unwrap() as u8
    };

    let attached_socket_index = {
        let sockets = world.get::<SocketIds>(attached_part).unwrap();

        sockets
            .0
            .iter()
            .position(|e| *e == attached_socket)
            .unwrap() as u8
    };

    let mut spawned = world.spawn(CreatureJoint {
        part_a: free_part,
        socket_a: free_socket_index,
        part_b: attached_part,
        socket_b: attached_socket_index,
    });

    let local_basis1 = free_local
        .try_normalize()
        .map_or(Rotation::IDENTITY, |vec| {
            Rotation::from_sin_cos(vec.y, vec.x)
        });
    let local_basis2 = attached_local.try_normalize().map_or(Rotation::PI, |vec| {
        let vec = vec2(-1.0, 0.0).rotate(vec);
        Rotation::from_sin_cos(vec.y, vec.x)
    });

    match socket_type {
        SocketKindId::Fixed => spawned.insert_if_new(
            RevoluteJoint::new(free_part, attached_part)
                .with_local_anchor1(free_local)
                .with_local_anchor2(attached_local)
                .with_local_basis1(local_basis1)
                .with_local_basis2(local_basis2)
                .with_point_compliance(point_compliance)
                .with_angle_limits(-PI / 32.0, PI / 32.0)
                .with_limit_compliance(0.00001),
        ),
        SocketKindId::Rotating => spawned.insert_if_new(
            RevoluteJoint::new(free_part, attached_part)
                .with_local_anchor1(free_local)
                .with_local_anchor2(attached_local)
                .with_local_basis1(local_basis1)
                .with_local_basis2(local_basis2)
                .with_point_compliance(point_compliance)
                .with_angle_limits(-PI / 8.0, PI / 8.0)
                .with_limit_compliance(0.00005),
        ),
        SocketKindId::Attachment => spawned.insert_if_new(
            RevoluteJoint::new(free_part, attached_part)
                .with_local_anchor1(free_local)
                .with_local_anchor2(attached_local)
                .with_local_basis1(local_basis1)
                .with_local_basis2(local_basis2)
                .with_point_compliance(point_compliance)
                .with_angle_limits(-PI / 4.0, PI / 4.0)
                .with_limit_compliance(0.00005),
        ),
        SocketKindId::Heart => spawned.insert_if_new(
            FixedJoint::new(free_part, attached_part)
                .with_local_anchor1(free_local)
                .with_local_anchor2(attached_local)
                .with_point_compliance(point_compliance),
        ),
    }
    .id()
}

/// Struct that implements [`Command`] to connect two sockets with an avian joint.
pub struct ConnectSockets {
    pub attached_socket: Entity,
    pub free_socket: Entity,
}

/// Implements a custom command for [`ConnectSockets`] that connects the two sockets provided.
/// It is important that the `attached_socket` and `free_socket` are correct as all parts connected in
/// the free sockets chain will be transfered to the `attached_sockets` chain and the `free_sockets` creature will be despawned.
///
/// # Examples
///
/// ```
/// commands.queue(ConnectSockets {
///     some_attached_socket,
///     some_free_socket,
/// });
/// ```
impl Command for ConnectSockets {
    type Out = ();

    fn apply(self, world: &mut World) {
        // Get their socket kinds and check if they are compatible.
        let Some(attached_type) = world.get::<SocketKindId>(self.attached_socket) else {
            return;
        };
        let Some(free_type) = world.get::<SocketKindId>(self.free_socket) else {
            return;
        };

        if attached_type != free_type {
            return;
        }

        if world.get::<Connected>(self.attached_socket).is_some() {
            return;
        }
        if world.get::<Connected>(self.free_socket).is_some() {
            return;
        }

        let socket_type = *attached_type;

        let Some(free_child_of) = world.get::<ChildOf>(self.free_socket) else {
            return;
        };

        let Some(attached_child_of) = world.get::<ChildOf>(self.attached_socket) else {
            return;
        };

        let free_part = free_child_of.parent();
        let attached_part = attached_child_of.parent();

        let Some(creature_child_of) = world.get::<ChildOf>(attached_part) else {
            return;
        };

        let creature = creature_child_of.parent();
        let mut total_weight: f32 = 0.0;

        // Checks for same creature and gathers the free creatures mass.
        if let Some(free_creature) = world.get::<ChildOf>(free_part) {
            if let Some(children) = world.get::<Children>(free_creature.parent()) {
                for child in children {
                    if let Some(properties) = world.get::<ColliderMassProperties>(*child) {
                        total_weight += properties.mass;
                    }
                }
            }

            // You are not allowed to connect to yourself.
            if creature == free_creature.parent() {
                return;
            }
        } else if let Some(properties) = world.get::<ColliderMassProperties>(free_part) {
            total_weight += properties.mass;
        }

        if total_weight <= 0.0 {
            total_weight = 1.0;
        }

        // Collects the indexes of the sockets for adding the CreatureJoint component.
        let attached_local = world
            .get::<Transform>(self.attached_socket)
            .expect("Target socket has no transform")
            .translation
            .truncate();

        let free_local = world
            .get::<Transform>(self.free_socket)
            .expect("Target socket has no transform")
            .translation
            .truncate();

        let avian_joint = spawn_avian_joint(
            world,
            socket_type,
            self.attached_socket,
            attached_part,
            attached_local,
            self.free_socket,
            free_part,
            free_local,
            FREE_COMPLIANCE / total_weight,
        );

        if let Some(free_creature) = world.get::<ChildOf>(free_part) {
            let free_creature_entity = free_creature.parent();
            let children = world
                .get::<Children>(free_creature_entity)
                .unwrap()
                .to_vec();

            // Make joints and parts children of creature. (if the parts had a creature)
            world.entity_mut(creature).add_children(&children);
            world.despawn(free_creature_entity);
        } else {
            // Make joint and part children of creature.
            world
                .entity_mut(creature)
                .add_child(avian_joint)
                .add_child(free_part);
        }

        // Add the [`Connected`] to both sockets.
        world.entity_mut(self.attached_socket).insert(Connected {
            joint: avian_joint,
            socket: self.free_socket,
        });

        world.entity_mut(self.free_socket).insert(Connected {
            joint: avian_joint,
            socket: self.attached_socket,
        });

        world.trigger(CreatureModified {
            entity: attached_part,
            event: Reactable::Connect,
        });
        if world.get::<Player>(creature).is_some() {
            world.trigger(PlayerCreatureModified);
        }

        let start = world
            .get::<GlobalTransform>(self.attached_socket)
            .unwrap()
            .translation();
        let end = world
            .get::<GlobalTransform>(self.free_socket)
            .unwrap()
            .translation();

        let midpoint = start.midpoint(end);
        let difference = end.xy() - start.xy();

        world.spawn((
            Transform::from_translation(midpoint),
            AudioPlayer::new(world.resource::<SharedAudio>().get(AudioId::SocketConnect)),
            PlaybackSettings {
                spatial: true,
                speed: rand::rng().random_range(0.95..1.05),
                ..PlaybackSettings::DESPAWN
            },
        ));

        // Why 12.3? Because the socket colliders have a radius of 1.75,
        // and we want their distance squared plus some epsilon: (2.0 * 1.75)^2 + 0.05
        if difference.length_squared() <= 12.3 {
            world.trigger(SocketCollision {
                socket1: self.attached_socket,
                socket2: self.free_socket,
            });
        } else {
            let shared_materials = world.get_resource::<SharedMaterials>().unwrap();
            let shared_meshes = world.get_resource::<SharedMeshes>().unwrap();

            world.spawn((
                Mesh3d(shared_meshes.get(MeshId::ConnectionLine)),
                Transform::from_translation(midpoint)
                    .with_rotation(Quat::from_rotation_z(Vec2::Y.angle_to(difference))),
                MeshMaterial3d(shared_materials.get(MaterialId::ConnectionLine)),
                LineSegment {
                    start: self.attached_socket,
                    end: self.free_socket,
                },
            ));
        }
    }
}

/// Struct that implements [`Command`] to connect two sockets with an avian joint.
/// This is completely unchecked and doesn't change any parent relashipship.
/// It also doesn't check for [`Connected`], [`SocketKindId`] and only cares about `first_socket`'s  [`Creature`] grandparent
pub struct ConnectSocketsUnchecked {
    pub first_socket: Entity,
    pub second_socket: Entity,
}

/// Implements a custom command for [`ConnectSocketsUnchecked`] that connects the two sockets provided.
/// As long as the `first_socket` has a [`Creature`] and a [`SocketKindId`] this function should work.
/// Using this on already connected sockets or similar will work but will have unintended consequences.
///
/// [`CreatureModified`] or [`PlayerCreatureModified`] events are not triggered by this function.
///
/// # Examples
///
/// ```
/// commands.queue(ConnectSocketsUnchecked {
///     some_socket,
///     some_other_socket,
/// });
/// ```
impl Command for ConnectSocketsUnchecked {
    type Out = ();

    fn apply(self, world: &mut World) {
        let Some(socket_type) = world.get::<SocketKindId>(self.first_socket) else {
            return;
        };

        let Some(first_child_of) = world.get::<ChildOf>(self.first_socket) else {
            return;
        };
        let Some(second_child_of) = world.get::<ChildOf>(self.second_socket) else {
            return;
        };

        let first_part = first_child_of.parent();
        let second_part = second_child_of.parent();

        let Some(creature_child_of) = world.get::<ChildOf>(first_part) else {
            return;
        };

        let creature = creature_child_of.parent();

        let first_local = world
            .get::<Transform>(self.first_socket)
            .expect("Target socket has no transform")
            .translation
            .truncate();

        let second_local = world
            .get::<Transform>(self.second_socket)
            .expect("Target socket has no transform")
            .translation
            .truncate();

        // Spawn avian2d joint that will connect the sockets.
        let avian_joint = spawn_avian_joint(
            world,
            *socket_type,
            self.first_socket,
            first_part,
            first_local,
            self.second_socket,
            second_part,
            second_local,
            0.0,
        );

        // Make joint and part children of creature.
        world.entity_mut(creature).add_child(avian_joint);

        // Add the [`Connected`] to both sockets.
        world.entity_mut(self.first_socket).insert(Connected {
            joint: avian_joint,
            socket: self.second_socket,
        });

        world.entity_mut(self.second_socket).insert(Connected {
            joint: avian_joint,
            socket: self.first_socket,
        });
    }
}

/// [`Command`] that disconnects the socket provided with the socket its connected to.
/// If the socket isn't connected this function will return without logging or modifying anything.
///
/// Only one of the sockets connected has to be provided and the other one will be found automatically.
///
/// # Examples
///
/// ```
/// pub fn on_clicked_socket(
///     event: On<Pointer<Click>>,
///     mut commands: Commands,
/// ) {
///     commands.queue(DisconnectSocket {
///         connected_socket: event.event_target(),
///     });
/// }
/// ```
pub struct DisconnectSocket {
    pub connected_socket: Entity,
}
impl Command for DisconnectSocket {
    type Out = ();

    fn apply(self, world: &mut World) {
        let Some(connected) = world.get::<Connected>(self.connected_socket) else {
            return;
        };

        let joint = connected.joint;
        let other_socket = connected.socket;

        // Remove the `Connected` from both sockets.
        // Remove parent-child relationship from connected_socket.
        world
            .entity_mut(self.connected_socket)
            .remove::<Connected>();
        world.entity_mut(other_socket).remove::<Connected>();

        // Makes both sockets visibility inherited again
        *world
            .entity_mut(self.connected_socket)
            .get_mut::<Visibility>()
            .unwrap() = Visibility::Inherited;
        *world
            .entity_mut(other_socket)
            .get_mut::<Visibility>()
            .unwrap() = Visibility::Inherited;

        // Despawn the Avian joint.
        world.despawn(joint);

        let part1 = world
            .get::<ChildOf>(self.connected_socket)
            .unwrap()
            .parent();

        let part2 = world.get::<ChildOf>(other_socket).unwrap().parent();

        let larger_creature = world.get::<ChildOf>(part2).unwrap().parent();

        // Bundle all requirements into one tuple.
        let mut state: SystemState<(
            Query<&CreaturePartKindId>,
            Res<JointGraph>,
            Query<(Entity, &LineSegment)>,
        )> = SystemState::new(world);

        let (parts_query, joint_graph, line_query) = state.get(world).unwrap();

        // Gather all lines that were connected to this socket.
        let mut despawn_list = Vec::<Entity>::new();
        'outer: for (line_entities, segments) in line_query.contiguous_iter_inner().unwrap() {
            for (&line_entity, segment) in line_entities.iter().zip(segments) {
                if segment.start == self.connected_socket || segment.end == self.connected_socket {
                    despawn_list.push(line_entity);
                    break 'outer;
                }
            }
        }

        let traverse_creature_half = |part: Entity| {
            let mut connected_parts = Vec::from([part]);
            traverse_connected(part, &joint_graph, &mut |connected_part, _| {
                connected_parts.push(connected_part);
            });

            (
                parts_query
                    .iter_many_inner(&connected_parts)
                    .map(|part| part.metadata().0 as u32)
                    .sum::<u32>(),
                connected_parts,
            )
        };
        let (power1, connected1) = traverse_creature_half(part1);
        let (power2, connected2) = traverse_creature_half(part2);

        let (larger_head, smaller_head, smaller_creature_parts, smaller_power) = {
            if power1 < power2 {
                (part2, part1, connected1, power1)
            } else {
                (part1, part2, connected2, power2)
            }
        };

        let smaller_creature_creature = Creature {
            desired_movement: None,
            attack: Attack::None,
            dash: false,
            depth: world.get::<Creature>(larger_creature).unwrap().depth,
        };

        let smaller_creature = {
            let mut smaller_creature = world.spawn((
                smaller_creature_creature,
                Transform::from_xyz(0.0, 0.0, smaller_creature_creature.z_from_depth()),
            ));
            if smaller_power == 0 {
                // The creature must consist of just a [`crate::creature::part::heart::Heart`].
                smaller_creature.insert_if_new(npc::HeartOnly);
            } else {
                smaller_creature.insert_if_new(npc::PlayerFriendly);
            }
            smaller_creature.id()
        };

        for part in smaller_creature_parts {
            world.entity_mut(smaller_creature).add_child(part);
        }

        if let Some(creature_of_head) = world.get::<CreatureOfHead>(larger_creature) {
            let creature_with_head = world.get::<ChildOf>(creature_of_head.0).unwrap().parent();
            if creature_with_head == smaller_creature {
                world
                    .entity_mut(larger_head)
                    .add_one_related::<CreatureOfHead>(larger_creature);
            }

            world
                .entity_mut(smaller_head)
                .add_one_related::<CreatureOfHead>(smaller_creature);
        }

        world.trigger(CreatureModified {
            entity: larger_head,
            event: Reactable::Disconnect,
        });
        world.trigger(CreatureModified {
            entity: smaller_head,
            event: Reactable::Spawn,
        });

        let player_depth = world
            .query_filtered::<&Creature, With<Player>>()
            .single(world)
            .unwrap()
            .depth;

        world.trigger(RelativeDepthLayerChanged {
            affected_creature: smaller_creature,
            player_depth,
        });

        if world
            .get::<Player>(world.get::<ChildOf>(larger_head).unwrap().parent())
            .is_some()
        {
            world.trigger(PlayerCreatureModified);
        }

        // Despawn the lines we previously found were connected.
        for entity in despawn_list {
            world.despawn(entity);
        }
    }
}
