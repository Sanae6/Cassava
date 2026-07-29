use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
	nw::{
		FileHeaderError,
		bfres::{self, ModelArchive},
		bntx::{self, TextureArchive},
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
		#[snafu(backtrace)]
		source: FileHeaderError,
	},
	#[snafu(transparent)]
	BlockError {
		#[snafu(backtrace)]
		source: BlockError,
	},
	#[snafu(transparent)]
	ModelArchiveError {
		#[snafu(backtrace)]
		source: bfres::Error,
	},
	#[snafu(transparent)]
	TextureArchiveError {
		#[snafu(backtrace)]
		source: bntx::Error,
	},
	#[snafu(display("effect archive has no embedded model archive (BFRES)"))]
	NoModelArchive,
	#[snafu(display("effect archive has no embedded texture archive (BNTX)"))]
	NoTextureArchive,
	#[snafu(display("effect archive has no embedded shader archive (BNSH)"))]
	NoShaderArchive,
}

#[derive(Snafu, Debug)]
pub enum BlockError {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	NextSiblingOutOfBounds {
		block_start: usize,
		backtrace: snafu::Backtrace,
	},
	FirstChildOutOfBounds {
		block_start: usize,
		backtrace: snafu::Backtrace,
	},
	FirstAttributeOutOfBounds {
		block_start: usize,
		backtrace: snafu::Backtrace,
	},
	DataOutOfBounds {
		block_start: usize,
		backtrace: snafu::Backtrace,
	},
	MissingDataStart {
		block_start: usize,
		backtrace: snafu::Backtrace,
	},
}

#[derive(Debug)]
pub struct EffectArchive {
	model_archive: ModelArchive,
	texture_archive: TextureArchive,
}

struct BinaryData {
	magic: [u8; 4],
	block_size: u32,
	first_child: Option<usize>,
	next_sibling: Option<usize>,
	first_attribute: Option<usize>,
	data_start: usize,
	child_count: u16,
}

const END_OF_LIST_MARKER: usize = u32::MAX as usize;

impl EffectArchive {
	pub fn read(reader: &mut BinaryReader) -> Result<EffectArchive, Error> {
		let (file_info, file_header) =
			read_binary_file_header(reader, Some(*b"VFXB    ")).context(FileHeaderSnafu)?;
		reader.seek(file_header.first_block_offset as usize)?;
		let mut model_archive = None;
		let mut texture_archive = None;
		loop {
			let binary_data = Self::read_binary_data(reader)?;
			// println!("{} {first_child_block:X}");
			let magic_string = String::from_utf8_lossy(&binary_data.magic);
			println!("root block: {magic_string:?}");
			println!("{} {:?}", binary_data.child_count, binary_data.first_child);
			match &binary_data.magic {
				b"ESTA" => {
					Self::read_emitter_sets(reader, binary_data.first_child).unwrap();
				}
				b"PRMA" => {}
				// G3D Primitives
				b"G3PR" => model_archive = Some(Self::read_model_archive(reader, binary_data.data_start)?),
				// G3D Resource Shaders
				b"GRSN" => {
					Self::read_shader_archive(reader, binary_data.data_start);
					eprintln!("grsning it");
				}
				b"GRTF" => {
					texture_archive = Some(Self::read_texture_archive(reader, binary_data.data_start)?);
				}
				_ => {}
			}
			let Some(next_sibling) = binary_data.next_sibling else {
				break;
			};
			reader.seek(next_sibling)?;
		}
		Ok(EffectArchive {
			model_archive: model_archive.ok_or(Error::NoModelArchive)?,
			texture_archive: texture_archive.ok_or(Error::NoTextureArchive)?,
		})
	}

	fn read_binary_data(reader: &mut BinaryReader) -> Result<BinaryData, BlockError> {
		let block_start = reader.position();
		let magic = reader.read_bytes::<4>()?;
		let block_size = reader.read_u32()?;
		let read_offset = |reader: &mut BinaryReader, error: BlockError| {
			let offset = reader.read_u32()? as usize;
			if offset != END_OF_LIST_MARKER {
				block_start
					.checked_add(offset)
					.filter(|_| offset < reader.len())
					.map(Some)
					.ok_or_else(|| error)
			} else {
				Ok(None)
			}
		};
		let first_child = read_offset(reader, FirstChildOutOfBoundsSnafu { block_start }.build())?;
		let next_sibling = read_offset(reader, NextSiblingOutOfBoundsSnafu { block_start }.build())?;
		let first_attribute = read_offset(
			reader,
			FirstAttributeOutOfBoundsSnafu { block_start }.build(),
		)?;
		let data_start = read_offset(reader, DataOutOfBoundsSnafu { block_start }.build())?
			.ok_or(MissingDataStartSnafu { block_start }.build())?;
		reader.seek_rel(4)?;
		let child_count = reader.read_u16()?;

		Ok(BinaryData {
			magic,
			block_size,
			first_child,
			next_sibling,
			first_attribute,
			data_start,
			child_count,
		})
	}

