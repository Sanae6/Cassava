use std::{collections::HashMap, ops::Range};

use bitfield_struct::bitfield;
use glam::{
	BVec2, BVec3, BVec4, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, Quat, UVec2, UVec3, UVec4, Vec2,
	Vec3, Vec4,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
	nw::{
		BinaryFileInfo, FileHeaderError,
		bntx::GpuAccessFlags,
		read_binary_file_header, read_owned_string_at, read_string_at,
		res_dict::{Dict, DictError},
	},
	util::{self, BinaryReader, ByteOrder},
};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
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
		#[snafu(backtrace)]
		source: DictError<ModelError>,
	},
	EmbeddedFileError {
		#[snafu(backtrace)]
		source: DictError<util::Error>,
	},
}

#[derive(Snafu, Debug)]
pub enum ModelError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectModelMagic,
	#[snafu(context(suffix(ModelSnafu)))]
	MismatchedName,
	#[snafu(transparent)]
	SkeletonError {
		#[snafu(backtrace)]
		source: SkeletonError,
	},
	#[snafu(transparent)]
	ShapeError {
		#[snafu(backtrace)]
		source: DictError<ShapeError>,
	},
	#[snafu(transparent)]
	MaterialError {
		#[snafu(backtrace)]
		source: MaterialError,
	},
}

#[derive(Snafu, Debug)]
pub enum SkeletonError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectSkeletonMagic,
	#[snafu(transparent)]
	BoneError {
		#[snafu(backtrace)]
		source: BoneError,
	},
}

#[derive(Snafu, Debug)]
pub enum BoneError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(context(suffix(BoneSnafu)))]
	MismatchedName,
}

#[derive(Snafu, Debug)]
pub enum VertexInfoError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectVertexMagic,
	#[snafu(transparent)]
	VertexAttribError {
		#[snafu(backtrace)]
		source: DictError<VertexAttribError>,
	},
	VertexBufferOutsideOfMemoryPool,
}

#[derive(Snafu, Debug)]
pub enum VertexAttribError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
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
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectShapeMagic,
	#[snafu(transparent)]
	VertexInfoError {
		#[snafu(backtrace)]
		source: VertexInfoError,
	},
	#[snafu(transparent)]
	KeyShapeError {
		#[snafu(backtrace)]
		source: DictError<util::Error>,
	},
	InvalidPrimitiveTopology,
	InvalidIndexFormat,
	IndexBufferOutsideOfMemoryPool,
	#[snafu(context(suffix(ShapeSnafu)))]
	MismatchedName,
	MaterialNotFound,
}

#[derive(Snafu, Debug)]
pub enum MaterialError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	IncorrectMaterialMagic,
	#[snafu(transparent)]
	RenderInfoError {
		#[snafu(backtrace)]
		source: DictError<RenderInfoError>,
	},
	#[snafu(transparent)]
	ShaderAssignError {
		#[snafu(backtrace)]
		source: ShaderAssignError,
	},
	#[snafu(context(suffix(MaterialSnafu)))]
	SamplerError {
		#[snafu(backtrace)]
		source: DictError<SamplerError>,
	},
	#[snafu(transparent)]
	ShaderParamError {
		#[snafu(backtrace)]
		source: DictError<ShaderParamError>,
	},
}

#[derive(Snafu, Debug)]
pub enum RenderInfoError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(context(suffix(RenderInfoSnafu)))]
	MismatchedName,
	InvalidRenderDataType,
}

#[derive(Snafu, Debug)]
pub enum ShaderAssignError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	AttributesError {
		#[snafu(backtrace)]
		source: DictError<util::Error>,
	},
	#[snafu(context(suffix(ShaderSnafu)))]
	SamplerError {
		#[snafu(backtrace)]
		source: DictError<util::Error>,
	},
	ShaderOptionError {
		#[snafu(backtrace)]
		source: DictError<util::Error>,
	},
}

