//! Background creation and animation.

use std::array;

use avian2d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    math::NormedVectorSpace,
    mesh::{AnnulusMeshBuilder, CircleMeshBuilder},
    prelude::*,
};
use rand::{
    distr::{Open01, Uniform},
    prelude::*,
};

use crate::{
    consts::PARTICLE_IOR,
    creature::{Player, part::CreaturePartKindId, projectile::Projectile},
};

/// Plugin handling the background.
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct BackgroundPlugin;
impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, sys_startup)
            .add_systems(Update, sys_particles);
    }
}

/// Handle storage to materials used by particles.
///
/// All particles share these materials,
/// and the materials are identical in every way except opacity.
#[derive(Clone, Eq, PartialEq, Resource, Hash)]
#[component(immutable)]
pub struct MaterialHandles([Handle<StandardMaterial>; Self::LEN as usize]);
impl MaterialHandles {
    /// Returns the handle to the material with closest opacity to `alpha * ALPHA_MUL`.
    #[must_use]
    fn get_from_alpha(&self, alpha: f32) -> Option<Handle<StandardMaterial>> {
        ((alpha * Self::LEN as f32).round_ties_even() as usize)
            .checked_sub(1)
            .map(|i| self.0[i].clone())
    }

    /// Returns the handle to the material with closest opacity to `alpha * ALPHA_MUL`, falling back to the lowest possible one if `alpha` is unrepresentable.
    #[must_use]
    fn get_or_min_from_alpha(&self, alpha: f32) -> Handle<StandardMaterial> {
        self.get_from_alpha(alpha)
            .unwrap_or_else(|| self.0[0].clone())
    }
}

/// Component storing particle state.
#[derive(Clone, Component, Copy, Default, PartialEq)]
pub struct Particle {
    life: f32,
    opacity: f32,
    scale: f32,
}

