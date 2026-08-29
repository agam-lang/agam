//! # GPU Image Texture Upload & Rasterization (`agam_gui::image`)
//!
//! Bridges `agam_std::image::ImageBuffer` to Vello and GPU textures.
//! Preserves zero-identity-leak invariants and adheres to Nyāya error reporting.

use std::sync::Arc;

use agam_std::image::{ImageBuffer, Rgba8};
use serde::{Deserialize, Serialize};
use vello::peniko::{Blob, ImageAlphaType, ImageFormat};

use crate::diagnostic::{GuiError, GuiResult};
use crate::scene::{Rect, Size};

/// Image scaling and aspect ratio fitting behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImageFit {
    /// Scale image to fit inside target rect while preserving aspect ratio (letterbox/pillarbox).
    #[default]
    Contain,
    /// Scale image to fill target rect while preserving aspect ratio (crop overflow).
    Cover,
    /// Stretch image non-uniformly to exactly match target bounds.
    Fill,
    /// Keep native pixel dimensions without scaling.
    None,
}

/// A GPU-compatible RGBA image texture.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageTexture {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageTexture")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("byte_len", &self.data.len())
            .finish()
    }
}

impl ImageTexture {
    /// Construct an `ImageTexture` from raw RGBA8 bytes.
    pub fn from_rgba_bytes(width: u32, height: u32, bytes: Vec<u8>) -> GuiResult<Self> {
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or_else(|| {
                GuiError::new(
                    "Image texture dimensions exceed address space",
                    "Texture width * height * 4 exceeds usize limits",
                    Some("Reduce image resolution or downsample before texture construction"),
                    "RFC-gui-engine §2: Texture allocation must return structured Nyāya diagnostics",
                )
            })?;

        if bytes.len() != expected_len {
            return Err(GuiError::new(
                "Image byte length does not match specified dimensions",
                format!(
                    "Received {} bytes, expected {} for {}x{} RGBA",
                    bytes.len(),
                    expected_len,
                    width,
                    height
                ),
                Some("Ensure raw buffer contains exactly 4 bytes (RGBA) per pixel"),
                "RFC-gui-engine §2: Invalid texture buffers must be rejected with diagnostic proofs",
            ));
        }

        Ok(Self {
            width,
            height,
            data: bytes,
        })
    }

    /// Construct an `ImageTexture` directly from an Agam standard library `ImageBuffer<Rgba8>`.
    pub fn from_image_buffer(buffer: &ImageBuffer<Rgba8>) -> GuiResult<Self> {
        let width = u32::try_from(buffer.width()).map_err(|_| {
            GuiError::new(
                "ImageBuffer width exceeds u32 dimensions",
                "Width cannot exceed 4,294,967,295 pixels",
                Some("Reduce buffer width"),
                "RFC-gui-engine §2: Image dimensions must fit within GPU texture limits",
            )
        })?;

        let height = u32::try_from(buffer.height()).map_err(|_| {
            GuiError::new(
                "ImageBuffer height exceeds u32 dimensions",
                "Height cannot exceed 4,294,967,295 pixels",
                Some("Reduce buffer height"),
                "RFC-gui-engine §2: Image dimensions must fit within GPU texture limits",
            )
        })?;

        let slice = buffer.as_slice();
        let mut bytes = Vec::with_capacity(slice.len() * 4);
        for pixel in slice {
            bytes.push(pixel.r);
            bytes.push(pixel.g);
            bytes.push(pixel.b);
            bytes.push(pixel.a);
        }

        Ok(Self {
            width,
            height,
            data: bytes,
        })
    }

    /// Texture width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Texture height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Natural dimensions as a `Size`.
    pub fn size(&self) -> Size {
        Size::new(self.width as f64, self.height as f64)
    }

    /// Access raw RGBA8 byte buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Convert into a Vello Peniko `ImageData` ready for scene compositing.
    pub fn to_peniko(&self) -> vello::peniko::ImageData {
        let blob = Blob::new(Arc::new(self.data.clone()));
        vello::peniko::ImageData {
            data: blob,
            format: ImageFormat::Rgba8,
            width: self.width,
            height: self.height,
            alpha_type: ImageAlphaType::Alpha,
        }
    }

    /// Compute destination bounding box and source UV alignment based on `ImageFit`.
    pub fn compute_fit_rect(&self, target_bounds: Rect, fit: ImageFit) -> Rect {
        let img_w = self.width as f64;
        let img_h = self.height as f64;
        if img_w <= 0.0 || img_h <= 0.0 || target_bounds.width <= 0.0 || target_bounds.height <= 0.0
        {
            return target_bounds;
        }

        match fit {
            ImageFit::Fill => target_bounds,
            ImageFit::None => Rect::new(
                target_bounds.origin.x + (target_bounds.width - img_w) / 2.0,
                target_bounds.origin.y + (target_bounds.height - img_h) / 2.0,
                img_w,
                img_h,
            ),
            ImageFit::Contain => {
                let scale_x = target_bounds.width / img_w;
                let scale_y = target_bounds.height / img_h;
                let scale = scale_x.min(scale_y);
                let w = img_w * scale;
                let h = img_h * scale;
                Rect::new(
                    target_bounds.origin.x + (target_bounds.width - w) / 2.0,
                    target_bounds.origin.y + (target_bounds.height - h) / 2.0,
                    w,
                    h,
                )
            }
            ImageFit::Cover => {
                let scale_x = target_bounds.width / img_w;
                let scale_y = target_bounds.height / img_h;
                let scale = scale_x.max(scale_y);
                let w = img_w * scale;
                let h = img_h * scale;
                Rect::new(
                    target_bounds.origin.x + (target_bounds.width - w) / 2.0,
                    target_bounds.origin.y + (target_bounds.height - h) / 2.0,
                    w,
                    h,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_texture_from_rgba_bytes() {
        let bytes = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let res = ImageTexture::from_rgba_bytes(2, 2, bytes);
        assert!(res.is_ok());
        if let Ok(tex) = res {
            assert_eq!(tex.width(), 2);
            assert_eq!(tex.height(), 2);
            assert_eq!(tex.as_bytes().len(), 16);
        }
    }

    #[test]
    fn test_image_texture_from_image_buffer() {
        let mut buf = ImageBuffer::<Rgba8>::new(4, 4);
        buf.put_pixel(0, 0, Rgba8::new(255, 128, 64, 255));
        let res = ImageTexture::from_image_buffer(&buf);
        assert!(res.is_ok());
        if let Ok(tex) = res {
            assert_eq!(tex.width(), 4);
            assert_eq!(tex.height(), 4);
            assert_eq!(tex.as_bytes()[0], 255);
            assert_eq!(tex.as_bytes()[1], 128);
        }
    }

    #[test]
    fn test_image_fit_contain_and_cover() {
        let bytes = vec![0; 400]; // 10x10
        let res = ImageTexture::from_rgba_bytes(10, 10, bytes);
        assert!(res.is_ok());
        if let Ok(tex) = res {
            let target = Rect::new(0.0, 0.0, 100.0, 50.0);

            let contain_rect = tex.compute_fit_rect(target, ImageFit::Contain);
            assert_eq!(contain_rect.width, 50.0);
            assert_eq!(contain_rect.height, 50.0);
            assert_eq!(contain_rect.origin.x, 25.0);

            let cover_rect = tex.compute_fit_rect(target, ImageFit::Cover);
            assert_eq!(cover_rect.width, 100.0);
            assert_eq!(cover_rect.height, 100.0);
            assert_eq!(cover_rect.origin.y, -25.0);
        }
    }
}
