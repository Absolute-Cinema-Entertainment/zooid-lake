//! User input handling.

use std::hash::{BuildHasher, Hash};

use avian2d::{physics_transform::Position, picking::PhysicsPickable};
use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    ecs::relationship::Relationship,
    input::common_conditions::input_just_pressed,
    light::cluster::ClusterConfig,
    math::{I64Vec2, U8Vec2, u8vec2},
    pbr::{ScreenSpaceTransmission, ScreenSpaceTransmissionQuality},
    picking::PickingSettings,
    platform::hash::FixedHasher,
    post_process::bloom::Bloom,
    prelude::*,
    window::{CursorGrabMode, CursorIcon, CursorOptions, SystemCursorIcon},
};

use crate::{
    GameState, VirtualTimer,
    background::Particle,
    building::BuildingState,
    consts::{
        ATTACK_AIMED, ATTACK_NOT_AIMED, CAMERA_FOV, CAMERA_MOVEMENT_SPEED, CAMERA_PAUSED_Z,
        CREATURE_Z, CURSOR_Z, DASH, DOWN, EAST, ENV_BASE_HUE, ENV_BRIGHTNESS, ENV_CHROMA,
        ENV_HUE_SHIFT, ENV_HUE_SHIFT_SCALE, ENV_LIGHT, ENV_LIGHTNESS, ENV_OPTICAL_DENSITY,
        EXIT_PAUSE, NORTH, RENDER_DEPTH, SOUTH, TOGGLE_BUILD, TOGGLE_PAUSE, UP, WEST,
        WINDOW_TITLE_PAUSED,
    },
    creature::{
        Attack, Creature, CreatureOfHead, Player, event::RelativeDepthLayerChanged,
        part::HeadOfCreature,
    },
    session,
};

/// Plugin handling user input.
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, sys_startup)
            .add_systems(
                Update,
                (
                    (
                        sys_cursor_camera,
                        sys_creature_control.run_if(in_state(GameState::Playing)),
                    )
                        .chain(),
                    sys_click.run_if(in_state(GameState::Playing)),
                    sys_toggle_build.run_if(input_just_pressed(TOGGLE_BUILD)),
                    sys_toggle_pause.run_if(input_just_pressed(TOGGLE_PAUSE).or_else(
                        in_state(GameState::Paused).and_then(input_just_pressed(EXIT_PAUSE)),
                    )),
                    #[cfg(not(target_family = "wasm"))]
                    sys_toggle_fullscreen
                        .run_if(input_just_pressed(crate::consts::TOGGLE_FULLSCREEN)),
                ),
            )
            .init_resource::<Cursor>();
    }
}

/// User input control of the player creature.
fn sys_creature_control(
    mut commands: Commands,
    mut player: Single<(&mut Creature, &CreatureOfHead, &Player)>,
    non_player_creatures: Query<Entity, (With<Creature>, Without<Player>)>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Single<&Transform, With<Cursor>>,
    heads: Query<&Position, With<HeadOfCreature>>,
    building_state: Res<State<BuildingState>>,
) {
    let mut keyboard_movement = Vec2::ZERO;

    if keys.any_pressed(NORTH) {
        keyboard_movement.y += 1.0;
    }
    if keys.any_pressed(WEST) {
        keyboard_movement.x -= 1.0;
    }
    if keys.any_pressed(SOUTH) {
        keyboard_movement.y -= 1.0;
    }
    if keys.any_pressed(EAST) {
        keyboard_movement.x += 1.0;
    }

    // Use keyboard movement input if it exists,
    // and fall back to direction from head to cursor.
    player.0.desired_movement = Dir2::new(keyboard_movement).ok().or_else(|| {
        if *building_state.get() == BuildingState::Playing {
            let diff = cursor.translation.xy() - heads.get(player.1.get()).unwrap().0;
            if diff.length() > 1.0 {
                Dir2::new(diff).ok()
            } else {
                None
            }
        } else {
            None
        }
    });

    player.0.attack = if buttons.pressed(ATTACK_AIMED) {
        Attack::WithTarget(cursor.translation.xy())
    } else if buttons.pressed(ATTACK_NOT_AIMED) {
        Attack::WithoutTarget
    } else {
        Attack::None
    };

    player.0.dash = buttons.pressed(DASH);

    let up_pressed = keys.just_pressed(UP);
    let down_pressed = keys.just_pressed(DOWN);

    let prev_depth = player.0.depth;
    if up_pressed && !down_pressed {
        player.0.depth = player.0.depth.saturating_sub(1);
    } else if down_pressed && !up_pressed {
        player.0.depth = player.0.depth.saturating_add(1).min(player.2.max_depth);
    }

    if player.0.depth != prev_depth {
        non_player_creatures.into_iter().for_each(|creature| {
            commands.trigger(RelativeDepthLayerChanged {
                affected_creature: creature,
                player_depth: player.0.depth,
            });
        });
    }
}

