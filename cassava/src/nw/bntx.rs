use std::{collections::HashMap, num::NonZeroU32};

use bitfield_struct::bitfield;
use glam::UVec3;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use snafu::{OptionExt, ResultExt, Snafu, ensure};
use tegra_swizzle::{SwizzleError, surface::BlockDim};

use crate::{
	nw::{
		BinaryFileInfo, FileHeaderError, read_binary_file_header,
		res_dict::{DictError, read_dict},
		user_data::{UserData, UserDataError, read_gfx_user_data},
	},
	util::BinaryReader,
};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("failed to read file header"))]
	FileHeaderError {
		#[snafu(backtrace)]
		source: FileHeaderError,
	},
	UnsupportedVersion,
	#[snafu(display("expected magic to be 'BNTX\0\0\0\0'"))]
	IncorrectContainerMagic,
	#[snafu(transparent)]
	TextureError {
		#[snafu(backtrace)]
		source: DictError<TextureError>,
	},
}

#[derive(Snafu, Debug)]
pub enum TextureError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("expected magic to be 'BRTI'"))]
	IncorrectTextureInfoMagic { backtrace: snafu::Backtrace },
	#[snafu(display("invalid storage dimension"))]
	InvalidStorageDimension {
		storage_dimension: u8,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("invalid tile mode {tile_mode:X}"))]
	InvalidTileMode {
		tile_mode: u16,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("invalid channel format {channel_format:X}"))]
	InvalidChannelFormat {
		channel_format: u8,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("invalid type format {type_format:X}"))]
	InvalidTypeFormat {
		type_format: u8,
		backtrace: snafu::Backtrace,
	},
	#[snafu(transparent)]
	UserDataError {
		#[snafu(backtrace)]
		source: DictError<UserDataError>,
	},
	#[snafu(transparent)]
	SwizzleError {
		source: SwizzleError,
		backtrace: snafu::Backtrace,
	},
}

#[derive(FromPrimitive, ToPrimitive, Clone, Copy, Debug)]
pub enum StorageDimension {
	Type1D,
	Type2D,
	Type3D,
}

#[derive(FromPrimitive, ToPrimitive, Clone, Copy, Debug)]
pub enum ChannelFormat {
	None = 0x1,
	R8 = 0x2,
	R4G4B4A4 = 0x3,
	R5G5B5A1 = 0x5,
	A1B5G5R5 = 0x6,
	R5G6B5 = 0x7,
	B5G6R5 = 0x8,
	R8G8 = 0x9,
	R16 = 0xa,
	R8G8B8A8 = 0xb,
	B8G8R8A8 = 0xc,
	R9G9B9E5F = 0xd,
	R10G10B10A2 = 0xe,
	R11G11B10F = 0xf,
	R16G16 = 0x12,
	D24S8 = 0x13,
	R32 = 0x14,
	R16G16B16A16 = 0x15,
	D32FS8 = 0x16,
	R32G32 = 0x17,
	R32G32B32 = 0x18,
	R32G32B32A32 = 0x19,
	BC1 = 0x1a,
	BC2 = 0x1b,
	BC3 = 0x1c,
	BC4 = 0x1d,
	BC5 = 0x1e,
	BC6H = 0x1f,
	BC7U = 0x20,
	Astc4x4 = 0x2d,
	Astc5x4 = 0x2e,
	Astc5x5 = 0x2f,
	Astc6x5 = 0x30,
	Astc6x6 = 0x31,
	Astc8x5 = 0x32,
	Astc8x6 = 0x33,
	Astc8x8 = 0x34,
	Astc10x5 = 0x35,
	Astc10x6 = 0x26,
	Astc10x8 = 0x37,
	Astc10x10 = 0x38,
	Astc12x10 = 0x39,
	Astc12x12 = 0x3a,
	B5G5R5A1 = 0x3b,
}

