use std::{
    f32::consts::{PI, TAU},
    iter::repeat_n,
    ops::Range,
    time::Duration,
};

use avian2d::{dynamics::rigid_body::RigidBodyDisabled, physics_transform::Position};
use bevy::{ecs::relationship::Relationship, math::FloatOrd, prelude::*};
use rand::{RngExt, distr::Uniform, rngs::SmallRng};
use strum::EnumCount;
use tinyvec::ArrayVec;

use crate::{
    VirtualTimer,
    consts::{FRIENDLY_FOLLOW_DIST, HEART_FOLLOW_DIST, HOSTILE_FOLLOW_DIST},
    creature::{
        Attack, Creature, CreatureInBackground, CreatureInactive, CreatureKindId, CreatureOfHead,
        Player,
        connect::Connected,
        generic_socket::SocketKindId,
        max_depth_from_power,
        part::CreaturePartData,
        part::{self, CreaturePartKindId, HeadOfCreature, PartInBackground, SocketIds},
        sockets,
    },
    shared_assets::{SharedMaterials, SharedMeshes, SharedShapes},
};

/// Component marking the singular entity controlling NPC spawning.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Resource)]
#[component(immutable)]
#[require(VirtualTimer(Timer::new(Duration::from_secs(5), TimerMode::Repeating)))]
pub struct NpcSpawner;
impl NpcSpawner {
    pub(super) fn cond_timer_finished(timer: Single<&VirtualTimer, With<Self>>) -> bool {
        timer.0.just_finished()
    }

