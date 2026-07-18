use glam::{UVec3, Vec3};
use snafu::{OptionExt, Snafu, ensure};

use crate::util::{BinaryReader, ByteOrder};

mod octree;
pub use octree::*;

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	Io {
		source: std::io::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("model array is past end of file"))]
	ModelArrayPastEOF,
	#[snafu(display("while reading model {model_index}"))]
	Model {
		#[snafu(backtrace)]
		source: ModelError,
		model_index: usize,
	},
	#[snafu(transparent)]
	Octree {
		#[snafu(backtrace)]
		source: OctreeError,
	},
}

#[derive(Snafu, Debug)]
pub enum ModelError {
	#[snafu(display("failed to read"))]
	#[snafu(context(false))]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("position array offset was misaligned"))]
	MisalignedPositions { backtrace: snafu::Backtrace },
	#[snafu(display("normal array offset was misaligned"))]
	MisalignedNormals { backtrace: snafu::Backtrace },
	#[snafu(display("prism array offset was misaligned"))]
	MisalignedPrisms { backtrace: snafu::Backtrace },
	#[snafu(display("octree offset was misaligned"))]
	MisalignedOctree { backtrace: snafu::Backtrace },
	#[snafu(display("position array is placed after normals"))]
	PositionsAfterNormals,
	#[snafu(display("normal array is placed after prisms"))]
	NormalsAfterPrisms,
	#[snafu(display("prism array is placed after octree"))]
	PrismsAfterOctree,
	#[snafu(display("position array is past end of file"))]
	PositionsAfterEOF { backtrace: snafu::Backtrace },
	#[snafu(display("position array size is not a multiple of 12"))]
	InvalidPositionsSize,
	#[snafu(display("normal array is past end of file"))]
	NormalsAfterEOF,
	#[snafu(display("normal array size is not a multiple of 12"))]
	InvalidNormalsSize,
	#[snafu(display("prism array is past end of file"))]
	PrismsAfterEOF,
	#[snafu(display("prism array size is not a multiple of 20"))]
	InvalidPrismsSize,
	#[snafu(display("octree is past end of file"))]
	OctreeAfterEOF,
	#[snafu(transparent)]
	Octree {
		#[snafu(backtrace)]
		source: OctreeError,
	},
}

#[derive(Snafu, Debug)]
pub enum OctreeError {
	#[snafu(transparent)]
	Io {
		source: std::io::Error,
		#[snafu(backtrace)]
		backtrace: snafu::Backtrace,
	},
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		#[snafu(backtrace)]
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("octree node type is invalid"))]
	InvalidOctreeNodeType,
}

pub struct KclPrism {
	pub length: f32,
	pub position_index: u16,
	pub face_normal_index: u16,
	pub edge_normal_index: [u16; 3],
	pub collision_type: u16,
	pub triangle_index: u32,
}

pub struct KclMesh {
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub prisms: Vec<KclPrism>,
	pub octree: PrismOctree,
	pub thickness: f32,
	pub octree_origin: Vec3,
	pub width_mask: UVec3,
	pub block_width_shift: u32,
	pub x_blocks_shift: u32,
	pub root_xy_shift: u32,
	pub hitbox_radius_cap: f32,
}

