use std::fmt::Display;

use serde::{
	Deserializer,
	de::{EnumAccess, MapAccess, SeqAccess, VariantAccess, value::StringDeserializer},
};
use webkit6::{glib::GString, javascriptcore::Value};

use super::{Error, get_array_buffer_data};

impl serde::de::Error for Error {
	fn custom<T>(msg: T) -> Self
	where
		T: Display,
	{
		Error::Custom {
			message: msg.to_string(),
		}
	}
}

pub fn deserialize<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Error> {
	T::deserialize(ValueDeserializer { value })
}
struct ValueDeserializer {
	value: Value,
}

impl ValueDeserializer {
	pub fn visit_buffer<R>(self, visit: impl FnOnce(&[u8]) -> Result<R, Error>) -> Result<R, Error> {
		if self.value.is_typed_array() {
			let buffer = self
				.value
				.typed_array_get_buffer()
				.ok_or(Error::NoBufferInTypedArray)?;
			let offset = self.value.typed_array_get_offset();
			let size = self.value.typed_array_get_size();
			let data = get_array_buffer_data(&buffer);
			return visit(&data[offset..offset + size]);
		}
		if self.value.is_array_buffer() {
			let data = get_array_buffer_data(&self.value);
			return visit(&data);
		}
		Err(Error::ExpectedBytes)
	}
}

impl<'de> Deserializer<'de> for ValueDeserializer {
	type Error = Error;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		if self.value.is_boolean() {
			return visitor.visit_bool(self.value.to_boolean());
		}
		if self.value.is_string() {
			return visitor.visit_string(self.value.to_string());
		}
		if self.value.is_number() {
			return visitor.visit_f64(self.value.to_double());
		}
		if self.value.is_null() || self.value.is_undefined() {
			return visitor.visit_unit();
		}
		if self.value.is_array() {
			return visitor.visit_seq(Seq {
				value: self.value,
				index: 0,
			});
		}
		if self.value.is_object() {
			if self.value.object_has_property("___u") {
				return self.deserialize_u64(visitor);
			} else if self.value.object_has_property("___s") {
				return self.deserialize_i64(visitor);
			}

			return visitor.visit_map(Map {
				properties: self.value.object_enumerate_properties(),
				value: self.value,
				index: 0,
			});
		}
		todo!()
	}

	fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_i8(self.value.to_double() as _)
	}

	fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_i16(self.value.to_double() as _)
	}

	fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_i32(self.value.to_double() as _)
	}

	fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_u8(self.value.to_double() as _)
	}

	fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_u16(self.value.to_double() as _)
	}

	fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_u32(self.value.to_double() as _)
	}

	fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_u32(self.value.to_double() as _)
	}

	fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let low = self
			.value
			.object_get_property("l")
			.ok_or(Error::MissingLow)?;
		let high = self
			.value
			.object_get_property("h")
			.ok_or(Error::MissingHigh)?;
		visitor.visit_u64(
			low.to_int32().cast_unsigned() as u64 | (high.to_int32().cast_unsigned() as u64) << 32,
		)
	}
	fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let low = self
			.value
			.object_get_property("l")
			.ok_or(Error::MissingLow)?;
		let high = self
			.value
			.object_get_property("h")
			.ok_or(Error::MissingHigh)?;
		visitor.visit_i64(
			(low.to_int32().cast_unsigned() as u64 | (high.to_int32().cast_unsigned() as u64) << 32)
				.cast_signed(),
		)
	}

	fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_bool(self.value.to_boolean())
	}

	fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_f64(self.value.to_double())
	}

	fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let str = self.value.to_str();
		let mut chars = str.chars();
		let char = chars.next().ok_or(Error::StringIsNotOneCharacter)?;
		if chars.next().is_some() {
			return Err(Error::StringIsNotOneCharacter);
		}
		visitor.visit_char(char)
	}

	fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let str = self.value.to_str();
		visitor.visit_str(&str)
	}

	fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let str = self.value.to_str().as_str().to_owned();
		visitor.visit_string(str)
	}

	fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		self.visit_buffer(|data| visitor.visit_bytes(data))
	}

	fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		self.visit_buffer(|data| visitor.visit_byte_buf(data.to_vec()))
	}

	fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		if self.value.is_null() || self.value.is_undefined() {
			visitor.visit_none()
		} else {
			visitor.visit_some(ValueDeserializer { value: self.value })
		}
	}

	fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_unit()
	}

	fn deserialize_unit_struct<V>(
		self,
		name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let str = self.value.to_str();
		if name != str {
			return Err(Error::NameMismatch);
		}
		visitor.visit_unit()
	}

	fn deserialize_newtype_struct<V>(
		self,
		_name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_newtype_struct(self)
	}

	fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_seq(Seq {
			value: self.value,
			index: 0,
		})
	}

	fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_seq(Seq {
			value: self.value,
			index: 0,
		})
	}

	fn deserialize_tuple_struct<V>(
		self,
		_name: &'static str,
		_len: usize,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		visitor.visit_seq(Seq {
			value: self.value,
			index: 0,
		})
	}

	fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		let properties = self.value.object_enumerate_properties();
		visitor.visit_map(Map {
			value: self.value,
			properties,
			index: 0,
		})
	}

	fn deserialize_struct<V>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		self.deserialize_map(visitor)
	}

	fn deserialize_enum<V>(
		self,
		_name: &'static str,
		_variants: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		if self.value.is_string() {
			return visitor.visit_enum(StringDeserializer::new(
				self.value.to_str().as_str().to_owned(),
			));
		}
		let properties = self.value.object_enumerate_properties();
		if properties.len() != 1 {
			return Err(Error::TooManyVariantKeys);
		}
		let variant = self.value.object_get_property(&properties[0]).unwrap();
		visitor.visit_enum(Enum { value: variant })
	}

	fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		todo!()
	}

	fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		todo!()
	}
}