impl ChannelFormat {
	fn stride(&self) -> u32 {
		match self {
			ChannelFormat::None => 0,
			ChannelFormat::R8 => 1,
			ChannelFormat::R4G4B4A4 => 2,
			ChannelFormat::R5G5B5A1 => 2,
			ChannelFormat::A1B5G5R5 => 2,
			ChannelFormat::R5G6B5 => 2,
			ChannelFormat::B5G6R5 => 2,
			ChannelFormat::R8G8 => 2,
			ChannelFormat::R16 => 2,
			ChannelFormat::R8G8B8A8 => 4,
			ChannelFormat::B8G8R8A8 => 4,
			ChannelFormat::R9G9B9E5F => 4,
			ChannelFormat::R10G10B10A2 => 4,
			ChannelFormat::R11G11B10F => 4,
			ChannelFormat::R16G16 => 4,
			ChannelFormat::D24S8 => 4,
			ChannelFormat::R32 => 4,
			ChannelFormat::R16G16B16A16 => 8,
			ChannelFormat::D32FS8 => 8,
			ChannelFormat::R32G32 => 8,
			ChannelFormat::R32G32B32 => 12,
			ChannelFormat::R32G32B32A32 => 16,
			ChannelFormat::BC1 | ChannelFormat::BC4 => 8,
			ChannelFormat::BC2 | ChannelFormat::BC3 => 16,
			ChannelFormat::BC5 => 16,
			ChannelFormat::BC6H => 16,
			ChannelFormat::BC7U => 16,
			ChannelFormat::Astc4x4
			| ChannelFormat::Astc5x4
			| ChannelFormat::Astc5x5
			| ChannelFormat::Astc6x5
			| ChannelFormat::Astc6x6
			| ChannelFormat::Astc8x5
			| ChannelFormat::Astc8x6
			| ChannelFormat::Astc8x8
			| ChannelFormat::Astc10x5
			| ChannelFormat::Astc10x6
			| ChannelFormat::Astc10x8
			| ChannelFormat::Astc10x10
			| ChannelFormat::Astc12x10
			| ChannelFormat::Astc12x12 => 16,
			ChannelFormat::B5G5R5A1 => 2,
		}
	}
}

#[derive(FromPrimitive, ToPrimitive, Clone, Copy, Debug)]
pub enum TypeFormat {
	Unorm = 1,
	Snorm = 2,
	UInt = 3,
	SInt = 4,
	Float = 5,
	SRGB = 6,
	Depth = 7,
	UScaled = 8,
	SScaled = 9,
	UF32 = 10,
}

pub struct FormatInfo {
	pub bytes_per_pixel: u32,
	pub block_dim: BlockDim,
}

impl FormatInfo {
	const fn block(
		bytes_per_pixel: u32,
		block_width: u32,
		block_height: u32,
		block_depth: u32,
	) -> FormatInfo {
		Self {
			bytes_per_pixel,
			block_dim: BlockDim {
				width: NonZeroU32::new(block_width).unwrap(),
				height: NonZeroU32::new(block_height).unwrap(),
				depth: NonZeroU32::new(block_depth).unwrap(),
			},
		}
	}

	pub fn from_image_format(channel_format: ChannelFormat, type_format: TypeFormat) -> FormatInfo {
		match (channel_format, type_format) {
			(ChannelFormat::BC1, _) => Self::block(8, 4, 4, 1),
			(ChannelFormat::BC2, _) => Self::block(16, 4, 4, 1),
			(ChannelFormat::BC3, _) => Self::block(16, 4, 4, 1),
			(ChannelFormat::BC4, _) => Self::block(8, 4, 4, 1),
			(ChannelFormat::BC5, _) => Self::block(16, 4, 4, 1),
			(ChannelFormat::BC6H, _) => Self::block(16, 4, 4, 1),
			(ChannelFormat::BC7U, _) => Self::block(16, 4, 4, 1),
			(ChannelFormat::Astc4x4, _) => Self::block(16, 4, 4, 1),
			(ChannelFormat::Astc5x4, _) => Self::block(16, 5, 4, 1),
			(ChannelFormat::Astc5x5, _) => Self::block(16, 5, 5, 1),
			(ChannelFormat::Astc6x5, _) => Self::block(16, 6, 5, 1),
			(ChannelFormat::Astc6x6, _) => Self::block(16, 6, 6, 1),
			(ChannelFormat::Astc8x5, _) => Self::block(16, 8, 5, 1),
			(ChannelFormat::Astc8x6, _) => Self::block(16, 8, 6, 1),
			(ChannelFormat::Astc8x8, _) => Self::block(16, 8, 8, 1),
			(ChannelFormat::Astc10x5, _) => Self::block(16, 10, 5, 1),
			(ChannelFormat::Astc10x6, _) => Self::block(16, 10, 6, 1),
			(ChannelFormat::Astc10x8, _) => Self::block(16, 10, 8, 1),
			(ChannelFormat::Astc10x10, _) => Self::block(16, 10, 10, 1),
			(ChannelFormat::Astc12x10, _) => Self::block(16, 12, 10, 1),
			(ChannelFormat::Astc12x12, _) => Self::block(16, 12, 12, 1),
			(channel_format, _) => Self::block(channel_format.stride(), 1, 1, 1),
		}
	}
}

