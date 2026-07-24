use std::{collections::HashMap, ops::Range};

use bitfield_struct::bitfield;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
	nw::{
		BinaryFileInfo, FileHeaderError,
		bntx::GpuAccessFlags,
		read_binary_file_header,
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
	EmbeddedFileError {
		source: DictError<util::Error>,
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
	#[snafu(context(suffix(ModelSnafu)))]
	MismatchedName,
	#[snafu(transparent)]
	VertexInfoError {
		source: VertexInfoError,
	},
	#[snafu(transparent)]
	ShapeError {
		source: DictError<ShapeError>,
	},
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
	VertexBufferOutsideOfMemoryPool,
}

#[derive(Snafu, Debug)]
pub enum VertexAttribError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	InvalidAttribFormat,
	#[snafu(context(suffix(AttribSnafu)))]
	MismatchedName,
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
	InvalidPrimitiveTopology,
	InvalidIndexFormat,
	IndexBufferOutsideOfMemoryPool,
	#[snafu(context(suffix(ShapeSnafu)))]
	MismatchedName,
}

#[derive(Snafu, Debug)]
pub enum MaterialError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectMaterialMagic,
	#[snafu(context(suffix(MaterialSnafu)))]
	MismatchedName,
}

pub struct ModelArchive {
	pub file_info: BinaryFileInfo,
	pub file_name: String,
	pub models: Dict<Model>,
	pub materials: Dict<Material>,
	pub embedded_files: Dict<Vec<u8>>,
}

impl std::fmt::Debug for ModelArchive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ModelArchive")
			.field("file_info", &self.file_info)
			.field("file_name", &self.file_name)
			.field("models", &self.models)
			.field(
				"embedded_files",
				&self
					.embedded_files
					.iter()
					.map(|key| (key.0, key.1.len()))
					.collect::<HashMap<_, _>>(),
			)
			.finish()
	}
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
	pub vertex_count: u32,
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

