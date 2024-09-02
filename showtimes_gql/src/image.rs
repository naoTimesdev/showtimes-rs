//! A simple image/avatar handling

use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub const MAX_IMAGE_SIZE: u64 = 2 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 2048;

/// A simple image/avatar handling format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormatMini {
    /// PNG
    Png,
    /// JPEG
    Jpeg,
    /// GIF
    Gif,
    /// WEBP
    WebP,
    /// AVIF
    Avif,
}

impl ImageFormatMini {
    pub fn as_content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
        }
    }

    pub fn as_extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Avif => "avif",
        }
    }
}

impl From<ImageFormatMini> for image::ImageFormat {
    fn from(format: ImageFormatMini) -> Self {
        match format {
            ImageFormatMini::Png => image::ImageFormat::Png,
            ImageFormatMini::Jpeg => image::ImageFormat::Jpeg,
            ImageFormatMini::Gif => image::ImageFormat::Gif,
            ImageFormatMini::WebP => image::ImageFormat::WebP,
            ImageFormatMini::Avif => image::ImageFormat::Avif,
        }
    }
}

static MAGIC_BYTES: [(&[u8], ImageFormatMini); 7] = [
    (b"\x89PNG\r\n\x1a\n", ImageFormatMini::Png),
    (&[0xff, 0xd8, 0xff], ImageFormatMini::Jpeg),
    (b"GIF89a", ImageFormatMini::Gif),
    (b"GIF87a", ImageFormatMini::Gif),
    (b"RIFF", ImageFormatMini::WebP), // TODO: better magic byte detection, see https://github.com/image-rs/image/issues/660
    (b"\0\0\0 ftypavif", ImageFormatMini::Avif),
    (b"\0\0\0\x1cftypavif", ImageFormatMini::Avif),
];

pub async fn detect_upload_data(
    image_data: &mut tokio::fs::File,
) -> Result<ImageFormatMini, std::io::Error> {
    // Check file size, since we limit image size to just 2MB
    let metadata = image_data.metadata().await?;
    if metadata.len() > MAX_IMAGE_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Image size too large, max 2MB",
        ));
    }

    let mut buffer = [0u8; 8];
    image_data.read_exact(&mut buffer).await?;

    // Seek back to the start of the file
    image_data.seek(std::io::SeekFrom::Start(0)).await?;

    for (magic_bytes, format) in MAGIC_BYTES.iter() {
        if buffer.starts_with(magic_bytes) {
            // Complete buffer
            let mut complete_buffer = Vec::new();
            image_data.read_to_end(&mut complete_buffer).await?;

            let read_img = image::load_from_memory_with_format(&complete_buffer, (*format).into())
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid image format: {}", e),
                    )
                })?;

            // Check image dimensions
            if read_img.height() > MAX_IMAGE_DIMENSION {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Image height too large, max 2048px",
                ));
            }

            if read_img.width() > MAX_IMAGE_DIMENSION {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Image width too large, max 2048px",
                ));
            }

            // Return the format
            return Ok(*format);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Invalid image format",
    ))
}
