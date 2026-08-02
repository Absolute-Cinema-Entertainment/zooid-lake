//! Implementations of weapon creature parts.
use std::time::Duration;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    FixedTimer, PhysicsLayers,
    creature::{
        Attack, Creature, CreatureInactive,
        generic_socket::SocketKindId,
        part::{CreaturePartKind, CreaturePartKindId},
        projectile::{self, Needle},
    },
    shared_assets::{MeshId, ShapeId, SharedMaterials, SharedMeshes, SharedShapes},
};

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[component(immutable)]
#[require(
    CreaturePartKindId::Spear,
    CenterOfMass::ZERO,
    FixedTimer(Timer::new(Duration::from_secs(2), TimerMode::Once,))
)]
pub struct Spear;
impl CreaturePartKind for Spear {
    const POWER: u8 = 8;
    const MESH: MeshId = MeshId::PartSpear;
    const SHAPE: ShapeId = ShapeId::PartSpear;
    const LAYER: PhysicsLayers = PhysicsLayers::Default;
    const SOCKETS: &[(Vec2, SocketKindId)] = &[(vec2(-0.5, 0.0), SocketKindId::Fixed)];
    const FORCE: f32 = 0.1;
}
impl Spear {
    pub(in super::super) fn sys_nervous(
        mut commands: Commands,
        spears: Query<
            (Forces, &mut FixedTimer, &ChildOf),
            (With<Self>, Without<RigidBodyDisabled>),
        >,
        creatures: Query<(Entity, &Creature), Without<CreatureInactive>>,
        shared_meshes: Res<SharedMeshes>,
        shared_materials: Res<SharedMaterials>,
        shared_shapes: Res<SharedShapes>,
    ) {
        spears
            .iter_inner()
            .for_each(|(mut forces, mut timer, child_of)| {
                let creature = creatures.get(child_of.parent()).unwrap();

                if creature.1.attack != Attack::None && timer.0.is_finished() {
                    timer.0.reset();

                    let rot = *forces.rotation();
                    let launch_dir = vec2(rot.cos, rot.sin);

                    commands
                        .entity(creature.0)
                        .with_child((
                            projectile::bundle::<Needle>(
                                *forces.position(),
                                rot,
                                &shared_meshes,
                                &shared_materials,
                                &shared_shapes,
                            ),
                            LinearVelocity(forces.linear_velocity() + launch_dir * 32.0),
                        ))
                        .observe(Needle::on_collision_start);

                    forces.apply_force(-launch_dir * 512.0);
                }
            });
    }
}
