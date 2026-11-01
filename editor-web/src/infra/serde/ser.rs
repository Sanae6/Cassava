use std::fmt::Display;

use serde::{
	Serializer,
	ser::{
		Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
		SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
	},
};
use webkit6::javascriptcore::{Context, TypedArrayType, Value};

use super::{Error, get_array_buffer_data, new_array_buffer, new_object};

impl serde::ser::Error for Error {
	fn custom<T>(msg: T) -> Self
	where
		T: Display,
	{
		Error::Custom {
			message: msg.to_string(),
		}
	}
}

pub fn serialize<T: serde::Serialize>(context: &Context, value: T) -> Result<Value, Error> {
	value.serialize(ValueSerializer { context })
}
#[derive(Clone, Copy)]
pub struct ValueSerializer<'a> {
	context: &'a Context,
}

impl<'a> Serializer for ValueSerializer<'a> {
	type Ok = Value;
	type Error = Error;

	type SerializeSeq = Sequence<'a>;
	type SerializeTuple = Sequence<'a>;
	type SerializeTupleStruct = Sequence<'a>;
	type SerializeTupleVariant = Sequence<'a>;
	type SerializeMap = Map<'a>;
	type SerializeStruct = Map<'a>;
	type SerializeStructVariant = MapVariant<'a>;

	fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_boolean(&self.context, v))
	}

	fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		let obj = new_object(&self.context);
		obj.object_set_property("___l", &Value::new_number(self.context, (v & 0xFFFFFFFF) as f64));
		obj.object_set_property("___s", &Value::new_number(self.context, (v >> 32) as f64));
		Ok(obj)
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		let obj = new_object(&self.context);
		obj.object_set_property("___l", &Value::new_number(self.context, (v & 0xFFFFFFFF) as f64));
		obj.object_set_property("___u", &Value::new_number(self.context, (v >> 32) as f64));
		Ok(obj)
	}

	fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v as _))
	}

	fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_number(&self.context, v))
	}

	fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
		let mut data = [0u8; 4];
		let data = v.encode_utf8(&mut data);
		Ok(Value::new_string(&self.context, Some(data)))
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_string(&self.context, Some(v)))
	}

	fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
		let bytes = new_array_buffer(&self.context, v.len());
		get_array_buffer_data(&bytes).copy_from_slice(v);
		Ok(
			bytes
				.new_typed_array_with_buffer(TypedArrayType::Uint8, 0, v.len() as _)
				.unwrap(),
		)
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_undefined(&self.context))
	}

	fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		value.serialize(self)
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_null(&self.context))
	}

	fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(name)
	}

	fn serialize_unit_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
	) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(variant)
	}

	fn serialize_newtype_struct<T>(
		self,
		_name: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		value.serialize(self)
	}

	fn serialize_newtype_variant<T>(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let object = new_object(self.context);
		object.object_set_property(variant, &value.serialize(self.clone())?);
		Ok(object)
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		Ok(Sequence {
			context: self.context,
			values: len.map(Vec::with_capacity).unwrap_or_default(),
		})
	}

	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		Ok(Sequence {
			context: self.context,
			values: Vec::with_capacity(len),
		})
	}

	fn serialize_tuple_struct(
		self,
		_name: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleStruct, Self::Error> {
		Ok(Sequence {
			context: self.context,
			values: Vec::with_capacity(len),
		})
	}

	fn serialize_tuple_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		_variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleVariant, Self::Error> {
		Ok(Sequence {
			context: self.context,
			values: Vec::with_capacity(len),
		})
	}

	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		Ok(Map {
			context: self.context,
			value: new_object(self.context),
			next_key: None,
		})
	}

	fn serialize_struct(
		self,
		_name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStruct, Self::Error> {
		Ok(Map {
			context: self.context,
			value: new_object(self.context),
			next_key: None,
		})
	}

	fn serialize_struct_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStructVariant, Self::Error> {
		let object = new_object(self.context);
		let struct_object = new_object(self.context);
		object.object_set_property(variant, &struct_object);
		Ok(MapVariant {
			value: object,
			map: Map {
				context: self.context,
				value: struct_object,
				next_key: None,
			},
		})
	}
}

