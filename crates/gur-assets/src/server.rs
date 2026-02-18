//! [`AssetServer`] — the central asset loading and caching authority.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use crate::handle::{Handle, HandleId};
use crate::texture::Texture;

// ─── Internal Cache ───────────────────────────────────────────────────────────

type AnyBox = Box<dyn Any + Send + Sync>;

/// ID counter — monotonically increasing, wraps at u64::MAX (never in practice).
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> HandleId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ─── AssetServer ─────────────────────────────────────────────────────────────

/// Loads, caches, and provides typed access to game assets.
///
/// ## Thread Safety
/// The server uses `RwLock` internally. Load operations take a write lock briefly.
/// Read operations (via `get`) take a read lock and are concurrent-safe.
pub struct AssetServer {
    /// Filesystem root for all asset paths.
    root: PathBuf,
    /// Cache: (TypeId, path_string) → (HandleId, Box<dyn Any>)
    cache: RwLock<HashMap<(TypeId, String), (HandleId, AnyBox)>>,
    /// Reverse map: HandleId → canonical path (for hot-reload).
    id_to_path: RwLock<HashMap<HandleId, String>>,
}

impl AssetServer {
    /// Create a new server with the given asset root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            cache: RwLock::new(HashMap::new()),
            id_to_path: RwLock::new(HashMap::new()),
        }
    }

    /// Resolve a relative asset path to an absolute filesystem path.
    fn resolve(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    // ─── Texture Loading ─────────────────────────────────────────────────

    /// Load a texture from disk (or return the cached copy).
    ///
    /// Supported formats: PNG, JPEG (via the `image` crate).
    pub fn load_texture(&self, path: &str) -> anyhow::Result<Handle<Texture>> {
        let key = (TypeId::of::<Texture>(), path.to_owned());

        // Fast path: already cached
        {
            let cache = self.cache.read();
            if let Some((id, _)) = cache.get(&key) {
                return Ok(Handle::new(*id));
            }
        }

        // Slow path: load from disk
        let full_path = self.resolve(path);
        let texture = Texture::load_from_path(&full_path)
            .map_err(|e| anyhow::anyhow!("Failed to load texture `{}`: {}", path, e))?;

        let id = next_id();
        {
            let mut cache = self.cache.write();
            cache.insert(key, (id, Box::new(texture)));
        }
        {
            let mut reverse = self.id_to_path.write();
            reverse.insert(id, path.to_owned());
        }

        log::debug!("Loaded texture: {} (id={})", path, id);
        Ok(Handle::new(id))
    }

    /// Get a reference to a cached texture by handle.
    ///
    /// Returns `None` if the handle is stale (asset was evicted).
    pub fn get_texture(&self, handle: &Handle<Texture>) -> Option<impl std::ops::Deref<Target = Texture> + '_> {
        let cache = self.cache.read();
        // We can't return a reference into the RwLockReadGuard easily without unsafe,
        // so we keep the guard alive via a wrapper.
        // For now, clone the data. Hot path optimization: switch to Arc<T> storage later.
        let _ = cache; // suppress unused warning
        None::<TextureRef> // placeholder until wgpu upload layer is wired in
    }

    /// Returns the root path of this server.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Placeholder type for the deref return of `get_texture`.
struct TextureRef;
impl std::ops::Deref for TextureRef {
    type Target = Texture;
    fn deref(&self) -> &Texture { unreachable!() }
}