impl KclMesh {
	fn read(reader: &mut BinaryReader) -> Result<KclMesh, ModelError> {
		let start = reader.position() as usize;
		let positions_offset = reader.read_u32()? as usize;
		let normals_offset = reader.read_u32()? as usize;
		let prisms_offset = reader.read_u32()? as usize;
		let octree_offset = reader.read_u32()? as usize;
		let thickness = reader.read_f32()?;
		let octree_origin = reader.read_vec3()?;
		let width_mask = reader.read_uvec3()?;
		let block_width_shift = reader.read_u32()?;
		let x_blocks_shift = reader.read_u32()?;
		let root_xy_shift = reader.read_u32()?;
		let hitbox_radius_cap = reader.read_f32()?;

		ensure!(positions_offset & 0b11 == 0, MisalignedPositionsSnafu);
		ensure!(normals_offset & 0b11 == 0, MisalignedNormalsSnafu);
		ensure!(prisms_offset & 0b11 == 0, MisalignedPrismsSnafu);
		ensure!(octree_offset & 0b11 == 0, MisalignedOctreeSnafu);
		ensure!(
			positions_offset < normals_offset,
			PositionsAfterNormalsSnafu
		);
		ensure!(normals_offset < prisms_offset, NormalsAfterPrismsSnafu);
		ensure!(prisms_offset < octree_offset, PrismsAfterOctreeSnafu);
		let positions_start = start
			.checked_add(positions_offset)
			.context(PositionsAfterEOFSnafu)?;
		let positions_size = normals_offset - positions_offset;
		let normals_size = prisms_offset - normals_offset;
		let prisms_size = octree_offset - prisms_offset;

		reader.seek(positions_start)?;
		let mut positions = Vec::with_capacity(positions_size / size_of::<Vec3>());
		for _ in 0..positions.capacity() {
			positions.push(reader.read_vec3()?);
		}
		let mut normals = Vec::with_capacity(normals_size / size_of::<Vec3>());
		for _ in 0..normals.capacity() {
			normals.push(reader.read_vec3()?);
		}
		let mut prisms = Vec::with_capacity(prisms_size / 20);
		for _ in 0..prisms.capacity() {
			prisms.push(KclPrism {
				length: reader.read_f32()?,
				position_index: reader.read_u16()?,
				face_normal_index: reader.read_u16()?,
				edge_normal_index: [reader.read_u16()?, reader.read_u16()?, reader.read_u16()?],
				collision_type: reader.read_u16()?,
				triangle_index: reader.read_u32()?,
			});
		}

		let octree_offset = start
			.checked_add(octree_offset)
			.context(OctreeAfterEOFSnafu)?;
		reader.seek(octree_offset)?;
		// println!("{} {width_shift}", !width_mask);

		let bounds = !width_mask.as_ivec3() >> block_width_shift;
		let node_count = (bounds.x + 1) * (bounds.y + 1) * (bounds.z + 1);
		// println!(
		// 	"node count {node_count} {} {} {}",
		// 	(bounds.x + 1),
		// 	(bounds.y + 1),
		// 	(bounds.z + 1)
		// );
		let octree = PrismOctree::start(reader, octree_offset, node_count as usize)?;

		Ok(KclMesh {
			positions,
			normals,
			prisms,
			octree,
			thickness,
			octree_origin,
			width_mask,
			block_width_shift,
			x_blocks_shift,
			root_xy_shift,
			hitbox_radius_cap,
		})
	}

	/*
	implementation of https://old.tockdom.com/wiki/KCL_(File_Format)#Section_4_-_Spatial_Index

	pub fn point_collision(&self, point: Vec3) -> Option<&KclPrism> {
		let PrismOctree::Tree(block) = &self.octree else {
			unreachable!()
		};
		let point = (point - self.octree_origin).as_ivec3();
		let masked = point.as_uvec3() & self.width_mask;
		if masked.element_sum() != 0 {
			return None;
		}

		let shift = self.block_width_shift;

		let mut index = (point.z as u32 >> shift) << self.root_xy_shift
			| (point.y as u32 >> shift) << self.x_blocks_shift
			| point.x as u32 >> shift;

		loop {
			let Some(block) = block.get(index as usize) else {
				return None;
			};
		}
	}
	*/
}

pub struct Kcl {
	pub models: Vec<KclMesh>,
	pub bounds_min: Vec3,
	pub bounds_max: Vec3,
	pub octree: ModelOctree,
}

impl Kcl {
	pub fn read(input: &[u8]) -> Result<Self, Error> {
		let mut reader = BinaryReader::new(input, ByteOrder::Little);
		let version = reader.read_u32()?;
		println!("{version:08X}");
		match version {
			0x00000202 => reader.set_byte_order(ByteOrder::Little),
			0x02020000 => reader.set_byte_order(ByteOrder::Big),
			_ => {
				todo!("v1/big endian not supported yet")
			}
		};

		let model_octree_offset = reader.read_u32()?;
		let model_array_offset = reader.read_u32()? as usize;
		let model_count = reader.read_u32()? as usize;
		let (bounds_min, bounds_max) = (reader.read_vec3()?, reader.read_vec3()?);
		let shift = reader.read_uvec3()?;
		let _prism_count = reader.read_i32()?;

		let _model_array_end = model_array_offset
			.checked_add(model_count)
			.filter(|end| *end <= input.len())
			.ok_or(Error::ModelArrayPastEOF)?;

		dbg!(model_count);
		let mut models = Vec::with_capacity(model_count);
		reader.seek(model_array_offset)?;
		let offsets = reader.read_boxed_array(model_count, |reader, _| reader.read_u32())?;
		for (i, offset) in offsets.into_iter().enumerate() {
			reader.seek(offset as _)?;
			let model = KclMesh::read(&mut reader).map_err(|error| Error::Model {
				source: error,
				model_index: i,
			})?;
			models.push(model);
		}

		let octree = ModelOctree::start(&mut reader, model_octree_offset as usize)?;

		Ok(Kcl {
			models,
			octree,
			bounds_min,
			bounds_max,
		})
	}
}