impl VertexAttribFormat {
	pub fn element_size(&self) -> usize {
		match self {
			VertexAttribFormat::Unorm8
			| VertexAttribFormat::U8
			| VertexAttribFormat::Snorm8
			| VertexAttribFormat::I8
			| VertexAttribFormat::U8F
			| VertexAttribFormat::I8F => 1,
			VertexAttribFormat::Vec2Unorm4
			| VertexAttribFormat::Unorm16
			| VertexAttribFormat::U16
			| VertexAttribFormat::Snorm16
			| VertexAttribFormat::I16
			| VertexAttribFormat::F16
			| VertexAttribFormat::U16F
			| VertexAttribFormat::I16F
			| VertexAttribFormat::Vec2Unorm8
			| VertexAttribFormat::Vec2U8
			| VertexAttribFormat::Vec2Snorm8
			| VertexAttribFormat::Vec2I8
			| VertexAttribFormat::Vec2U8F
			| VertexAttribFormat::Vec2I8F => 2,
			VertexAttribFormat::Vec2Unorm16
			| VertexAttribFormat::Vec2Snorm16
			| VertexAttribFormat::Vec2U16
			| VertexAttribFormat::Vec2I16
			| VertexAttribFormat::Vec2F16
			| VertexAttribFormat::Vec2U16F
			| VertexAttribFormat::Vec2I16F
			| VertexAttribFormat::Vec3F10F11F11
			| VertexAttribFormat::Vec4Unorm8
			| VertexAttribFormat::Vec4Snorm8
			| VertexAttribFormat::Vec4U8
			| VertexAttribFormat::Vec4I8
			| VertexAttribFormat::Vec4U8F
			| VertexAttribFormat::Vec4I8F
			| VertexAttribFormat::Vec4Unorm10_2
			| VertexAttribFormat::Vec4U10_2
			| VertexAttribFormat::Vec4Snorm10x3Unorm2
			| VertexAttribFormat::Vec4I10_2 => 4,
			VertexAttribFormat::Vec4Unorm16
			| VertexAttribFormat::Vec4Snorm16
			| VertexAttribFormat::Vec4U16
			| VertexAttribFormat::Vec4I16
			| VertexAttribFormat::Vec4F16
			| VertexAttribFormat::Vec4U32F
			| VertexAttribFormat::Vec4S32F => 8,
			VertexAttribFormat::U32 => 4,
			VertexAttribFormat::I32 => 4,
			VertexAttribFormat::F32 => 4,
			VertexAttribFormat::Vec2U32 => 8,
			VertexAttribFormat::Vec2I32 => 8,
			VertexAttribFormat::Vec2F32 => 8,
			VertexAttribFormat::Vec3U32 => 12,
			VertexAttribFormat::Vec3I32 => 12,
			VertexAttribFormat::Vec3F32 => 12,
			VertexAttribFormat::Vec4U32 => 16,
			VertexAttribFormat::Vec4I32 => 16,
			VertexAttribFormat::Vec4F32 => 16,
		}
	}
	pub fn component_size(&self) -> usize {
		match self {
			VertexAttribFormat::Unorm8
			| VertexAttribFormat::U8
			| VertexAttribFormat::Snorm8
			| VertexAttribFormat::I8
			| VertexAttribFormat::U8F
			| VertexAttribFormat::I8F
			| VertexAttribFormat::Unorm16
			| VertexAttribFormat::U16
			| VertexAttribFormat::Snorm16
			| VertexAttribFormat::I16
			| VertexAttribFormat::F16
			| VertexAttribFormat::U16F
			| VertexAttribFormat::I16F
			| VertexAttribFormat::U32
			| VertexAttribFormat::I32
			| VertexAttribFormat::F32 => 1,
			VertexAttribFormat::Vec2Unorm4
			| VertexAttribFormat::Vec2Unorm8
			| VertexAttribFormat::Vec2U8
			| VertexAttribFormat::Vec2Snorm8
			| VertexAttribFormat::Vec2I8
			| VertexAttribFormat::Vec2U8F
			| VertexAttribFormat::Vec2I8F
			| VertexAttribFormat::Vec2Unorm16
			| VertexAttribFormat::Vec2Snorm16
			| VertexAttribFormat::Vec2U16
			| VertexAttribFormat::Vec2I16
			| VertexAttribFormat::Vec2F16
			| VertexAttribFormat::Vec2U16F
			| VertexAttribFormat::Vec2I16F
			| VertexAttribFormat::Vec2U32
			| VertexAttribFormat::Vec2I32
			| VertexAttribFormat::Vec2F32 => 2,
			VertexAttribFormat::Vec3U32
			| VertexAttribFormat::Vec3I32
			| VertexAttribFormat::Vec3F32
			| VertexAttribFormat::Vec3F10F11F11 => 3,
			VertexAttribFormat::Vec4Unorm8
			| VertexAttribFormat::Vec4Snorm8
			| VertexAttribFormat::Vec4U8
			| VertexAttribFormat::Vec4I8
			| VertexAttribFormat::Vec4U8F
			| VertexAttribFormat::Vec4I8F
			| VertexAttribFormat::Vec4Unorm10_2
			| VertexAttribFormat::Vec4U10_2
			| VertexAttribFormat::Vec4Snorm10x3Unorm2
			| VertexAttribFormat::Vec4I10_2
			| VertexAttribFormat::Vec4Unorm16
			| VertexAttribFormat::Vec4Snorm16
			| VertexAttribFormat::Vec4U16
			| VertexAttribFormat::Vec4I16
			| VertexAttribFormat::Vec4F16
			| VertexAttribFormat::Vec4U32F
			| VertexAttribFormat::Vec4S32F
			| VertexAttribFormat::Vec4U32
			| VertexAttribFormat::Vec4I32
			| VertexAttribFormat::Vec4F32 => 4,
		}
	}
}

#[derive(Debug)]
pub struct Shape {
	pub vertex_info: VertexInfo,
	pub meshes: Vec<Mesh>,
	pub flags: ShapeFlags,
	pub material_index: u16,
	pub base_bone_index: u16,
	// todo bounding box
	// todo bounding sphere
}
#[bitfield(u32)]
pub struct ShapeFlags {
	has_vertex_buffer: bool,

