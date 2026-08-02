//! Global constants.

use std::{
    num::NonZero,
    ops::{Range, RangeInclusive},
};

use bevy::prelude::*;

use crate::{
    ambient_sound::MusicPlayer,
    background::{MaterialHandles, Particle},
    creature::{
        npc::NpcSpawner,
        part::{Reactable, SocketIds},
    },
    input::Cursor,
    session::Autosave,
};

// User input keyboard/mouse button bindings.
pub const NORTH: [KeyCode; 2] = [KeyCode::KeyW, KeyCode::ArrowUp];
pub const WEST: [KeyCode; 2] = [KeyCode::KeyA, KeyCode::ArrowLeft];
pub const SOUTH: [KeyCode; 2] = [KeyCode::KeyS, KeyCode::ArrowDown];
pub const EAST: [KeyCode; 2] = [KeyCode::KeyD, KeyCode::ArrowRight];

pub const DASH: MouseButton = MouseButton::Left;
pub const ATTACK_AIMED: MouseButton = MouseButton::Right;
pub const ATTACK_NOT_AIMED: MouseButton = MouseButton::Middle;

pub const UP: KeyCode = KeyCode::KeyQ;
pub const DOWN: KeyCode = KeyCode::KeyE;

pub const TOGGLE_BUILD: KeyCode = KeyCode::Space;
pub const TOGGLE_PAUSE: KeyCode = KeyCode::Escape;
pub const EXIT_PAUSE: KeyCode = KeyCode::Enter;

#[cfg(not(target_family = "wasm"))]
pub const TOGGLE_FULLSCREEN: KeyCode = KeyCode::F11;

pub const ATTACHED_COMPLIANCE: f32 = 0.0001;
pub const FREE_COMPLIANCE: f32 = 0.1;

pub const PART_ANGULAR_DAMPING: f32 = 1.5;

/// The [`StandardMaterial::ior`] of the environment,
/// applied by dividing the IOR of other materials (below),
/// since the actual IOR of the environment isn't configurable.
const ENV_IOR: f32 = 1.33;

/// The [`StandardMaterial::ior`] of background particles.
pub const PARTICLE_IOR: f32 = 1.0 / ENV_IOR;

/// The [`StandardMaterial::ior`] of creature parts.
pub const PART_IOR: f32 = 1.38 / ENV_IOR;

/// The [`StandardMaterial::ior`] of GUI elements.
pub const GUI_IOR: f32 = 1.52 / ENV_IOR;

/// The [`StandardMaterial::base_color`] of creature parts.
pub const PART_BASE_COLOR: Color = Color::WHITE;

/// Maximum environment/ambient light brightness in cd/m^2 (nits).
pub const ENV_LIGHT: f32 = 5000.0;

/// Attenuation of enviroment/ambient light over depth.
pub const ENV_OPTICAL_DENSITY: f32 = 0.0025;

/// Brightness of environment/water color in relation to ambient light.
pub const ENV_BRIGHTNESS: f32 = 0.000_05;

/// [Lightness](https://oklch.com/) of environment/water tint.
pub const ENV_LIGHTNESS: f32 = 0.915;

/// [Chroma](https://oklch.com/) of environment/water tint.
pub const ENV_CHROMA: f32 = 0.04;

/// Average [hue](https://oklch.com/) of environment/water tint.
pub const ENV_BASE_HUE: f32 = 235.0;

/// Maximum positive or negative shift in environment/water tint [hue](https://oklch.com/) in half degrees (pi/360 radians).
pub const ENV_HUE_SHIFT: f32 = 90.0;

/// Scale in world space XY coordinates of environment/water [hue](https://oklch.com/) shift noise.
pub const ENV_HUE_SHIFT_SCALE: f32 = 64.0;

/// Camera far plane distance on the world space Z axis. Nothing beyond this is visible.
pub const RENDER_DEPTH: f32 = 128.0;

/// Camera-relative Z world space coordinate of background particles closest to the camera.
pub const BACKGROUND_Z: f32 = -30.0;

/// Camera-relative Z world space coordinate of the cursor entity,
/// not affected by [`CAMERA_PAUSED_Z`].
pub const CURSOR_Z: f32 = -32.0;

/// Camera-relative Z world space coordinate of active creatures,
/// not affected by [`CAMERA_PAUSED_Z`].
pub const CREATURE_Z: f32 = -33.0;

/// Camera-relative Z world space coordinate of GUI elements,
/// not affected by [`CAMERA_PAUSED_Z`].
pub const GUI_Z: f32 = 32.0;

/// World space Z coordinate offset of the camera when the game is paused,
/// compared to its position otherwise.
pub const CAMERA_PAUSED_Z: f32 = 64.0;

/// Distance on the world space Z axis between depth levels.
pub const DEPTH_LEVEL_STEP: f32 = 256.0;

/// Speed of the creature transition animation between player depth levels.
pub const DEPTH_LEVEL_SPEED: f32 = 0.75;

/// Creature power value required per increased depth level.
pub const DEPTH_LEVEL_POWER: NonZero<u32> = NonZero::new(32).unwrap();

/// Speed of camera movement, including the transition animation between Z levels.
pub const CAMERA_MOVEMENT_SPEED: f32 = 2.0;

/// Camera vertical field of view in radians.
pub const CAMERA_FOV: f32 = std::f32::consts::FRAC_PI_6;

/// Speed of GUI animations.
pub const GUI_ANIM_SPEED: f32 = 16.0;

impl MaterialHandles {
    /// The number of materials usable for particles, or the amount of possible steps in alpha.
    /// Higher values decrease performance.
    pub const LEN: u16 = 128;

