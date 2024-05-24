use crate::file_reading_utils::{
    read_color, read_f32, read_i32, read_null_terminated_str, read_u32,
};
use bitflags::bitflags;

use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum TextureFormat {
    RGBA8888 = 0,
    DXT5 = 4,
    DXT3 = 6,
    DXT1 = 7,
    RG88 = 8,
    R8 = 9,
}

impl TryFrom<u32> for TextureFormat {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::RGBA8888,
            4 => Self::DXT5,
            6 => Self::DXT3,
            7 => Self::DXT1,
            8 => Self::RG88,
            9 => Self::R8,
            _ => return Err(()),
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TextureFlags: u32 {
        const NoInterpolation = 1;
        const ClampUVs = 1 << 1;
        const IsGIF = 1 << 2;
    }
}

pub struct Container {
    version: ContainerVersion,
    image_count: u32,
    freeimage_format: Option<u32>,
}

pub struct MipmapEntry {
    width: u32,
    height: u32,
    is_compressed: bool,
    image_size_uncompressed: Option<u32>,
    image_size: u32,
    mipmap_pixels: Vec<u8>,
}

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
    gif_width: Option<u32>,
    gif_height: Option<u32>,
}

struct FrameInfo {
    image_id: i32,
    frame_time: f32,
    x: f32,
    y: f32,
    width: f32,
    width_y: f32,
    height_x: f32,
    height: f32,
}

pub struct TexFile {
    header: Header,
    container: Container,
    images: Vec<Vec<MipmapEntry>>,
    frames_infos: Option<FrameInfoContainer>,
}

impl TexFile {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::debug!("Unpacking Tex File !");

        let mut data: Cursor<Vec<u8>> = Cursor::new(fs::read(path)?);
        let data_length = data.get_ref().len();
        tracing::debug!("Data Length : {data_length}");

        let header = read_header(&mut data);
        let container = read_container(&mut data);

        let mut images = vec![];

        for image in 0..container.image_count {
            tracing::debug!("Reading Image {image}: ");

            let mipmap_count = read_u32(&mut data);
            let mut mipmap_entries = vec![];

            tracing::debug!("\tMipmap Count: {mipmap_count}");

            for i in 0..mipmap_count {
                tracing::debug!("\tReading Mipmap {i} :");
                mipmap_entries.push(read_mipmap(&mut data, &container.version));
            }

            images.push(mipmap_entries);
        }

        let frames_infos = if header.texture_flags.contains(TextureFlags::IsGIF) {
            tracing::debug!("Reading Frameinfo:");
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
    let container_version_str = read_null_terminated_str(data);
    tracing::debug!("Container version: {container_version_str}");

    let version = ContainerVersion::try_from(container_version_str.as_str()).unwrap();

    let image_count = read_u32(data);
    let freeimage_format = match version {
        ContainerVersion::TEXB001 | ContainerVersion::TEXB002 => None,
        ContainerVersion::TEXB003 => Some(read_u32(data)),
    };

    tracing::debug!("\tImage Count: {image_count}");
    if let Some(format) = freeimage_format {
        tracing::debug!("\tFreeimage Format: {format}");
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

    let mut bytes = vec![];
    cursor
        .take(u64::from(image_size))
        .read_to_end(&mut bytes)
        .unwrap();

    MipmapEntry {
        width,
        height,
        is_compressed,
        image_size_uncompressed,
        image_size,
        mipmap_pixels: vec![],
    }
}

fn read_frame_info(data: &mut Cursor<Vec<u8>>) -> FrameInfoContainer {
    let version =
        FrameInfoContainerVersion::try_from(read_null_terminated_str(data).as_str()).unwrap();

    let (gif_width, gif_height) = match version {
        FrameInfoContainerVersion::TEXS0001 | FrameInfoContainerVersion::TEXS0002 => (None, None),
        FrameInfoContainerVersion::TEXS0003 => (Some(read_u32(data)), Some(read_u32(data))),
    };
    
    let frame_count = read_u32(data);
    tracing::debug!("\tFrame Count: {frame_count}");
    
    
    let mut frames = vec![];

    for i in 0..frame_count {
        tracing::debug!("\tReading frame {i} infos:");
        
        let frame = match version {
            FrameInfoContainerVersion::TEXS0001 => FrameInfo {
                image_id: read_i32(data),
                frame_time: read_f32(data),
                x: read_i32(data) as f32,
                y: read_i32(data) as f32,
                width: read_i32(data) as f32,
                width_y: read_i32(data) as f32,
                height_x: read_i32(data) as f32,
                height: read_i32(data) as f32,
            },
            FrameInfoContainerVersion::TEXS0002 | FrameInfoContainerVersion::TEXS0003 => {
                FrameInfo {
                    image_id: read_i32(data),
                    frame_time: read_f32(data),
                    x: read_f32(data),
                    y: read_f32(data),
                    width: read_f32(data),
                    width_y: read_f32(data),
                    height_x: read_f32(data),
                    height: read_f32(data),
                }
            }
        };
        
        tracing::debug!("\t\tImage ID: {}", frame.image_id);
        tracing::debug!("\t\tFrame Time: {}", frame.frame_time);
        tracing::debug!("\t\tX: {}", frame.x);
        tracing::debug!("\t\tY: {}", frame.y);
        tracing::debug!("\t\tWidth: {}", frame.width);
        tracing::debug!("\t\tWidth Y: {}", frame.width_y);
        tracing::debug!("\t\tHeight X: {}", frame.height_x);
        tracing::debug!("\t\tHeight: {}", frame.height);
        
        frames.push(frame);
    }
    
    FrameInfoContainer {
        version,
        frame_infos: frames,
        gif_width,
        gif_height,
    }
}
