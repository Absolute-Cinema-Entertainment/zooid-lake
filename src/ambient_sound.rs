//! Ambient sound & music playback.
//!
//! Two entities persist throughout the entire lifetime of the app,
//! both with [`GenericTimer`],
//! one with [`MusicPlayer`] and one without.
//!
//! The entity with [`MusicPlayer`] holds the timer for music playback,
//! and contains the audio playback components controlling the music,
//! while the entity without [`MusicPlayer`] spawns other entities with the audio playback components.
//! This is because non-music ambient sounds should be able to overlap,
//! while music should not.

#![allow(
    unused,
    reason = "We can't enable the plugin before the required assets are in place"
)]

use std::{path::Path, time::Duration};

use bevy::prelude::*;
use rand::prelude::*;

use crate::{
    VirtualTimer,
    consts::{AMBIENT_SILENCE, AMBIENT_TRACKS},
};

/// Plugin handling ambient sound & music playback.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub struct AmbientSoundPlugin;
impl Plugin for AmbientSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (MusicPlayer::sys_play, NonMusicPlayer::sys_play))
            .init_resource::<MusicPlayer>()
            .init_resource::<NonMusicPlayer>();
    }
}

/// Component marking the singular entity controlling music ambient sound playback.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Resource)]
#[component(immutable)]
#[require(VirtualTimer(Timer::new(
    Duration::from_secs(rand::rng().random_range(MusicPlayer::SILENCE) as u64),
    TimerMode::Repeating,
)))]
pub struct MusicPlayer;
impl MusicPlayer {
    /// Music playback.
    fn sys_play(
        mut commands: Commands,
        mut player: Single<(Entity, &mut VirtualTimer), (With<Self>, Without<AudioSink>)>, // This filter will only let the system run when music isn't already playing.
        asset_server: Res<AssetServer>,
    ) {
        if player.1.0.just_finished() {
            let mut rng = rand::rng();

            // Change the duration of the timer to a new random value.
            //
            // If the timer resets but this doesn't run (because music is already playing),
            // it will loop with the same duration again.
            player
                .1
                .0
                .set_duration(Duration::from_secs(rng.random_range(Self::SILENCE) as u64));

            commands.entity(player.0).insert((
                AudioPlayer::new(asset_server.load(Path::new(
                    Self::TRACKS[rng.random_range(0..Self::TRACKS.len())],
                ))),
                PlaybackSettings::REMOVE,
            ));
        }
    }
}

/// Component marking the singular entity controlling non-music ambient sound playback.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Resource)]
#[component(immutable)]
#[require(VirtualTimer(Timer::new(
    Duration::from_secs(rand::rng().random_range(AMBIENT_SILENCE) as u64),
    TimerMode::Repeating,
)))]
pub struct NonMusicPlayer;
impl NonMusicPlayer {
    /// Non-music ambient sound playback.
    fn sys_play(
        mut commands: Commands,
        mut timer: Single<&mut VirtualTimer, With<Self>>,
        asset_server: Res<AssetServer>,
    ) {
        if timer.0.just_finished() {
            let mut rng = rand::rng();

            timer
                .0
                .set_duration(Duration::from_secs(rng.random_range(AMBIENT_SILENCE) as u64));

            commands.spawn((
                AudioPlayer::new(asset_server.load(Path::new(
                    AMBIENT_TRACKS[rng.random_range(0..AMBIENT_TRACKS.len())],
                ))),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

/*
    // TODO: This might be better instead of using a custom timer when we get delayed commands in Bevy 0.19 (replacing the commands below with randomly delayed ones).

    impl MusicPlayer {
        /// Returns a bundle that can be inserted next to the [`MusicPlayer`] to start a random track.
        #[must_use]
        fn play_bundle(
            asset_server: Res<AssetServer>,
            mut rng: ThreadRng,
        ) -> (AudioPlayer, PlaybackSettings) {
            (
                AudioPlayer::new(asset_server.load(Path::new(Self::TRACKS[rng.random_range(0..Self::TRACKS.len())]))),
                PlaybackSettings::REMOVE,
            )
        }
    }

    fn sys_obs_music(
        trigger: On<Remove, AudioSink>,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
    ) {
        commands
            .entity(trigger.entity)
            .insert(MusicPlayer::play_bundle(&asset_server));
    }

    fn sys_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands
            .spawn((MusicPlayer, MusicPlayer::play_bundle(&asset_server)))
            .observe(sys_obs_music);
    }
*/