#[derive(Snafu, Debug)]
pub enum SamplerError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	InvalidWrapMode,
	InvalidCompareOp,
	InvalidBorderColor,
}

#[derive(Snafu, Debug)]
pub enum ShaderParamError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(context(suffix(ShaderParamSnafu)))]
	MismatchedName,
	InvalidParamType,
	InvalidTextureSrtMode,
}

pub struct ModelArchive {
	pub file_info: BinaryFileInfo,
	pub file_name: String,
	pub models: Dict<Model>,
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
	pub shapes: Dict<Shape>,
}

#[derive(Debug)]
pub struct Skeleton {
	pub bones: Vec<Bone>,
	pub matrices_to_bones: Vec<u16>,
	pub inverse_model_matrices: Vec<Mat4>,
	pub scaling_mode: ScalingMode,
	pub use_euler_rotation: bool,
}

#[bitfield(u32)]
struct SkeletonFlags {
	#[bits(8)]
	_unused: u32,
	#[bits(2)]
	scaling_mode: u8,
	#[bits(2)]
	_unused: u8,
	use_euler_rotation: bool,
	#[bits(19)]
	_unused: u32,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingMode {
	None,
	Standard,
	Maya,
	Softimage,
}

#[derive(Debug)]
pub struct Bone {
	pub name: String,
	pub index: u16,
	pub parent_index: Option<u16>,
	pub smooth_matrix_index: Option<u16>,
	pub rigid_matrix_index: Option<u16>,
	pub billboard_index: Option<u16>,
	pub scale: Vec3,
	pub rotation: BoneRotation,
	pub position: Vec3,
	// flags
	pub visible: bool,

	pub billboard_transformation: BillboardTransform,
	pub transform: TransformFlags,
	pub cumulative_transform: TransformFlags,
}

#[derive(Debug)]
pub enum BoneRotation {
	Quat(Quat),
	Euler(Vec3),
}
/// see [bfreslibrary](https://github.com/KillzXGaming/BfresLibrary/blob/master/BfresLibrary/Shared/Models/Skeleton/Bone.cs#L597)
/// there are some explanations that are useful to describe these
#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillboardTransform {
	None,
	Child,
	WorldViewVector,
	WorldViewPoint,
	ScreenViewVector,
	ScreenViewPoint,
	YAxisViewVector,
	YAxisViewPoint,
}
#[bitfield(u8)]
pub struct TransformFlags {
	scale_uniform: bool,
	scale_volume_one: bool,
	rotate_zero: bool,
	translate_zero: bool,
	#[bits(4)]
	_padding: u8,
}
#[bitfield(u32)]
struct BoneFlags {
	visible: bool,
	#[bits(11)]
	_unused: u32,
	use_euler_rotation: bool,
	#[bits(3)]
	_unused: u32,
	#[bits(3)]
	billboard_transform: u8,
	#[bits(5)]
	_unused: u32,
	#[bits(4)]
	transform: TransformFlags,
	#[bits(4)]
	transform_cumulative: TransformFlags,
}

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
	pub key_shapes: Dict<KeyShape>,
	pub flags: ShapeFlags,
	pub material: Material,
	pub base_bone_index: u16,
	// todo bounding box
	// todo bounding sphere
}
#[derive(Debug)]
pub struct KeyShape {
	pub target_attributes: [u8; 18],
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

#[derive(Clone, Debug)]
pub struct Material {
	pub name: String,
	pub render_info: Dict<Vec<RenderValue>>,
	pub shader_assign: Option<ShaderAssign>,
	pub texture_names: Vec<String>,
	pub samplers: Dict<Sampler>,
	pub shader_params: Dict<ShaderParam>,
	pub texture_slots: Vec<i64>,
	pub sampler_slots: Vec<i64>,
}

#[derive(Clone)]
pub enum RenderValue {
	I32(i32),
	F32(f32),
	String(String),
}

impl std::fmt::Debug for RenderValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::I32(arg0) => arg0.fmt(f),
			Self::F32(arg0) => arg0.fmt(f),
			Self::String(arg0) => arg0.fmt(f),
		}
	}
}

