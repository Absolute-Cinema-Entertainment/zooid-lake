//! Implementation of heart creature parts.

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
#[require(CreaturePartKindId::Heart, CenterOfMass::ZERO)]
pub struct Heart;
impl CreaturePartKind for Heart {
    const POWER: u8 = 0; // This must be 0 to not allow a situation where a heart is the more powerful creature in a disconnection. No other part should have a power of 0.
    const MESH: MeshId = MeshId::PartHeart;
    const SHAPE: ShapeId = ShapeId::PartHeart;
    const LAYER: PhysicsLayers = PhysicsLayers::Damage;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[(vec2(0.0, 0.0), SocketKindId::Heart)];
    const FORCE: f32 = 0.5;
}
