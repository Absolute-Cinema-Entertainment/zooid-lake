use std::{array, f32::consts::TAU};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{
        AnnulusMeshBuilder, CircleMeshBuilder, EllipseMeshBuilder, RectangleMeshBuilder,
        RingMeshBuilder, Triangle2dMeshBuilder,
    },
    prelude::*,
};
use strum::{EnumCount, EnumIter, IntoEnumIterator};
use tinyvec::ArrayVec;

/// Handle storage to unique, shared meshes.
///
/// All creature parts of the same type share the same meshes.
#[derive(Clone, Eq, PartialEq, Resource, Hash)]
#[component(immutable)]
pub struct SharedMeshes([Handle<Mesh>; MeshId::COUNT]);
impl SharedMeshes {
    /// Shared mesh initialization.
    pub(super) fn sys_startup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
        commands.insert_resource(Self({
            let mut variants = MeshId::iter();
            array::from_fn(|_| meshes.add(variants.next().unwrap().create()))
        }));
    }

    /// Returns the handle corresponding to `id`.
    #[must_use]
    pub fn get(&self, id: MeshId) -> Handle<Mesh> {
        self.0[id as usize].clone()
    }
}
/// Enum identifying a unique, shared [`Mesh`].
#[derive(Clone, Copy, EnumCount, EnumIter, Eq, Hash, PartialEq)]
pub enum MeshId {
    PartHugeBlob,
    PartTriBlob,
    PartSmallOval,
    PartDiamond,
    PartSpear,
    PartLeg,
    PartSquare,
    PartHeart,

    SocketRegular,
    SocketRotating,
    SocketAttachment,
    SocketHeart,

    ProjectileNeedle,

    ConnectionLine,
}
impl MeshId {
    /// Creates the [`Mesh`] corresponding to the [`MeshId`].
    #[must_use]
    fn create(self) -> Mesh {
        let mut mesh = match self {
            Self::PartHugeBlob => AnnulusMeshBuilder::new(2.45, 2.5, 48).build(),
            Self::PartTriBlob => AnnulusMeshBuilder::new(0.45, 0.5, 32).build(),
            Self::PartSmallOval => RingMeshBuilder::<Ellipse> {
                inner_shape_builder: EllipseMeshBuilder::new(0.5, 0.85, 32),
                outer_shape_builder: EllipseMeshBuilder::new(0.55, 0.9, 32),
            }
            .build(),
            Self::PartDiamond => Rhombus::new(1.0, 1.5).to_ring(0.05).into(),
            Self::PartSpear => {
                let mut mesh = Triangle2dMeshBuilder::new(
                    vec2(-0.25, 0.15),
                    vec2(-0.5, 0.01),
                    vec2(0.25, 0.01),
                )
                .build();

                mesh.merge(
                    &Triangle2dMeshBuilder::new(
                        vec2(-0.25, -0.15),
                        vec2(-0.5, -0.01),
                        vec2(0.25, -0.01),
                    )
                    .build(),
                )
                .unwrap();

                mesh
            }
            Self::PartLeg => {
                let mut mesh = RectangleMeshBuilder::new(0.025, 1.5).build();
                mesh.merge(
                    &AnnulusMeshBuilder::new(0.04, 0.05, 8)
                        .build()
                        .transformed_by(Transform::from_xyz(0.0, -0.8, 0.0)),
                )
                .unwrap();
                mesh
            }
            Self::PartSquare => {
                let mut rect: Mesh = Rectangle::new(0.3, 0.3).to_ring(0.025).into();
                rect.merge(&Rectangle::new(0.2, 0.2).to_ring(0.025).into())
                    .unwrap();
                rect
            }
            Self::PartHeart => CircleMeshBuilder::new(0.2, 16).build(),

            Self::SocketRegular => AnnulusMeshBuilder::new(0.125, 0.15, 16).build(),
            Self::SocketRotating => {
                const SEGMENT_INNER_DIST: f32 = 0.075;
                const SEGMENT_OUTER_DIST: f32 = 0.14;
                const SEGMENTS: u8 = 6;
                const ANGLE_STEP: f32 = TAU / SEGMENTS as f32;

                let mut vertices = ArrayVec::<[Vec2; (SEGMENTS * 4 + 1) as usize]>::new();

                (0..SEGMENTS).for_each(|i| {
                    let i = i as f32;
                    let segment_vec = Vec2::from_angle(i * ANGLE_STEP);

                    vertices.push(segment_vec * SEGMENT_OUTER_DIST);
                    vertices.push(segment_vec * SEGMENT_INNER_DIST);
                    vertices.push(segment_vec * SEGMENT_OUTER_DIST);

                    vertices.push(Vec2::from_angle((i + 0.5) * ANGLE_STEP) * SEGMENT_OUTER_DIST);
                });

                vertices.push(vec2(1.0, 0.0) * SEGMENT_OUTER_DIST);

                let mut mesh: Mesh = Polyline2d::new(vertices).into();

                mesh.duplicate_vertices();
                mesh.merge_duplicate_vertices().unwrap();

                mesh
            }
            Self::SocketAttachment => AnnulusMeshBuilder::new(0.075, 0.1, 12).build(),
            Self::SocketHeart => AnnulusMeshBuilder::new(0.275, 0.30, 16).build(),

            Self::ProjectileNeedle => Segment2d::new(vec2(-0.2, 0.0), vec2(0.2, 0.0)).into(),

            Self::ConnectionLine => RectangleMeshBuilder::new(0.05, 1.0).build(),
        };

        mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;

        mesh
    }
}