#[derive(FromPrimitive, ToPrimitive, Clone, Copy, Debug)]
pub enum TileMode {
	Optimal,
	Linear,
}

#[bitfield(u32)]
pub struct GpuAccessFlags {
	#[bits(2)]
	_reserved: u8,
	pub is_vertex_buffer: bool,
	pub index_buffer: bool,
	pub uniform_buffer: bool,
	pub texture: bool,
	pub storage_buffer: bool,
	pub render_target_color: bool,
	pub render_target_depth: bool,
	pub indirect_draw: bool,
	pub display_texture: bool,
	pub counter: bool,
	#[bits(20)]
	_reserved: u32,
	// VertexBuffer = 1 << 2,
	// IndexBuffer = 1 << 3,
	// UniformBuffer = 1 << 4,
	// Texture = 1 << 5,
	// StorageBuffer = 1 << 6,
	// RenderTargetColor = 1 << 7,
	// RenderTargetDepth = 1 << 8,
	// IndirectDraw = 1 << 9,
	// DisplayTexture = 1 << 10,
	// Counter = 1 << 11,
}

#[derive(Clone, Debug)]
pub struct GfxTextureInfo {
	pub packaged: bool,
	pub sparse_binding: bool,
	pub sparse: bool,
	pub res_texture: bool,
	pub storage_dimension: StorageDimension,
	pub tile_mode: TileMode,
	pub swizzle: u16,
	pub mip_levels: u16,
	pub sample_count: u16,
	pub type_format: TypeFormat,
	pub channel_format: ChannelFormat,
	pub gpu_access_flags: GpuAccessFlags,
	pub width: u32,
	pub height: u32,
	pub depth: u32,
	pub array_layers: u32,
	pub packaged_texture_layout: u32,
}

impl GfxTextureInfo {
	/// For ease of pattern matching on the image format of a texture
	pub fn image_format(&self) -> (ChannelFormat, TypeFormat) {
		(self.channel_format, self.type_format)
	}
}

#[derive(Clone, Debug)]
pub struct TextureInfo {
	pub gfx: GfxTextureInfo,
	pub data_alignment: u32,
	pub channel_sources: [u8; 4],
	pub image_dimension: u8,
	pub texture_name: String,
	// pub mipmaps: Vec<Vec<Vec<u8>>>,
	pub image_data: Vec<u8>,
	pub mip_offsets: Vec<usize>,
	pub user_data: HashMap<String, UserData>,
}

#[derive(Debug)]
pub struct TextureArchive {
	pub file_info: BinaryFileInfo,
	pub textures: HashMap<String, TextureInfo>,
}

const DICT: [u8; 4] = *b"_DIC";