/// Particle animation, respawning and collision-like interaction with physical bodies.
fn sys_particles(
    mut particles: Query<(
        &mut Transform,
        &mut Particle,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    repelling_bodies: Query<&Position, Or<(With<CreaturePartKindId>, With<Projectile>)>>,
    player: Single<&Transform, (With<Player>, Without<Particle>)>,
    camera: Single<(&Transform, &GlobalTransform, &Camera), Without<Particle>>,
    material_handles: Res<MaterialHandles>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();

    let camera_pos = camera.0.translation;
    let x_distr = Uniform::new_inclusive(
        camera_pos.x - Particle::XY_DIST,
        camera_pos.x + Particle::XY_DIST,
    )
    .unwrap();
    let y_distr = Uniform::new_inclusive(
        camera_pos.y - Particle::XY_DIST,
        camera_pos.y + Particle::XY_DIST,
    )
    .unwrap();
    let z_distr = Uniform::new_inclusive(
        Particle::Z_RANGE.start() + camera_pos.z,
        Particle::Z_RANGE.end() + camera_pos.z,
    )
    .unwrap();
    let jitter_distr = Uniform::new_inclusive(-Particle::JITTER, Particle::JITTER).unwrap();

    particles
        .par_iter_mut()
        .for_each(|(mut transform, mut particle, mut mat_handle)| {
            let transform = transform.as_mut();
            let particle = particle.as_mut();

            let mut rng = SmallRng::seed_from_u64(
                transform
                    .translation
                    .x
                    .to_bits()
                    .wrapping_add(transform.translation.y.to_bits()) as u64,
            );

            particle.life = delta.mul_add(-0.125, particle.life);

            // Respawn particles if their normalized device coordinates are too far outside the screen or somehow invalid,
            // or if their opacity is too low to be represented by a material.
            if let Some(ndc_pos) = camera.2.world_to_ndc(camera.1, transform.translation)
                && ndc_pos.xy().abs().max_element() <= Particle::DESPAWN_DIST
                && let Some(new_mat_handle) =
                    material_handles.get_from_alpha(particle.life * particle.opacity)
            {
                mat_handle.set_if_neq(MeshMaterial3d(new_mat_handle));

                // Translate particles by a random jitter,
                // and repel them away from active creature parts if they're close to the creature plane.
                let mut movement = vec2(rng.sample(jitter_distr), rng.sample(jitter_distr));

                if transform
                    .translation
                    .z
                    .distance_squared(player.translation.z)
                    < (16.0 * 16.0)
                {
                    repelling_bodies
                        .contiguous_iter_inner()
                        .unwrap()
                        .for_each(|bodies| {
                            bodies.iter().for_each(|body| {
                                let body_to_particle = transform.translation.xy() - body.0;

                                movement += body_to_particle
                                    * (body_to_particle.length_squared().mul_add(0.005, 1.0))
                                        .powi(-32);
                            });
                        });
                }

                transform.translation = transform
                    .translation
                    .with_xy(movement.mul_add(Vec2::splat(delta), transform.translation.xy()));

                transform.scale = transform
                    .scale
                    .with_xy(Vec2::splat((1.0 - particle.life) * particle.scale));
            } else {
                particle.life = 1.0;

                transform.translation = vec3(
                    rng.sample(x_distr),
                    rng.sample(y_distr),
                    rng.sample(z_distr),
                );
                transform.scale = transform.scale.with_xy(Vec2::ZERO);

                // We skip setting the material here since the particle is infinitely small.
                // It will be set properly the next frame, when the particle becomes visible.
            }
        });
}

/// Particles and particle material handles initialization.
fn sys_startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_handles = MaterialHandles({
        let particle_base = StandardMaterial {
            base_color: Color::LinearRgba(Particle::COLOR),
            alpha_mode: AlphaMode::Blend,
            ior: PARTICLE_IOR,
            diffuse_transmission: 0.5,
            ..default()
        };

        // Create materials for every step in opacity.
        array::from_fn(|i| {
            materials.add(StandardMaterial {
                base_color: particle_base.base_color.with_alpha(
                    MaterialHandles::ALPHA_MUL * ((i + 1) as f32 / MaterialHandles::LEN as f32),
                ),
                ..particle_base.clone()
            })
        })
    });

    // OUTDATED faster blending-less alternative to the above that emulates opacity and lighting by modifying the base color.
    /*
        let material_handles = MaterialHandles({
            let ambient_light = 0.0025
                * crate::consts::AMBIENT_LIGHT
                * crate::consts::WATER_TINT
                    .mix(&LinearRgba::WHITE, 0.5)
                    .to_vec3(); // TODO: Replace when we start changing the ambient light.

            let color = StandardMaterial {
                base_color: Color::LinearRgba(LinearRgba::from_vec3(
                    Particle::COLOR.to_vec3() * ambient_light,
                )),
                unlit: true,
                ..default()
            };

            // Create materials for every step in "opacity".
            array::from_fn(|i| {
                materials.add({
                    let mut material = color.clone();

                    material.base_color.mix_assign(
                        Color::LinearRgba(WATER_COLOR),
                        1.0 - (i + 1) as f32 / MaterialHandles::LEN as f32,
                    );

                    material
                })
            })
        });
    */

    let mut rng = rand::make_rng::<SmallRng>();
    let xy_distr = Uniform::new(-Particle::XY_DIST, Particle::XY_DIST).unwrap();
    let z_distr = Uniform::try_from(Particle::Z_RANGE).unwrap();

    // Spawn annulus-shaped particles.
    {
        let annulus_handle = Mesh3d(meshes.add({
            let mut annulus = AnnulusMeshBuilder::new(0.25, 0.3, 14).build();
            annulus.asset_usage = RenderAssetUsages::RENDER_WORLD;
            annulus
        }));
        let opacity_distr = Uniform::new(0.2, 1.0).unwrap();
        let scale_distr = Uniform::new_inclusive(0.1, 1.0).unwrap();

        for _ in 0..Particle::ANNULUS_COUNT {
            let life = rng.sample(Open01);
            let opacity = rng.sample(opacity_distr);

            commands.spawn((
                annulus_handle.clone(),
                MeshMaterial3d(material_handles.get_or_min_from_alpha(life * opacity)),
                Transform::from_xyz(
                    rng.sample(xy_distr),
                    rng.sample(xy_distr),
                    rng.sample(z_distr),
                ),
                Particle {
                    life,
                    opacity,
                    scale: rng.sample(scale_distr),
                },
            ));
        }
    }

    // Spawn circle-shaped particles.
    {
        let circle_handle = Mesh3d(meshes.add({
            let mut circle = const { CircleMeshBuilder::new(0.1, 4) }.build();
            circle.asset_usage = RenderAssetUsages::RENDER_WORLD;
            circle
        }));
        let opacity_distr = Uniform::new(0.2, 0.5).unwrap();
        let scale_distr = Uniform::new_inclusive(0.8, 1.0).unwrap();

        for _ in 0..Particle::CIRCLE_COUNT {
            let life = rng.sample(Open01);
            let opacity = rng.sample(opacity_distr);

            commands.spawn((
                circle_handle.clone(),
                MeshMaterial3d(material_handles.get_or_min_from_alpha(life * opacity)),
                Transform::from_xyz(
                    rng.sample(xy_distr),
                    rng.sample(xy_distr),
                    rng.sample(z_distr),
                ),
                Particle {
                    life,
                    opacity,
                    scale: rng.sample(scale_distr),
                },
            ));
        }
    }

    commands.insert_resource(material_handles);
}
