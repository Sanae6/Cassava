use std::{fs, mem::MaybeUninit, path::PathBuf};

use glam::{IVec3, UVec3, Vec2, Vec3};
use snafu::prelude::*;

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(display("alignment must be a power of two"))]
	InvalidAlignment,
	#[snafu(display("tried to read past EOF"))]
	OutOfBoundsRead,
	#[snafu(display("tried to seek past EOF"))]
	OutOfBoundsSeek,
}

#[derive(Debug, Clone, Copy)]
pub enum ByteOrder {
	Big,
	Little,
}

pub struct BinaryWriter {
	byte_order: ByteOrder,
	cursor: usize,
	inner: Vec<u8>,
}

impl BinaryWriter {
	pub fn new(byte_order: ByteOrder) -> Self {
		Self {
			inner: vec![],
			cursor: 0,
			byte_order,
		}
	}

	pub fn position(&self) -> usize {
		self.cursor
	}

	pub fn take_inner(self) -> Vec<u8> {
		self.inner
	}

	pub fn save_to_file(&self, path: &PathBuf) -> Result<(), std::io::Error> {
		fs::write(path, &self.inner)?;
		Ok(())
	}

	pub fn seek(&mut self, offset: usize) {
		self.cursor = offset;
		if self.cursor > self.inner.len() {
			self
				.inner
				.append(&mut vec![0; self.cursor - self.inner.len()]);
		}
	}

	pub fn seek_rel(&mut self, offset: usize) {
		self.seek(self.cursor + offset);
	}

	pub fn align_up(&mut self, alignment: usize) -> Result<(), Error> {
		if alignment.count_ones() == 1 {
			self.cursor = (self.cursor + alignment - 1) & !(alignment - 1);
			Ok(())
		} else {
			Err(Error::InvalidAlignment)
		}
	}

	pub fn write_bytes(&mut self, data: &[u8]) {
		let data_end = self.cursor + data.len();
		if data_end > self.inner.len() {
			self.inner.append(&mut vec![0; data_end - self.inner.len()]);
		}

		for byte in data {
			self.inner[self.cursor] = *byte;
			self.cursor += 1;
		}
	}

	pub fn write_u8(&mut self, data: u8) {
		self.write_bytes(&[data]);
	}

	pub fn write_u16(&mut self, data: u16) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_u32(&mut self, data: u32) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_u64(&mut self, data: u64) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_i8(&mut self, data: i8) {
		self.write_bytes(&data.to_be_bytes());
	}

	pub fn write_i16(&mut self, data: i16) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_i32(&mut self, data: i32) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_i64(&mut self, data: i64) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_string(&mut self, data: &str, has_null_term: bool) {
		self.write_bytes(&data.bytes().collect::<Vec<u8>>());
		if has_null_term {
			self.write_u8(0);
		}
	}
}

pub struct BinaryReader<'a> {
	byte_order: ByteOrder,
	cursor: usize,
	inner: &'a [u8],
}

impl<'a> BinaryReader<'a> {
	pub fn new(data: &'a [u8], byte_order: ByteOrder) -> Self {
		Self {
			byte_order,
			cursor: 0,
			inner: data,
		}
	}

	pub fn set_byte_order(&mut self, byte_order: ByteOrder) {
		self.byte_order = byte_order;
	}

	pub fn position(&self) -> usize {
		self.cursor
	}

	pub fn seek(&mut self, offset: usize) -> Result<(), Error> {
		if offset > self.inner.len() {
			Err(Error::OutOfBoundsSeek)
		} else {
			self.cursor = offset;
			Ok(())
		}
	}

	pub fn seek_rel(&mut self, offset: usize) -> Result<(), Error> {
		self.seek(self.cursor + offset)
	}

	pub fn read_bytes(&mut self, count: usize) -> Result<&[u8], Error> {
		let start = self.cursor;
		let end = start + count;
		if end > self.inner.len() {
			return Err(Error::OutOfBoundsRead);
		}
		self.cursor += count;
		Ok(&self.inner[start..end])
	}

