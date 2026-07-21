use std::collections::HashMap;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
	nw::{
		BinaryFileInfo, FileHeaderError, read_binary_file_header,
		res_dict::{Dict, DictError},
		user_data::UserData,
	},
	util::{self, BinaryReader, ByteOrder},
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
		source: FileHeaderError,
	},
	UnsupportedVersion,
	#[snafu(display("expected magic to be 'BNTX\0\0\0\0'"))]
	IncorrectContainerMagic,
	#[snafu(transparent)]
	ModelError {
		source: DictError<ModelError>,
	},
}

#[derive(Snafu, Debug)]
pub enum ModelError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectModelMagic,
	#[snafu(transparent)]
	VertexInfoError {
		source: VertexInfoError,
	},
	#[snafu(transparent)]
	ShapeError {
		source: DictError<ShapeError>,
	}
}
#[derive(Snafu, Debug)]
pub enum VertexInfoError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectVertexMagic,
	#[snafu(transparent)]
	VertexAttribError {
		source: DictError<VertexAttribError>,
	},
}

#[derive(Snafu, Debug)]
pub enum VertexAttribError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	InvalidAttribFormat,
}

#[derive(Snafu, Debug)]
pub enum ShapeError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectShapeMagic,
	#[snafu(transparent)]
	VertexInfoError {
		source: VertexInfoError,
	},
}

#[derive(Debug)]
pub struct ModelArchive {
	pub file_info: BinaryFileInfo,
	pub file_name: String,
	pub models: Dict<Model>,
}

#[derive(Debug)]
pub struct Model {
	pub skeleton: Skeleton,
	pub vertex_info: VertexInfo,
	pub shapes: Dict<Shape>,
	pub materials: Dict<Material>,
	pub user_data: Dict<UserData>,
}

#[derive(Debug)]
pub struct Skeleton;

#[derive(Debug)]
pub struct VertexInfo {
	pub attributes: Dict<VertexAttrib>,
	pub buffers: Vec<VertexBuffer>,
}

pub struct VertexBuffer {
	pub stride: u32,
	pub access_flags: u32,
	pub data: Vec<u8>,
}

impl std::fmt::Debug for VertexBuffer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("VertexBuffer")
			.field("stride", &self.stride)
			.field("access_flags", &self.access_flags)
			.field("len(data)", &self.data.len())
			.finish()
	}
}
#[derive(Debug)]
pub struct VertexAttrib {
	pub format: VertexAttribFormat,
	pub buffer_offset: u16,
	pub buffer_index: u16,
}
#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexAttribFormat {
	// 8 bits (8 x 1)
	Unorm8 = 0x102,
	U8 = 0x302,
	Snorm8 = 0x202,
	I8 = 0x402,
	U8F = 0x802,
	I8F = 0xA02,
	// 8 bits (4 x 2)
	Vec2Unorm4 = 0x001,
	// 16 bits (16 x 1)
	Unorm16 = 0x10A,
	U16 = 0x20A,
	Snorm16 = 0x30A,
	I16 = 0x40A,
	F16 = 0x50A,
	U16F = 0x803,
	I16F = 0xA03,
	// 16 bits (8 x 2)
	Vec2Unorm8 = 0x109,
	Vec2U8 = 0x309,
	Vec2Snorm8 = 0x209,
	Vec2I8 = 0x409,
	Vec2U8F = 0x804,
	Vec2I8F = 0xA04,
	// 32 bits (16 x 2)
	Vec2Unorm16 = 0x112,
	Vec2Snorm16 = 0x212,
	Vec2U16 = 0x312,
	Vec2I16 = 0x412,
	Vec2F16 = 0x512,
	Vec2U16F = 0x807,
	Vec2I16F = 0xA07,
	// 32 bits (10/11 x 3)
	Vec3F10F11F11 = 0x809,
	// 32 bits (8 x 4)
	Vec4Unorm8 = 0x10B,
	Vec4Snorm8 = 0x20B,
	Vec4U8 = 0x30B,
	Vec4I8 = 0x40B,
	Vec4U8F = 0x80B,
	Vec4I8F = 0xA0B,
	// 32 bits (10 x 3 + 2)
	Vec4Unorm10_2 = 0x00B,
	Vec4U10_2 = 0x90B,
	Vec4Snorm10x3Unorm2 = 0x20E,
	Vec4I10_2 = 0x99B,
	// 64 bits (16 x 4)
	Vec4Unorm16 = 0x115,
	Vec4Snorm16 = 0x215,
	Vec4U16 = 0x315,
	Vec4I16 = 0x415,
	Vec4F16 = 0x515,
	Vec4U32F = 0x80E,
	Vec4S32F = 0xA0E,
	// 32 bits (32 x 1)
	U32 = 0x314,
	I32 = 0x416,
	F32 = 0x516,
	// 64 bits (32 x 2)
	Vec2U32 = 0x317,
	Vec2I32 = 0x417,
	Vec2F32 = 0x517,
	// 96 bits (32 x 3)
	Vec3U32 = 0x318,
	Vec3I32 = 0x418,
	Vec3F32 = 0x518,
	// 128 bits (32 x 4)
	Vec4U32 = 0x319,
	Vec4I32 = 0x419,
	Vec4F32 = 0x519,
}
#[derive(Debug)]
pub struct Shape {
	vertex_info: VertexInfo,
}
#[derive(Debug)]
pub struct Material {}

