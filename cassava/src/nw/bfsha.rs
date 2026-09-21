use indexmap::IndexMap;
use snafu::{ResultExt, Snafu};

use crate::{
	nw::{
		BinaryFileInfo, FileHeaderError, read_binary_file_header, read_owned_string_at,
		res_dict::{DictError, read_dict},
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
		source: FileHeaderError,
	},
	UnsupportedVersion,
	#[snafu(transparent)]
	ModelError {
		#[snafu(backtrace)]
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
	#[snafu(display("failed to read shader container file header"))]
	#[snafu(context(suffix(ModelSnafu)))]
	FileHeaderError { source: FileHeaderError },
}

#[derive(Debug)]
pub struct ShaderArchive {
	pub file_info: BinaryFileInfo,
	pub file_name: String,
	pub models: IndexMap<String, ShaderModel>,
}
pub struct ShaderModel {
	pub name: String,
	pub container: Vec<u8>,
}

impl std::fmt::Debug for ShaderModel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ShaderModel")
			.field("name", &self.name)
			.field("container(len)", &self.container.len())
			.finish()
	}
}

const DICT: [u8; 4] = [0; 4];

impl ShaderArchive {
	pub fn read(reader: &mut BinaryReader) -> Result<Self, Error> {
		let (file_info, _file_header, mut reader) =
			read_binary_file_header(reader, Some(*b"FSHA    ")).context(FileHeaderSnafu)?;
		// println!("{file_info:?}");

		let archive_start = reader.read_u64()? as _;

		reader.seek(archive_start)?;
		println!("{archive_start:?}");

		let file_name = reader.read_u64()? as usize;
		let file_name = read_owned_string_at(&mut reader, file_name)?;
		// println!("file name: {file_name}");
		let _original_path = reader.read_u64()? as usize;
		let (shader_model_array, shader_model_dictionary) =
			(reader.read_u64()? as usize, reader.read_u64()? as usize);

		let models = read_dict::<ShaderModel, ModelError>(
			&mut reader,
			DICT,
			shader_model_array,
			shader_model_dictionary,
			|mut reader, _name| {
				// 0xe8
				let start = reader.position();
				let name = reader.read_u64()? as usize;
				let name = read_owned_string_at(&mut reader, name)?;
				reader.seek(start + 0x98)?;
				let bnsh_offset = reader.read_u64()? as usize;
				let file_size = reader.scope(bnsh_offset, |reader| -> Result<u32, ModelError> {
					let (_file_info, file_header, mut _reader) =
						read_binary_file_header(reader, Some(*b"BNSH\0\0\0\0"))
							.context(FileHeaderModelSnafu)?;
					Ok(file_header.file_size)
				})?;
				let container = reader
					.read_slice_at(bnsh_offset, file_size as usize)?
					.to_vec();
				Ok(ShaderModel { name, container })
			},
		)?;

		Ok(ShaderArchive {
			file_info,
			file_name,
			models,
		})
	}
}
