use std::collections::HashMap;

use snafu::Snafu;

use crate::{
	nw::res_dict::{self, DictError, read_dict},
	util::{self, BinaryReader},
};

#[derive(Snafu, Debug)]
pub enum UserDataError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
}

#[derive(Clone, Debug)]
pub enum UserData {
	S32(i32),
	F32(f32),
	String(String),
	EmbeddedFile(Vec<u8>),
}

pub fn read_gfx_user_data(
	reader: &mut BinaryReader,
	value_list: usize,
	dictionary_list: usize,
) -> Result<HashMap<String, UserData>, res_dict::DictError<UserDataError>> {
  if value_list == 0 || dictionary_list == 0 {
    return Ok(HashMap::new())
  }

	read_dict(reader, value_list, dictionary_list, |_reader, _| {
    todo!()
	})
}