    /// Alpha multiplier applied to all particle materials.
    pub const ALPHA_MUL: f32 = 0.8;
}

impl Particle {
    pub const COLOR: LinearRgba = LinearRgba::rgb(0.7, 0.85, 0.9);

    /// Maximum camera-relative world space XY coordinate distance of particle spawning.
    pub const XY_DIST: f32 = 32.0;

    /// Camera-relative world space Z coordinate range of particle spawning.
    pub const Z_RANGE: RangeInclusive<f32> = -(RENDER_DEPTH * 0.95)..=BACKGROUND_Z;

    /// Scale of random jitter applied to particle positions every frame.
    pub const JITTER: f32 = 0.8;

    /// Chebyshev distance in NDC space XY coordinates from the camera beyond which particles respawn immediately.
    pub const DESPAWN_DIST: f32 = 48.0;

    /// The number of annulus-shaped particles in the background.
    pub const ANNULUS_COUNT: u16 = 256;

    /// The number of circle-shaped particles in the background.
    pub const CIRCLE_COUNT: u16 = 1536;
}

impl Cursor {
    /// Intensity in lumens of the point light attached to the cursor when the player is clicking.
    pub const CLICK_INTENSITY: f32 = 262_144.0;

    /// Intensity in lumens of the point light attached to the cursor when the player is not clicking.
    pub const DEFAULT_INTENSITY: f32 = 65_536.0;

    /// Speed of intensity transition.
    pub const INTENSITY_TRANSITION_SPEED: f32 = 4.0;
}

/// Multiplier applied to creature part force when dashing.
pub const DASH_MUL: f32 = 12.0;

/// Multiplier applied to the value determining how much creature parts should prioritize turning over moving a creature.
pub const TURN_SHARPNESS: f32 = 0.5;

/// World space XY distance range within which friendly creatures follow the player.
pub const FRIENDLY_FOLLOW_DIST: Range<f32> = 32.0..128.0;

/// World space XY distance range within which hostile creatures hunt hearts.
pub const HOSTILE_FOLLOW_DIST: Range<f32> = 4.0..48.0;

/// World space XY distance range within which heart-only creatures follow empty heart sockets.
pub const HEART_FOLLOW_DIST: Range<f32> = 0.0..16.0;

/// Max distance in joint connections between any creature part and a heart within which the part stays alive/able to be connected.
pub const MAX_HEART_DIST: NonZero<u16> = NonZero::new(3).unwrap();

/// The root string of the window title.
pub const WINDOW_TITLE_ROOT: &str = "Zooid Lake";

/// String appended to the window title used when the game is paused.
pub const WINDOW_TITLE_PAUSED: &str = " - Paused";

/// Log file name used when not logging to standard IO.
#[cfg(not(any(feature = "debug", target_family = "wasm")))]
pub const LOG_NAME: &str = "latest.log";

/// Message ending the log file when a panic has occured with the `debug` feature disabled.
#[cfg(not(feature = "debug"))]
pub const PANIC_FOOTER: &str = "\nTo report this problem, please open an issue and upload this log file at:\nhttps://github.com/Absolute-Cinema-Entertainment/zooid-lake/issues";

impl NpcSpawner {
    /// Offset in the allowed max depth of NPCs spawning compared to the current depth of the player.
    ///
    /// Higher values cause more powerful NPCs to spawn.
    pub const DIFFICULTY: u16 = 0;

    /// Camera-relative world space XY coordinate distance of NPC spawning in the active layer.
    pub const XY_DIST: f32 = 32.0;

    /// Maximum number of creatures in the active depth layer, excluding the player,
    /// at which NPC spawning stops entirely.
    pub const MAX_CREATURES: usize = 32;
}

impl Reactable {
    /// Time divided by [`Self::SPEED`] after a reaction happening (`reaction.0 = 0.0`) within which it is visible.
    ///
    /// See [intensity curve of the visible effect](https://www.desmos.com/calculator/qanbadqvdp).
    pub const FALLOFF: f32 = 5.0;

    /// Speed of time in reaction animations and delay.
    pub const SPEED: f32 = 5.0;
}

impl SocketIds {
    /// The maximum number of sockets that a single part can have.
    pub const MAX: NonZero<u8> = NonZero::new(13).unwrap();
}

impl MusicPlayer {
    /// File paths inside the assets folder of music/soundtracks (non-overlapping ambient sounds).
    pub const TRACKS: [&str; 0] = [];

    /// Range of the randomly chosen time in seconds between every time the music player tries to start playing.
    ///
    /// If music is already playing when the timer inside the music player finishes, the timer resets with the same duration. Otherwise,
    /// a new duration is chosen within this range and a randomly chosen soundtrack from [`Self::TRACKS`] is played.
    pub const SILENCE: Range<u8> = 90..120;
}

/// File paths inside the assets folder of possibly overlapping ambient sounds.
pub const AMBIENT_TRACKS: [&str; 0] = [];

/// Range of the randomly chosen time in seconds between every time an ambient sound is played.
pub const AMBIENT_SILENCE: Range<u8> = 2..30;

/// Directory where the game sessions are being saved in.
#[cfg(not(any(target_family = "wasm")))]
pub const SAVE_DIRECTORY_PATH: &str = "sessions";

/// Saved binary files' file extension.
#[cfg(not(any(target_family = "wasm")))]
pub const SAVE_FILE_EXTENSION: &str = "zlsave";

impl Autosave {
    /// Autosave period in seconds.
    pub const DELAY: f32 = 30.0;
}
