use avian2d::{collision::collider::EllipseColliderShape, parry::shape::SharedShape};
use bevy::prelude::*;
use strum::{EnumCount, EnumIter};

/// Storage for shared collider shapes.
///
/// It is usually better to add multiple scaled variants of shapes here instead of using [`avian2d::prelude::Collider::set_scale`],
/// since that function will also duplicate the shape.
///
/// All creature parts of the same type share the same shapes.
#[derive(Clone, Resource)]
#[component(immutable)]
pub struct SharedShapes(pub [SharedShape; ShapeId::COUNT]);
impl SharedShapes {
    /// Returns the [`SharedShape`] corresponding to `id`.
    #[must_use]
    pub fn get(&self, id: ShapeId) -> SharedShape {
        self.0[id as usize].clone()
    }
}
/// Enum identifying a unique, shared [`SharedShape`].
#[derive(Clone, Copy, EnumCount, EnumIter, Eq, Hash, PartialEq)]
pub enum ShapeId {
    PartHugeBlob,
    PartTriBlob,
    PartSmallOval,
    // PartLargeOval,
    PartDiamond,
    PartSpear,
    PartLeg,
    PartSquare,
    PartHeart,

    SocketNonHeart,
    SocketHeart,

    ProjectileNeedle,
}
impl ShapeId {
    /// Creates the [`SharedShape`] corresponding to the [`ShapeId`].
    #[must_use]
    pub(super) fn create(self) -> SharedShape {
        match self {
            // The diamonds vertex positions are dependant on the Diamond mesh.
            Self::PartHugeBlob => SharedShape::ball(2.45),
            Self::PartTriBlob => SharedShape::ball(0.45),
            Self::PartSmallOval => SharedShape::new(EllipseColliderShape(Ellipse::new(0.5, 0.85))),
            // Self::PartLargeOval => SharedShape::new(EllipseColliderShape(Ellipse::new(1.0, 1.5))),
            Self::PartDiamond => SharedShape::convex_polyline_unmodified(vec![
                vec2(0.475, 0.0),
                vec2(0.0, 0.725),
                vec2(-0.475, 0.0),
                vec2(0.0, -0.725),
            ])
            .unwrap(),
            Self::PartSpear => SharedShape::convex_polyline_unmodified(vec![
                vec2(-0.2, 0.1),
                vec2(-0.45, 0.0),
                vec2(-0.2, -0.1),
                vec2(0.2, 0.0),
            ])
            .unwrap(),
            Self::PartLeg => SharedShape::cuboid(0.0125, 0.75),
            Self::PartSquare => SharedShape::cuboid(0.15, 0.15),
            Self::PartHeart => SharedShape::ball(0.175),

            Self::SocketNonHeart => SharedShape::ball(0.15),
            Self::SocketHeart => SharedShape::ball(0.25),

            Self::ProjectileNeedle => SharedShape::segment(vec2(-0.2, 0.0), vec2(0.2, 0.0)),
        }
    }
}
