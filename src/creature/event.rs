use std::iter::repeat;

use avian2d::{dynamics::solver::joint_graph::JointGraph, prelude::*};
use bevy::prelude::*;

use crate::{
    PhysicsLayers,
    creature::{
        Creature, CreatureInBackground, CreatureInactive, Player, max_depth_from_power,
        part::{
            CreaturePartKindId, PartInBackground, Reactable, Reactables, SocketIds,
            traverse_connected,
        },
    },
};

/// Event that gets triggered every time a new [`Reactable`] should be added to all parts in a creature.
///
/// The target entity is the part that ripple effects will be centered on.
#[derive(Clone, Copy, EntityEvent, Eq, Hash, PartialEq)]
pub(super) struct CreatureModified {
    pub entity: Entity,
    pub event: Reactable,
}
impl CreatureModified {
    /// Runs every time [`CreatureModified`] is triggered on a creature part.
    pub(super) fn on(
        event: On<Self>,
        mut parts: Query<&mut Reactables>,
        joint_graph: Res<JointGraph>,
    ) {
        let mut push_reactable = |part, depth| {
            let new_reactable = (depth as f32, event.event().event);
            let active_reactables = &mut parts.get_mut(part).unwrap().active;

            if let Some(overflowed_reactables) = active_reactables.try_push(new_reactable) {
                active_reactables.remove(0);
                active_reactables.push(overflowed_reactables);
            }
        };

        let origin = event.event_target();

        push_reactable(origin, 0);

        traverse_connected(origin, &joint_graph, &mut |part, depth| {
            push_reactable(part, depth.get());
        });
    }
}

/// Event gets triggered every time parts are added or removed from the player creature.
#[derive(Clone, Copy, Default, Eq, Event, Hash, PartialEq)]
pub(super) struct PlayerCreatureModified;
impl PlayerCreatureModified {
    /// Calculates the total power of all parts in the player creature and updates its [`Player::max_depth`] accordingly,
    /// if it should change.
    pub(super) fn on(
        _: On<Self>,
        parts: Query<&CreaturePartKindId, Without<RigidBodyDisabled>>,
        mut player: Single<(&mut Player, &Children)>,
    ) {
        let new_max_depth = max_depth_from_power(
            parts
                .iter_many_inner(player.1)
                .map(|part| part.metadata().0 as u32)
                .sum(),
        );

        player.0.set_if_neq(Player {
            max_depth: new_max_depth,
        });
    }
}

/// Event triggered on a creature when its depth level relative to the player has changed.
#[derive(Clone, Copy, EntityEvent, Eq, Hash, PartialEq)]
pub struct RelativeDepthLayerChanged {
    #[event_target]
    pub affected_creature: Entity,
    pub player_depth: u16,
}
impl RelativeDepthLayerChanged {
    pub fn on(
        event: On<Self>,
        mut commands: Commands,
        non_player_creatures: Query<(&Children, &Creature), Without<Player>>,
        parts: Query<(Entity, &SocketIds, &CreaturePartKindId)>,
    ) {
        /// Enum describing the depth layer that a creature is in relative to the current depth of the player.
        #[derive(Clone, Copy, Eq, Hash, PartialEq)]
        pub enum RelativeDepthLayer {
            Active,
            Background,
            Inactive,
        }
        impl RelativeDepthLayer {
            /// Returns [`Self`] of a creature at `depth` when the player is at `player_depth`.
            #[must_use]
            #[inline]
            pub const fn from_cmp(depth: u16, player_depth: u16) -> Self {
                if depth == player_depth {
                    Self::Active
                } else if depth == player_depth + 1 {
                    Self::Background
                } else {
                    Self::Inactive
                }
            }
        }

        let event_data = event.event();
        if let Ok(creature) = non_player_creatures.get(event_data.affected_creature) {
            let new_layer = RelativeDepthLayer::from_cmp(creature.1.depth, event_data.player_depth);

            if new_layer == RelativeDepthLayer::Active {
                parts
                    .iter_many_inner(creature.0)
                    .for_each(|(_, socket_ids, _)| {
                        socket_ids.0.iter().for_each(|&socket_id| {
                            commands.entity(socket_id).remove::<ColliderDisabled>();
                        });
                    });
            } else {
                parts
                    .iter_many_inner(creature.0)
                    .for_each(|(_, socket_ids, _)| {
                        commands.insert_batch_if_new(
                            socket_ids
                                .0
                                .clone()
                                .into_iter()
                                .zip(repeat(ColliderDisabled)),
                        );
                    });
            }

            if new_layer == RelativeDepthLayer::Background {
                commands
                    .entity(event_data.affected_creature)
                    .insert_if_new(CreatureInBackground);

                #[expect(clippy::unnecessary_to_owned, reason = "I don't know how to fix this")]
                commands.insert_batch(creature.0.to_vec().into_iter().zip(repeat((
                    PartInBackground,
                    CollisionLayers::new(PhysicsLayers::Background, PhysicsLayers::Background),
                ))));
            } else {
                commands
                    .entity(event_data.affected_creature)
                    .remove::<CreatureInBackground>();

                parts
                    .iter_many_inner(creature.0)
                    .for_each(|(part_id, _, part)| {
                        let physics_layer = part.metadata().4;

                        commands
                            .entity(part_id)
                            .remove::<PartInBackground>()
                            .insert(CollisionLayers::new(physics_layer, physics_layer));
                    });
            }

            let mut creature_commands = commands.entity(event_data.affected_creature);

            if new_layer == RelativeDepthLayer::Inactive {
                creature_commands.insert_if_new(CreatureInactive);

                #[expect(clippy::unnecessary_to_owned, reason = "I don't know how to fix this")]
                commands.insert_batch_if_new(
                    creature
                        .0
                        .to_vec()
                        .into_iter()
                        .zip(repeat((ColliderDisabled, RigidBodyDisabled))),
                );
            } else {
                creature_commands.remove::<CreatureInactive>();

                parts
                    .iter_many_inner(creature.0)
                    .for_each(|(part_id, _, _)| {
                        commands
                            .entity(part_id)
                            .remove::<(ColliderDisabled, RigidBodyDisabled)>();
                    });
            }
        }
    }
}
