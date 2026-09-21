use snafu::{Snafu, ensure};

use crate::util::{self, BinaryReader};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	InvalidByteOrder,
	InvalidMagic,
	IncorrectHeaderSize,
	UnsupportedVersion,
}

#[derive(Debug)]
pub struct SoundArchive {}

impl SoundArchive {
	pub fn read(reader: &mut BinaryReader) -> Result<SoundArchive, Error> {
		let magic = reader.read_bytes::<4>()?;
		ensure!(&magic == b"FSAR", InvalidMagicSnafu);
		match reader.read_bytes::<2>()? {
			[0xfe, 0xff] => reader.set_byte_order(util::ByteOrder::Big),
			[0xff, 0xfe] => reader.set_byte_order(util::ByteOrder::Little),
			_ => return Err(Error::InvalidByteOrder),
		}

		ensure!(reader.read_u16()? == 0x40, IncorrectHeaderSizeSnafu);
		let (revision, minor, major) = (reader.read_u8()?, reader.read_u8()?, reader.read_u16()?);
		ensure!(
			(major, minor, revision) == (2, 4, 0),
			UnsupportedVersionSnafu
		);
		let _file_size = reader.read_u32()?;
		let section_count = reader.read_u16()?;
		let _ = reader.read_u16()?;

		todo!()
	}
}
