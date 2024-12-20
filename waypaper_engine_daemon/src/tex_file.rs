use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use bitflags::bitflags;
use cgmath::{InnerSpace, Vector2};
use image::ImageFormat;
use lz4_flex::decompress;
use num_enum_derive::TryFromPrimitive;

use crate::file_reading_utils::{
    read_color, read_f32, read_i32, read_null_terminated_str, read_u32,
};

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u32)]
pub enum TextureFormat {
    RGBA8888 = 0,
    DXT5 = 4,
    DXT3 = 6,
    DXT1 = 7,
    RG88 = 8,
    R8 = 9,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TextureFlags: u32 {
        const NoInterpolation = 1;
        const ClampUVs = 1 << 1;
        const IsSpritesheet = 1 << 2;
    }
}

pub struct Container {
    version: ContainerVersion,
    image_count: u32,
    freeimage_format: Option<FreeImageFormat>,
}

// This enum comes from FreeImage as Wallpaper Engine relies on it to provide us the image format
#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u32)]
pub enum FreeImageFormat {
    Bmp = 0,
    Ico = 1,
    Jpeg = 2,
    Jng = 3,
    Koala = 4,
    LbmOrIff = 5,
    Mng = 6,
    Pbm = 7,
    PbmRaw = 8,
    Pcd = 9,
    Pcx = 10,
    Pgm = 11,
    PgmRaw = 12,
    Png = 13,
    Ppm = 14,
    PpmRaw = 15,
    Ras = 16,
    Targa = 17,
    Tiff = 18,
    Wbmp = 19,
    Psd = 20,
    Cut = 21,
    Xbm = 22,
    Xpm = 23,
    Dds = 24,
    Gif = 25,
    Hdr = 26,
    // This format is disabled in FreeImage itself for security reasons, so it shouldn't be used in wallpaper engine textures anyway
    // FAXG3 = 27,
    Sgi = 28,
    Exr = 29,
    J2K = 30,
    Jp2 = 31,
    Pfm = 32,
    Pict = 33,
    Raw = 34,
    WebP = 35,
    Jxr = 36,
}

impl TryInto<ImageFormat> for FreeImageFormat {
    type Error = ();

    fn try_into(self) -> Result<ImageFormat, Self::Error> {
        match self {
            FreeImageFormat::Bmp => Ok(ImageFormat::Bmp),
            FreeImageFormat::Ico => Ok(ImageFormat::Ico),
            FreeImageFormat::Jpeg => Ok(ImageFormat::Jpeg),
            FreeImageFormat::Jng => Err(()),
            FreeImageFormat::Koala => Err(()),
            FreeImageFormat::LbmOrIff => Err(()),
            FreeImageFormat::Mng => Err(()),
            FreeImageFormat::Pbm => Ok(ImageFormat::Pnm),
            FreeImageFormat::PbmRaw => Ok(ImageFormat::Pnm),
            FreeImageFormat::Pcd => Err(()),
            FreeImageFormat::Pcx => Err(()),
            FreeImageFormat::Pgm => Ok(ImageFormat::Pnm),
            FreeImageFormat::PgmRaw => Ok(ImageFormat::Pnm),
            FreeImageFormat::Png => Ok(ImageFormat::Png),
            FreeImageFormat::Ppm => Ok(ImageFormat::Pnm),
            FreeImageFormat::PpmRaw => Ok(ImageFormat::Pnm),
            FreeImageFormat::Ras => Err(()),
            FreeImageFormat::Targa => Ok(ImageFormat::Tga),
            FreeImageFormat::Tiff => Ok(ImageFormat::Tiff),
            FreeImageFormat::Wbmp => Err(()),
            FreeImageFormat::Psd => Err(()),
            FreeImageFormat::Cut => Err(()),
            FreeImageFormat::Xbm => Err(()),
            FreeImageFormat::Xpm => Err(()),
            FreeImageFormat::Dds => Ok(ImageFormat::Dds),
            FreeImageFormat::Gif => Ok(ImageFormat::Gif),
            FreeImageFormat::Hdr => Ok(ImageFormat::Hdr),
            FreeImageFormat::Sgi => Err(()),
            FreeImageFormat::Exr => Ok(ImageFormat::OpenExr),
            FreeImageFormat::J2K => Err(()),
            FreeImageFormat::Jp2 => Err(()),
            FreeImageFormat::Pfm => Err(()),
            FreeImageFormat::Pict => Err(()),
            FreeImageFormat::Raw => Err(()),
            FreeImageFormat::WebP => Ok(ImageFormat::WebP),
            FreeImageFormat::Jxr => Err(()),
        }
    }
}