    pub(super) fn sys_try_spawn(
        mut commands: Commands,
        shared_meshes: Res<SharedMeshes>,
        shared_materials: Res<SharedMaterials>,
        shared_shapes: Res<SharedShapes>,
        creatures: Query<
            (Entity, &CreatureOfHead, Has<CreatureInBackground>),
            (With<Creature>, Without<Player>, Without<CreatureInactive>),
        >,
        heads: Query<&Position, With<HeadOfCreature>>,
        player: Single<&Creature, With<Player>>,
        camera: Single<&Transform, With<Camera>>,
        mut timer: Single<&mut VirtualTimer, With<Self>>,
    ) {
        let enabled_creature_count = creatures.count();

        let mut can_spawn = enabled_creature_count < Self::MAX_CREATURES;
        if !can_spawn {
            let mut furthest = (0.0, None);

            creatures.contiguous_iter_inner().unwrap().for_each(
                |(entities, creature_heads, is_background)| {
                    entities
                        .iter()
                        .zip(creature_heads)
                        .for_each(|(&entity, creature_head)| {
                            let mut dist = heads
                                .get(creature_head.get())
                                .unwrap()
                                .distance_squared(camera.translation.xy());

                            if is_background {
                                // Treat background creatures as if they're closer,
                                // since they're visible at larger XY distances.
                                dist *= 0.1;
                            }

                            if dist > furthest.0 {
                                furthest = (dist, Some(entity));
                            }
                        });
                },
            );

            can_spawn = if let Some(furthest_creature) = furthest.1 {
                commands.entity(furthest_creature).try_despawn();
                true
            } else {
                false
            };
        }

        if can_spawn {
            let mut rng = rand::make_rng::<SmallRng>();

            timer
                .0
                .set_duration(Duration::from_secs(enabled_creature_count as u64));

            let full_circle_distr = Uniform::try_from(0.0..TAU).unwrap();
            let creature_rot = rng.sample(full_circle_distr);
            let mut creature_pos = Vec2::from_angle(rng.sample(full_circle_distr)) * Self::XY_DIST;
            let is_background = rng.random_bool(0.5);
            if is_background {
                creature_pos *= rng.random_range(0.0..4.0);
            }
            creature_pos += camera.translation.xy();

            let mut parts = Vec::with_capacity(1);
            let mut joints = Vec::with_capacity(1);

            let mut create_part = || {
                let kind =
                    CreaturePartKindId::from_repr(rng.random_range(1..CreaturePartKindId::COUNT))
                        .unwrap();
                let meta = kind.metadata();

                (
                    kind,
                    repeat_n(true, meta.3.len())
                        .collect::<ArrayVec<[bool; SocketIds::MAX.get() as usize]>>(), // Flags in the same order as [`crate::crature::part::CreaturePartKind::SOCKETS`], indicating which sockets are empty.
                    meta,
                )
            };

            // Add first part (head).
            let (head_kind, head_mask, head_meta) = create_part();
            parts.push((head_kind, head_mask, vec2(0.0, 0.0), creature_rot));
            let mut power = head_meta.0 as u32;

            while max_depth_from_power(power) < player.depth + Self::DIFFICULTY {
                let (kind, mut mask, meta) = create_part();
                let mut pos = Vec2::ZERO;
                let mut rot = 0.0;
                let sockets = meta.3;

                let parts_len = parts.len() as u16;

                let mut success = false;

                'try_add_part: for (other_part_i, (other_part, other_mask, other_pos, other_rot)) in
                    parts.iter_mut().enumerate()
                {
                    for (other_socket_i, (other_empty, &(other_socket_pos, other_socket_kind))) in
                        other_mask
                            .into_iter()
                            .zip(other_part.metadata().3)
                            .enumerate()
                    {
                        if *other_empty {
                            for (socket_i, &(socket_pos, socket_kind)) in sockets.iter().enumerate()
                            {
                                if socket_kind != SocketKindId::Heart
                                    && socket_kind == other_socket_kind
                                {
                                    // Other part has a free, compatible socket. Add and connect our new part.
                                    success = true;

                                    power += meta.0 as u32;

                                    joints.push((
                                        (parts_len, socket_i as u8),
                                        (other_part_i as u16, other_socket_i as u8),
                                    ));

                                    // TODO: Please, someone else look through this and make sure it's correct.
                                    rot = *other_rot + other_socket_pos.to_angle()
                                        - socket_pos.to_angle()
                                        + PI;
                                    pos = *other_pos
                                        + Vec2::from_angle(*other_rot).rotate(other_socket_pos)
                                        - Vec2::from_angle(rot).rotate(socket_pos);

                                    // Mark the connected sockets as occupied.
                                    mask[socket_i] = false;
                                    *other_empty = false;

                                    break 'try_add_part;
                                }
                            }
                        }
                    }
                }

                if success {
                    parts.push((kind, mask, pos, rot));
                } else {
                    break; // Give up and stop adding parts.
                }
            }

            // Add hearts to all empty heart sockets.
            let parts_len = parts.len() as u16;

            let mut heart_transforms = Vec::new();

            parts
                .iter_mut()
                .enumerate()
                .for_each(|(part_i, (kind, mask, pos, rot))| {
                    mask.into_iter()
                        .zip(kind.metadata().3)
                        .enumerate()
                        .for_each(|(socket_i, (socket_empty, &(socket_pos, socket_kind)))| {
                            if socket_kind == SocketKindId::Heart {
                                // The socket will never already be full, since we don't spawn hearts randomly.
                                *socket_empty = false;
                                joints.push((
                                    (part_i as u16, socket_i as u8),
                                    (parts_len + heart_transforms.len() as u16, 0),
                                ));

                                heart_transforms.push((
                                    *pos + Vec2::from_angle(*rot).rotate(socket_pos),
                                    *rot + PI,
                                ));
                            }
                        });
                });

            heart_transforms.into_iter().for_each(|(pos, rot)| {
                parts.push((CreaturePartKindId::Heart, ArrayVec::new(), pos, rot));
            }); // This mask is invalid but it won't be used.

            Creature::spawn(
                &mut commands,
                &shared_meshes,
                &shared_materials,
                &shared_shapes,
                parts
                    .into_iter()
                    .map(|(kind, _, pos, rot)| CreaturePartData {
                        kind,
                        pos: creature_pos + pos,
                        rot,
                    }),
                joints,
                0,
                player.depth + is_background as u16,
                player.depth,
                CreatureKindId::from_repr(rng.random_range(2..CreatureKindId::COUNT)).unwrap(), // Don't spawn [`CreatureKindId::Player`] or [`CreatureKindId::HeartOnly`].
            );
        }
    }
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[require(Creature, CreatureKindId::PlayerFriendly)]
#[component(immutable)]
pub(super) struct PlayerFriendly;
impl PlayerFriendly {
    pub(super) fn sys_control(
        creatures: Query<
            (&mut Creature, &CreatureOfHead, Has<CreatureInBackground>),
            (With<Self>, Without<CreatureInactive>),
        >,
        player: Single<&CreatureOfHead, (With<Player>, Without<Self>)>,
        heads: Query<
            &Position,
            (
                With<HeadOfCreature>,
                Without<PartInBackground>,
                Without<RigidBodyDisabled>,
            ),
        >,
        time: Res<Time>,
    ) {
        let delta = time.delta_secs();
        let mut rng = rand::make_rng::<SmallRng>();
        let player_pos = heads.get(player.get()).unwrap();

        creatures.contiguous_iter_inner().unwrap().for_each(
            |(creatures, creature_of_heads, is_background)| {
                creatures.into_iter().zip(creature_of_heads).for_each(
                    |(creature, creature_of_head)| {
                        let mut dir = creature.desired_movement.unwrap_or(Dir2::Y);

                        if !is_background {
                            const SQ_FOLLOW_DIST: Range<f32> = (FRIENDLY_FOLLOW_DIST.start
                                * FRIENDLY_FOLLOW_DIST.start)
                                ..(FRIENDLY_FOLLOW_DIST.end * FRIENDLY_FOLLOW_DIST.end);

                            let to_player =
                                player_pos.0 - heads.get(creature_of_head.get()).unwrap().0;

                            if SQ_FOLLOW_DIST.contains(&to_player.length_squared())
                                && let Ok(to_player) = Dir2::new(to_player)
                            {
                                dir.smooth_nudge(&to_player, 0.25, delta);
                            }
                        }

                        creature.desired_movement = Some(
                            Dir2::new(
                                dir.rotate(Vec2::from_angle(rng.random_range(-delta..delta) * 5.0)),
                            )
                            .unwrap(),
                        );
                    },
                );
            },
        );
    }
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[require(Creature, CreatureKindId::HeartOnly)]
#[component(immutable)]
pub(super) struct HeartOnly;
impl HeartOnly {
    pub(super) fn sys_control(
        creatures: Query<
            (&mut Creature, &CreatureOfHead, Has<CreatureInBackground>),
            (With<Self>, Without<CreatureInactive>),
        >,
        heads: Query<&Position, (With<HeadOfCreature>, Without<RigidBodyDisabled>)>,
        active_non_hearts: Query<
            (),
            (
                With<CreaturePartKindId>,
                Without<PartInBackground>,
                Without<RigidBodyDisabled>,
                Without<part::heart::Heart>,
            ),
        >,
        empty_heart_sockets: Query<
            (&Position, &ChildOf),
            (With<sockets::Heart>, Without<Connected>),
        >,
        time: Res<Time>,
    ) {
        let delta = time.delta_secs();
        let mut rng = rand::make_rng::<SmallRng>();

        creatures.contiguous_iter_inner().unwrap().for_each(
            |(creatures, creature_of_heads, is_background)| {
                creatures.into_iter().zip(creature_of_heads).for_each(
                    |(creature, creature_of_head)| {
                        let mut dir = creature.desired_movement.unwrap_or(Dir2::Y);

                        if !is_background {
                            const SQ_FOLLOW_DIST: Range<f32> = (HEART_FOLLOW_DIST.start
                                * HEART_FOLLOW_DIST.start)
                                ..(HEART_FOLLOW_DIST.end * HEART_FOLLOW_DIST.end);

                            let head = heads.get(creature_of_head.get()).unwrap().0;

                            let mut to_socket: Option<Vec2> = None;
                            empty_heart_sockets
                                .contiguous_iter_inner()
                                .unwrap()
                                .for_each(|(sockets, child_ofs)| {
                                    sockets
                                        .iter()
                                        .zip(child_ofs)
                                        .for_each(|(socket, child_of)| {
                                            // Filter out parts from the non-active depth layers.
                                            if active_non_hearts.contains(child_of.parent()) {
                                                let new_to_socket = socket.0 - head;

                                                if to_socket.is_none_or(|to_socket| {
                                                    to_socket.length_squared()
                                                        > new_to_socket.length_squared()
                                                }) {
                                                    to_socket = Some(new_to_socket);
                                                }
                                            }
                                        });
                                });

                            if let Some(to_socket) = to_socket
                                && SQ_FOLLOW_DIST.contains(&to_socket.length_squared())
                                && let Ok(to_socket) = Dir2::new(to_socket)
                            {
                                dir.smooth_nudge(&to_socket, 2.0, delta);
                            }
                        }

                        creature.desired_movement =
                            Some(
                                Dir2::new(dir.rotate(Vec2::from_angle(
                                    rng.random_range(-delta..delta) * 10.0,
                                )))
                                .unwrap(),
                            );
                    },
                );
            },
        );
    }
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[require(Creature, CreatureKindId::Wandering)]
#[component(immutable)]
pub(super) struct Wandering;
impl Wandering {
    pub(super) fn sys_control(
        creatures: Query<&mut Creature, (With<Self>, Without<CreatureInactive>)>,
        time: Res<Time>,
    ) {
        let delta = time.delta_secs();
        let mut rng = rand::make_rng::<SmallRng>();

        creatures
            .contiguous_iter_inner()
            .unwrap()
            .for_each(|creatures| {
                creatures.into_iter().for_each(|creature| {
                    creature.desired_movement = Some(
                        Dir2::new(
                            creature
                                .desired_movement
                                .unwrap_or(Dir2::Y)
                                .rotate(Vec2::from_angle(rng.random_range(-delta..delta) * 20.0)),
                        )
                        .unwrap(),
                    );
                });
            });
    }
}

#[derive(Clone, Component, Copy, Default, Eq, PartialEq, Hash)]
#[require(Creature, CreatureKindId::Hostile)]
#[component(immutable)]
pub(super) struct Hostile;
impl Hostile {
    pub(super) fn sys_control(
        creatures: Query<
            (
                Entity,
                &mut Creature,
                &CreatureOfHead,
                Has<CreatureInBackground>,
            ),
            (With<Self>, Without<CreatureInactive>),
        >,
        hearts: Query<
            (&Position, &ChildOf),
            (
                With<part::heart::Heart>,
                Without<PartInBackground>,
                Without<RigidBodyDisabled>,
            ),
        >,
        heads: Query<
            &Position,
            (
                With<HeadOfCreature>,
                Without<PartInBackground>,
                Without<RigidBodyDisabled>,
            ),
        >,
        time: Res<Time>,
    ) {
        let delta = time.delta_secs();
        let mut rng = rand::make_rng::<SmallRng>();

        creatures.contiguous_iter_inner().unwrap().for_each(
            |(entities, creatures, creature_of_heads, is_background)| {
                entities
                    .iter()
                    .zip(creatures)
                    .zip(creature_of_heads)
                    .for_each(|((&entity, creature), creature_of_head)| {
                        let mut dir = creature.desired_movement.unwrap_or(Dir2::Y);

                        if is_background {
                            creature.dash = false;
                        } else {
                            const SQ_FOLLOW_DIST: Range<f32> = (HOSTILE_FOLLOW_DIST.start
                                * HOSTILE_FOLLOW_DIST.start)
                                ..(HOSTILE_FOLLOW_DIST.end * HOSTILE_FOLLOW_DIST.end);

                            let head = heads.get(creature_of_head.get()).unwrap().0;

                            let heart = hearts
                                .iter()
                                .filter_map(|(pos, child_of)| {
                                    if child_of.parent() == entity {
                                        None
                                    } else {
                                        let to_heart = pos.0 - head;
                                        Some((pos.0, to_heart, to_heart.length_squared()))
                                    }
                                })
                                .min_by_key(|(_, _, heart_dist)| FloatOrd(*heart_dist));

                            creature.dash = if let Some((_, to_heart, heart_dist)) = heart
                                && SQ_FOLLOW_DIST.contains(&heart_dist)
                                && let Ok(to_heart) = Dir2::new(to_heart)
                            {
                                dir.smooth_nudge(&to_heart, 1.0, delta);
                                true
                            } else {
                                false
                            };

                            creature.attack = if let Some((heart_pos, _, heart_dist)) = heart
                                && heart_dist < SQ_FOLLOW_DIST.end
                            {
                                Attack::WithTarget(heart_pos)
                            } else {
                                Attack::None
                            };
                        }

                        creature.desired_movement =
                            Some(
                                Dir2::new(dir.rotate(Vec2::from_angle(
                                    rng.random_range(-delta..delta) * 10.0,
                                )))
                                .unwrap(),
                            );
                    });
            },
        );
    }
}
