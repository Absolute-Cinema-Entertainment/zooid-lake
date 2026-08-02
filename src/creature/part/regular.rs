//! Implementations of regular creature parts.

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
#[require(CreaturePartKindId::TriBlob, CenterOfMass::ZERO)]
pub struct TriBlob;
impl CreaturePartKind for TriBlob {
    const POWER: u8 = 3;
    const MESH: MeshId = MeshId::PartTriBlob;
    const SHAPE: ShapeId = ShapeId::PartTriBlob;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[
        (vec2(0.5, 0.0), SocketKindId::Fixed),
        (vec2(-0.5, 0.0), SocketKindId::Attachment),
        (vec2(-0.25, 0.433_013), SocketKindId::Fixed),
        (vec2(0.25, -0.433_013), SocketKindId::Attachment),
        (vec2(-0.25, -0.433_013), SocketKindId::Fixed),
        (vec2(0.25, 0.433_013), SocketKindId::Attachment),
        (Vec2::ZERO, SocketKindId::Heart),
    ];
    const FORCE: f32 = 1.0;
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(CreaturePartKindId::SmallOval, CenterOfMass::ZERO)]
pub struct SmallOval;
impl CreaturePartKind for SmallOval {
    const POWER: u8 = 5;
    // TODO
    const MESH: MeshId = MeshId::PartSmallOval;
    const SHAPE: ShapeId = ShapeId::PartSmallOval;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[
        (vec2(0.0, 0.9), SocketKindId::Fixed),
        (vec2(0.0, -0.9), SocketKindId::Fixed),
        (vec2(0.55, 0.0), SocketKindId::Attachment),
        (vec2(-0.55, 0.0), SocketKindId::Attachment),
        (vec2(0.0, 0.325), SocketKindId::Heart),
        (vec2(0.0, -0.325), SocketKindId::Heart),
    ];
    const FORCE: f32 = 1.0;
}

/// Rhombus / diamond shaped part, medium sized.
#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(CreaturePartKindId::Diamond, CenterOfMass::ZERO)]
pub struct Diamond;
impl CreaturePartKind for Diamond {
    const POWER: u8 = 3;
    const MESH: MeshId = MeshId::PartDiamond;
    const SHAPE: ShapeId = ShapeId::PartDiamond;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[
        (vec2(0.0, 0.75), SocketKindId::Rotating),
        (vec2(0.0, -0.75), SocketKindId::Attachment),
        (vec2(-0.5, 0.0), SocketKindId::Attachment),
        (vec2(0.5, 0.0), SocketKindId::Attachment),
    ];
    const FORCE: f32 = 1.0;
}

/// Giant annulus / circular part, maybe health later?
#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(CreaturePartKindId::HugeBlob, CenterOfMass::ZERO)]
pub struct HugeBlob;
impl CreaturePartKind for HugeBlob {
    const POWER: u8 = 32;
    const MESH: MeshId = MeshId::PartHugeBlob;
    const SHAPE: ShapeId = ShapeId::PartHugeBlob;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[
        (vec2(2.5, 0.0), SocketKindId::Fixed), // Rotated by `2*PI * 0/8`.
        (vec2(1.767_763_9, 1.767_763_9), SocketKindId::Attachment), // Rotated by `2*PI * 1/8`.
        (vec2(0.0, 2.5), SocketKindId::Rotating), // Rotated by `2*PI * 2/8`.
        (vec2(-1.767_763_9, 1.767_763_9), SocketKindId::Attachment), // Rotated by `2*PI * 3/8`.
        (vec2(-2.5, 0.0), SocketKindId::Fixed), // Rotated by `2*PI * 4/8`.
        (vec2(-1.767_763_9, -1.767_763_9), SocketKindId::Attachment), // Rotated by `2*PI * 5/8`.
        (vec2(0.0, -2.5), SocketKindId::Rotating), // Rotated by `2*PI * 6/8`.
        (vec2(1.767_763_9, -1.767_763_9), SocketKindId::Attachment), // Rotated by `2*PI * 7/8`.
        (vec2(1.0, 0.0), SocketKindId::Heart), // Rotated by `2*PI * 0/5`.
        (vec2(0.309_015, 0.951_055), SocketKindId::Heart), // Rotated by `2*PI * 1/5`.
        (vec2(-0.809_015, 0.587_785), SocketKindId::Heart), // Rotated by `2*PI * 2/5`.
        (vec2(-0.809_015, -0.587_785), SocketKindId::Heart), // Rotated by `2*PI * 3/5`.
        (vec2(0.309_015, -0.951_055), SocketKindId::Heart), // Rotated by `2*PI * 4/5`.
    ];
    const FORCE: f32 = 4.0;
}