fn sys_toggle_build(
    cursor_icon: Single<&mut CursorIcon>,
    mut picking_settings: ResMut<PickingSettings>,
    state: Res<State<BuildingState>>,
    mut next_state: ResMut<NextState<BuildingState>>,
) {
    let cursor_icon = cursor_icon.into_inner().into_inner();

    let is_playing = *state.get() == BuildingState::Playing;

    picking_settings.is_enabled = is_playing;

    next_state.set(if is_playing {
        *cursor_icon = CursorIcon::System(SystemCursorIcon::Default);

        BuildingState::Building
    } else {
        *cursor_icon = CursorIcon::System(SystemCursorIcon::Crosshair);

        BuildingState::Playing
    });
}

/// Toggles [`GameState`], releasing or confining the cursor to the window.
fn sys_toggle_pause(
    cursor_options: Single<&mut CursorOptions>,
    mut cursor_icon: Single<&mut CursorIcon>,
    mut window: Single<&mut Window>,
    mut autosave: Single<&mut VirtualTimer, With<session::Autosave>>,
    picking_settings: ResMut<PickingSettings>,
    game_state: Res<State<GameState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    building_state: Res<State<BuildingState>>,
) {
    next_game_state.set({
        let cursor_options = cursor_options.into_inner().into_inner();
        let mut picking_enabled = picking_settings.map_unchanged(|v| &mut v.is_enabled);

        if *game_state.get() == GameState::Playing {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_icon.set_if_neq(CursorIcon::System(SystemCursorIcon::Default));
            picking_enabled.set_if_neq(true);

            window.title.push_str(WINDOW_TITLE_PAUSED);

            autosave.0.pause();

            GameState::Paused
        } else {
            cursor_options.grab_mode = CursorGrabMode::Confined;

            if *building_state.get() == BuildingState::Playing {
                **cursor_icon = CursorIcon::System(SystemCursorIcon::Crosshair);
                picking_enabled.set_if_neq(false);
            }

            let title_len = window.title.len();
            window.title.truncate(title_len - WINDOW_TITLE_PAUSED.len());

            autosave.0.unpause();

            GameState::Playing
        }
    });
}

/// Toggles the fullscreen mode between borderless and windowed.
#[cfg(not(target_family = "wasm"))]
fn sys_toggle_fullscreen(
    mut window: Single<&mut Window>,
    mut cursor_options: Single<&mut CursorOptions>,
    state: Res<State<GameState>>,
) {
    use bevy::window::WindowMode;

    window.mode = if window.mode == WindowMode::Windowed {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };

    if *state.get() == GameState::Playing {
        cursor_options.grab_mode = CursorGrabMode::Confined;
    }
}

/// Animation of the cursor entity's light intensity based on player click input.
fn sys_click(
    mut click: Single<&mut PointLight, With<Cursor>>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    if buttons.any_just_pressed([DASH, ATTACK_AIMED, ATTACK_NOT_AIMED]) {
        click.intensity = Cursor::CLICK_INTENSITY;
    } else {
        click.intensity.smooth_nudge(
            &Cursor::DEFAULT_INTENSITY,
            Cursor::INTENSITY_TRANSITION_SPEED,
            time.delta_secs(),
        );
    }
}

/// Marker component for the player's cursor.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Resource)]
#[require(Transform::from_xyz(0.0, 0.0, CURSOR_Z), PointLight {
    range: 8.0,
    intensity: Cursor::DEFAULT_INTENSITY,
    ..default()
})]
#[component(immutable)]
pub struct Cursor;