#[derive(Clone, Debug)]
pub struct ShaderAssign {
	pub shader_archive: String,
	pub shading_model_name: String,
	pub attributes: Dict<String>,
	pub samplers: Dict<String>,
	pub shader_options: Dict<String>,
	pub revision: u32,
}

#[derive(Clone, Debug)]
pub struct ShaderParam {
	pub uniform_offset: i32,
	pub depended_index: u16,
	pub depend_index: u16,
	pub value: ParamValue,
}
#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
enum ParamType {
	Bool = 0,
	BVec2 = 1,
	BVec3 = 2,
	BVec4 = 3,
	I32 = 4,
	IVec2 = 5,
	IVec3 = 6,
	IVec4 = 7,
	U32 = 8,
	UVec2 = 9,
	UVec3 = 10,
	UVec4 = 11,
	F32 = 12,
	Vec2 = 13,
	Vec3 = 14,
	Vec4 = 15,
	Mat2 = 17,
	Mat2x3 = 18,
	Mat2x4 = 19,
	Mat3x2 = 21,
	Mat3 = 22,
	Mat3x4 = 23,
	Mat4x2 = 25,
	Mat4x3 = 26,
	Mat4 = 27,
	Srt2D = 28,
	Srt3D = 29,
	TexSrt = 30,
}
#[derive(Clone, Debug)]
pub enum ParamValue {
	Bool(bool),
	BVec2(BVec2),
	BVec3(BVec3),
	BVec4(BVec4),
	I32(i32),
	IVec2(IVec2),
	IVec3(IVec3),
	IVec4(IVec4),
	U32(u32),
	UVec2(UVec2),
	UVec3(UVec3),
	UVec4(UVec4),
	F32(f32),
	Vec2(Vec2),
	Vec3(Vec3),
	Vec4(Vec4),
	Mat2(Mat2),
	Mat2x3(Mat3),
	Mat2x4(Mat4),
	Mat3x2(Mat3),
	Mat3(Mat3),
	Mat3x4(Mat4),
	Mat4x2(Mat4),
	Mat4x3(Mat4),
	Mat4(Mat4),
	Srt2D {
		scale: Vec2,
		rotation: f32,
		translation: Vec2,
	},
	Srt3D {
		scale: Vec3,
		rotation: Vec3,
		translation: Vec3,
	},
	TextureSrt {
		mode: TextureSrtMode,
		scale: Vec2,
		rotation: f32,
		translation: Vec2,
	},
}
#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureSrtMode {
	Maya,
	Autodesk3dsMax,
	Softimage,
}

#[derive(Clone, Debug)]
pub struct Sampler {
	pub wrap_modes: [WrapMode; 3],
	pub compare_op: CompareOp,
	pub border_color: BorderColor,
	pub anisotropic_level: u8,
	pub filter_flags: u16,
	pub min_lod: f32,
	pub max_lod: f32,
	pub lod_bias: f32,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
	Repeat,
	MirrorRepeat,
	ClampToEdge,
	ClampToBorder,
	MirrorClampToBorder,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
	Never,
	LessThan,
	Equal,
	LessThanEqual,
	GreaterThan,
	NotEqual,
	GreaterThanEqual,
	Always,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderColor {
	White,
	TransparentBlack,
	Black,
}

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
			let memory_pool_size = reader.read_u32()?;
			let memory_pool_offset = reader.read_u64()?;

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
			|reader, dict_name, _| Self::read_model(reader, dict_name, &memory_pool),
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

