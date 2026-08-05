use std::{
	io::Read,
	pin::Pin,
	task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use snafu::prelude::*;

use crate::util::{self, BinaryReader};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	Io {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("file signature must be `Yaz0`"))]
	InvalidSignature,
	#[snafu(display("offset was larger than the current output buffer size"))]
	BufferOverrun,
}

pub fn decompress(input: &[u8]) -> Result<(Vec<u8>, u32), Error> {
	let mut reader = BinaryReader::new(input).with_byte_order(util::ByteOrder::Big);

	if !input.starts_with(b"Yaz0") {
		return Err(Error::InvalidSignature);
	};

	reader.seek_rel(4)?;
	let uncompressed_size = reader.read_u32()? as usize;
	let alignment = reader.read_u32()?;
	reader.seek_rel(4)?;

	let mut out = vec![0; uncompressed_size];
	let mut dst_idx = 0;

	while reader.position() < reader.len() {
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

pub struct Yaz0Stream<'a> {
	alignment: u32,
	decompressed_size: usize,
	coroutine: Option<Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>>,
}

impl<'a> Yaz0Stream<'a> {
	pub fn new(input: &'a [u8]) -> Result<Self, Error> {
		let mut reader = BinaryReader::new(input).with_byte_order(util::ByteOrder::Big);
		if !input.starts_with(b"Yaz0") {
			return Err(Error::InvalidSignature);
		};

		reader.seek_rel(4)?;
		let decompressed_size = reader.read_u32()? as usize;
		let alignment = reader.read_u32()?;
		reader.seek_rel(4)?;

		let coroutine = Box::pin(Self::coroutine(reader.clone(), decompressed_size));

		Ok(Self {
			alignment,
			decompressed_size,
			coroutine: Some(coroutine),
		})
	}

	pub fn alignment(&self) -> u32 {
		self.alignment
	}

	pub fn decompressed_size(&self) -> usize {
		self.decompressed_size
	}

	fn coroutine(
		mut reader: BinaryReader<'a>,
		decompressed_size: usize,
	) -> impl Future<Output = Result<(), Error>> + 'a {
		async move {
			const MAX_BACKREF: usize = 0x1000;
			let mut out_ring = [0u8; MAX_BACKREF];
			let mut dst_idx = 0;

			while reader.position() < reader.len() {
				let code_byte = reader.read_u8()?;
				for i in (0..=7).rev() {
					let is_copy = (code_byte >> i) & 0x01 == 1;

					if dst_idx >= decompressed_size {
						break;
					}

					if is_copy {
						let byte = reader.read_u8()?;
						out_ring[dst_idx % MAX_BACKREF] = byte;
						push_byte(byte).await;
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
							let value = out_ring[(dst_idx - offset) % MAX_BACKREF];
							out_ring[dst_idx % MAX_BACKREF] = value;
							push_byte(value).await;
							dst_idx += 1;
						}
					}
				}
			}

			Ok(())
		}
	}
}

#[repr(C, packed)]
struct OuterData<'a> {
	slice: &'a mut [u8],
	index: usize,
}

		// life isn't worth living without a little danger :)
fn push_byte(value: u8) -> impl Future<Output = ()> {
	enum Fut {
		Pushing(u8),
		Pushed,
	}

	impl Future for Fut {
		type Output = ();

		fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
			match &mut *self {
				Fut::Pushing(value) => {
					let data = cx.waker().data().cast::<OuterData>().cast_mut();
					let data = unsafe { &mut *data };
					if data.slice.len() == data.index {
						return Poll::Pending;
					}
					data.slice[data.index] = *value;
					data.index += 1;
					*self = Fut::Pushed;
					if data.slice.len() == data.index {
						Poll::Pending
					} else {
						Poll::Ready(())
					}
				}
				Fut::Pushed => Poll::Ready(()),
			}
		}
	}
	Fut::Pushing(value)
}
// taken directly from rust's standard library
fn noop() -> &'static RawWakerVTable {
	static NOOP: RawWakerVTable =
		RawWakerVTable::new(|data| RawWaker::new(data, &NOOP), |_| {}, |_| {}, |_| {});
	&NOOP
}

impl<'a> Read for Yaz0Stream<'a> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let Some(coroutine) = self.coroutine.as_mut() else {
			return Ok(0);
		};
		let mut data_ptr = OuterData {
			slice: buf,
			index: 0,
		};
		let waker = unsafe { Waker::from_raw(RawWaker::new((&raw mut data_ptr).cast(), &noop())) };
		let mut context = Context::from_waker(&waker);
		match coroutine.as_mut().poll(&mut context) {
			Poll::Ready(result) => {
				self.coroutine.take();
				result.map(|_| buf.len()).map_err(std::io::Error::other)
			}
			Poll::Pending => Ok(buf.len()),
		}
	}
}