impl ModelArchive {
	pub fn read(reader: &mut BinaryReader) -> Result<ModelArchive, Error> {
		let (file_info, _file_header) =
			read_binary_file_header(reader, Some(*b"FRES    ")).context(FileHeaderSnafu)?;

		let name_offset = reader.read_u64()?;
		let file_name = reader.scope::<_, Error>(name_offset as _, |reader| {
			let name_size = reader.read_u16()?;
			Ok(reader.read_string(name_size as _)?.to_owned())
		})?;
		let (model_array, model_dictionary) = (reader.read_u64()? as _, reader.read_u64()? as _);
		let models = read_dict::<_, ModelError>(reader, model_array, model_dictionary, |reader, _| {
			ensure!(
				reader.read_bytes::<4>()? == *b"FMDL",
				IncorrectModelMagicSnafu
			);

			reader.seek_rel(12)?;
			let _name_offset = reader.read_u64()?;
			let _ = reader.read_u64()?;
			let _skeleton_offset = reader.read_u64()? as usize;
			let vertex_info_offset = reader.read_u64()? as usize;
			let vertex_info = reader.scope(vertex_info_offset, Self::read_vertex_info)?;
		let (shape_array, shape_dictionary) = (reader.read_u64()? as _, reader.read_u64()? as _);
		let shapes = read_dict(reader, shape_array, shape_dictionary, |reader, _| {
Self::read_shape(reader)
		})?;

			Ok(Model {
				skeleton: Skeleton,
				vertex_info,
				shapes,
				materials: Dict::new(),
				user_data: Dict::new(),
			})
		})?;

		Ok(ModelArchive {
			file_info,
			file_name,
			models,
		})
	}

