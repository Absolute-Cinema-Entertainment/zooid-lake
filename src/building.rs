//! Creature building-related functionality.
//!
//! Only sockets which are pickable by the player contain the [`PhysicsPickable`] component.

use avian2d::{picking::PhysicsPickable, prelude::*};

use bevy::{
    prelude::*,
    window::{CursorIcon, SystemCursorIcon},
};
use rand::RngExt;

use crate::{
    consts::ATTACHED_COMPLIANCE,
    creature::{
        Player,
        connect::{ConnectSockets, Connected, DisconnectSocket},
        generic_socket::SocketKindId,
        part::SocketIds,
    },
    input::Cursor,
    shared_assets::{AudioId, MaterialId, MeshId, SharedAudio, SharedMaterials, SharedMeshes},
};

#[derive(Clone, Copy, Eq, Event, Hash, PartialEq)]
pub struct SocketCollision {
    pub socket1: Entity,
    pub socket2: Entity,
}

/// Component given to the line connecting the cursor and socket while socket is being dragged.
#[derive(Clone, Component, Copy, Default, Eq, Hash, PartialEq)]
#[component(immutable)]
pub struct ConnectingLine;

/// Component given to all lines connecting two points.
#[derive(Clone, Component, Copy, Eq, Hash, PartialEq)]
#[component(immutable)]
pub struct LineSegment {
    pub start: Entity,
    pub end: Entity,
}

/// The state that determins if the player is building and connecting parts.
/// Both [`BuildingState::Dragging`] and [`BuildingState::Building`] disables player movement.
/// The entity on [`BuildingState::Dragging`] is the entity currently being dragged.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
pub enum BuildingState {
    #[default]
    Playing,
    Building,
    Dragging,
}

/// Plugin for everything that has to do with the building mode.
pub struct BuildingPlugin;
impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sys_line_watch)
            .init_state::<BuildingState>()
            .add_observer(on_drag_end)
            .add_observer(on_socket_collision);
    }
}

/// Runs in [`Update`] and adjusts the length and position of every [`LineSegment`] in the scene.
///
/// Each line is drawn from the global position of its [`LineSegment::start`] to the global position of its [`LineSegment::end`].
pub fn sys_line_watch(
    line_query: Query<(&mut Transform, &LineSegment)>,
    transform_query: Query<
        &GlobalTransform,
        Or<((With<SocketKindId>, With<PhysicsPickable>), With<Cursor>)>,
    >,
    player_transform: Single<&Transform, (With<Player>, Without<LineSegment>)>,
) {
    line_query
        .contiguous_iter_inner()
        .unwrap()
        .for_each(|(line_transforms, segments)| {
            line_transforms
                .into_iter()
                .zip(segments)
                .for_each(|(line_transform, segment)| {
                    if let Ok(start) = transform_query.get(segment.start)
                        && let Ok(end) = transform_query.get(segment.end)
                    {
                        let start = start.translation().xy();
                        let end = end.translation().xy();

                        let dir = end - start;
                        let length = dir.length();
                        let midpoint = (start + end) / 2.0;
                        let rotation = Quat::from_rotation_z(Vec2::Y.angle_to(end - start));

                        line_transform.translation =
                            midpoint.extend(player_transform.translation.z);
                        line_transform.scale.y = length;
                        line_transform.rotation = rotation;
                    }
                });
        });
}

pub fn on_enter(_: On<Pointer<Enter>>, mut cursor_icon: Single<&mut CursorIcon>) {
    **cursor_icon = CursorIcon::System(SystemCursorIcon::Grab);
}

pub fn on_leave(_: On<Pointer<Leave>>, mut cursor_icon: Single<&mut CursorIcon>) {
    **cursor_icon = CursorIcon::System(SystemCursorIcon::Default);
}

pub fn on_press(
    event: On<Pointer<Press>>,
    mut commands: Commands,
    children: Query<&ChildOf, Or<((With<SocketKindId>, With<PhysicsPickable>), With<SocketIds>)>>,
    player: Single<Entity, With<Player>>,
) {
    if let Ok(socket) = children.get(event.event_target())
        && let Ok(part) = children.get(socket.parent())
        && part.parent() == *player
    {
        commands.queue(DisconnectSocket {
            connected_socket: event.event_target(),
        });
    }
}

/// Activates whenever the cursor gets dragged starting from a sockets collider.
/// Turns [`BuildingState::Building`] to [`BuildingState::Dragging`].
pub fn on_drag_start(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<BuildingState>>,
    shared_audio: Res<SharedAudio>,
    shared_meshes: Res<SharedMeshes>,
    shared_materials: Res<SharedMaterials>,
    mut cursor_icon: Single<&mut CursorIcon>,
    cursor: Single<(&Transform, Entity), With<Cursor>>,
    sockets: Query<&GlobalTransform, (With<SocketKindId>, With<PhysicsPickable>)>,
) {
    next_state.set(BuildingState::Dragging);
    **cursor_icon = CursorIcon::System(SystemCursorIcon::Grabbing);

    if let Ok(socket) = sockets.get(event.event_target()) {
        let start = socket.translation().xy();
        let end = cursor.0.translation;

        let midpoint = Transform::from_translation(start.midpoint(end.xy()).extend(end.z));

        commands.spawn((
            DespawnOnExit(BuildingState::Dragging),
            Mesh3d(shared_meshes.get(MeshId::ConnectionLine)),
            midpoint.with_rotation(Quat::from_rotation_z(Vec2::Y.angle_to(end.xy() - start))),
            MeshMaterial3d(shared_materials.get(MaterialId::ConnectionLine)),
            ConnectingLine,
            LineSegment {
                start: event.event_target(),
                end: cursor.1,
            },
        ));

        commands.spawn((
            midpoint,
            AudioPlayer::new(shared_audio.get(AudioId::SocketDrag)),
            PlaybackSettings {
                spatial: true,
                speed: rand::rng().random_range(0.95..1.05),
                ..PlaybackSettings::DESPAWN
            },
        ));
    }
}

