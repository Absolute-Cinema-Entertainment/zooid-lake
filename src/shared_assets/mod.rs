//! Shared meshes and collider shapes.

use std::array;

use bevy::prelude::*;
use strum::IntoEnumIterator;

pub use crate::shared_assets::{
    audio::{AudioId, SharedAudio},
    materials::{MaterialId, SharedMaterials},
    meshes::{MeshId, SharedMeshes},
    shapes::{ShapeId, SharedShapes},
};

pub mod audio;
pub mod materials;
pub mod meshes;
pub mod shapes;

/// Plugin creating shared assets.
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct SharedAssetPlugin;
impl Plugin for SharedAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreStartup,
            (
                SharedAudio::sys_startup,
                SharedMeshes::sys_startup,
                SharedMaterials::sys_startup,
            ),
        )
        .insert_resource(SharedShapes({
            let mut variants = ShapeId::iter();
            array::from_fn(|_| variants.next().unwrap().create())
        }));
    }
}

/*
    /// DRAFT: Generic shared assets implementation.
    #[derive(Clone, Eq, PartialEq, Resource, Hash)]
    pub struct SharedAssetStorage<T: Send + Sync>(T);
    impl<A: Clone + Send + Sync + 'static, T: Send + Sync + Index<usize, Output = A>> SharedAssetStorage<T> {
        /// Returns the handle to the asset corresponding to `id`.
        #[must_use]
        #[inline]
        pub fn get(&self, id: impl Into<usize>) -> T::Output {
            self.0[id.into()].clone()
        }

        #[must_use]
        #[inline]
        pub fn create_all<K: EnumCount + IntoEnumIterator>(
            mut commands: Commands,
            mut f: impl FnMut(K) -> T::Output,
        ) {
            let mut variants = K::iter();
            commands.insert_resource(Self(array::from_fn(|_| f(variants.next().unwrap()))));
        }
    }
*/