impl TextureArchive {
	pub fn read(reader: &mut BinaryReader) -> Result<TextureArchive, Error> {
		let (file_info, _file_header, mut reader) =
			read_binary_file_header(reader, Some(*b"BNTX\0\0\0\0")).context(FileHeaderSnafu)?;

		ensure!(
			reader.read_slice(4)? == b"NX  ",
			IncorrectContainerMagicSnafu
		);

		let _texture_count = reader.read_u32()? as usize;
		let texture_array = reader.read_u64()? as usize;
		reader.seek_rel(8)?;
		let texture_dictionary = reader.read_u64()? as usize;
		let textures = read_dict::<TextureInfo, TextureError>(
			&mut reader,
			DICT,
			texture_array,
			texture_dictionary,
			|reader, texture_name| {
				let offset = reader.read_u64()? as usize;
				reader.seek(offset)?;
				if reader.read_bytes::<4>()? != *b"BRTI" {
					return Err(IncorrectTextureInfoMagicSnafu.build());
				}

				reader.seek_rel(12)?;
				let bitfield = reader.read_u8()?;
				let storage_dimension = reader.read_u8()?;
				let storage_dimension = StorageDimension::from_u8(storage_dimension)
					.context(InvalidStorageDimensionSnafu { storage_dimension })?;
				let tile_mode = reader.read_u16()?;
				let tile_mode =
					TileMode::from_u16(tile_mode).context(InvalidTileModeSnafu { tile_mode })?;
				let (swizzle, mip_levels, sample_count) =
					(reader.read_u16()?, reader.read_u16()?, reader.read_u16()?);
				reader.seek_rel(2)?;
				let image_format = reader.read_u32()?;
				let (type_format, channel_format) = (
					TypeFormat::from_u32(image_format & 0xFF).context(InvalidTypeFormatSnafu {
						type_format: (image_format & 0xFF) as u8,
					})?,
					ChannelFormat::from_u32((image_format & 0xFF00) >> 8).context(
						InvalidChannelFormatSnafu {
							channel_format: ((image_format & 0xFF00) >> 8) as u8,
						},
					)?,
				);
				let gpu_access_flags = GpuAccessFlags::from_bits(reader.read_u32()?);
				let (width, height, depth) = (reader.read_u32()?, reader.read_u32()?, reader.read_u32()?);
				let array_layers = reader.read_u32()?;
				let packaged_texture_layout = reader.read_u32()?;

				let gfx_info = GfxTextureInfo {
					packaged: bitfield & 0b0001 != 0,
					sparse_binding: bitfield & 0b0010 != 0,
					sparse: bitfield & 0b100 != 0,
					res_texture: bitfield & 0b1000 != 0,
					storage_dimension,
					tile_mode,
					swizzle,
					mip_levels,
					sample_count,
					type_format,
					channel_format,
					gpu_access_flags,
					width,
					height,
					depth,
					array_layers,
					packaged_texture_layout,
				};

				let _packaged_texture_layout_2 = reader.read_u32()?;
				reader.seek_rel(0x14)?;
				let total_texture_size = reader.read_u32()? as usize;
				let data_alignment = reader.read_u32()?;
				let channel_sources = reader.read_bytes::<4>()?;
				let image_dimension = reader.read_u8()?;
				reader.seek_rel(3)?;
				let _texture_name = reader.read_u64()? as usize;
				reader.seek_rel(8)?;
				let mipmaps = reader.read_u64()? as usize;
				let image_data = reader.scope::<_, TextureError>(mipmaps, |reader| {
					let start = reader.read_u64()? as usize;
					reader.seek(start)?;
					let swizzled_data = reader.read_slice(total_texture_size)?;
					let format_info = FormatInfo::from_image_format(channel_format, type_format);
					Ok(tegra_swizzle::surface::deswizzle_surface(
						width,
						height,
						depth,
						swizzled_data,
						format_info.block_dim,
						None,
						format_info.bytes_per_pixel,
						mip_levels as _,
						array_layers as _,
					)?)
				})?;
				let mip_offsets = reader
					.read_boxed_array(mip_levels as usize * array_layers as usize, |reader, _| {
						reader.read_u64().map(|value| value as usize)
					})?
					.into();
				let gfx_user_data_values = reader.read_u64()? as usize;
				reader.seek_rel(24)?;
				let gfx_user_data_dict = reader.read_u64()? as usize;

				let user_data = read_gfx_user_data(reader, DICT, gfx_user_data_values, gfx_user_data_dict)?;

				Ok(TextureInfo {
					gfx: gfx_info,
					data_alignment,
					channel_sources,
					image_dimension,
					texture_name: texture_name.to_owned(),
					// mipmaps,
					image_data,
					mip_offsets,
					user_data,
				})
			},
		)?;

		Ok(TextureArchive {
			file_info,
			textures,
		})
	}
}

impl TextureInfo {
	pub fn dimensions(&self) -> UVec3 {
		UVec3::new(self.gfx.width, self.gfx.height, self.gfx.depth)
	}
}
