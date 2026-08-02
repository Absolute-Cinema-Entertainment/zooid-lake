//! Implementations of all creature joints and sockets.

use bevy::prelude::*;

use crate::{
    creature::generic_socket::{SocketKind, SocketKindId},
    shared_assets::{MeshId, ShapeId},
};

/// Example Socket.
#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(SocketKindId::Fixed)]
pub(super) struct Fixed;
impl SocketKind for Fixed {
    const MESH: MeshId = MeshId::SocketRegular;
    const SHAPE: ShapeId = ShapeId::SocketNonHeart;
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(SocketKindId::Rotating)]
pub(super) struct Rotating;
impl SocketKind for Rotating {
    const MESH: MeshId = MeshId::SocketRotating;
    const SHAPE: ShapeId = ShapeId::SocketNonHeart;
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(SocketKindId::Attachment)]
pub(super) struct Attachment;
impl SocketKind for Attachment {
    const MESH: MeshId = MeshId::SocketAttachment;
    const SHAPE: ShapeId = ShapeId::SocketNonHeart;
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(SocketKindId::Heart)]
pub(super) struct Heart;
impl SocketKind for Heart {
    const MESH: MeshId = MeshId::SocketHeart;
    const SHAPE: ShapeId = ShapeId::SocketHeart;
}
