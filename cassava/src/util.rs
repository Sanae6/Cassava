use std::{fs, mem::MaybeUninit, path::PathBuf};

use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use num_traits::PrimInt;
use snafu::prelude::*;

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(display("alignment must be a power of two"))]
	InvalidAlignment,
	#[snafu(display("tried to read past EOF"))]
	OutOfBoundsRead,
	#[snafu(display("tried to seek past EOF"))]
	OutOfBoundsSeek,
	#[snafu(display("data must be utf-8 text"))]
	InvalidUtf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

	pub fn byte_order(&self) -> ByteOrder {
		self.byte_order
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

	pub fn write_f32(&mut self, data: f32) {
		self.write_bytes(&match self.byte_order {
			ByteOrder::Big => data.to_be_bytes(),
			ByteOrder::Little => data.to_le_bytes(),
		});
	}

	pub fn write_f64(&mut self, data: f64) {
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

#[derive(Clone)]
pub struct BinaryReader<'a> {
	byte_order: ByteOrder,
	cursor: usize,
	inner: &'a [u8],
}

impl<'a> BinaryReader<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self {
			byte_order: ByteOrder::Little,
			cursor: 0,
			inner: data,
		}
	}

	pub fn with_byte_order(self, byte_order: ByteOrder) -> Self {
		Self { byte_order, ..self }
	}

	pub fn set_byte_order(&mut self, byte_order: ByteOrder) {
		self.byte_order = byte_order;
	}

	pub fn byte_order(&self) -> ByteOrder {
		self.byte_order
	}

	pub fn position(&self) -> usize {
		self.cursor
	}

	pub fn len(&self) -> usize {
		self.inner.len()
	}

	pub fn seek(&mut self, offset: usize) -> Result<(), Error> {
		if offset > self.inner.len() {
			Err(Error::OutOfBoundsSeek)
		} else {
			self.cursor = offset;
			Ok(())
		}
	}

	#[doc(alias = "skip")]
	pub fn seek_rel(&mut self, offset: isize) -> Result<(), Error> {
		self.seek(
			self
				.cursor
				.checked_add_signed(offset)
				.ok_or(Error::OutOfBoundsSeek)?,
		)
	}

	pub fn align_up(&mut self, alignment: usize) -> Result<(), Error> {
		self.seek(align_up(self.position(), alignment))
	}

	pub fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
		let start = self.cursor;
		let end = start + N;
		if end > self.inner.len() {
			return Err(Error::OutOfBoundsRead);
		}
		self.cursor += N;
		Ok(self.inner[start..end].try_into().unwrap())
	}

	pub fn read_slice(&mut self, count: usize) -> Result<&'a [u8], Error> {
		let start = self.cursor;
		let end = start + count;
		if end > self.inner.len() {
			return Err(Error::OutOfBoundsRead);
		}
		self.cursor += count;
		Ok(&self.inner[start..end])
	}

	pub fn read_slice_at(&mut self, offset: usize, count: usize) -> Result<&'a [u8], Error> {
		self.scope(offset,|reader| reader.read_slice(count))
	}

	pub fn read_until(&mut self, terminator: u8) -> &'a [u8] {
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

	pub fn read_null_terminated_string(&mut self) -> Result<&'a str, Error> {
		str::from_utf8(self.read_until(b'\0')).map_err(|_| Error::InvalidUtf8)
	}

	pub fn read_string(&mut self, count: usize) -> Result<&'a str, Error> {
		str::from_utf8(self.read_slice(count)?).map_err(|_| Error::InvalidUtf8)
	}

	pub fn read_owned_string(&mut self, count: usize) -> Result<String, Error> {
		str::from_utf8(self.read_slice(count)?)
			.map_err(|_| Error::InvalidUtf8)
			.map(str::to_owned)
	}

	pub fn read_array<const N: usize, R, E>(
		&mut self,
		mut func: impl FnMut(&mut Self, usize) -> Result<R, E>,
	) -> Result<[R; N], E> {
		let mut array = [const { MaybeUninit::uninit() }; N];

		for (index, el) in array.iter_mut().enumerate() {
			*el = MaybeUninit::new(func(self, index)?);
		}

		// safety: this point can only be reached once every element in the array contains
		// an initialized value.
		Ok(unsafe { array.map(|el| el.assume_init()) })
	}

	pub fn read_boxed_array<R, E>(
		&mut self,
		count: usize,
		mut func: impl FnMut(&mut Self, usize) -> Result<R, E>,
	) -> Result<Box<[R]>, E> {
		let mut array = Box::new_uninit_slice(count);

		for (index, el) in array.iter_mut().enumerate() {
			*el = MaybeUninit::new(func(self, index)?);
		}

		// safety: this point can only be reached once every element in the array contains
		// an initialized value.
		Ok(unsafe { array.assume_init() })
	}

	pub fn read_vec<R, E>(
		&mut self,
		count: usize,
		func: impl FnMut(&mut Self, usize) -> Result<R, E>,
	) -> Result<Vec<R>, E> {
		self.read_boxed_array(count, func).map(<[R]>::into_vec)
	}

	pub fn scope<R: 'a, E: From<Error>>(
		&mut self,
		seek_to: usize,
		func: impl FnOnce(&mut Self) -> Result<R, E>,
	) -> Result<R, E> {
		let current = self.position();
		self.seek(seek_to)?;
		let result = func(self);
		self.seek(current)?;
		result
	}

	pub fn subfile_scope<R: 'a, E: From<Error>>(
		&mut self,
		seek_to: usize,
		func: impl FnOnce(&mut Self) -> Result<R, E>,
	) -> Result<R, E> {
		let current = self.position();
		self.seek(seek_to)?;
		let mut sub_reader =
			BinaryReader::new(&self.inner[self.cursor..]).with_byte_order(self.byte_order);
		let result = func(&mut sub_reader);
		self.seek(current)?;
		result
	}

	pub fn read_u8(&mut self) -> Result<u8, Error> {
		let data = self.read_slice(1)?;
		Ok(data[0])
	}

	#[doc(alias = "read_s8")]
	pub fn read_i8(&mut self) -> Result<i8, Error> {
		Ok(self.read_u8()? as i8)
	}

	pub fn read_u16(&mut self) -> Result<u16, Error> {
		let byte_order = self.byte_order;
		let data = self.read_slice(2)?.try_into().unwrap();
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
		let data = self.read_slice(4)?.try_into().unwrap();
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
		let data = self.read_slice(8)?.try_into().unwrap();
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
		let data = self.read_slice(4)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => f32::from_be_bytes(data),
			ByteOrder::Little => f32::from_le_bytes(data),
		})
	}

	pub fn read_f64(&mut self) -> Result<f64, Error> {
		let byte_order = self.byte_order;
		let data = self.read_slice(4)?.try_into().unwrap();
		Ok(match byte_order {
			ByteOrder::Big => f64::from_be_bytes(data),
			ByteOrder::Little => f64::from_le_bytes(data),
		})
	}

	pub fn read_bool(&mut self) -> Result<bool, Error> {
		self.read_u8().map(|value| value != 0)
	}

	pub fn read_uvec2(&mut self) -> Result<UVec2, Error> {
		Ok(UVec2::new(self.read_u32()?, self.read_u32()?))
	}

	pub fn read_ivec2(&mut self) -> Result<IVec2, Error> {
		Ok(IVec2::new(self.read_i32()?, self.read_i32()?))
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

	pub fn read_vec4(&mut self) -> Result<Vec4, Error> {
		Ok(Vec4::new(
			self.read_f32()?,
			self.read_f32()?,
			self.read_f32()?,
			self.read_f32()?,
		))
	}

	pub fn read_ivec4(&mut self) -> Result<IVec4, Error> {
		Ok(IVec4::new(
			self.read_i32()?,
			self.read_i32()?,
			self.read_i32()?,
			self.read_i32()?,
		))
	}

	pub fn read_uvec4(&mut self) -> Result<UVec4, Error> {
		Ok(UVec4::new(
			self.read_u32()?,
			self.read_u32()?,
			self.read_u32()?,
			self.read_u32()?,
		))
	}
}

pub fn align_up<T: PrimInt>(value: T, alignment: T) -> T {
	(value + alignment - T::one()) & !(alignment - T::one())
}

pub fn try_align_up<T: PrimInt>(value: T, alignment: T) -> Option<T> {
	Some(
		value
			.checked_add(&alignment)
			.and_then(|value| value.checked_sub(&T::one()))?
			& !alignment.checked_sub(&T::one())?,
	)
}
