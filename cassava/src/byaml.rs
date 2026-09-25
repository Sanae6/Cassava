use std::{backtrace::Backtrace, collections::HashSet, ptr, sync::Arc};

use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use snafu::{OptionExt, Snafu, ensure};

use crate::util::{self, BinaryReader, BinaryWriter, ByteOrder, align_up, try_align_up};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
	InvalidEndianness,
	UnsupportedVersion,
	InvalidStringTableType,
	InvalidContainerType,
	InvalidDataType,
	MissingString {
		backtrace: Backtrace,
	},
	MissingKey {
		backtrace: Backtrace,
	},
}

#[derive(Debug)]
pub struct Byml {
	pub endianness: ByteOrder,
	pub version: u16,
	pub root_node: Option<Container>,
}

pub type Dictionary = IndexMap<SmolStr, Element>;
pub type Array = Vec<Element>;

#[derive(Debug, Clone, Eq)]
pub enum Container {
	Array(Arc<AtomicRefCell<Array>>),
	Dictionary(Arc<AtomicRefCell<Dictionary>>),
}

impl PartialEq for Container {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Array(l0), Self::Array(r0)) => ptr::eq(l0, r0) || *l0.borrow() == *r0.borrow(),
			(Self::Dictionary(l0), Self::Dictionary(r0)) => {
				ptr::eq(l0, r0) || *l0.borrow() == *r0.borrow()
			}
			_ => false,
		}
	}
}

impl Container {
	fn byte_size(&self) -> Option<usize> {
		let size = match self {
			Container::Array(array) => {
				let array = array.borrow();
				4 + align_up(array.len(), 4) + align_up(array.len() * 4, 4)
			}
			Container::Dictionary(dict) => 4 + dict.borrow().len() * 8,
		};

		(size <= 0xFFFFFF).then_some(size)
	}

	pub fn as_element(&self) -> Element {
		match self.clone() {
			Container::Array(value) => Element::Array(value),
			Container::Dictionary(value) => Element::Dictionary(value),
		}
	}

	pub fn as_dictionary(&self) -> Option<Arc<AtomicRefCell<Dictionary>>> {
		match self.clone() {
			Container::Dictionary(value) => Some(value),
			_ => None,
		}
	}

	pub fn as_array(&self) -> Option<Arc<AtomicRefCell<Array>>> {
		match self.clone() {
			Container::Array(value) => Some(value),
			_ => None,
		}
	}
}

impl std::hash::Hash for Container {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Container::Array(array) => array.borrow().hash(state),
			Container::Dictionary(ref_cell) => ref_cell.borrow().as_slice().hash(state),
		}
	}
}

#[derive(Debug, Clone)]
pub enum Element {
	Array(Arc<AtomicRefCell<Array>>),
	Dictionary(Arc<AtomicRefCell<Dictionary>>),
	String(SmolStr),
	Boolean(bool),
	I32(i32),
	F32(f32),
	U32(u32),
	I64(i64),
	U64(u64),
	F64(f64),
	Null,
}

impl Element {
	pub fn data_type(&self) -> DataType {
		match self {
			Element::Array(_) => DataType::Array,
			Element::Dictionary(_) => DataType::Dictionary,
			Element::String(_) => DataType::String,
			Element::Boolean(_) => DataType::Bool,
			Element::I32(_) => DataType::I32,
			Element::F32(_) => DataType::F32,
			Element::U32(_) => DataType::U32,
			Element::I64(_) => DataType::I64,
			Element::U64(_) => DataType::U64,
			Element::F64(_) => DataType::F64,
			Element::Null => DataType::Null,
		}
	}

	pub fn is_null(&self) -> bool {
		matches!(self, Element::Null)
	}
}

macro_rules! accessors {
	($($name: ident: $ty: ty),+ $(,)?) => {
		paste::paste!{
			impl Element {
				$(
					pub fn [<as_ $name:snake>](&self) -> Option<$ty> {
						match self {
							Element::$name(value) => Some(value.clone()),
							_ => None
						}
					}

					pub fn [<unwrap_ $name:snake>](&self) -> $ty {
						match self {
							Element::$name(value) => value.clone(),
							_ => panic!("expected {}, got {:?}", stringify!($ty), self.data_type())
						}
					}

					pub fn [<get_index_ $name:snake>](&self, index: usize) -> Option<$ty> {
						self.as_array()?.borrow().get(index)?.[<as_ $name:snake>]()
					}

					pub fn [<get_key_ $name:snake>](&self, key: &str) -> Option<$ty> {
						self.as_dictionary()?.borrow().get(key)?.[<as_ $name:snake>]()
					}
				)+
			}
		}
	};
}