struct Seq {
	value: Value,
	index: u32,
}

impl<'de> SeqAccess<'de> for Seq {
	type Error = Error;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: serde::de::DeserializeSeed<'de>,
	{
		seed
			.deserialize(ValueDeserializer {
				value: self
					.value
					.object_get_property_at_index(self.index)
					.expect("burger"),
			})
			.map(Some)
			.inspect(|_| {
				self.index += 1;
			})
	}
}

struct Map {
	value: Value,
	properties: Vec<GString>,
	index: usize,
}
impl<'de> MapAccess<'de> for Map {
	type Error = Error;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: serde::de::DeserializeSeed<'de>,
	{
		let Some(property) = self.properties.get(self.index) else {
			return Ok(None);
		};
		let de = StringDeserializer::new(property.as_str().to_owned());
		seed.deserialize(de).map(Some)
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::DeserializeSeed<'de>,
	{
		let value: Value = self
			.value
			.object_get_property(self.properties[self.index].as_str())
			.expect("value must exist");
		self.index += 1;
		seed.deserialize(ValueDeserializer { value })
	}
}

struct Enum {
	value: Value,
}
impl<'de> EnumAccess<'de> for Enum {
	type Error = Error;
	type Variant = Enum;

	fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
	where
		V: serde::de::DeserializeSeed<'de>,
	{
		seed
			.deserialize(ValueDeserializer {
				value: self.value.clone(),
			})
			.map(|value| (value, Enum { value: self.value }))
	}
}

impl<'de> VariantAccess<'de> for Enum {
	type Error = Error;

	fn unit_variant(self) -> Result<(), Self::Error> {
		unreachable!("handled in deserialize_enum")
	}

	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: serde::de::DeserializeSeed<'de>,
	{
		seed.deserialize(ValueDeserializer { value: self.value })
	}

	fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		ValueDeserializer { value: self.value }.deserialize_tuple(len, visitor)
	}

	fn struct_variant<V>(
		self,
		fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: serde::de::Visitor<'de>,
	{
		ValueDeserializer { value: self.value }.deserialize_struct("unused string", fields, visitor)
	}
}
