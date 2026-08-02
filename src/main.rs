#![cfg_attr(not(feature = "debug"), windows_subsystem = "windows")]

use avian2d::prelude::*;
use bevy::{
    audio::{AudioPlugin, SpatialScale},
    picking::PickingSettings,
    prelude::*,
    window::{CompositeAlphaMode, CursorGrabMode, CursorOptions, ExitCondition, WindowTheme},
};

use crate::consts::{CREATURE_Z, WINDOW_TITLE_ROOT};

mod ambient_sound;
mod background;
mod building;
mod consts;
mod creature;
mod gui;
mod input;
mod log;
mod session;
mod shared_assets;

/// The current state of the game.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
enum GameState {
    #[default]
    Playing,
    Paused,
}

/// Layers used as both `memberships` and `filters` by every [`CollisionLayers`].
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, PhysicsLayer)]
enum PhysicsLayers {
    #[default]
    Default,
    Damage,
    Socket,
    Background,
}

/// Component containing a timer ticking in virtual time.
#[derive(Clone, Component, Eq, PartialEq)]
struct VirtualTimer(Timer);
impl VirtualTimer {
    /// Timer ticking.
    fn sys_tick(timers: Query<&mut Self>, time: Res<Time>) {
        timers.contiguous_iter_inner().unwrap().for_each(|timers| {
            timers.into_iter().for_each(|timer| {
                timer.0.tick(time.delta());
            });
        });
    }
}

/// Component containing a timer ticking in virtual time.
#[derive(Clone, Component, Eq, PartialEq)]
struct FixedTimer(Timer);
impl FixedTimer {
    /// Timer ticking.
    fn sys_tick(timers: Query<&mut Self>, time: Res<Time>) {
        timers.contiguous_iter_inner().unwrap().for_each(|timers| {
            timers.into_iter().for_each(|timer| {
                timer.0.tick(time.delta());
            });
        });
    }
}

fn main() -> AppExit {
    log::setup();
    App::new()
        .add_plugins((
            {
                let default_plugins = DefaultPlugins
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: format!("{WINDOW_TITLE_ROOT} (0)"),
                            name: Some("zooid-lake".to_owned()),
                            window_theme: Some(WindowTheme::Dark),
                            composite_alpha_mode: CompositeAlphaMode::Opaque,
                            fit_canvas_to_parent: true,
                            // prevent_default_event_handling: false, // TODO: Change attack to not require right click, then enable this.
                            #[cfg(target_family = "wasm")]
                            canvas: Some("#bevy".to_owned()),
                            #[cfg(not(target_family = "wasm"))]
                            mode: bevy::window::WindowMode::BorderlessFullscreen(
                                MonitorSelection::Primary,
                            ),
                            ..default()
                        }),
                        primary_cursor_options: Some(CursorOptions {
                            grab_mode: CursorGrabMode::Confined,
                            ..default()
                        }),
                        exit_condition: ExitCondition::OnPrimaryClosed,
                        ..default()
                    })
                    .set(AudioPlugin {
                        default_spatial_scale: SpatialScale(Vec2::splat(0.05).extend(0.01)),
                        ..default()
                    });

                cfg_select! {
                    any(feature = "debug", target_family = "wasm") => default_plugins,
                    _ => default_plugins.set(bevy::log::LogPlugin {
                        fmt_layer: log::fmt_layer_to_file,
                        ..default()
                    })
                }
            },
            PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()),
            PhysicsPickingPlugin,
            crate::shared_assets::SharedAssetPlugin,
            crate::creature::CreaturePlugin,
            crate::building::BuildingPlugin,
            crate::background::BackgroundPlugin,
            crate::input::InputPlugin,
            // crate::ambient_sound::AmbientSoundPlugin,
            crate::session::SessionPlugin,
            crate::gui::GuiPlugin,
        ))
        .add_systems(Update, VirtualTimer::sys_tick)
        .add_systems(FixedUpdate, FixedTimer::sys_tick)
        .init_state::<GameState>()
        .insert_resource(PickingSettings {
            is_enabled: false,
            is_window_picking_enabled: false,
            ..default()
        })
        .insert_resource(PhysicsPickingSettings {
            require_markers: true,
            z_plane: CREATURE_Z,
        })
        .insert_resource(Gravity::ZERO)
        .run()
}