pub fn on_drag_enter(
    event: On<Pointer<DragEnter>>,
    mut commands: Commands,
    shared_audio: Res<SharedAudio>,
    sockets: Query<&GlobalTransform, (With<SocketKindId>, With<PhysicsPickable>)>,
) {
    if let Ok(target) = sockets.get(event.event_target())
        && let Ok(dragged) = sockets.get(event.dragged)
    {
        let start = dragged.translation();
        let end = target.translation().xy();

        let midpoint = Transform::from_translation(start.xy().midpoint(end).extend(start.z));

        commands.spawn((
            midpoint,
            AudioPlayer::new(shared_audio.get(AudioId::SocketHoverWithLine)),
            PlaybackSettings {
                spatial: true,
                speed: rand::rng().random_range(0.95..1.05),
                ..PlaybackSettings::DESPAWN
            },
        ));
    }
}

/// If the picked up socket is dropped on another socket it will try and connect them.
pub fn on_drag_drop(
    event: On<Pointer<DragDrop>>,
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    children: Query<&ChildOf, Or<((With<SocketKindId>, With<PhysicsPickable>), With<SocketIds>)>>,
) {
    if let Ok(target_socket) = children.get(event.event_target())
        && let Ok(target_part) = children.get(target_socket.parent())
        && let Ok(dropped_socket) = children.get(event.dropped)
        && let Ok(dropped_part) = children.get(dropped_socket.parent())
    {
        let (attached_socket, free_socket) = if target_part.parent() == *player {
            (event.event_target(), event.dropped)
        } else if dropped_part.parent() == *player {
            (event.dropped, event.event_target())
        } else {
            return;
        };

        commands.queue(ConnectSockets {
            attached_socket,
            free_socket,
        });
    }
}

/// No matter what the picked up socket is dropped on this will run.
///
/// Turns [`BuildingState::Dragging`] -> [`BuildingState::Building`].
/// Despawns the line connecting the cursor and the socket.
pub fn on_drag_end(
    _: On<Pointer<DragEnd>>,
    mut cursor_icon: Single<&mut CursorIcon>,
    mut next_state: ResMut<NextState<BuildingState>>,
) {
    **cursor_icon = CursorIcon::System(SystemCursorIcon::Default);
    next_state.set(BuildingState::Building);
}

/// Runs when two sockets collide with each other and triggers [`SocketCollision`] event.
pub fn on_collision_start(event: On<CollisionStart>, mut commands: Commands) {
    commands.trigger(SocketCollision {
        socket1: event.collider1,
        socket2: event.collider2,
    });
}

/// Runs every time [`SocketCollision`] is triggered and changes sockets from "free" to "attached".
/// If they are suppose to be connected it will change their point compliance to [`ATTACHED_COMPLIANCE`].
/// Hides one of the sockets and makes the other one [`Visibility::Inherited`].
/// This function gets run by both sockets so it's not determenistic which one becomes [`Visibility::Inherited`] and which one becomes [`Visibility::Hidden`].
pub fn on_socket_collision(
    event: On<SocketCollision>,
    mut commands: Commands,
    connected_query: Query<&Connected, With<PhysicsPickable>>,
    mut revolute_joint_query: Query<&mut RevoluteJoint>,
    mut fixed_joint_query: Query<&mut FixedJoint>,
    mut distance_joint_query: Query<&mut DistanceJoint>,
    mut prismatic_joint_query: Query<&mut PrismaticJoint>,
    mut visibility_query: Query<&mut Visibility, (With<SocketKindId>, With<PhysicsPickable>)>,
    line_query: Query<(Entity, &LineSegment)>,
) {
    if let Ok(joint_entity) = connected_query.get(event.socket1)
        && joint_entity.socket == event.socket2
    {
        if let Ok(mut revolute_joint) = revolute_joint_query.get_mut(joint_entity.joint) {
            revolute_joint.point_compliance = ATTACHED_COMPLIANCE;
        } else if let Ok(mut fixed_joint) = fixed_joint_query.get_mut(joint_entity.joint) {
            fixed_joint.point_compliance = ATTACHED_COMPLIANCE;
        } else if let Ok(mut distance_joint) = distance_joint_query.get_mut(joint_entity.joint) {
            distance_joint.compliance = ATTACHED_COMPLIANCE;
        } else if let Ok(mut prismatic_joint) = prismatic_joint_query.get_mut(joint_entity.joint) {
            prismatic_joint.limit_compliance = ATTACHED_COMPLIANCE;
        }

        *visibility_query.get_mut(event.socket1).unwrap() = Visibility::Hidden;
        *visibility_query.get_mut(event.socket2).unwrap() = Visibility::Inherited;

        line_query
            .contiguous_iter_inner()
            .unwrap()
            .for_each(|(line_entity, segment)| {
                line_entity
                    .iter()
                    .zip(segment)
                    .for_each(|(&line_entity, segment)| {
                        if segment.start == event.socket1 || segment.start == event.socket2 {
                            commands.entity(line_entity).despawn();
                        }
                    });
            });
    }
}