	pub fn read_until(&mut self, terminator: u8) -> &[u8] {
		let start = self.cursor;
		let mut end = self.inner.len();
		for (idx, byte) in self.inner[start..].iter().enumerate() {
			if *byte == terminator {
				self.cursor += idx;
				end = start + idx;
				break;
			}
		}
		&self.inner[start..end]
	}

	pub fn read_array<const N: usize, R>(
		&mut self,
		mut func: impl FnMut(&mut Self) -> Result<R, Error>,
	) -> Result<[R; N], Error> {
		let mut array = [const { MaybeUninit::uninit() }; N];

		for el in &mut array {
			*el = MaybeUninit::new(func(self)?);
		}

		// safety: this point can only be reached once every element in the array contains
		// an initialized value.
		Ok(unsafe { array.map(|el| el.assume_init()) })
	}

	pub fn read_boxed_array<R>(
		&mut self,
		count: usize,
		mut func: impl FnMut(&mut Self) -> Result<R, Error>,
	) -> Result<Box<[R]>, Error> {
		let mut array = Box::new_uninit_slice(count);

		for el in &mut array {
			*el = MaybeUninit::new(func(self)?);
		}

		// safety: this point can only be reached once every element in the array contains
		// an initialized value.
		Ok(unsafe { array.assume_init() })
	}

	pub fn read_u8(&mut self) -> Result<u8, Error> {
		let data = self.read_bytes(1)?;
		Ok(data[0])
	}

	#[doc(alias = "read_s8")]
	pub fn read_i8(&mut self) -> Result<i8, Error> {
		Ok(self.read_u8()? as i8)
	}

	pub fn read_u16(&mut self) -> Result<u16, Error> {
		let byte_order = self.byte_order;
		let data = self.read_bytes(2)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => u16::from_be_bytes(data),
			ByteOrder::Little => u16::from_le_bytes(data),
		})
	}

	#[doc(alias = "read_s16")]
	pub fn read_i16(&mut self) -> Result<i16, Error> {
		Ok(self.read_u16()? as i16)
	}

	pub fn read_u32(&mut self) -> Result<u32, Error> {
		let byte_order = self.byte_order;
		let data = self.read_bytes(4)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => u32::from_be_bytes(data),
			ByteOrder::Little => u32::from_le_bytes(data),
		})
	}

	#[doc(alias = "read_s32")]
	pub fn read_i32(&mut self) -> Result<i32, Error> {
		Ok(self.read_u32()? as i32)
	}

	pub fn read_u64(&mut self) -> Result<u64, Error> {
		let byte_order = self.byte_order;
		let data = self.read_bytes(8)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => u64::from_be_bytes(data),
			ByteOrder::Little => u64::from_le_bytes(data),
		})
	}

	#[doc(alias = "read_s64")]
	pub fn read_i64(&mut self) -> Result<i64, Error> {
		Ok(self.read_u64()? as i64)
	}

	pub fn read_f32(&mut self) -> Result<f32, Error> {
		let byte_order = self.byte_order;
		let data = self.read_bytes(4)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => f32::from_be_bytes(data),
			ByteOrder::Little => f32::from_le_bytes(data),
		})
	}

	pub fn read_f64(&mut self) -> Result<f64, Error> {
		let byte_order = self.byte_order;
		let data = self.read_bytes(4)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => f64::from_be_bytes(data),
			ByteOrder::Little => f64::from_le_bytes(data),
		})
	}

	pub fn read_vec2(&mut self) -> Result<Vec2, Error> {
		Ok(Vec2::new(self.read_f32()?, self.read_f32()?))
	}

	pub fn read_vec3(&mut self) -> Result<Vec3, Error> {
		Ok(Vec3::new(
			self.read_f32()?,
			self.read_f32()?,
			self.read_f32()?,
		))
	}

	pub fn read_ivec3(&mut self) -> Result<IVec3, Error> {
		Ok(IVec3::new(
			self.read_i32()?,
			self.read_i32()?,
			self.read_i32()?,
		))
	}

	pub fn read_uvec3(&mut self) -> Result<UVec3, Error> {
		Ok(UVec3::new(
			self.read_u32()?,
			self.read_u32()?,
			self.read_u32()?,
		))
	}
}