accessors!(
	Array: Arc<AtomicRefCell<Array>>,
	Dictionary: Arc<AtomicRefCell<Dictionary>>,
	String: SmolStr,
	Boolean: bool,
	I32: i32,
	U32: u32,
	F32: f32,
	I64: i64,
	U64: u64,
	F64: f64,
);

impl Eq for Element {}

impl PartialEq for Element {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Array(l0), Self::Array(r0)) => ptr::eq(l0, r0) || *l0.borrow() == *r0.borrow(),
			(Self::Dictionary(l0), Self::Dictionary(r0)) => {
				ptr::eq(l0, r0) || *l0.borrow() == *r0.borrow()
			}
			(Self::String(l0), Self::String(r0)) => l0 == r0,
			(Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
			(Self::I32(l0), Self::I32(r0)) => l0 == r0,
			(Self::F32(l0), Self::F32(r0)) => OrderedFloat(*l0) == OrderedFloat(*r0),
			(Self::U32(l0), Self::U32(r0)) => l0 == r0,
			(Self::I64(l0), Self::I64(r0)) => l0 == r0,
			(Self::U64(l0), Self::U64(r0)) => l0 == r0,
			(Self::F64(l0), Self::F64(r0)) => OrderedFloat(*l0) == OrderedFloat(*r0),
			(Self::Null, Self::Null) => true,
			_ => false,
		}
	}
}

impl std::hash::Hash for Element {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Element::Array(array) => array.borrow().hash(state),
			Element::Dictionary(ref_cell) => ref_cell.borrow().as_slice().hash(state),
			Element::String(value) => value.hash(state),
			Element::Boolean(value) => value.hash(state),
			Element::I32(value) => value.hash(state),
			Element::F32(value) => OrderedFloat(*value).hash(state),
			Element::U32(value) => value.hash(state),
			Element::I64(value) => value.hash(state),
			Element::U64(value) => value.hash(state),
			Element::F64(value) => OrderedFloat(*value).hash(state),
			Element::Null => todo!(),
		}
	}
}

impl Byml {
	pub fn read(reader: &mut BinaryReader) -> Result<Byml, Error> {
		match &reader.read_bytes::<2>()? {
			b"YB" => reader.set_byte_order(ByteOrder::Little),
			b"BY" => reader.set_byte_order(ByteOrder::Big),
			_ => return Err(Error::InvalidEndianness),
		};

		let version = reader.read_u16()?;
		if version > 3 {
			return Err(Error::UnsupportedVersion);
		}

		fn read_string_table(reader: &mut BinaryReader) -> Result<Box<[SmolStr]>, Error> {
			let at = reader.read_u32()?;
			if at == 0 {
				// panic!();
				return Ok(Box::new([]));
			}

			reader.scope(at as usize, |reader| {
				let table_start = reader.position();
				let data_type =
					DataType::from_u8(reader.read_u8()?).context(InvalidStringTableTypeSnafu)?;
				ensure!(
					data_type == DataType::StringTable,
					InvalidStringTableTypeSnafu
				);
				let count = read_u24(reader)? as usize;
				let offsets = reader.read_boxed_array(count + 1, |reader, _| reader.read_u32())?;
				let strings = reader.read_boxed_array(count + 1, |reader, i| {
					if i == count {
						return Ok(SmolStr::new(""));
					}
					let (start, end) = (offsets[i] as usize, offsets[i + 1] as usize);
					reader.seek(table_start + start)?;
					reader
						.read_string(end - start - 1)
						.map(|str| Into::<SmolStr>::into(str))
				})?;
				Ok(strings)
			})
		}
		let hash_key_table = read_string_table(reader)?;
		let string_table = read_string_table(reader)?;
		let root_node = reader.read_u32()? as usize;
		if root_node == 0 {
			return Ok(Byml {
				endianness: reader.byte_order(),
				version,
				root_node: None,
			});
		}

		let root_node = reader.scope(root_node, |reader| {
			Self::read_container(
				reader,
				&mut ReadTables {
					keys: hash_key_table,
					strings: string_table,
					found_containers: Default::default(),
				},
			)
		})?;

		return Ok(Byml {
			endianness: reader.byte_order(),
			version,
			root_node: Some(root_node),
		});
	}

