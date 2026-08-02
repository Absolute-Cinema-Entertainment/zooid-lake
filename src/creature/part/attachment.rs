//! Implementations of attachment creature parts.
use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    PhysicsLayers,
    creature::{
        generic_socket::SocketKindId,
        part::{CreaturePartKind, CreaturePartKindId},
    },
    shared_assets::{MeshId, ShapeId},
};

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(CreaturePartKindId::Leg, CenterOfMass::new(0.0, 0.5))]
pub struct Leg;
impl CreaturePartKind for Leg {
    const POWER: u8 = 1;
    const MESH: MeshId = MeshId::PartLeg;
    const SHAPE: ShapeId = ShapeId::PartLeg;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[(vec2(0.0, 0.75), SocketKindId::Attachment)];
    const FORCE: f32 = 0.5;
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(CreaturePartKindId::Rectangle, CenterOfMass::ZERO)]
pub struct Rectangle;
impl CreaturePartKind for Rectangle {
    const POWER: u8 = 1;
    const MESH: MeshId = MeshId::PartSquare;
    const SHAPE: ShapeId = ShapeId::PartSquare;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[(vec2(0.2, 0.2), SocketKindId::Attachment)];
    const FORCE: f32 = 0.2;
}