pub struct MipmapEntry {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerVersion {
    TEXB001,
    TEXB002,
    TEXB003,
}

impl TryFrom<&str> for ContainerVersion {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "TEXB0001" => Self::TEXB001,
            "TEXB0002" => Self::TEXB002,
            "TEXB0003" => Self::TEXB003,
            _ => return Err(()),
        })
    }
}

pub struct Header {
    format: TextureFormat,
    texture_flags: TextureFlags,
    texture_width: u32,
    texture_height: u32,
    image_width: u32,
    image_height: u32,
    dominant_color: (u8, u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameInfoContainerVersion {
    TEXS0001,
    TEXS0002,
    TEXS0003,
}

impl TryFrom<&str> for FrameInfoContainerVersion {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "TEXS0001" => Self::TEXS0001,
            "TEXS0002" => Self::TEXS0002,
            "TEXS0003" => Self::TEXS0003,
            _ => return Err(()),
        })
    }
}

pub struct FrameInfoContainer {
    version: FrameInfoContainerVersion,
    frame_infos: Vec<FrameInfo>,
    sprite_size: Option<Vector2<u32>>,
}

pub struct FrameInfo {
    image_id: i32,
    frame_time: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    x_axis: Vector2<f32>,
    y_axis: Vector2<f32>,
}

pub struct TexFile {
    header: Header,
    container: Container,
    images: Vec<Vec<MipmapEntry>>,
    frames_infos: Option<FrameInfoContainer>,
}

impl TexFile {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::debug!("Unpacking Tex File !");
        let data: Vec<u8> = fs::read(path)?;
        Self::from_bytes(data)
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut data = Cursor::new(bytes);
        let data_length = data.get_ref().len();
        tracing::debug!("Data Length : {data_length}");

        let header = read_header(&mut data);
        let container = read_container(&mut data);

        let images = read_images(&mut data, &container);

        let frames_infos = if header.texture_flags.contains(TextureFlags::IsSpritesheet) {
            tracing::debug!("Reading Frames Infos:");
            Some(read_frame_info(&mut data))
        } else {
            None
        };

        assert_eq!(data.position() as usize, data_length, "Malformed Tex File");

        Ok(Self {
            header,
            container,
            images,
            frames_infos,
        })
    }
}

fn read_header(data: &mut Cursor<Vec<u8>>) -> Header {
    let version = read_null_terminated_str(data);
    assert_eq!(version, "TEXV0005");
    let version2 = read_null_terminated_str(data);
    assert_eq!(version2, "TEXI0001");

    tracing::debug!("{version} - {version2}");

    let format = TextureFormat::try_from(read_u32(data)).unwrap();
    let flags = TextureFlags::from_bits(read_u32(data)).unwrap();
    let texture_width = read_u32(data);
    let texture_height = read_u32(data);
    let image_width = read_u32(data);
    let image_height = read_u32(data);
    let dominant_color = read_color(data);

    tracing::debug!("Texture info:");
    tracing::debug!("\tFormat: {format:?}");
    tracing::debug!("\tFlags: {flags:?}");
    tracing::debug!("\tTexture Size: {texture_width}x{texture_height}");
    tracing::debug!("\tImage Size: {image_width}x{image_height}");
    tracing::debug!("\tDominant Color: {dominant_color:?}");

    Header {
        format,
        texture_flags: flags,
        texture_width,
        texture_height,
        image_width,
        image_height,
        dominant_color,
    }
}

fn read_container(data: &mut Cursor<Vec<u8>>) -> Container {
    let version = ContainerVersion::try_from(read_null_terminated_str(data).as_str()).unwrap();
    tracing::debug!("Container version: {version:?}");

    let image_count = read_u32(data);
    let freeimage_format = match version {
        ContainerVersion::TEXB001 | ContainerVersion::TEXB002 => None,
        ContainerVersion::TEXB003 => {
            let format = read_i32(data);
            if format > 0 {
                Some(FreeImageFormat::try_from(format as u32).unwrap())
            } else {
                None
            }
        }
    };

    tracing::debug!("\tImage Count: {image_count}");
    match freeimage_format {
        None => tracing::debug!("\tImage Format: No format"),
        Some(ref format) => tracing::debug!("\tImage Format: {format:?}"),
    }

    Container {
        version,
        image_count,
        freeimage_format,
    }
}