	fn read_container(
		reader: &mut BinaryReader,
		tables: &mut ReadTables,
	) -> Result<Container, Error> {
		match DataType::from_u8(reader.read_u8()?).context(InvalidContainerTypeSnafu)? {
			DataType::Array => Self::read_array(reader, tables).map(Container::Array),
			DataType::Dictionary => Self::read_dictionary(reader, tables).map(Container::Dictionary),
			_ => return Err(Error::InvalidContainerType),
		}
	}
	// should be after data type byte
	fn read_array(
		reader: &mut BinaryReader,
		tables: &mut ReadTables,
	) -> Result<Arc<AtomicRefCell<Array>>, Error> {
		let start = reader.position();
		if let Some(Container::Array(array)) = tables.found_containers.get(&start) {
			return Ok(array.clone());
		}
		let count = read_u24(reader)? as usize;
		let data_type = reader.read_boxed_array(count, |reader, _| reader.read_u8())?;
		reader.align_up(4)?;
		let mut elements = Array::with_capacity(count);
		for data_type in data_type {
			let data_type = DataType::from_u8(data_type).context(InvalidDataTypeSnafu)?;
			let value = reader.read_u32()?;
			elements.push(Self::read_element(reader, tables, data_type, value)?);
		}
		let elements = Arc::new(AtomicRefCell::new(elements));
		tables
			.found_containers
			.insert(start, Container::Array(elements.clone()));
		Ok(elements)
	}

	fn read_dictionary(
		reader: &mut BinaryReader,
		tables: &mut ReadTables,
	) -> Result<Arc<AtomicRefCell<Dictionary>>, Error> {
		let start = reader.position();
		if let Some(Container::Dictionary(dict)) = tables.found_containers.get(&start) {
			return Ok(dict.clone());
		}
		let count = read_u24(reader)? as usize;
		reader.align_up(4)?;
		let mut dictionary = Dictionary::new();
		for _ in 0..count {
			let name = read_u24(reader)?;
			let name = tables
				.keys
				.get(name as usize)
				.cloned()
				.context(MissingKeySnafu)?;
			let data_type = DataType::from_u8(reader.read_u8()?).context(InvalidDataTypeSnafu)?;
			let value = reader.read_u32()?;
			dictionary.insert(name, Self::read_element(reader, tables, data_type, value)?);
		}
		let dictionary = Arc::new(AtomicRefCell::new(dictionary));
		tables
			.found_containers
			.insert(start, Container::Dictionary(dictionary.clone()));
		Ok(dictionary)
	}

	fn read_element(
		reader: &mut BinaryReader,
		tables: &mut ReadTables,
		data_type: DataType,
		value: u32,
	) -> Result<Element, Error> {
		Ok(match data_type {
			DataType::String => Element::String(
				tables
					.strings
					.get(value as usize)
					.cloned()
					.context(MissingStringSnafu)?,
			),
			DataType::Array => reader.scope::<_, Error>(value as usize, |reader| {
				reader.seek_rel(1)?;
				Ok(Element::Array(Self::read_array(reader, tables)?))
			})?,
			DataType::Dictionary => reader.scope::<_, Error>(value as usize, |reader| {
				reader.seek_rel(1)?;
				Ok(Element::Dictionary(Self::read_dictionary(reader, tables)?))
			})?,
			DataType::Bool => Element::Boolean(value != 0),
			DataType::I32 => Element::I32(value.cast_signed()),
			DataType::F32 => Element::F32(f32::from_ne_bytes(value.to_ne_bytes())),
			DataType::U32 => Element::U32(value),
			DataType::I64 | DataType::U64 | DataType::F64 => {
				reader.scope::<_, util::Error>(value as usize, |reader| {
					Ok(match data_type {
						DataType::I64 => Element::I64(reader.read_i64()?),
						DataType::U64 => Element::U64(reader.read_u64()?),
						DataType::F64 => Element::F64(reader.read_f64()?),
						_ => unreachable!(),
					})
				})?
			}
			DataType::Null => Element::Null,
			_ => return Err(Error::InvalidDataType),
		})
	}

	// utilities
	pub fn as_element(&self) -> Element {
		match self.root_node.as_ref() {
			Some(container) => container.as_element(),
			None => Element::Null,
		}
	}

	pub fn as_dictionary(&self) -> Option<Arc<AtomicRefCell<Dictionary>>> {
		self.root_node.as_ref().and_then(Container::as_dictionary)
	}

	pub fn as_array(&self) -> Option<Arc<AtomicRefCell<Array>>> {
		self.root_node.as_ref().and_then(Container::as_array)
	}
}

