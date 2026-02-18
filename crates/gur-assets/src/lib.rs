//! # gur-assets
//!
//! Typed asset loading, caching, and hot-reload for the GUR engine.
//!
//! ## Design
//! - All assets are loaded through a central [`AssetServer`]
//! - Assets are referenced by [`Handle<T>`] — a typed, refcounted identifier
//! - The server caches loaded assets; re-requesting the same path returns the cached copy
//! - Hot-reload in dev builds: file-watcher thread signals changed assets
//!
//! ## Usage
//! ```rust,ignore
//! let handle: Handle<Texture> = assets.load("sprites/player.png")?;
//! let tex: &Texture = assets.get(&handle)?;
//! ```

#![warn(missing_docs)]

pub mod handle;
pub mod server;
pub mod texture;

pub use handle::{Handle, HandleId};
pub use server::AssetServer;
pub use texture::Texture;

/// Plugin that registers the [`AssetServer`] as a resource.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers the asset server with a configurable root path.
    pub struct AssetsPlugin {
        /// Root directory for all game assets (relative to binary).
        pub root: String,
    }

    impl Default for AssetsPlugin {
        fn default() -> Self {
            Self { root: "assets".to_string() }
        }
    }

    impl Plugin for AssetsPlugin {
        fn name(&self) -> &'static str { "gur_assets" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            let server = super::AssetServer::new(&self.root);
            world.insert_resource(server);
            log::info!("AssetsPlugin ready — root: {}", self.root);
            Ok(())
        }
    }
}
