#[allow(unused)]
pub mod bffnt;
pub mod bfres;
pub mod bfsha;
pub mod bnsh;
pub mod bntx;
pub mod ptcl;
pub mod res_dict;
pub mod user_data;
pub mod bfsar;

use snafu::{OptionExt, ResultExt, Snafu};

use crate::util::{self, BinaryReader};

#[derive(Snafu, Debug)]
pub enum FileHeaderError {
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
	#[snafu(display("incorrect file magic"))]
	IncorrectFileMagic,
	#[snafu(display("unexpected byte order"))]
	InvalidByteOrder,
	// #[snafu(display("unsupported address size: {address_size}"))]
	// UnsupportedAddressSize { address_size: u16 },
	#[snafu(display("arithmetic overflow"))]
	FileNameOutOfBounds,
	#[snafu(display("file-defined file size is too small"))]
	IncorrectFileSize {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("unable to seek to after the file header"))]
	FileTooSmall {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
}

#[derive(Debug)]
pub struct BinaryFileInfo {
	pub micro_version: u8,
	pub minor_version: u8,
	pub major_version: u16,
	pub packed_alignment: u8,
	pub file_name: String,
}

#[allow(unused)]
#[derive(Debug)]
pub struct BinaryFileHeader {
	magic: [u8; 8],
	first_block_offset: u16,
	relocation_table_offset: u32,
	file_size: u32,
}

pub fn read_binary_file_header<'a>(
	reader: &mut BinaryReader<'a>,
	expected_magic: Option<[u8; 8]>,
) -> Result<(BinaryFileInfo, BinaryFileHeader, BinaryReader<'a>), FileHeaderError> {
	let start = reader.position();
	reader.seek_rel(12)?;
	match reader.read_bytes::<2>()? {
		[0xfe, 0xff] => reader.set_byte_order(util::ByteOrder::Big),
		[0xff, 0xfe] => reader.set_byte_order(util::ByteOrder::Little),
		_ => return Err(FileHeaderError::InvalidByteOrder),
	}

	reader.seek_rel(-14).unwrap();
	let magic = reader.read_bytes::<8>()?;
	if let Some(expected_magic) = expected_magic
		&& expected_magic != magic
	{
		return Err(FileHeaderError::IncorrectFileMagic);
	}

	let (micro_version, minor_version, major_version) =
		(reader.read_u8()?, reader.read_u8()?, reader.read_u16()?);
	reader.seek_rel(2)?;
	let packed_alignment = reader.read_u8()?;
	let _address_size = reader.read_u8()?;
	// not true lol
	// snafu::ensure!(
	// 	address_size == 64,
	// 	UnsupportedAddressSizeSnafu { address_size }
	// );
	let file_name_offset = reader.read_u32()?;
	reader.seek_rel(2)?;
	let (first_block_offset, relocation_table_offset, file_size) =
		(reader.read_u16()?, reader.read_u32()?, reader.read_u32()?);
	let file_name_offset = start
		.checked_add(file_name_offset as _)
		.context(FileNameOutOfBoundsSnafu)?;
	let file_name = reader.scope::<_, FileHeaderError>(file_name_offset, |reader| {
		Ok(reader.read_null_terminated_string()?.to_owned())
	})?;
	let end_of_header = reader.position();
	let mut reader = BinaryReader::new(
		reader
			.scope(start, |reader| reader.read_slice(file_size as usize))
			.context(IncorrectFileSizeSnafu)?,
	);
	reader
		.seek_rel(end_of_header as _)
		.context(FileTooSmallSnafu)?;

	Ok((
		BinaryFileInfo {
			micro_version,
			minor_version,
			major_version,
			packed_alignment,
			file_name,
		},
		BinaryFileHeader {
			magic,
			first_block_offset,
			relocation_table_offset,
			file_size,
		},
		reader,
	))
}

fn read_string_at<'a>(
	reader: &mut BinaryReader<'a>,
	offset: usize,
) -> Result<&'a str, util::Error> {
	reader.scope(offset, |reader| {
		let name_size = reader.read_u16()?;
		reader.read_string(name_size as _)
	})
}

fn read_owned_string_at(reader: &mut BinaryReader, offset: usize) -> Result<String, util::Error> {
	reader.scope(offset, |reader| {
		let name_size = reader.read_u16()?;
		reader.read_owned_string(name_size as _)
	})
}
