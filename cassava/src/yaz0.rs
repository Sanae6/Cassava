use std::io::{Cursor, Seek};

use byteordered::ByteOrdered;
use snafu::prelude::*;

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	Io {
		source: std::io::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("file signature must be `Yaz0`"))]
	InvalidSignature,
	#[snafu(display("offset was larger than the current output buffer size"))]
	BufferOverrun,
}

pub fn decompress(input: &[u8]) -> Result<(Vec<u8>, u32), Error> {
	let mut reader = ByteOrdered::be(Cursor::new(input));

	if !input.starts_with(b"Yaz0") {
		return Err(Error::InvalidSignature);
	};

	reader.seek_relative(4)?;
	let uncompressed_size = reader.read_u32()? as usize;
	let alignment = reader.read_u32()?;
	reader.seek_relative(4)?;

	let mut out = vec![0; uncompressed_size];
	let mut dst_idx = 0;

	while reader.inner_mut().position() < (input.len() as u64) {
		let code_byte = reader.read_u8()?;
		for i in (0..=7).rev() {
			let is_copy = (code_byte >> i) & 0x01 == 1;

			if dst_idx >= uncompressed_size {
				break;
			}

			if is_copy {
				let byte = reader.read_u8()?;
				out[dst_idx] = byte;
				dst_idx += 1;
			} else {
				let data = reader.read_u16()?;

				let count = match data >> 0xc {
					0 => reader.read_u8()? as u16 + 0x12,
					count => count + 2,
				} as usize;

				let offset = (data & 0xfff) as usize + 1;
				if offset > dst_idx {
					return Err(Error::BufferOverrun);
				}

				for _ in 0..count {
					out[dst_idx] = out[dst_idx - offset];
					dst_idx += 1;
				}
			}
		}
	}

	Ok((out, alignment))
}