		Ok(ModelArchive {
			file_info,
			file_name,
			models,
			embedded_files,
		})
	}

	fn read_model(
		reader: &mut BinaryReader<'_>,
		dict_name: &str,
		memory_pool: &[u8],
	) -> Result<Model, ModelError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FMDL", IncorrectModelMagicSnafu);

		reader.seek_rel(12)?;
		let name_offset = reader.read_u64()? as usize;
		let name = read_string_at(reader, name_offset)?;
		ensure!(name == dict_name, MismatchedNameModelSnafu);
		let _ = reader.read_u64()?;
		let skeleton_offset = reader.read_u64()? as usize;
		let _vertex_info_array = reader.read_u64()? as usize; // shapes point to their vertex buffers anyways
		let (shape_array, shape_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (material_array, _material_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		// unused
		let (_user_data_array, _user_data_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let _user_ptr = reader.read_u64()?;
		// unread because shapes point to the used vertex infos
		let _vertex_info_count = reader.read_u16()?;
		let _shape_count = reader.read_u16()?;
		let material_count = reader.read_u16()? as usize;
		let _user_data_count = reader.read_u16()?;
		let _total_vertices = reader.read_u32()?;
		reader.seek_rel(4)?;

		let skeleton = reader.scope(skeleton_offset, Self::read_skeleton)?;

		let material_list = reader.scope(material_array, |reader| {
			reader.read_boxed_array(material_count, |reader, _| Self::read_material(reader))
		})?;
		let shapes = read_dict(reader, shape_array, shape_dictionary, |reader, name, _| {
			Self::read_shape(reader, name, memory_pool, &material_list)
		})?;

		Ok(Model { skeleton, shapes })
	}

	fn read_skeleton(reader: &mut BinaryReader) -> Result<Skeleton, SkeletonError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FSKL", IncorrectSkeletonMagicSnafu);
		reader.seek_rel(12)?;

		let (_user_data_dictionary, bone_array) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let matrix_to_bone_array = reader.read_u64()? as usize; // smooth + rigid
		let inverse_model_matrix_array = reader.read_u64()? as usize; // smooth
		reader.seek_rel(24)?;
		let flags = SkeletonFlags::from_bits(reader.read_u32()?);
		let bone_count = reader.read_u16()? as usize;
		let smooth_matrix_count = reader.read_u16()? as usize;
		let rigid_matrix_count = reader.read_u16()? as usize;
		reader.seek_rel(6)?;
		let matrices_to_bones = reader.scope(matrix_to_bone_array, |reader| {
			reader.read_vec(smooth_matrix_count + rigid_matrix_count, |reader, _| {
				reader.read_u16()
			})
		})?;
		let inverse_model_matrices = reader.scope(inverse_model_matrix_array, |reader| {
			reader.read_vec(smooth_matrix_count, |reader, _| -> Result<_, util::Error> {
				let mat34 = reader.read_array::<4, _, _>(|reader, _| Ok(reader.read_vec3()?.to_array()))?;
				let mut rows = [[0.0f32; 4]; 4];
				for j in 0..3 {
					for i in 0..4 {
						rows[j][i] = mat34[i][j];
					}
				}
				Ok(Mat4::from_cols_array_2d(&rows))
			})
		})?;

		let bones = reader.scope(bone_array, |reader| {
			reader.read_vec(bone_count, |reader, _| Self::read_bone(reader))
		})?;

		Ok(Skeleton {
			bones,
			matrices_to_bones,
			inverse_model_matrices,
			scaling_mode: ScalingMode::from_u8(flags.scaling_mode()).unwrap(),
			use_euler_rotation: flags.use_euler_rotation(),
		})
	}

	fn read_bone(reader: &mut BinaryReader) -> Result<Bone, BoneError> {
		let name_offset = reader.read_u64()? as usize;
		let name = read_owned_string_at(reader, name_offset)?;
		reader.seek_rel(32)?; // skipping user data
		let optional = |value: i16| (value >= 0).then_some(value.cast_unsigned());
		let index = reader.read_u16()?;
		let parent_index = optional(reader.read_i16()?);
		let smooth_matrix_index = optional(reader.read_i16()?);
		let rigid_matrix_index = optional(reader.read_i16()?);
		let billboard_index = optional(reader.read_i16()?);
		let _user_data_count = reader.read_u16()?;
		let flags = BoneFlags::from_bits(reader.read_u32()?);
		let scale = reader.read_vec3()?;
		let rotation = reader.read_vec4()?;
		let position = reader.read_vec3()?;
		let rotation = match flags.use_euler_rotation() {
			true => BoneRotation::Euler(Vec3::from_slice(&rotation.to_array()[..3])),
			false => BoneRotation::Quat(Quat::from_array(rotation.to_array())),
		};

		Ok(Bone {
			name,
			index,
			parent_index,
			smooth_matrix_index,
			rigid_matrix_index,
			billboard_index,
			scale,
			rotation,
			position,
			visible: flags.visible(),
			billboard_transformation: BillboardTransform::from_u8(flags.billboard_transform()).unwrap(),
			transform: flags.transform(),
			cumulative_transform: flags.transform_cumulative(),
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

				let name = read_string_at(reader, name_offset)?;
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

		let buffers = reader.read_vec::<_, VertexInfoError>(buffer_count as _, |reader, index| {
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
		})?;

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
		materials: &[Material],
	) -> Result<Shape, ShapeError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FSHP", IncorrectShapeMagicSnafu);
		reader.seek_rel(12)?;
		let name_offset = reader.read_u64()? as _;
		let name = read_string_at(reader, name_offset)?;
		ensure!(name == dict_name, MismatchedNameShapeSnafu);

		let vertex_info_offset = reader.read_u64()? as usize;
		let vertex_info = reader.scope(vertex_info_offset, |reader| {
			Self::read_vertex_info(reader, memory_pool)
		})?;
		let mesh_array_offset = reader.read_u64()? as usize;
		let skinning_bone_array = reader.read_u64()? as usize;
		let key_shape_array = reader.read_u64()? as usize;
		let key_shape_dictionary = reader.read_u64()? as usize;
		let _bounding_box_array = reader.read_u64()? as usize;
		let _bounding_sphere_array = reader.read_u64()? as usize;
		let _user_data_array = reader.read_u64()? as usize;
		let flags = ShapeFlags::from_bits(reader.read_u32()?);
		let _ = reader.read_u16()?;
		let material_index = reader.read_u16()? as usize;
		let base_bone_index = reader.read_u16()?;
		let _vertex_index = reader.read_u16()? as usize;
		let skin_bone_index_count = reader.read_u16()? as usize;
		let vertex_skin_weight_count = reader.read_u8()? as usize;
		let mesh_count = reader.read_u8()? as usize;
		let key_shape_count = reader.read_u8()? as usize;
		let target_attribute_count = reader.read_u8()? as usize;
		let material = materials
			.get(material_index)
			.cloned()
			.context(MaterialNotFoundSnafu)?;
		reader.seek_rel(6)?;
		let meshes = reader.scope(mesh_array_offset, |reader| {
			reader.read_vec(mesh_count as usize, |reader, _| {
				Self::read_mesh(reader, memory_pool)
			})
		})?;
		let key_shapes = read_dict(
			reader,
			key_shape_array,
			key_shape_dictionary,
			|reader, _, _| -> Result<_, util::Error> {
				let target_attributes = reader.read_bytes::<18>()?;
				reader.seek_rel(2)?;
				Ok(KeyShape { target_attributes })
			},
		)?;

		Ok(Shape {
			vertex_info,
			meshes,
			key_shapes,
			flags,
			material,
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
			reader.read_vec(sub_mesh_count, |reader, _| -> Result<_, ShapeError> {
				let offset = reader.read_u32()?;
				let size = reader.read_u32()?;
				Ok(offset..offset + size)
			})
		})?;

		Ok(Mesh {
			sub_meshes,
			index_buffer,
			primitive_topology,
			base_vertex,
		})
	}

	fn read_material(reader: &mut BinaryReader) -> Result<Material, MaterialError> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(magic == *b"FMAT", IncorrectMaterialMagicSnafu);
		reader.seek_rel(12)?;
		let name_offset = reader.read_u64()? as usize;
		let name = read_owned_string_at(reader, name_offset)?;
		let (render_info_array, render_info_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let shader_assign = reader.read_u64()? as usize;
		let (_texture_array, texture_name_array) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let _runtime_sampler_array = reader.read_u64()?;
		let (sampler_array, sampler_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (shader_param_array, shader_param_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let source_param_data_array = reader.read_u64()? as usize;
		let (_user_data_array, _user_data_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let volatile_flag_array = reader.read_u64()?;
		let _user_ptr = reader.read_u64()?;
		let sampler_slot_array = reader.read_u64()? as usize;
		let texture_slot_array = reader.read_u64()? as usize;
		let flags = reader.read_u32()?;
		let _index = reader.read_u16()?;
		let _render_info_count = reader.read_u16()?;
		let texture_count = reader.read_u8()? as usize;
		let sampler_count = reader.read_u8()? as usize;
		let _shader_param_count = reader.read_u16()?;
		let shader_param_volatile_count = reader.read_u16()?;
		let _source_param_count = reader.read_u16()?;
		let _raw_param_size = reader.read_u16()?;
		let _user_data_count = reader.read_u16()?;
		reader.seek_rel(4)?;

		let render_info = read_dict::<_, RenderInfoError>(
			reader,
			render_info_array,
			render_info_dictionary,
			|reader, dict_name, _| {
				let name_offset = reader.read_u64()? as usize;
				let name = read_owned_string_at(reader, name_offset)?;

				ensure!(name == dict_name, MismatchedNameRenderInfoSnafu);
				let data_offset = reader.read_u64()? as usize;

				let count = reader.read_u16()? as usize;
				let data_type = reader.read_u8()?;
				reader.seek_rel(5)?;

				reader.scope(data_offset, |reader| {
					Ok(match data_type {
						0 => reader.read_vec(count, |reader, _| reader.read_i32().map(RenderValue::I32))?,
						1 => reader.read_vec(count, |reader, _| reader.read_f32().map(RenderValue::F32))?,
						2 => reader.read_vec(count, |reader, _| {
							let offset = reader.read_u64()? as usize;
							read_owned_string_at(reader, offset).map(RenderValue::String)
						})?,
						_ => return Err(RenderInfoError::InvalidRenderDataType),
					})
				})
			},
		)?;

		let shader_assign = (shader_assign != 0)
			.then(|| reader.scope(shader_assign, Self::read_shader_assign))
			.transpose()?;

		let texture_names = reader.scope(texture_name_array, |reader| {
			reader.read_vec(texture_count, |reader, _| {
				let offset = reader.read_u64()? as usize;
				read_owned_string_at(reader, offset)
			})
		})?;
		let samplers = read_dict(reader, sampler_array, sampler_dictionary, |reader, _, _| {
			Self::read_sampler(reader)
		})
		.context(SamplerMaterialSnafu)?;
		let shader_params = read_dict(
			reader,
			shader_param_array,
			shader_param_dictionary,
			|reader, dict_name, _| Self::read_shader_param(reader, dict_name, source_param_data_array),
		)?;
		let texture_slots = reader.scope(texture_slot_array, |reader| {
			reader.read_vec(texture_count, |reader, _| reader.read_i64())
		})?;
		let sampler_slots = reader.scope(sampler_slot_array, |reader| {
			reader.read_vec(sampler_count, |reader, _| reader.read_i64())
		})?;

		Ok(Material {
			name,
			render_info,
			shader_assign,
			texture_names,
			samplers,
			shader_params,
			texture_slots,
			sampler_slots,
		})
	}

	fn read_shader_assign(reader: &mut BinaryReader) -> Result<ShaderAssign, ShaderAssignError> {
		let shader_archive = reader.read_u64()? as usize;
		let shading_model_name = reader.read_u64()? as usize;
		let (attributes_array, attributes_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (sampler_array, sampler_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let (options_array, options_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);
		let revision = reader.read_u32()?;
		let _attrib_assign_count = reader.read_u8()?;
		let _sampler_assign_count = reader.read_u8()?;
		let _shader_option_count = reader.read_u16()?;

		let shader_archive = read_owned_string_at(reader, shader_archive)?;
		let shading_model_name = read_owned_string_at(reader, shading_model_name)?;
		let attributes = read_dict(
			reader,
			attributes_array,
			attributes_dictionary,
			|reader, _, _| {
				let offset = reader.read_u64()? as usize;
				read_owned_string_at(reader, offset)
			},
		)
		.context(AttributesSnafu)?;
		let samplers = read_dict(reader, sampler_array, sampler_dictionary, |reader, _, _| {
			let offset = reader.read_u64()? as usize;
			read_owned_string_at(reader, offset)
		})
		.context(SamplerShaderSnafu)?;
		let shader_options = read_dict(reader, options_array, options_dictionary, |reader, _, _| {
			let offset = reader.read_u64()? as usize;
			read_owned_string_at(reader, offset)
		})
		.context(ShaderOptionSnafu)?;

		Ok(ShaderAssign {
			shader_archive,
			shading_model_name,
			attributes,
			samplers,
			shader_options,
			revision,
		})
	}

	fn read_sampler(reader: &mut BinaryReader) -> Result<Sampler, SamplerError> {
		let wrap_modes: [WrapMode; 3] = reader
			.read_array(|reader, _| WrapMode::from_u8(reader.read_u8()?).context(InvalidWrapModeSnafu))?;
		let compare_op = CompareOp::from_u8(reader.read_u8()?).context(InvalidCompareOpSnafu)?;
		let border_color = BorderColor::from_u8(reader.read_u8()?).context(InvalidBorderColorSnafu)?;
		let anisotropic_level = reader.read_u8()?;
		let filter_flags = reader.read_u16()?;
		let min_lod = reader.read_f32()?;
		let max_lod = reader.read_f32()?;
		let lod_bias = reader.read_f32()?;
		reader.seek_rel(12)?;

		Ok(Sampler {
			wrap_modes,
			compare_op,
			border_color,
			anisotropic_level,
			filter_flags,
			min_lod,
			max_lod,
			lod_bias,
		})
	}

	fn read_shader_param(
		reader: &mut BinaryReader,
		dict_name: &str,
		data_start: usize,
	) -> Result<ShaderParam, ShaderParamError> {
		let _callback_ptr = reader.read_u64()?;
		let name_offset = reader.read_u64()? as _;
		let name = read_string_at(reader, name_offset)?;
		ensure!(name == dict_name, MismatchedNameShaderParamSnafu);
		let param_type = ParamType::from_u8(reader.read_u8()?).context(InvalidParamTypeSnafu)?;
		let _data_size = reader.read_u8()? as usize;
		let data_offset = reader.read_u16()? as usize;
		let uniform_offset = reader.read_i32()?;
		let depended_index = reader.read_u16()?;
		let depend_index = reader.read_u16()?;
		reader.seek_rel(4)?;

		// fn until<const N: usize>(
		// 	index: usize,
		// 	func: impl FnOnce() -> Result<f32, util::Error>,
		// ) -> Result<f32, util::Error> {
		// 	if index >= N { Ok(0.0) } else { func() }
		// }

		let value = reader.scope(
			data_start + data_offset,
			|reader| -> Result<_, ShaderParamError> {
				Ok(match param_type {
					ParamType::Bool => ParamValue::Bool(reader.read_bool()?),
					ParamType::BVec2 => ParamValue::BVec2(BVec2::from_array(
						reader.read_array(|reader, _| reader.read_bool())?,
					)),
					ParamType::BVec3 => ParamValue::BVec3(BVec3::from_array(
						reader.read_array(|reader, _| reader.read_bool())?,
					)),
					ParamType::BVec4 => ParamValue::BVec4(BVec4::from_array(
						reader.read_array(|reader, _| reader.read_bool())?,
					)),
					ParamType::I32 => ParamValue::I32(reader.read_i32()?),
					ParamType::IVec2 => ParamValue::IVec2(reader.read_ivec2()?),
					ParamType::IVec3 => ParamValue::IVec3(reader.read_ivec3()?),
					ParamType::IVec4 => ParamValue::IVec4(reader.read_ivec4()?),
					ParamType::U32 => ParamValue::U32(reader.read_u32()?),
					ParamType::UVec2 => ParamValue::UVec2(reader.read_uvec2()?),
					ParamType::UVec3 => ParamValue::UVec3(reader.read_uvec3()?),
					ParamType::UVec4 => ParamValue::UVec4(reader.read_uvec4()?),
					ParamType::F32 => ParamValue::F32(reader.read_f32()?),
					ParamType::Vec2 => ParamValue::Vec2(reader.read_vec2()?),
					ParamType::Vec3 => ParamValue::Vec3(reader.read_vec3()?),
					ParamType::Vec4 => ParamValue::Vec4(reader.read_vec4()?),
					ParamType::Mat2 => ParamValue::Mat2(
						Mat2::from_cols_array(&reader.read_array(|reader, _| reader.read_f32())?).transpose(),
					),
					ParamType::Mat2x3 => todo!(),
					ParamType::Mat2x4 => todo!(),
					ParamType::Mat3x2 => todo!(),
					ParamType::Mat3 => ParamValue::Mat3(
						Mat3::from_cols_array(&reader.read_array(|reader, _| reader.read_f32())?).transpose(),
					),
					ParamType::Mat3x4 => todo!(),
					ParamType::Mat4x2 => todo!(),
					ParamType::Mat4x3 => todo!(),
					ParamType::Mat4 => ParamValue::Mat4(
						Mat4::from_cols_array(&reader.read_array(|reader, _| reader.read_f32())?).transpose(),
					),
					ParamType::Srt2D => ParamValue::Srt2D {
						scale: reader.read_vec2()?,
						rotation: reader.read_f32()?,
						translation: reader.read_vec2()?,
					},
					ParamType::Srt3D => ParamValue::Srt3D {
						scale: reader.read_vec3()?,
						rotation: reader.read_vec3()?,
						translation: reader.read_vec3()?,
					},
					ParamType::TexSrt => {
						let mode =
							TextureSrtMode::from_u32(reader.read_u32()?).context(InvalidTextureSrtModeSnafu)?;
						ParamValue::TextureSrt {
							mode,
							scale: reader.read_vec2()?,
							rotation: reader.read_f32()?,
							translation: reader.read_vec2()?,
						}
					}
				})
			},
		)?;

		Ok(ShaderParam {
			uniform_offset,
			depended_index,
			depend_index,
			value,
		})
	}
}

pub fn read_dict<'a, T: 'a, E: snafu::Error + snafu::ErrorCompat + From<util::Error>>(
	reader: &mut BinaryReader<'a>,
	value_list: usize,
	dictionary_list: usize,
	mut element_reader: impl FnMut(&mut BinaryReader<'a>, &str, usize) -> Result<T, E>,
) -> Result<Dict<T>, DictError<E>> {
	if value_list == 0 || dictionary_list == 0 {
		return Ok(Dict::new());
	}

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
			reader.seek(dictionary_list)?;
			reader.seek_rel((24 + i * 16 + 8) as isize)?;
			let name_offset = reader.read_u64()? as usize;
			let name = read_owned_string_at(reader, name_offset)?;
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
