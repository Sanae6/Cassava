use snafu::{ResultExt, Snafu};

use crate::{
	nw::{
		FileHeaderError,
		bfres::{self, ModelArchive},
		read_binary_file_header,
	},
	util::{self, BinaryReader},
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
	#[snafu(transparent)]
	ModelArchiveError {
		source: bfres::Error,
	},
	NoModelArchive,
}

#[derive(Debug)]
pub struct EffectArchive {
	model_archive: ModelArchive,
}

impl EffectArchive {
	pub fn read(reader: &mut BinaryReader) -> Result<EffectArchive, Error> {
		let (file_info, file_header) =
			read_binary_file_header(reader, Some(*b"VFXB    ")).context(FileHeaderSnafu)?;
		reader.seek(file_header.first_block_offset as usize)?;
		let mut model_archive = None;
		loop {
			let block_start = reader.position();
			let magic = reader.read_bytes::<4>()?;
			dbg!(String::from_utf8_lossy(&magic));
			let node_size = reader.read_u32()?;
			let next_child_node = block_start + reader.read_u32()? as usize;
			let sibling_node_offset = reader.read_u32()? as usize;
			if sibling_node_offset == u32::MAX as usize {
				break;
			}
			let first_attribute = block_start + reader.read_u32()? as usize;
			let binary_data = block_start + reader.read_u32()? as usize;
			reader.seek_rel(4)?;
			let child_count = reader.read_u16()?;
			match &magic {
				b"G3PR" => model_archive = Some(Self::read_g3pr(reader, binary_data)?),
				_ => {}
			}
			reader.seek(block_start + sibling_node_offset as usize)?;
		}
		Ok(EffectArchive {
			model_archive: model_archive.ok_or(Error::NoModelArchive)?,
		})
	}

	fn read_g3pr(
		reader: &mut BinaryReader,
		binary_data: usize,
	) -> Result<ModelArchive, bfres::Error> {
		reader
			.subfile_scope(binary_data, |reader| -> Result<(), util::Error> {
				let ex = read_binary_file_header(reader, None).unwrap();
				reader.seek(0)?;
				let data = reader.read_slice(ex.1.file_size as usize)?;
				std::fs::write("target/EffectData.bfres", data).unwrap();
				Ok(())
			})
			.unwrap();
		reader.subfile_scope(binary_data, ModelArchive::read)
	}
}