fn read_mipmap(cursor: &mut Cursor<Vec<u8>>, container_version: &ContainerVersion) -> MipmapEntry {
    let width = read_u32(cursor);
    let height = read_u32(cursor);

    let (is_compressed, image_size_uncompressed) = match container_version {
        ContainerVersion::TEXB001 => (false, None),
        ContainerVersion::TEXB002 | ContainerVersion::TEXB003 => {
            let compression_flag = read_u32(cursor);
            assert!(compression_flag == 0 || compression_flag == 1);
            let is_compressed = compression_flag != 0;

            let image_size_uncompressed = read_u32(cursor);

            (is_compressed, Some(image_size_uncompressed))
        }
    };

    let image_size = read_u32(cursor);

    let mut bytes = vec![];

    if is_compressed {
        let mut raw_bytes = vec![];

        cursor
            .take(u64::from(image_size))
            .read_to_end(&mut raw_bytes)
            .unwrap();

        let uncompressed_size = image_size_uncompressed.unwrap() as usize;

        bytes = decompress(&raw_bytes, uncompressed_size).unwrap();
        assert_eq!(
            bytes.len(),
            uncompressed_size,
            "Failed texture decompression"
        );
    } else {
        cursor
            .take(u64::from(image_size))
            .read_to_end(&mut bytes)
            .unwrap();
    }

    tracing::debug!("\t\tWidth: {width}");
    tracing::debug!("\t\tHeight: {height}");
    tracing::debug!("\t\tIs Compressed: {is_compressed}");

    if is_compressed {
        tracing::debug!(
            "\t\tImage Size Uncompressed: {}",
            image_size_uncompressed.unwrap()
        );
    }

    tracing::debug!("\t\tImage Size: {image_size}",);

    MipmapEntry {
        width,
        height,
        bytes,
    }
}

fn read_images(data: &mut Cursor<Vec<u8>>, container: &Container) -> Vec<Vec<MipmapEntry>> {
    let mut images = vec![];

    for i in 0..container.image_count {
        tracing::debug!("Reading Image {i}: ");

        let mipmap_count = read_u32(data);
        tracing::debug!("\tMipmap Count: {mipmap_count}");

        let mut mipmap_entries = vec![];

        for i in 0..mipmap_count {
            tracing::debug!("\tReading Mipmap {i} :");
            mipmap_entries.push(read_mipmap(data, &container.version));
        }

        images.push(mipmap_entries);
    }

    images
}

fn read_frame_info(data: &mut Cursor<Vec<u8>>) -> FrameInfoContainer {
    let version =
        FrameInfoContainerVersion::try_from(read_null_terminated_str(data).as_str()).unwrap();

    tracing::debug!("\tFrame Info Container version: {version:?}");

    let frame_count = read_i32(data);
    tracing::debug!("\tFrame Count: {frame_count}");

    let sprite_size = match version {
        FrameInfoContainerVersion::TEXS0001 | FrameInfoContainerVersion::TEXS0002 => None,
        FrameInfoContainerVersion::TEXS0003 => Some(Vector2::new(read_u32(data), read_u32(data))),
    };

    tracing::debug!("\tSprite Size: {sprite_size:?}");

    let mut frames = vec![];

    for i in 0..frame_count {
        tracing::debug!("\tReading frame {i} infos:");

        let image_id = read_i32(data);

        let (frame_time, x, y, x_axis, y_axis) = match version {
            FrameInfoContainerVersion::TEXS0001 => (
                read_f32(data),
                read_i32(data) as f32,
                read_i32(data) as f32,
                Vector2::new(read_i32(data) as f32, read_i32(data) as f32),
                Vector2::new(read_i32(data) as f32, read_i32(data) as f32),
            ),
            FrameInfoContainerVersion::TEXS0002 | FrameInfoContainerVersion::TEXS0003 => (
                read_f32(data),
                read_f32(data),
                read_f32(data),
                Vector2::new(read_f32(data), read_f32(data)),
                Vector2::new(read_f32(data), read_f32(data)),
            ),
        };

        let width = x_axis.magnitude();
        let height = y_axis.magnitude();

        let frame = FrameInfo {
            image_id,
            frame_time,
            x,
            y,
            width: x_axis.magnitude(),
            height: y_axis.magnitude(),
            x_axis: x_axis / width,
            y_axis: y_axis / height,
        };

        tracing::debug!("\t\tImage ID: {}", frame.image_id);
        tracing::debug!("\t\tFrame Time: {}", frame.frame_time);
        tracing::debug!("\t\tX: {}", frame.x);
        tracing::debug!("\t\tY: {}", frame.y);
        tracing::debug!("\t\tX Axis: {:?}", frame.x_axis);
        tracing::debug!("\t\tY Axis: {:?}", frame.y_axis);
        tracing::debug!("\t\tWidth: {}", frame.width);
        tracing::debug!("\t\tHeight: {}", frame.height);

        frames.push(frame);
    }

    FrameInfoContainer {
        version,
        frame_infos: frames,
        sprite_size,
    }
}