#[derive(Snafu, Debug)]
pub enum WriteError {
	#[snafu(display("byml is too large to save"))]
	TooLarge,
}
impl Byml {
	pub fn write(&self) -> Result<Vec<u8>, WriteError> {
		let mut writer = BinaryWriter::new(self.endianness);
		match self.endianness {
			ByteOrder::Little => writer.write_bytes(b"YB"),
			ByteOrder::Big => writer.write_bytes(b"BY"),
		};
		writer.write_u16(self.version);
		let Some(root_node) = &self.root_node else {
			writer.write_bytes(&[0; 12]);
			return Ok(writer.take_inner());
		};
		let mut tables = FirstTraversal::default();
		Self::find_containers(root_node.clone(), &mut tables);

		let string_table_size = |strings: &[SmolStr]| -> Result<usize, WriteError> {
			if strings.len() == 0 {
				return Ok(0);
			}
			let mut size = strings
				.len()
				.checked_mul(4)
				.and_then(|size| size.checked_add(8)) // header + last string offset
				.context(TooLargeSnafu)?;
			for string in strings {
				size = size
					.checked_add(string.as_bytes().len())
					.and_then(|size| size.checked_add(1))
					.context(TooLargeSnafu)?;
			}
			try_align_up(size, 4).context(TooLargeSnafu)
		};
		let mut hash_keys: Vec<_> = tables.keys.into_iter().collect();
		let mut strings: Vec<_> = tables.strings.into_iter().collect();
		hash_keys.sort();
		strings.sort();

		let hash_table_size = string_table_size(&hash_keys)?;
		let string_table_size = string_table_size(&strings)?;
		let hash_table_offset = if hash_table_size > 0 { 16 } else { 0 };
		let string_table_offset = if string_table_size > 0 {
			16 + hash_table_size as u32
		} else {
			0
		};
		writer.write_u32(hash_table_offset);
		writer.write_u32(string_table_offset);

		let mut container_offsets = IndexMap::new();
		let mut offset = 16 + string_table_size + hash_table_size;
		for container in tables.containers {
			let size = container.byte_size().context(TooLargeSnafu)?;
			container_offsets.insert(container, offset);
			offset += size;
		}
		writer.write_u32(*container_offsets.get(root_node).unwrap() as u32);

		let mut write_sorted_string_table = |offset: usize, strings: &[SmolStr]| {
			writer.seek(offset);
			writer.write_u8(DataType::StringTable as u8);
			write_u24(&mut writer, strings.len() as u32);
			let strings_start = 4 + strings.len() * 4 + 4;
			let mut offset = 0;
			for string in strings {
				writer.write_u32((strings_start + offset).try_into().unwrap());
				offset += string.len() + 1;
			}
			writer.write_u32((strings_start + offset).try_into().unwrap());
			for string in strings {
				writer.write_bytes(string.as_bytes());
				writer.write_u8(0);
			}

			writer.align_up(4).unwrap();
		};

		write_sorted_string_table(hash_table_offset as _, &hash_keys);
		write_sorted_string_table(string_table_offset as _, &strings);

		writer.seek(offset);
		let mut long_type_offsets = IndexMap::new();
		for long_type in tables.long_types {
			long_type_offsets.insert(long_type, writer.position());
			match long_type {
				LongElement::I64(value) => writer.write_i64(value),
				LongElement::U64(value) => writer.write_u64(value),
				LongElement::F64(value) => writer.write_f64(*value),
			}
		}

		for (container, offset) in &container_offsets {
			writer.seek(*offset);
			match container {
				Container::Array(array) => {
					let array = array.borrow();
					writer.write_u8(DataType::Array as u8);
					write_u24(&mut writer, array.len() as _);
					for el in array.iter() {
						writer.write_u8(el.data_type() as u8);
					}
					writer.align_up(4).map_err(|_| WriteError::TooLarge)?;
					for el in array.iter() {
						Self::write_element(
							&mut writer,
							el,
							&strings,
							&container_offsets,
							&long_type_offsets,
						);
					}
				}
				Container::Dictionary(dict) => {
					let dict = dict.borrow();
					writer.write_u8(DataType::Dictionary as u8);
					write_u24(&mut writer, dict.len() as _);
					for (key, value) in dict.iter() {
						let (index, _) = hash_keys
							.iter()
							.enumerate()
							.find(|other| **other.1 == **key)
							.expect("failed to match key in hash keys");
						write_u24(&mut writer, index as u32);
						writer.write_u8(value.data_type() as u8);
						Self::write_element(
							&mut writer,
							value,
							&strings,
							&container_offsets,
							&long_type_offsets,
						);
					}
				}
			}
		}

		Ok(writer.take_inner())
	}

