//! # High-Performance 4K Image Processing & Vectorized 2D Convolution (`agam_std::image`)
//!
//! Provides memory-contiguous raster representations, SIMD-accelerated 2D convolution filters,
//! spatial resampling algorithms, and Netpbm binary format encoding/decoding.

use agam_runtime::simd::simd_fma_f32;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Nyāya-grounded structured diagnostic error for image operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl ImageError {
    pub fn new(
        cause: impl Into<String>,
        context: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            cause: cause.into(),
            context: context.into(),
            remedy: remedy.into(),
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageError: {}\n  Context: {}\n  Remedy: {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for ImageError {}

/// Trait implemented by all discrete pixel color representations.
pub trait Pixel: Copy + Clone + Send + Sync + 'static {
    const CHANNELS: usize;
    fn to_rgb(&self) -> Rgb8;
    fn to_gray(&self) -> Gray8;
}

/// 8-bit single-channel grayscale pixel representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Gray8(pub u8);

impl Pixel for Gray8 {
    const CHANNELS: usize = 1;

    #[inline]
    fn to_rgb(&self) -> Rgb8 {
        Rgb8 {
            r: self.0,
            g: self.0,
            b: self.0,
        }
    }

    #[inline]
    fn to_gray(&self) -> Gray8 {
        *self
    }
}

/// 24-bit 3-channel RGB color pixel representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb8 {
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl Pixel for Rgb8 {
    const CHANNELS: usize = 3;

    #[inline]
    fn to_rgb(&self) -> Rgb8 {
        *self
    }

    #[inline]
    fn to_gray(&self) -> Gray8 {
        // ITU-R BT.601 standard luma coefficients: Y = 0.299R + 0.587G + 0.114B
        let luma = (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        Gray8(luma)
    }
}

/// 32-bit 4-channel RGBA color pixel representation with alpha channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl Pixel for Rgba8 {
    const CHANNELS: usize = 4;

    #[inline]
    fn to_rgb(&self) -> Rgb8 {
        Rgb8 {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }

    #[inline]
    fn to_gray(&self) -> Gray8 {
        let luma = (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        Gray8(luma)
    }
}

/// Contiguous row-major 2D image buffer supporting 4K UHD resolutions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBuffer<P: Pixel> {
    width: usize,
    height: usize,
    data: Vec<P>,
}

impl<P: Pixel + Default> ImageBuffer<P> {
    /// Create a zero-initialized image buffer with the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        let size = width.saturating_mul(height);
        Self {
            width,
            height,
            data: vec![P::default(); size],
        }
    }
}

impl<P: Pixel> ImageBuffer<P> {
    /// Construct an image buffer from pre-allocated contiguous pixel storage.
    pub fn from_raw(width: usize, height: usize, data: Vec<P>) -> Result<Self, ImageError> {
        let expected_len = width.checked_mul(height).ok_or_else(|| {
            ImageError::new(
                "Image dimensions overflow usize address space",
                format!("Width {} * Height {} exceeds memory bounds", width, height),
                "Reduce image resolution to fit within addressable memory",
            )
        })?;

        if data.len() != expected_len {
            return Err(ImageError::new(
                "Raw buffer length does not match specified dimensions",
                format!(
                    "Buffer length {} != expected width * height ({})",
                    data.len(),
                    expected_len
                ),
                "Verify pixel count equals width * height before constructing ImageBuffer",
            ));
        }

        Ok(Self {
            width,
            height,
            data,
        })
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn as_slice(&self) -> &[P] {
        &self.data
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [P] {
        &mut self.data
    }

    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> Option<P> {
        if x < self.width && y < self.height {
            Some(self.data[y * self.width + x])
        } else {
            None
        }
    }

    #[inline]
    pub fn put_pixel(&mut self, x: usize, y: usize, pixel: P) -> bool {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = pixel;
            true
        } else {
            false
        }
    }

    /// Convert the image buffer to single-channel 8-bit grayscale.
    pub fn to_grayscale(&self) -> ImageBuffer<Gray8> {
        let mut gray = ImageBuffer::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let p = self.data[y * self.width + x];
                gray.put_pixel(x, y, p.to_gray());
            }
        }
        gray
    }

    /// Resize image using nearest-neighbor interpolation.
    pub fn resize_nearest(
        &self,
        target_width: usize,
        target_height: usize,
    ) -> Result<Self, ImageError>
    where
        P: Default,
    {
        if target_width == 0 || target_height == 0 {
            return Err(ImageError::new(
                "Invalid resize target dimensions",
                format!(
                    "Target dimensions {}x{} must be non-zero",
                    target_width, target_height
                ),
                "Provide target width and height greater than 0",
            ));
        }

        let mut out = ImageBuffer::new(target_width, target_height);
        let x_ratio = (self.width as f64) / (target_width as f64);
        let y_ratio = (self.height as f64) / (target_height as f64);

        for y in 0..target_height {
            let src_y = ((y as f64 * y_ratio).floor() as usize).min(self.height - 1);
            for x in 0..target_width {
                let src_x = ((x as f64 * x_ratio).floor() as usize).min(self.width - 1);
                out.put_pixel(x, y, self.data[src_y * self.width + src_x]);
            }
        }

        Ok(out)
    }
}

impl ImageBuffer<Gray8> {
    /// Resize grayscale image using bilinear spatial interpolation.
    pub fn resize_bilinear(
        &self,
        target_width: usize,
        target_height: usize,
    ) -> Result<ImageBuffer<Gray8>, ImageError> {
        if target_width == 0 || target_height == 0 {
            return Err(ImageError::new(
                "Invalid bilinear target dimensions",
                format!(
                    "Target dimensions {}x{} must be non-zero",
                    target_width, target_height
                ),
                "Provide positive dimensions for image scaling",
            ));
        }

        let mut out = ImageBuffer::new(target_width, target_height);
        let x_ratio = if target_width > 1 {
            (self.width - 1) as f32 / (target_width - 1) as f32
        } else {
            0.0
        };
        let y_ratio = if target_height > 1 {
            (self.height - 1) as f32 / (target_height - 1) as f32
        } else {
            0.0
        };

        for y in 0..target_height {
            let src_y = y as f32 * y_ratio;
            let y_low = src_y.floor() as usize;
            let y_high = (y_low + 1).min(self.height - 1);
            let y_weight = src_y - y_low as f32;

            for x in 0..target_width {
                let src_x = x as f32 * x_ratio;
                let x_low = src_x.floor() as usize;
                let x_high = (x_low + 1).min(self.width - 1);
                let x_weight = src_x - x_low as f32;

                let p00 = self.data[y_low * self.width + x_low].0 as f32;
                let p10 = self.data[y_low * self.width + x_high].0 as f32;
                let p01 = self.data[y_high * self.width + x_low].0 as f32;
                let p11 = self.data[y_high * self.width + x_high].0 as f32;

                // Bilinear interpolation formula
                let top = p00 * (1.0 - x_weight) + p10 * x_weight;
                let bottom = p01 * (1.0 - x_weight) + p11 * x_weight;
                let val = top * (1.0 - y_weight) + bottom * y_weight;

                out.put_pixel(x, y, Gray8(val.round().clamp(0.0, 255.0) as u8));
            }
        }

        Ok(out)
    }
}

/// SIMD-vectorized 2D Convolution Kernel Engine.
pub struct Convolve2D;

impl Convolve2D {
    /// Execute a 3x3 convolution filter over a grayscale image with SIMD FMA acceleration.
    pub fn convolve_3x3_gray(
        src: &ImageBuffer<Gray8>,
        kernel: &[[f32; 3]; 3],
        bias: f32,
    ) -> ImageBuffer<Gray8> {
        let width = src.width;
        let height = src.height;
        let mut out = ImageBuffer::new(width, height);

        if width < 3 || height < 3 {
            return out;
        }

        // Flatten kernel row vectors for SIMD FMA accumulation
        let k0 = [kernel[0][0], kernel[0][1], kernel[0][2], 0.0];
        let k1 = [kernel[1][0], kernel[1][1], kernel[1][2], 0.0];
        let k2 = [kernel[2][0], kernel[2][1], kernel[2][2], 0.0];

        for y in 1..height - 1 {
            let row_prev = &src.data[(y - 1) * width..y * width];
            let row_curr = &src.data[y * width..(y + 1) * width];
            let row_next = &src.data[(y + 1) * width..(y + 2) * width];

            for x in 1..width - 1 {
                let r0 = [
                    row_prev[x - 1].0 as f32,
                    row_prev[x].0 as f32,
                    row_prev[x + 1].0 as f32,
                    0.0,
                ];
                let r1 = [
                    row_curr[x - 1].0 as f32,
                    row_curr[x].0 as f32,
                    row_curr[x + 1].0 as f32,
                    0.0,
                ];
                let r2 = [
                    row_next[x - 1].0 as f32,
                    row_next[x].0 as f32,
                    row_next[x + 1].0 as f32,
                    0.0,
                ];

                let mut acc0 = [0.0f32; 4];
                let mut acc1 = [0.0f32; 4];
                let mut acc2 = [0.0f32; 4];

                let _ = simd_fma_f32(&r0, &k0, &[0.0; 4], &mut acc0);
                let _ = simd_fma_f32(&r1, &k1, &acc0, &mut acc1);
                let _ = simd_fma_f32(&r2, &k2, &acc1, &mut acc2);

                let sum = acc2[0] + acc2[1] + acc2[2] + bias;
                let clamped = sum.round().clamp(0.0, 255.0) as u8;

                out.put_pixel(x, y, Gray8(clamped));
            }
        }

        out
    }

    /// Standard Sobel edge detection magnitude filter ($G = \sqrt{G_x^2 + G_y^2}$).
    pub fn sobel_edge_detector(src: &ImageBuffer<Gray8>) -> ImageBuffer<Gray8> {
        let width = src.width;
        let height = src.height;
        let mut out = ImageBuffer::new(width, height);

        if width < 3 || height < 3 {
            return out;
        }

        for y in 1..height - 1 {
            let row_prev = &src.data[(y - 1) * width..y * width];
            let row_curr = &src.data[y * width..(y + 1) * width];
            let row_next = &src.data[(y + 1) * width..(y + 2) * width];

            for x in 1..width - 1 {
                let p00 = row_prev[x - 1].0 as f32;
                let p02 = row_prev[x + 1].0 as f32;
                let p10 = row_curr[x - 1].0 as f32;
                let p12 = row_curr[x + 1].0 as f32;
                let p20 = row_next[x - 1].0 as f32;
                let p22 = row_next[x + 1].0 as f32;

                let p01 = row_prev[x].0 as f32;
                let p21 = row_next[x].0 as f32;

                let gx = (p02 - p00) + 2.0 * (p12 - p10) + (p22 - p20);
                let gy = (p20 - p00) + 2.0 * (p21 - p01) + (p22 - p02);

                let mag = (gx * gx + gy * gy).sqrt();
                out.put_pixel(x, y, Gray8(mag.round().clamp(0.0, 255.0) as u8));
            }
        }

        out
    }

    /// 3x3 Gaussian blur filter ($\sigma \approx 0.85$).
    pub fn gaussian_blur_3x3(src: &ImageBuffer<Gray8>) -> ImageBuffer<Gray8> {
        let kernel: [[f32; 3]; 3] = [
            [1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0],
            [2.0 / 16.0, 4.0 / 16.0, 2.0 / 16.0],
            [1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0],
        ];
        Self::convolve_3x3_gray(src, &kernel, 0.0)
    }

    /// Laplacian high-pass sharpening filter.
    pub fn laplacian_sharpen_3x3(src: &ImageBuffer<Gray8>) -> ImageBuffer<Gray8> {
        let kernel: [[f32; 3]; 3] = [[0.0, -1.0, 0.0], [-1.0, 5.0, -1.0], [0.0, -1.0, 0.0]];
        Self::convolve_3x3_gray(src, &kernel, 0.0)
    }
}

/// Binary Netpbm PPM (P6) & PGM (P5) Parser and Emitter.
pub struct NetpbmCodec;

impl NetpbmCodec {
    /// Encode an RGB image into Netpbm PPM (P6 binary) format.
    pub fn encode_ppm_p6(img: &ImageBuffer<Rgb8>) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", img.width, img.height);
        let mut bytes = Vec::with_capacity(header.len() + img.data.len() * 3);
        bytes.extend_from_slice(header.as_bytes());

        for pixel in &img.data {
            bytes.push(pixel.r);
            bytes.push(pixel.g);
            bytes.push(pixel.b);
        }
        bytes
    }

    /// Decode a Netpbm PPM (P6 binary) byte stream into an RGB image buffer.
    pub fn decode_ppm_p6(bytes: &[u8]) -> Result<ImageBuffer<Rgb8>, ImageError> {
        if !bytes.starts_with(b"P6") {
            return Err(ImageError::new(
                "Invalid PPM magic header",
                "Stream does not start with 'P6' binary signature",
                "Ensure input file is a valid Netpbm P6 PPM image",
            ));
        }

        let mut offset = 2;
        let width = Self::parse_next_int(bytes, &mut offset)?;
        let height = Self::parse_next_int(bytes, &mut offset)?;
        let maxval = Self::parse_next_int(bytes, &mut offset)?;

        if maxval != 255 {
            return Err(ImageError::new(
                "Unsupported PPM maxval scale",
                format!("Expected maxval 255, found {}", maxval),
                "Convert PPM image to standard 8-bit per channel color depth",
            ));
        }

        // Consume exactly 1 trailing whitespace byte after maxval
        if offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }

        let pixel_bytes = &bytes[offset..];
        let expected_bytes = width * height * 3;
        if pixel_bytes.len() < expected_bytes {
            return Err(ImageError::new(
                "Truncated PPM payload stream",
                format!(
                    "Expected {} bytes, received {}",
                    expected_bytes,
                    pixel_bytes.len()
                ),
                "Verify file is complete and not truncated during transmission",
            ));
        }

        let mut data = Vec::with_capacity(width * height);
        for chunk in pixel_bytes[..expected_bytes].chunks_exact(3) {
            data.push(Rgb8::new(chunk[0], chunk[1], chunk[2]));
        }

        ImageBuffer::from_raw(width, height, data)
    }

    /// Encode a Grayscale image into Netpbm PGM (P5 binary) format.
    pub fn encode_pgm_p5(img: &ImageBuffer<Gray8>) -> Vec<u8> {
        let header = format!("P5\n{} {}\n255\n", img.width, img.height);
        let mut bytes = Vec::with_capacity(header.len() + img.data.len());
        bytes.extend_from_slice(header.as_bytes());

        for pixel in &img.data {
            bytes.push(pixel.0);
        }
        bytes
    }

    /// Decode a Netpbm PGM (P5 binary) byte stream into a Grayscale image buffer.
    pub fn decode_pgm_p5(bytes: &[u8]) -> Result<ImageBuffer<Gray8>, ImageError> {
        if !bytes.starts_with(b"P5") {
            return Err(ImageError::new(
                "Invalid PGM magic header",
                "Stream does not start with 'P5' binary signature",
                "Ensure input file is a valid Netpbm P5 PGM image",
            ));
        }

        let mut offset = 2;
        let width = Self::parse_next_int(bytes, &mut offset)?;
        let height = Self::parse_next_int(bytes, &mut offset)?;
        let maxval = Self::parse_next_int(bytes, &mut offset)?;

        if maxval != 255 {
            return Err(ImageError::new(
                "Unsupported PGM maxval scale",
                format!("Expected maxval 255, found {}", maxval),
                "Convert PGM image to standard 8-bit depth",
            ));
        }

        if offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }

        let pixel_bytes = &bytes[offset..];
        let expected_bytes = width * height;
        if pixel_bytes.len() < expected_bytes {
            return Err(ImageError::new(
                "Truncated PGM payload stream",
                format!(
                    "Expected {} bytes, received {}",
                    expected_bytes,
                    pixel_bytes.len()
                ),
                "Verify file integrity before loading",
            ));
        }

        let data = pixel_bytes[..expected_bytes]
            .iter()
            .map(|&b| Gray8(b))
            .collect();

        ImageBuffer::from_raw(width, height, data)
    }

    fn parse_next_int(bytes: &[u8], offset: &mut usize) -> Result<usize, ImageError> {
        while *offset < bytes.len() {
            if bytes[*offset] == b'#' {
                // Skip comment line
                while *offset < bytes.len() && bytes[*offset] != b'\n' {
                    *offset += 1;
                }
            } else if bytes[*offset].is_ascii_whitespace() {
                *offset += 1;
            } else {
                break;
            }
        }

        let start = *offset;
        while *offset < bytes.len() && bytes[*offset].is_ascii_digit() {
            *offset += 1;
        }

        if start == *offset {
            return Err(ImageError::new(
                "Expected integer header parameter in Netpbm stream",
                format!("Failed parsing header token at byte offset {}", start),
                "Ensure PPM/PGM header adheres to Netpbm specification",
            ));
        }

        let token_str = std::str::from_utf8(&bytes[start..*offset]).map_err(|e| {
            ImageError::new(
                "Invalid UTF-8 token in Netpbm header",
                e.to_string(),
                "Sanitize header tokens to ASCII digits",
            )
        })?;

        token_str.parse::<usize>().map_err(|e| {
            ImageError::new(
                "Failed to parse Netpbm dimension as usize integer",
                e.to_string(),
                "Verify image dimensions are valid non-negative integers",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_buffer_creation_and_indexing() {
        let mut img = ImageBuffer::<Rgb8>::new(100, 50);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 50);

        let pixel = Rgb8::new(255, 128, 64);
        assert!(img.put_pixel(10, 20, pixel));
        assert_eq!(img.get_pixel(10, 20), Some(pixel));
        assert_eq!(img.get_pixel(100, 50), None);
    }

    #[test]
    fn test_sobel_edge_detector_identifies_contrast_boundary() {
        let mut src = ImageBuffer::<Gray8>::new(10, 10);
        // Vertical step edge: left black (0), right white (255)
        for y in 0..10 {
            for x in 5..10 {
                src.put_pixel(x, y, Gray8(255));
            }
        }

        let edges = Convolve2D::sobel_edge_detector(&src);
        // Edge detector should fire strongly on column 4 & 5
        let edge_val = edges.get_pixel(4, 5).map(|p| p.0).unwrap_or(0);
        assert!(
            edge_val > 200,
            "Sobel edge response must be strong on boundary, got {}",
            edge_val
        );
    }

    #[test]
    fn test_gaussian_blur_and_laplacian_sharpen_execution() {
        let mut src = ImageBuffer::<Gray8>::new(16, 16);
        src.put_pixel(8, 8, Gray8(255)); // Impulse center

        let blurred = Convolve2D::gaussian_blur_3x3(&src);
        let center_blurred = blurred.get_pixel(8, 8).map(|p| p.0).unwrap_or(255);
        let neighbor_blurred = blurred.get_pixel(7, 8).map(|p| p.0).unwrap_or(0);
        assert!(center_blurred < 255);
        assert!(neighbor_blurred > 0);

        let sharpened = Convolve2D::laplacian_sharpen_3x3(&src);
        assert_eq!(sharpened.width(), 16);
        assert_eq!(sharpened.height(), 16);
    }

    #[test]
    fn test_netpbm_ppm_p6_codec_roundtrip() -> Result<(), ImageError> {
        let mut img = ImageBuffer::<Rgb8>::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                img.put_pixel(x, y, Rgb8::new((x * 60) as u8, (y * 60) as u8, 128));
            }
        }

        let encoded = NetpbmCodec::encode_ppm_p6(&img);
        let decoded = NetpbmCodec::decode_ppm_p6(&encoded)?;

        assert_eq!(img, decoded);
        Ok(())
    }

    #[test]
    fn test_netpbm_pgm_p5_codec_roundtrip() -> Result<(), ImageError> {
        let mut img = ImageBuffer::<Gray8>::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                img.put_pixel(x, y, Gray8((x * y * 4) as u8));
            }
        }

        let encoded = NetpbmCodec::encode_pgm_p5(&img);
        let decoded = NetpbmCodec::decode_pgm_p5(&encoded)?;

        assert_eq!(img, decoded);
        Ok(())
    }

    #[test]
    fn test_bilinear_resampling_scales_dimensions() -> Result<(), ImageError> {
        let mut img = ImageBuffer::<Gray8>::new(2, 2);
        img.put_pixel(0, 0, Gray8(0));
        img.put_pixel(1, 0, Gray8(100));
        img.put_pixel(0, 1, Gray8(100));
        img.put_pixel(1, 1, Gray8(200));

        let scaled = img.resize_bilinear(4, 4)?;
        assert_eq!(scaled.width(), 4);
        assert_eq!(scaled.height(), 4);
        assert_eq!(scaled.get_pixel(0, 0), Some(Gray8(0)));
        assert_eq!(scaled.get_pixel(3, 3), Some(Gray8(200)));
        Ok(())
    }
}