	fn iter_blocks<'a>(reader: &BinaryReader<'a>, first_block: Option<usize>) -> BlockIter<'a> {
		BlockIter {
			reader: reader.clone(),
			next_block: first_block,
		}
	}

	fn read_emitter_sets(reader: &mut BinaryReader, first_child: Option<usize>) -> Result<(), Error> {
		for child in Self::iter_blocks(reader, first_child) {
			let (binary_data, mut reader) = child?;
			// let magic_string = String::from_utf8_lossy(&binary_data.magic);
			let name = reader.scope(binary_data.data_start + 0x10, |reader| {
				reader.read_null_terminated_string()
			})?;
			println!("\t{name}");

			for child in Self::iter_blocks(&reader, binary_data.first_child) {
				let (binary_data, mut reader) = child?;

				let magic_string = String::from_utf8_lossy(&binary_data.magic);
				let name = reader.scope(binary_data.data_start + 0x10, |reader| {
					reader.read_null_terminated_string()
				})?;
				println!("\t\t{name}");
				// println!("\t\t{magic_string}");
				for child in Self::iter_blocks(&reader, binary_data.first_child) {
					let (binary_data, mut reader) = child?;
					let name = reader.scope(binary_data.data_start + 0x10, |reader| {
						reader.read_null_terminated_string()
					})?;
					println!("\t\t\t{name}");

					let magic_string = String::from_utf8_lossy(&binary_data.magic);
					// println!("\t\t\t{magic_string}");
					for child in Self::iter_blocks(&reader, binary_data.first_attribute) {
						let (binary_data, reader) = child?;

						let magic_string = String::from_utf8_lossy(&binary_data.magic);
						println!("\t\t\t\t{magic_string}");
					}
				}
			}
		}

		Ok(())
	}

	fn read_texture_archive(
		reader: &mut BinaryReader,
		data_start: usize,
	) -> Result<TextureArchive, bntx::Error> {
		reader.subfile_scope(data_start, TextureArchive::read)
	}

	fn read_model_archive(
		reader: &mut BinaryReader,
		data_start: usize,
	) -> Result<ModelArchive, bfres::Error> {
		// reader
		// 	.subfile_scope(data_start, |reader| -> Result<(), util::Error> {
		// 		let ex = read_binary_file_header(reader, None).unwrap();
		// 		reader.seek(0)?;
		// 		let data = reader.read_slice(ex.1.file_size as usize)?;
		// 		std::fs::write("target/EffectData.bfres", data).unwrap();
		// 		Ok(())
		// 	})
		// 	.unwrap();

		reader.subfile_scope(data_start, ModelArchive::read)
	}

	fn read_shader_archive(reader: &mut BinaryReader, data_start: usize) {
		// reader
		// 	.subfile_scope(data_start, |reader| -> Result<(), util::Error> {
		// 		let ex = read_binary_file_header(reader, None).unwrap();
		// 		reader.seek(0)?;
		// 		let data = reader.read_slice(ex.1.file_size as usize)?;
		// 		std::fs::write("target/EffectData.bfsha", data).unwrap();
		// 		Ok(())
		// 	})
		// 	.unwrap();
	}
}

struct BlockIter<'a> {
	reader: BinaryReader<'a>,
	next_block: Option<usize>,
}

impl<'a> Iterator for BlockIter<'a> {
	type Item = Result<(BinaryData, BinaryReader<'a>), BlockError>;

	fn next(&mut self) -> Option<Self::Item> {
		let next_child = self.next_block?;
		let result = self
			.reader
			.scope(next_child, |reader| EffectArchive::read_binary_data(reader));
		match result {
			Ok(binary_data) => {
				self.next_block = binary_data.next_sibling;
				Some(Ok((binary_data, self.reader.clone())))
			}
			Err(error) => Some(Err(error)),
		}
	}
}