	/// The boundings in all submeshes are consistent.
	/// (bfreslibrary comment, not sure what it means)
	sub_mesh_boundary_consistent: bool,
	#[bits(30)]
	_reserved: u32,
}
#[derive(Debug)]
pub struct Mesh {
	/// divisions of the index buffer
	pub sub_meshes: Vec<Range<u32>>,
	pub index_buffer: IndexBuffer,
	pub primitive_topology: PrimitiveTopology,
	pub base_vertex: u32,
}
#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
	Points = 0x00,
	Lines = 0x01,
	LineStrip = 0x02,
	Triangles = 0x03,
	TriangleStrip = 0x04,
	LinesAdjacency = 0x05,
	LineStripAdjacency = 0x06,
	TrianglesAdjacency = 0x07,
	TriangleStripAdjacency = 0x08,
	Patches = 0x09,
}
#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
	U8 = 0,
	U16 = 1,
	U32 = 2,
}
pub struct IndexBuffer {
	pub data: Vec<u8>,
	pub access_flags: GpuAccessFlags,
	pub format: IndexFormat,
}

impl IndexBuffer {
	pub fn element_count(&self) -> usize {
		// self.data.len()
		match self.format {
			IndexFormat::U8 => self.data.len() / size_of::<u8>(),
			IndexFormat::U16 => self.data.len() / size_of::<u16>(),
			IndexFormat::U32 => self.data.len() / size_of::<u32>(),
		}
	}
}
impl std::fmt::Debug for IndexBuffer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("IndexBuffer")
			.field("len(data)", &self.data.len())
			.field("access_flags", &self.access_flags)
			.finish()
	}
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
		let (model_array, model_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		// reader.seek_rel(32)?;
		let (skel_anim_array, skel_anim_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (material_anim_array, material_anim_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (bone_vis_anim_array, bone_vis_anim_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (shape_anim_array, shape_anim_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (scene_anim_array, scene_anim_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		reader.seek_rel(8)?;
		let memory_pool_offset = reader.read_u64()? as usize;
		let (embedded_file_array, embedded_file_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let memory_pool = reader.scope(memory_pool_offset, |reader| -> Result<_, Error> {
			let _a = reader.read_u32()?;
			println!("{:X} {_a:X}", reader.position());
			let memory_pool_size = reader.read_u32()?;
			println!("{:X} {memory_pool_size:X}", reader.position());
			let memory_pool_offset = reader.read_u64()?;
			println!("{:X} {memory_pool_offset:X}", reader.position());

			println!("{memory_pool_offset:X} {memory_pool_size:X}");
			let memory_pool = reader.scope(memory_pool_offset as usize, |reader| {
				reader
					.read_slice(memory_pool_size as usize)
					.map(<[_]>::to_vec)
			})?;

			Ok(memory_pool)
		})?;
		let models = read_dict::<_, ModelError>(
			reader,
			model_array,
			model_dictionary,
			|reader, dict_name, _| {
				ensure!(
					reader.read_bytes::<4>()? == *b"FMDL",
					IncorrectModelMagicSnafu
				);

				reader.seek_rel(12)?;
				let name_offset = reader.read_u64()? as usize;
				let name = reader.scope(name_offset, |reader| {
					let name_size = reader.read_u16()?;
					reader.read_string(name_size as _).map(ToOwned::to_owned)
				})?;
				ensure!(name == dict_name, MismatchedNameModelSnafu);
				let _ = reader.read_u64()?;
				let _skeleton_offset = reader.read_u64()? as usize;
				let vertex_info_offset = reader.read_u64()? as usize;
				let vertex_info = reader.scope(vertex_info_offset, |reader| {
					Self::read_vertex_info(reader, &memory_pool)
				})?;
				let (shape_array, shape_dictionary) = (reader.read_u64()? as _, reader.read_u64()? as _);
				let shapes = read_dict(reader, shape_array, shape_dictionary, |reader, name, _| {
					Self::read_shape(reader, name, &memory_pool)
				})?;

				Ok(Model {
					skeleton: Skeleton,
					vertex_info,
					shapes,
					materials: Dict::new(),
					user_data: Dict::new(),
				})
			},
		)?;
		let embedded_files = read_dict::<_, util::Error>(
			reader,
			embedded_file_array,
			embedded_file_dictionary,
			|reader, _, _| {
				let offset = reader.read_u64()? as usize;
				let size = reader.read_u64()? as usize;
				reader.scope(offset, |reader| reader.read_slice(size).map(<[_]>::to_vec))
			},
		)
		.context(EmbeddedFileSnafu)?;
		let materials = Dict::new();

		Ok(ModelArchive {
			file_info,
			file_name,
			models,
			embedded_files,
			materials,
		})
	}

	fn read_vertex_info(
		reader: &mut BinaryReader,
		memory_pool: &[u8],
	) -> Result<VertexInfo, VertexInfoError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FVTX", IncorrectVertexMagicSnafu);
		reader.seek_rel(12)?;

		let (attrib_array, attrib_dictionary) = (reader.read_u64()? as _, reader.read_u64()? as _);

		let attributes = read_dict::<_, VertexAttribError>(
			reader,
			attrib_array,
			attrib_dictionary,
			|reader, dict_name, _| {
				let name_offset = reader.read_u64()? as _;

				let name = reader.scope(name_offset, |reader| {
					let name_size = reader.read_u16()?;
					reader.read_string(name_size as _).map(ToOwned::to_owned)
				})?;
				ensure!(name == dict_name, MismatchedNameAttribSnafu);
				let mut format = reader.read_u16()?;
				if reader.byte_order() == ByteOrder::Little {
					format = format.swap_bytes();
				}
				let format = VertexAttribFormat::from_u16(format).context(InvalidAttribFormatSnafu)?;
				reader.seek_rel(2)?;
				let buffer_offset = reader.read_u16()?;
				let buffer_index = reader.read_u16()?;

				Ok(VertexAttrib {
					format,
					buffer_offset,
					buffer_index,
				})
			},
		)?;

		reader.seek_rel(0x18)?;
		let buffer_sizes = reader.read_u64()? as usize;
		let buffer_stride = reader.read_u64()? as usize;
		reader.seek_rel(8)?;
		let mut buffer_offset = reader.read_u32()? as usize;
		let _vertex_attrib_count = reader.read_u8()?;
		let buffer_count = reader.read_u8()?;
		reader.seek_rel(2)?;
		let vertex_count = reader.read_u32()?;
		let _vertex_skin_count = reader.read_u32()?;
		reader.seek_rel(2)?;

		println!("actual vertex count: {vertex_count}");

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

				let data = buffer_offset
					.checked_add(size as usize)
					.and_then(|end| memory_pool.get(buffer_offset..end))
					.map(<[_]>::to_vec)
					.ok_or(VertexInfoError::VertexBufferOutsideOfMemoryPool)?;

				buffer_offset += size as usize;

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
			vertex_count,
		})
	}

	fn read_shape(
		reader: &mut BinaryReader,
		dict_name: &str,
		memory_pool: &[u8],
	) -> Result<Shape, ShapeError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FSHP", IncorrectShapeMagicSnafu);
		reader.seek_rel(12)?;
		let name_offset = reader.read_u64()? as _;
		let name = reader.scope(name_offset, |reader| {
			let name_size = reader.read_u16()?;
			reader.read_string(name_size as _).map(ToOwned::to_owned)
		})?;

		ensure!(name == dict_name, MismatchedNameShapeSnafu);

		let vertex_info_offset = reader.read_u64()? as usize;
		let vertex_info = reader.scope(vertex_info_offset, |reader| {
			Self::read_vertex_info(reader, memory_pool)
		})?;
		let mesh_array_offset = reader.read_u64()? as usize;
		let skinning_bone_array = reader.read_u64()? as usize;
		let key_shape_array = reader.read_u64()? as usize;
		let key_shape_dictionary = reader.read_u64()? as usize;
		let bounding_box_array = reader.read_u64()? as usize;
		let bounding_sphere_array = reader.read_u64()? as usize;
		let _user_data_array = reader.read_u64()? as usize;
		let flags = ShapeFlags::from_bits(reader.read_u32()?);
		let _ = reader.read_u16()?;
		let material_index = reader.read_u16()?;
		let base_bone_index = reader.read_u16()?;
		let vertex_index = reader.read_u16()?;
		let skin_bone_index_count = reader.read_u16()?;
		let vertex_skin_weight_count = reader.read_u8()?;
		let mesh_count = reader.read_u8()?;
		let key_shape_count = reader.read_u8()?;
		let target_attribute_count = reader.read_u8()?;
		reader.seek_rel(2)?;
		let meshes = reader.scope(mesh_array_offset, |reader| {
			reader
				.read_boxed_array(mesh_count as usize, |reader, _| {
					Self::read_mesh(reader, memory_pool)
				})
				.map(<[_]>::into_vec)
		})?;
		// let meshes = vec![];

		Ok(Shape {
			vertex_info,
			meshes,
			flags,
			material_index,
			base_bone_index,
		})
	}

	fn read_mesh(reader: &mut BinaryReader, memory_pool: &[u8]) -> Result<Mesh, ShapeError> {
		let sub_mesh_offset = reader.read_u64()? as usize;
		reader.seek_rel(16)?;
		let index_buffer_info_offset = reader.read_u64()? as usize;
		let index_buffer_offset = reader.read_u32()? as usize;
		let primitive_topology =
			PrimitiveTopology::from_u32(reader.read_u32()?).context(InvalidPrimitiveTopologySnafu)?;
		let index_format =
			IndexFormat::from_u32(reader.read_u32()?).context(InvalidIndexFormatSnafu)?;
		let _index_count = reader.read_u32()?;
		let base_vertex = reader.read_u32()?;
		let sub_mesh_count = reader.read_u16()? as usize;
		reader.seek_rel(2)?;
		let index_buffer = reader.scope(
			index_buffer_info_offset,
			|reader| -> Result<_, ShapeError> {
				let size = reader.read_u32()? as usize;
				let access_flags = GpuAccessFlags::from_bits(reader.read_u32()?);

				let data = index_buffer_offset
					.checked_add(size)
					.and_then(|end| memory_pool.get(index_buffer_offset..end))
					.map(<[_]>::to_vec)
					.ok_or(ShapeError::IndexBufferOutsideOfMemoryPool)?;
				Ok(IndexBuffer {
					data,
					access_flags,
					format: index_format,
				})
			},
		)?;
		let sub_meshes = reader.scope(sub_mesh_offset, |reader| {
			reader
				.read_boxed_array(sub_mesh_count, |reader, _| -> Result<_, ShapeError> {
					let offset = reader.read_u32()?;
					let size = reader.read_u32()?;
					Ok(offset..offset + size)
				})
				.map(<[_]>::into_vec)
		})?;

		Ok(Mesh {
			sub_meshes,
			index_buffer,
			primitive_topology,
			base_vertex,
		})
	}

	pub fn read_material(reader: &mut BinaryReader) -> Result<Material, MaterialError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FMAT", IncorrectMaterialMagicSnafu);
		reader.seek_rel(12)?;
		let name_offset = reader.read_u64()? as _;
		let name = reader.scope(name_offset, |reader| {
			let name_size = reader.read_u16()?;
			reader.read_string(name_size as _).map(ToOwned::to_owned)
		})?;
		Ok(Material {})
	}
}

pub fn read_dict<T, E: snafu::Error + From<util::Error>>(
	reader: &mut BinaryReader,
	value_list: usize,
	dictionary_list: usize,
	mut element_reader: impl FnMut(&mut BinaryReader, &str, usize) -> Result<T, E>,
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
					let element = element_reader(reader, &name, i);
					data_offset = reader.position();
					element
				})
				.map_err(|source| DictError::ElementError { source })?;
			map.insert(name, element);
		}

		Ok(map)
	})
}