/// Camera control and animation,
/// and synchronization of the cursor entity's position based on cursor position in the window.
///
/// Also updates clear color, distance fog and ambient light based on the camera's position.
fn sys_cursor_camera(
    window: Single<&Window>,
    mut camera: Single<
        (&mut Transform, &GlobalTransform, &Camera, &mut DistanceFog),
        (Without<Cursor>, Without<Particle>),
    >,
    mut cursor: Single<&mut Transform, (With<Cursor>, Without<Camera>, Without<Particle>)>,
    creature: Single<&CreatureOfHead, (With<Player>, Without<Cursor>, Without<Camera>)>,
    heads: Query<&GlobalTransform, (With<HeadOfCreature>, Without<Cursor>, Without<Camera>)>,
    time: Res<Time>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    game_state: Res<State<GameState>>,
) {
    if let Some(cursor_viewport_pos) = window.cursor_position()
        && let Ok(cursor_ndc_pos) = camera.2.viewport_to_ndc(cursor_viewport_pos)
    {
        let head_world_pos = heads.get(creature.get()).unwrap().translation();

        let camera_z = head_world_pos.z - CREATURE_Z;

        let mut target_camera_pos =
            (head_world_pos.xy() + cursor_ndc_pos * Vec2::splat(8.0)).extend(camera_z);

        if *game_state.get() == GameState::Paused {
            target_camera_pos.z += CAMERA_PAUSED_Z;
        }

        camera.0.translation.smooth_nudge(
            &target_camera_pos,
            CAMERA_MOVEMENT_SPEED,
            time.delta_secs(),
        );

        if let Ok(cursor_world_ray) = camera.2.viewport_to_world(camera.1, cursor_viewport_pos)
            && let Some(cursor_pos_xy) = cursor_world_ray.plane_intersection_point(
                vec3(0.0, 0.0, head_world_pos.z),
                InfinitePlane3d::new(Vec3::Z),
            )
        {
            cursor.translation = cursor_pos_xy.with_z(camera_z + CURSOR_Z); // Intersection point with the creature Z plane.
        }
    }

    // Beer–Lambert law light falloff over depth.
    // https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law
    ambient_light.brightness =
        ENV_LIGHT * f32::exp(camera.0.translation.z * ENV_OPTICAL_DENSITY).max(0.0);

    let env_tint = {
        let scaled_pos = camera.0.translation.xy() / ENV_HUE_SHIFT_SCALE;
        let coord = (scaled_pos.floor())
            .as_i64vec2()
            .rem_euclid(I64Vec2::splat(u8::MAX as i64 + 1))
            .as_u8vec2(); // Integer coordinates of the bottom left sample.
        let t = scaled_pos.fract_gl(); // Bilinear interpolation blend weights.

        // Generate a deterministic random value by hashing the current coordinates with offsets added for the non-bottom left samples.
        let sample = |offset: U8Vec2| FixedHasher.hash_one(coord.wrapping_add(offset)) as f32;

        // Bilinearly interpolate between 4 samples of hash noise around the camera position,
        // scaling and offsetting the result to [-0.5, 0.5].
        let hue_shift = f32::lerp(
            sample(U8Vec2::ZERO).lerp(sample(u8vec2(1, 0)), t.x),
            sample(u8vec2(0, 1)).lerp(sample(U8Vec2::splat(1)), t.x),
            t.y,
        ) / (u64::MAX as f32)
            - 0.5;

        debug_assert!((-0.5..=0.5).contains(&hue_shift));

        LinearRgba::from(
            Oklcha::new(ENV_LIGHTNESS, ENV_CHROMA, ENV_BASE_HUE, 1.0)
                .rotate_hue(hue_shift * ENV_HUE_SHIFT),
        )
    };

    ambient_light.color = Color::LinearRgba(LinearRgba::WHITE.mix(&env_tint, 0.5).with_alpha(1.0));

    clear_color.0 =
        Color::LinearRgba(env_tint * ambient_light.brightness * ENV_BRIGHTNESS).with_alpha(1.0);

    camera.3.color = clear_color.0; // Distance fog.
}

/// Camera & cursor icon initialization.
fn sys_startup(mut commands: Commands, window: Single<Entity, With<Window>>) {
    commands
        .entity(*window)
        .insert_if_new(CursorIcon::System(SystemCursorIcon::Crosshair));

    commands.spawn((
        Camera3d::default(),
        ScreenSpaceTransmission {
            steps: 0, // We don't use this.
            quality: ScreenSpaceTransmissionQuality::Low,
        },
        Transform::from_xyz(0.0, 0.0, CAMERA_PAUSED_Z),
        SpatialListener::new(-4.0),
        Exposure::INDOOR,
        Camera::default(),
        cfg_select! {
            target_family = "wasm" => {
                Msaa::Sample4
            },
            _ => Msaa::Sample8
        },
        Tonemapping::TonyMcMapface,
        Projection::Perspective(PerspectiveProjection {
            far: RENDER_DEPTH,
            fov: CAMERA_FOV,
            ..default()
        }),
        Bloom::default(),
        DistanceFog {
            falloff: FogFalloff::from_visibility(RENDER_DEPTH * 2.0),
            ..default()
        },
        PhysicsPickable,
        ClusterConfig::Single,
    ));
}