pub struct Sequence<'a> {
	context: &'a Context,
	values: Vec<Value>,
}

impl<'a> SerializeSeq for Sequence<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let value = value.serialize(ValueSerializer {
			context: self.context,
		})?;
		self.values.push(value);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_array_from_garray(&self.context, &self.values))
	}
}

impl<'a> SerializeTuple for Sequence<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let value = value.serialize(ValueSerializer {
			context: self.context,
		})?;
		self.values.push(value);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_array_from_garray(&self.context, &self.values))
	}
}

impl<'a> SerializeTupleStruct for Sequence<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let value = value.serialize(ValueSerializer {
			context: self.context,
		})?;
		self.values.push(value);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_array_from_garray(&self.context, &self.values))
	}
}

impl<'a> SerializeTupleVariant for Sequence<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let value = value.serialize(ValueSerializer {
			context: self.context,
		})?;
		self.values.push(value);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Value::new_array_from_garray(&self.context, &self.values))
	}
}

pub struct Map<'a> {
	context: &'a Context,
	value: Value,
	next_key: Option<String>,
}

impl<'a> SerializeMap for Map<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let string = self.next_key.insert(String::new());
		key.serialize(MapKeySerializer {
			value: string,
		})
	}

	fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		let value = value.serialize(ValueSerializer {
			context: self.context,
		})?;
		self
			.value
			.object_set_property(&self.next_key.take().expect("next key not found"), &value);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(self.value)
	}
}
impl<'a> SerializeStruct for Map<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		self.serialize_entry(key, value)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(self.value)
	}
}

pub struct MapVariant<'a> {
	value: Value,
	map: Map<'a>,
}

impl<'a> SerializeStructVariant for MapVariant<'a> {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		self.map.serialize_entry(key, value)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(self.value)
	}
}
struct MapKeySerializer<'a> {
	value: &'a mut String,
}
impl<'a> Serializer for MapKeySerializer<'a> {
	type Ok = ();
	type Error = Error;
	type SerializeSeq = Impossible<(), Error>;
	type SerializeTuple = Impossible<(), Error>;
	type SerializeTupleStruct = Impossible<(), Error>;
	type SerializeTupleVariant = Impossible<(), Error>;
	type SerializeMap = Impossible<(), Error>;
	type SerializeStruct = Impossible<(), Error>;
	type SerializeStructVariant = Impossible<(), Error>;

	fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_string();
		Ok(())
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		*self.value = v.to_owned();
		Ok(())
	}

	fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		Err(Error::KeyNotAString)
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
		*self.value = _name.to_owned();
		Ok(())
	}

	fn serialize_unit_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
	) -> Result<Self::Ok, Self::Error> {
		*self.value = variant.to_owned();
		Ok(())
	}

	fn serialize_newtype_struct<T>(
		self,
		_name: &'static str,
		_value: &T,
	) -> Result<Self::Ok, Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		Err(Error::KeyNotAString)
	}

	fn serialize_newtype_variant<T>(
		self,
		_name: &'static str,
		_variant_index: u32,
		_variant: &'static str,
		_value: &T,
	) -> Result<Self::Ok, Self::Error>
	where
		T: ?Sized + serde::Serialize,
	{
		Err(Error::KeyNotAString)
	}

	fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_tuple_struct(
		self,
		_name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeTupleStruct, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_tuple_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		_variant: &'static str,
		_len: usize,
	) -> Result<Self::SerializeTupleVariant, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_struct(
		self,
		_name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStruct, Self::Error> {
		Err(Error::KeyNotAString)
	}

	fn serialize_struct_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		_variant: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStructVariant, Self::Error> {
		Err(Error::KeyNotAString)
	}
}