	fn write_element(
		writer: &mut BinaryWriter,
		value: &Element,
		strings: &[SmolStr],
		container_offsets: &IndexMap<Container, usize>,
		long_type_offsets: &IndexMap<LongElement, usize>,
	) {
		match value {
			Element::Array(array) => {
				let array = container_offsets
					.get(&Container::Array(array.clone()))
					.unwrap();
				writer.write_u32(*array as u32);
			}
			Element::Dictionary(dict) => {
				let array = container_offsets
					.get(&Container::Dictionary(dict.clone()))
					.unwrap();
				writer.write_u32(*array as u32);
			}
			Element::String(value) => {
				let (index, _) = strings
					.iter()
					.enumerate()
					.find(|other| **other.1 == **value)
					.expect("failed to match key in strings");
				writer.write_u32(index as u32)
			}
			Element::Boolean(value) => writer.write_u32(*value as u32),
			Element::I32(value) => writer.write_i32(*value),
			Element::F32(value) => writer.write_f32(*value),
			Element::U32(value) => writer.write_u32(*value),
			Element::I64(value) => {
				writer.write_u32(*long_type_offsets.get(&LongElement::I64(*value)).unwrap() as u32)
			}
			Element::U64(value) => {
				writer.write_u32(*long_type_offsets.get(&LongElement::U64(*value)).unwrap() as u32)
			}
			Element::F64(value) => writer.write_u32(
				*long_type_offsets
					.get(&LongElement::F64(OrderedFloat(*value)))
					.unwrap() as u32,
			),
			Element::Null => writer.write_u32(0),
		}
	}

	fn find_containers<'a>(container: Container, context: &mut FirstTraversal) {
		if !context.containers.insert(container.clone()) {
			return;
		}

		let on_el = |el: &Element, context: &mut FirstTraversal| match el {
			Element::Array(array) => Self::find_containers(Container::Array(array.clone()), context),
			Element::Dictionary(dict) => {
				Self::find_containers(Container::Dictionary(dict.clone()), context)
			}
			Element::String(value) => {
				context.strings.insert(value.clone());
			}
			Element::I64(value) => {
				context.long_types.insert(LongElement::I64(*value));
			}
			Element::U64(value) => {
				context.long_types.insert(LongElement::U64(*value));
			}
			Element::F64(value) => {
				context
					.long_types
					.insert(LongElement::F64(OrderedFloat(*value)));
			}
			_ => {}
		};

		match container {
			Container::Array(array) => {
				for el in array.borrow().iter() {
					on_el(el, context);
				}
			}
			Container::Dictionary(dict) => {
				for (key, el) in dict.borrow().iter() {
					context.keys.insert(key.clone());
					on_el(el, context)
				}
			}
		}
	}
}

struct ReadTables {
	keys: Box<[SmolStr]>,
	strings: Box<[SmolStr]>,
	found_containers: IndexMap<usize, Container>,
}

#[derive(Default)]
struct FirstTraversal {
	keys: HashSet<SmolStr>,
	strings: HashSet<SmolStr>,
	long_types: HashSet<LongElement>,
	containers: HashSet<Container>,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum LongElement {
	I64(i64),
	U64(u64),
	F64(OrderedFloat<f64>),
}

fn read_u24(reader: &mut BinaryReader) -> Result<u32, util::Error> {
	let value = reader.read_bytes::<3>()?;
	match reader.byte_order() {
		ByteOrder::Big => {
			let [a, b, c] = value;
			Ok(u32::from_be_bytes([0, a, b, c]))
		}
		ByteOrder::Little => {
			let [a, b, c] = value;
			Ok(u32::from_le_bytes([a, b, c, 0]))
		}
	}
}

fn write_u24(writer: &mut BinaryWriter, value: u32) {
	let bytes = match writer.byte_order() {
		ByteOrder::Big => {
			let [_, a, b, c] = value.to_le_bytes();
			[a, b, c]
		}
		ByteOrder::Little => {
			let [a, b, c, _] = value.to_le_bytes();
			[a, b, c]
		}
	};

	writer.write_bytes(&bytes)
}

#[derive(Debug, FromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum DataType {
	String = 0xA0,
	Array = 0xC0,
	Dictionary = 0xC1,
	StringTable = 0xC2,
	Bool = 0xD0,
	I32 = 0xD1,
	F32 = 0xD2,
	U32 = 0xD3,
	I64 = 0xD4,
	U64 = 0xD5,
	F64 = 0xD6,
	Null = 0xFF,
}
