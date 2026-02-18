//! Typed asset handles — cheap, cloneable references to loaded assets.

use std::marker::PhantomData;
use std::sync::Arc;

/// A unique numeric identifier for a loaded asset.
pub type HandleId = u64;

/// A typed, refcounted reference to an asset managed by [`AssetServer`].
///
/// Cheap to clone (just increments a refcount).
/// When all handles to an asset are dropped, the asset *may* be evicted
/// from the cache (policy is configurable in `AssetServer`).
#[derive(Debug)]
pub struct Handle<T> {
    id: HandleId,
    // Shared ownership of the actual data.
    _inner: Arc<()>,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    /// Create a new handle wrapping the given id.
    pub(crate) fn new(id: HandleId) -> Self {
        Self {
            id,
            _inner: Arc::new(()),
            _marker: PhantomData,
        }
    }

    /// The raw numeric id of this handle.
    #[inline]
    pub fn id(&self) -> HandleId {
        self.id
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _inner: Arc::clone(&self._inner),
            _marker: PhantomData,
        }
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
