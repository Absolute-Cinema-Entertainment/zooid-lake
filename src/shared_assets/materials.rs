use std::array;

use bevy::prelude::*;
use strum::{EnumCount, EnumIter, IntoEnumIterator};

use crate::consts::{GUI_IOR, PART_BASE_COLOR, PART_IOR};

/// Handle storage to unique, shared materials.
#[derive(Clone, Eq, PartialEq, Resource, Hash)]
#[component(immutable)]
pub struct SharedMaterials([Handle<StandardMaterial>; MaterialId::COUNT]);
impl SharedMaterials {
    /// Shared material initialization.
    pub(super) fn sys_startup(
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands.insert_resource(Self({
            let mut variants = MaterialId::iter();
            array::from_fn(|_| materials.add(variants.next().unwrap().create()))
        }));
    }

    /// Returns the handle corresponding to `id`.
    #[must_use]
    pub fn get(&self, id: MaterialId) -> Handle<StandardMaterial> {
        self.0[id as usize].clone()
    }
}
/// Enum identifying a unique, shared [`StandardMaterial`].
#[derive(Clone, Copy, EnumCount, EnumIter, Eq, Hash, PartialEq)]
pub enum MaterialId {
    PartPlayer,
    PartPlayerFriendly,
    PartWandering,
    PartHostile,
    PartHeadPlayer,
    PartHeadPlayerFriendly,
    PartHeadWandering,
    PartHeadHostile,
    PartHeadHeartOnly,

    Socket,

    ConnectionLine,

    GuiDefault,
    GuiHovered,
    GuiPressed,
}
impl MaterialId {
    /// Creates the [`StandardMaterial`] corresponding to the [`MaterialId`].
    #[must_use]
    pub fn create(self) -> StandardMaterial {
        let part_base = StandardMaterial {
            base_color: PART_BASE_COLOR,
            ior: PART_IOR,
            emissive_exposure_weight: 1.0,
            diffuse_transmission: 0.5,
            ..default()
        };

        let gui_base = StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::new(1.0, 1.0, 1.0, 0.1)),
            thickness: 1.0,
            perceptual_roughness: 0.1,
            ior: GUI_IOR,
            alpha_mode: AlphaMode::Blend,
            emissive_exposure_weight: 1.0,
            diffuse_transmission: 0.5,
            ..default()
        };

        match self {
            Self::PartPlayer => StandardMaterial {
                emissive: LinearRgba::rgb(500.0, 500.0, 500.0),
                ..part_base
            },
            Self::PartPlayerFriendly => StandardMaterial {
                emissive: LinearRgba::rgb(250.0, 500.0, 2000.0),
                ..part_base
            },
            Self::PartWandering => StandardMaterial {
                emissive: LinearRgba::rgb(500.0, 4000.0, 500.0),
                ..part_base
            },
            Self::PartHostile => StandardMaterial {
                emissive: LinearRgba::rgb(4000.0, 500.0, 500.0),
                ..part_base
            },
            Self::PartHeadPlayer => StandardMaterial {
                emissive: LinearRgba::rgb(10_000.0, 10_000.0, 10_000.0),
                ..part_base
            },
            Self::PartHeadPlayerFriendly => StandardMaterial {
                emissive: LinearRgba::rgb(2000.0, 4000.0, 15_000.0),
                ..part_base
            },
            Self::PartHeadWandering => StandardMaterial {
                emissive: LinearRgba::rgb(1000.0, 40_000.0, 1000.0),
                ..part_base
            },
            Self::PartHeadHostile => StandardMaterial {
                emissive: LinearRgba::rgb(40_000.0, 1000.0, 1000.0),
                ..part_base
            },
            Self::PartHeadHeartOnly => StandardMaterial {
                emissive: LinearRgba::rgb(5000.0, 2000.0, 2000.0),
                ..part_base
            },

            Self::Socket => StandardMaterial {
                emissive: LinearRgba::rgb(250.0, 250.0, 250.0),
                ..part_base
            },

            Self::ConnectionLine => StandardMaterial {
                base_color: part_base.base_color.with_alpha(0.05),
                alpha_mode: AlphaMode::Blend,
                emissive: LinearRgba::rgb(25_000.0, 25_000.0, 25_000.0),
                ..part_base
            },

            Self::GuiDefault => StandardMaterial {
                emissive: LinearRgba::rgb(100_000.0, 100_000.0, 100_000.0),
                ..gui_base
            },
            Self::GuiHovered => StandardMaterial {
                emissive: LinearRgba::rgb(100_000.0, 100_000.0, 200_000.0),
                ..gui_base
            },
            Self::GuiPressed => StandardMaterial {
                emissive: LinearRgba::rgb(200_000.0, 200_000.0, 400_000.0),
                ..gui_base
            },
        }
    }
}
