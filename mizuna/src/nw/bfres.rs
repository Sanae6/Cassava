use snafu::Snafu;

use crate::nw::{self, BinaryFileInfo};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("failed to read file header"))]
	FileHeaderError {
		source: nw::FileHeaderError,
	},
	UnsupportedVersion,
	#[snafu(display("expected magic to be 'BNTX\0\0\0\0'"))]
	IncorrectContainerMagic,

}

pub struct ModelArchive {
	pub file_info: BinaryFileInfo,
}

impl ModelArchive {
  pub fn read() -> Result<ModelArchive, Error> {
    todo!()
  }
}
