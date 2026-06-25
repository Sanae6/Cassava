use glam::{U16Vec3, Vec3};

use crate::{
	kcl::{OctreeError, octree::KEY_MASK},
	util::BinaryReader,
};

pub enum PrismOctree {
	Tree(Box<[PrismOctree]>),
	Leaf { some_u16: u16, prisms: Box<[u16]> },
}

impl PrismOctree {
	pub(in crate::kcl) fn start(
		reader: &mut BinaryReader,
		offset: usize,
		node_count: usize,
	) -> Result<PrismOctree, OctreeError> {
		reader.seek(offset as usize)?;
		let branch_keys: Box<[u32]> =
			reader.read_boxed_array(node_count, |reader| reader.read_u32())?;

		let mut branches = Vec::with_capacity(branch_keys.len());
		for key in branch_keys {
			branches.push(Self::read(reader, offset, key)?);
		}

		Ok(PrismOctree::Tree(branches.into_boxed_slice()))
	}

	fn read(reader: &mut BinaryReader, offset: usize, key: u32) -> Result<PrismOctree, OctreeError> {
		let node_type = key >> 30;
		let value = key & KEY_MASK;
		match node_type {
			0b10 => {
				// leaf node
				reader.seek(offset + value as usize)?;
				let mut prisms = Vec::new();
				let unknown = reader.read_u16()?;

				loop {
					let index = reader.read_u16()?;
					if index == 0xFFFF {
						break;
					}
					prisms.push(index);
				}

				Ok(PrismOctree::Leaf {
					some_u16: unknown,
					prisms: prisms.into(),
				})
			}
			0b00 => {
				let offset = offset + value as usize;
				reader.seek(offset)?;
				let branch_keys: [u32; 8] = reader.read_array(|reader| reader.read_u32())?;
				let mut branches = [const { None }; 8];
				for i in 0..8 {
					branches[i] = Some(Self::read(reader, offset, branch_keys[i])?);
				}
				Ok(PrismOctree::Tree(Box::new(branches.map(Option::unwrap))))
			}
			_ => Err(OctreeError::InvalidOctreeNodeType),
		}
	}
}