	fn read_vertex_info(reader: &mut BinaryReader) -> Result<VertexInfo, VertexInfoError> {
		let magic = reader.read_bytes::<4>()?;
		let start = reader.position();
		// println!("{:04X} {}", start, String::from_utf8_lossy(&magic));
		ensure!(magic == *b"FVTX", IncorrectVertexMagicSnafu);
		reader.seek_rel(12)?;

		let (attrib_array, attrib_dictionary) = (reader.read_u64()? as _, reader.read_u64()? as _);
		// println!("{attrib_array:X} {attrib_dictionary:X}");

		let attributes = read_dict::<_, VertexAttribError>(
			reader,
			attrib_array,
			attrib_dictionary,
			|reader, dict_name| {
				let name_offset = reader.read_u64()? as _;

				let name = reader.scope(name_offset, |reader| {
					let name_size = reader.read_u16()?;
					reader.read_string(name_size as _).map(ToOwned::to_owned)
				})?;
				assert!(name == dict_name);
				let mut format = reader.read_u16()?;
				if reader.byte_order() == ByteOrder::Little {
					format = format.swap_bytes();
				}
				let format = VertexAttribFormat::from_u16(format).context(InvalidAttribFormatSnafu)?;
				let buffer_offset = reader.read_u16()?;
				let buffer_index = reader.read_u16()?;
				reader.seek_rel(2)?;

				Ok(VertexAttrib {
					format,
					buffer_offset,
					buffer_index,
				})
			},
		)?;

		// println!("{:04X}", reader.position() - start);
		reader.seek_rel(0x18)?;
		let buffer_sizes = reader.read_u64()? as usize;
		let buffer_stride = reader.read_u64()? as usize;
		reader.seek_rel(8)?;
		let buffer_offset = reader.read_u32()?;
		let _vertex_attrib_count = reader.read_u8()?;
		let buffer_count = reader.read_u8()?;
		reader.seek_rel(2)?;
		let vertex_count = reader.read_u32()?;
		let vertex_skin_count = reader.read_u32()?;
		reader.seek_rel(2)?;

		// println!(
		// 	"{buffer_sizes:04X} {buffer_stride:04X} {buffer_offset:04X} {_vertex_attrib_count:X} {buffer_count:04X}"
		// );

		let mut start_offset = 0;
		let buffers = reader
			.read_boxed_array::<_, VertexInfoError>(buffer_count as _, |reader, index| {
				let (size, access_flags) = reader.scope::<_, VertexInfoError>(buffer_sizes, |reader| {
					reader.seek_rel(index as isize * 0x10)?;
					Ok((reader.read_u32()?, reader.read_u32()?))
				})?;
				let stride = reader.scope(buffer_stride, |reader| {
					reader.seek_rel(index as isize * 0x10)?;
					reader.read_u32()
				})?;
				let data = reader.scope(start_offset as _, |reader| {
					reader.read_slice(size as usize).map(<[u8]>::to_vec)
				})?;

				start_offset += size;

				Ok(VertexBuffer {
					stride,
					access_flags,
					data,
				})
			})?
			.into_vec();

		Ok(VertexInfo {
			attributes: attributes,
			buffers,
		})
	}

	fn read_shape(reader: &mut BinaryReader) -> Result<Shape, ShapeError> {
		let magic = reader.read_bytes::<4>()?;
		let start = reader.position();
		// println!("{:04X} {}", start, String::from_utf8_lossy(&magic));
		ensure!(magic == *b"FSHP", IncorrectShapeMagicSnafu);
		reader.seek_rel(12)?;
		let name_offset = reader.read_u64()? as _;
		let name = reader.scope(name_offset, |reader| {
			let name_size = reader.read_u16()?;
			reader.read_string(name_size as _).map(ToOwned::to_owned)
		})?;

		let vertex_info_offset = reader.read_u64()? as usize;
		let vertex_info = reader.scope(vertex_info_offset, Self::read_vertex_info)?;
		println!("{name}");

		Ok(Shape {
			vertex_info,
		})
	}
}

pub fn read_dict<T, E: snafu::Error + From<util::Error>>(
	reader: &mut BinaryReader,
	value_list: usize,
	dictionary_list: usize,
	mut element_reader: impl FnMut(&mut BinaryReader, &str) -> Result<T, E>,
) -> Result<Dict<T>, DictError<E>> {
	reader.scope::<_, DictError<E>>(dictionary_list, |reader| {
		reader.seek(dictionary_list)?;
		let magic = reader.read_bytes::<4>()?;
		if magic != [0; 4] {
			return Err(DictError::InvalidMagic);
		}

		let count = reader.read_u32()? as _;

		let mut map = Dict::with_capacity(count);
		let mut data_offset = value_list;
		for i in 0..count {
			reader.seek(
				dictionary_list
					.checked_add(24 + i * 16 + 8)
					.ok_or(DictError::ReadError {
						source: util::Error::OutOfBoundsSeek,
					})?,
			)?;
			let name = reader.read_u64()? as usize;
			let name = reader.scope(name, |reader| {
				let count = reader.read_u16()?;
				reader.read_string(count as usize).map(ToOwned::to_owned)
			})?;
			let element = reader
				.scope(data_offset, |reader| {
					let element = element_reader(reader, &name);
					data_offset = reader.position();
					element
				})
				.map_err(|source| DictError::ElementError { source })?;
			map.insert(name, element);
		}

		Ok(map)
	})
}
