use crate::{kcl::{Error, OctreeError, octree::KEY_MASK}, util::BinaryReader};

pub enum ModelOctree {
	Tree(Box<[ModelOctree; 8]>),
	ModelRef(u32),
	Empty,
}

impl ModelOctree {
	pub(in crate::kcl) fn start(reader: &mut BinaryReader, offset: usize) -> Result<ModelOctree, Error> {
		reader.seek(offset as usize)?;
		let branch_keys: [u32; 8] = reader.read_array(|reader| reader.read_u32())?;
		let mut branches = [const { None }; 8];
		for (i, key) in branch_keys.into_iter().enumerate() {
			branches[i] = Some(Self::read(reader, offset, key)?);
		}

		Ok(ModelOctree::Tree(Box::new(branches.map(Option::unwrap))))
	}

	fn read(reader: &mut BinaryReader, offset: usize, key: u32) -> Result<ModelOctree, OctreeError> {
		let node_type = key >> 30;
		let value = key & KEY_MASK;
		// println!(
		// 	"{node_type} {:X} {:X} {value}",
		// 	key & !KEY_MASK,
		// 	(key & !KEY_MASK) >> 30
		// );
		match node_type {
			0b11 => Ok(ModelOctree::Empty),
			0b10 => Ok(ModelOctree::ModelRef(key & KEY_MASK)),
			0b00 => {
				let offset = offset + value as usize * size_of::<u32>();
				reader.seek(offset)?;
				let branch_keys: [u32; 8] = reader.read_array(|reader| reader.read_u32())?;
				let mut branches = [const { None }; 8];
				for i in 0..8 {
					branches[i] = Some(Self::read(reader, offset, branch_keys[i])?);
				}
				Ok(ModelOctree::Tree(Box::new(branches.map(Option::unwrap))))
			}
			_ => Err(OctreeError::InvalidOctreeNodeType),
		}
	}

}
