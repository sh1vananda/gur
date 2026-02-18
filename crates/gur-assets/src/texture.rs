//! Raw texture data (CPU-side). The renderer uploads this to the GPU.

/// Raw RGBA pixel data loaded from an image file.
///
/// Stored on the CPU side; [`gur-render`] uploads it to a wgpu texture.
/// The asset server caches `Texture` values by path.
#[derive(Debug, Clone)]
pub struct Texture {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw RGBA8 pixel data (width × height × 4 bytes).
    pub data: Vec<u8>,
}

impl Texture {
    /// Create a solid-color 1×1 texture (useful as a placeholder / default).
    pub fn solid(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            width: 1,
            height: 1,
            data: vec![r, g, b, a],
        }
    }

    /// Load from raw RGBA8 bytes. Asserts that `data.len() == width * height * 4`.
    pub fn from_rgba8(width: u32, height: u32, data: Vec<u8>) -> Self {
        assert_eq!(
            data.len(),
            (width * height * 4) as usize,
            "RGBA8 data length mismatch: expected {} got {}",
            width * height * 4,
            data.len()
        );
        Self { width, height, data }
    }

    /// Load a texture from a PNG/JPEG file path.
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, image::ImageError> {
        let img = image::open(path)?.into_rgba8();
        let (width, height) = img.dimensions();
        Ok(Self {
            width,
            height,
            data: img.into_raw(),
        })
    }
}
