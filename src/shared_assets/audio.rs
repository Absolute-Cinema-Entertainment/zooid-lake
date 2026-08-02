use std::{array, path::Path};

use bevy::prelude::*;
use strum::{EnumCount, EnumIter, IntoEnumIterator};

/// Handle storage to unique, shared audio sources.
///
/// These are created at startup and remain in memory forever,
/// so they can be used without triggering file I/O and associated playback latency.
///
/// Large audio sources which don't need to play immediately (such as music and ambient sound) should likely not use this system.
#[derive(Clone, Eq, PartialEq, Resource, Hash)]
#[component(immutable)]
pub struct SharedAudio([Handle<AudioSource>; AudioId::COUNT]);
impl SharedAudio {
    /// Shared audio initialization.
    pub(super) fn sys_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.insert_resource(Self({
            let mut variants = AudioId::iter();
            array::from_fn(|_| asset_server.load(variants.next().unwrap().path()))
        }));
    }

    /// Returns the handle corresponding to `id`.
    #[must_use]
    pub fn get(&self, id: AudioId) -> Handle<AudioSource> {
        self.0[id as usize].clone()
    }
}
/// Enum identifying a unique, shared [`AudioSource`].
#[derive(Clone, Copy, EnumCount, EnumIter, Eq, Hash, PartialEq)]
pub enum AudioId {
    SocketDrag,
    SocketHoverWithLine,
    SocketConnect,
}
impl AudioId {
    #[must_use]
    pub fn path(self) -> &'static Path {
        Path::new(match self {
            Self::SocketDrag => "socket_drag.flac",
            Self::SocketHoverWithLine => "socket_hover_with_line.flac",
            Self::SocketConnect => "socket_connect.flac",
        })
    }
}
