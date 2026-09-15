use std::fs;
use std::ops::Range;
use std::path::Path;

use indexmap::IndexMap;
use snafu::prelude::*;

use crate::util::{BinaryReader, BinaryWriter, ByteOrder};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	Io {
		source: std::io::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("file signature must be `{expected}`"))]
	InvalidSignature { expected: &'static str },
	#[snafu(display("{name} header must be {expected:#x} bytes (got {got:#x})"))]
	IncorrectHeaderSize {
		name: &'static str,
		expected: u16,
		got: u16,
	},
	#[snafu(display("got invalid byte order mark (got {bytes:#04x})"))]
	InvalidByteOrderMark { bytes: u16 },
	#[snafu(display("unsupported SARC version (only 1.0 supported, got {}.{})", version >> 8, version & 0xff))]
	UnsupportedVersion { version: u16 },
	#[snafu(display("SARC header file size does not match input file size"))]
	UnmatchingFileSize,
	#[snafu(display("SARC offset is past end of file"))]
	OffsetPastEOF,
	#[snafu(display("failed to parse UTF-8"))]
	UTF8Error,
	#[snafu(display("file data end offset is less than start offset"))]
	DataOffsetsOutOfOrder,
	#[snafu(display("filename hash is incorrect"))]
	HashMismatch,
	#[snafu(display("file is out of bounds"))]
	FileOutOfBounds,
	#[snafu(transparent)]
	ReadError { source: crate::util::Error },
}

pub struct Reader {
	archive_data: Vec<u8>,
	files: IndexMap<String, Range<usize>>,
	byte_order: ByteOrder,
}

impl Reader {
	pub fn new(input: Vec<u8>) -> Result<Self, Error> {
		let mut reader = BinaryReader::new(&input).with_byte_order(ByteOrder::Little);

		let signature = reader.read_slice(4)?;
		if signature != b"SARC" {
			return Err(Error::InvalidSignature { expected: "SARC" });
		}

		reader.seek_rel(2)?;

		let byte_order_mark = reader.read_u16()?;
		match byte_order_mark {
			0xfeff => reader.set_byte_order(ByteOrder::Little),
			0xfffe => reader.set_byte_order(ByteOrder::Big),
			_ => {
				return Err(Error::InvalidByteOrderMark {
					bytes: byte_order_mark,
				});
			}
		};

		reader.seek_rel(-4)?;
		let header_size = reader.read_u16()?;
		if header_size != 0x14 {
			return Err(Error::IncorrectHeaderSize {
				name: "SARC",
				expected: 0x14,
				got: header_size,
			});
		}
		reader.seek_rel(2)?;

		let file_size = reader.read_u32()? as usize;
		if file_size != input.len() {
			return Err(Error::UnmatchingFileSize);
		}

		let data_offset = reader.read_u32()? as usize;
		if data_offset >= file_size {
			return Err(Error::OffsetPastEOF);
		}

		let version = reader.read_u16()?;
		if version != 0x100 {
			return Err(Error::UnsupportedVersion { version });
		}

		reader.seek_rel(2)?;

		// SFAT - File Allocation Table

		let signature = reader.read_slice(4)?;
		if signature != b"SFAT" {
			return Err(Error::InvalidSignature { expected: "SFAT" });
		}

		let header_size = reader.read_u16()?;
		if header_size != 0xc {
			return Err(Error::IncorrectHeaderSize {
				name: "SFAT",
				expected: 0xc,
				got: header_size,
			});
		}

		let file_count = reader.read_u16()? as usize;
		let hash_multiplier = reader.read_u32()?; // always 0x65?

		let sfat_offset = reader.position();

		// SFNT - File Name Table

		reader.seek(sfat_offset + 0x10 * file_count)?;
		let signature = reader.read_slice(4)?;
		if signature != b"SFNT" {
			return Err(Error::InvalidSignature { expected: "SFNT" });
		}

		let header_size = reader.read_u16()?;
		if header_size != 0x8 {
			return Err(Error::IncorrectHeaderSize {
				name: "SFNT",
				expected: 0x8,
				got: header_size,
			});
		}

		reader.seek_rel(2)?;

		let filenames_offset = reader.position();

		let files = (0..file_count)
			.map(|file_idx| {
				reader.seek(sfat_offset + file_idx * 0x10)?;
				let name_hash = reader.read_u32()?;
				let name_attrs = reader.read_u32()?;
				let start_offset = reader.read_u32()? as usize;
				let end_offset = reader.read_u32()? as usize;

				let _data_size = end_offset
					.checked_sub(start_offset)
					.ok_or(Error::DataOffsetsOutOfOrder)?;

				ensure!(
					data_offset
						.checked_add(end_offset)
						.is_some_and(|value| value <= input.len()),
					FileOutOfBoundsSnafu
				);

				let _hash_collision_count = name_attrs >> 24;
				let name_offset = (name_attrs & 0xffffff) as usize * 4;
				reader.seek(filenames_offset + name_offset)?;

				let name = reader.read_until(0).to_vec();
				let hash_calced = calc_hash(&name, hash_multiplier);
				if name_hash != hash_calced {
					println!("{name_hash:#x} {hash_calced:#x}");
					return Err(Error::HashMismatch);
				}

				let name = String::from_utf8(name.to_vec()).map_err(|_| Error::UTF8Error)?;

				Ok((
					name,
					(data_offset + start_offset)..(data_offset + end_offset),
				))
			})
			.collect::<Result<_, Error>>()?;

		Ok(Reader {
			archive_data: input,
			files,
			byte_order: ByteOrder::Little,
		})
	}

	pub fn get_byte_order(&self) -> ByteOrder {
		self.byte_order
	}

	pub fn filenames(&self) -> impl Iterator<Item = &str> {
		self.files.keys().map(|string| string.as_str())
	}

	pub fn files(&self) -> impl Iterator<Item = (&str, &[u8])> {
		self.files.iter().map(|(name, range)| {
			(
				name.as_str(),
				self
					.archive_data
					.get(range.clone())
					.expect("file range should be in bounds"),
			)
		})
	}

	pub fn get_file(&self, name: &str) -> Option<&[u8]> {
		let range = self.files.get(name)?;

		Some(
			self
				.archive_data
				.get(range.clone())
				.expect("file range should be in-bounds"),
		)
	}
}

struct File {
	path: String,
	data: Vec<u8>,
	hash: u32,
}

pub struct Writer {
	files: Vec<File>,
	default_alignment: u32,
}

impl Writer {
	pub fn new(default_alignment: u32) -> Self {
		Self {
			files: vec![],
			default_alignment,
		}
	}

	pub fn add_file(&mut self, path: &str, data: Vec<u8>) {
		let hash = calc_hash(path.as_bytes(), 0x65);
		self.files.push(File {
			path: path.to_owned(),
			data,
			hash,
		})
	}

	pub fn save(&mut self, out_path: &Path, byte_order: ByteOrder) -> Result<(), Error> {
		let data = self.save_to_vec(byte_order)?;
		fs::write(out_path, data)?;

		Ok(())
	}

	pub fn save_to_vec(&mut self, byte_order: ByteOrder) -> Result<Vec<u8>, Error> {
		let mut writer = BinaryWriter::new(byte_order);
		writer.write_bytes(b"SARC"); // signature
		writer.write_u16(0x14); // header size
		writer.write_u16(0xfeff); // byte order mark
		writer.write_u32(0); // file size
		writer.write_u32(0); // data offset
		writer.write_u16(0x100); // version
		writer.write_u16(0); // padding

		let file_count = self.files.len();
		writer.write_bytes(b"SFAT"); // signature
		writer.write_u16(0xc); // header size
		writer.write_u16(file_count as u16); // file count
		writer.write_u32(0x65); // hash multiplier

		let sfat_entries_offset = writer.position();

		writer.seek_rel(file_count * 0x10);
		writer.write_bytes(b"SFNT"); // signature
		writer.write_u16(0x8); // header size
		writer.write_u16(0); // padding

		let sfnt_entries_offset = writer.position();

		self.files.sort_by(|a, b| a.hash.cmp(&b.hash));

		let mut max_alignment = 0;
		let mut hash_counts = IndexMap::new();
		for (idx, file) in self.files.iter().enumerate() {
			writer.align_up(4)?;
			let string_pos = writer.position();
			writer.write_string(&file.path, true);
			let string_end_pos = writer.position();

			let collision_num = *hash_counts
				.entry(file.hash)
				.and_modify(|e| {
					*e += 1;
				})
				.or_insert(1) as u8;

			let string_offset = (string_pos - sfnt_entries_offset) / 4;
			let file_attrs = (collision_num as u32) << 24 | string_offset as u32 & 0xffffff;

			writer.seek(sfat_entries_offset + idx * 0x10);
			writer.write_u32(file.hash);
			writer.write_u32(file_attrs);
			writer.seek(string_end_pos);

			max_alignment = u32::max(max_alignment, self.get_alignment(file));
		}

		writer.align_up(max_alignment as usize)?;
		let data_offset = writer.position();
		writer.seek(0xc);
		writer.write_u32(data_offset as u32);
		writer.seek(data_offset);

		let mut file_size = 0;
		for (idx, file) in self.files.iter().enumerate() {
			writer.align_up(self.get_alignment(file) as usize)?;
			let start_offset = writer.position();
			writer.write_bytes(&file.data);
			let end_offset = writer.position();
			file_size = end_offset;

			writer.seek(sfat_entries_offset + idx * 0x10 + 8);
			writer.write_u32((start_offset - data_offset) as u32);
			writer.write_u32((end_offset - data_offset) as u32);
			writer.seek(end_offset);
		}

		writer.seek(0x8);
		writer.write_u32(file_size as u32);

		Ok(writer.take_inner())
	}

	fn get_alignment(&self, file: &File) -> u32 {
		let extension = file.path.split(".").last();
		if file.data.starts_with(b"FRES") {
			return 0x1000;
		} else if file.data.starts_with(b"YB") || file.data.starts_with(b"BY") {
			return 0x80;
		} else if file.data.starts_with(b"AAMP") {
			return 0x80;
		} else if let Some(ext) = extension
			&& ext == "kcl"
		{
			return 0x80;
		}

		self.default_alignment
	}
}

fn calc_hash(string: &[u8], hash_multiplier: u32) -> u32 {
	let mut hash = 0;
	for byte in string {
		// bytes have to be sign extended utf-8 in order for the hash to be correct
		hash = u32::wrapping_add(u32::wrapping_mul(hash, hash_multiplier), *byte as i8 as u32);
	}
	hash
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use crate::yaz0;

	use super::*;

	#[test]
	fn test_all_romfs() {
		let base_path = PathBuf::from("/home/tetra/nx/smo/100/romfs");

		let _object_data_path = base_path.join("ObjectData");
		let _stage_data_path = base_path.join("StageData");
		let _system_data_path = base_path.join("SystemData");
		let _layout_data_path = base_path.join("LayoutData");
		let _localized_data_path = base_path.join("LocalizedData");
		let _event_data_path = base_path.join("EventData");
		let _effect_data_path = base_path.join("EffectData");
		let _shader_data_path = base_path.join("ShaderData");
		let _sound_data_path = base_path.join("SoundData");

		let match_szs = |path: PathBuf| {
			match path.extension() {
				None => return,
				Some(ext) => match ext.as_encoded_bytes() {
					b"szs" => (),
					_ => {
						println!("skipping file: {path:?}");
						return;
					}
				},
			}
			let contents = fs::read(&path).unwrap();

			let (contents, alignment) = yaz0::decompress(&contents).unwrap();

			let reader = Reader::new(contents.clone()).unwrap();
			let mut writer = Writer::new(alignment);

			for path in reader.filenames() {
				let data = reader.get_file(path).unwrap();
				writer.add_file(path, data.to_owned());
			}

			let out_data = writer.save_to_vec(reader.get_byte_order()).unwrap();

			if contents != out_data {
				fs::write("test/out.orig", contents).unwrap();
				fs::write("test/out.new", out_data).unwrap();

				panic!("{path:?}: did not save to the same bytes (alignment = {alignment:#x})");
			}
		};

		for entry in fs::read_dir(_sound_data_path).unwrap() {
			match_szs(entry.unwrap().path());
		}

		// for lang_entry in fs::read_dir(_localized_data_path).unwrap() {
		// 	let lang_entry = lang_entry.unwrap();
		// 	if lang_entry.file_name().into_string().unwrap() == "Common" {
		// 		match_szs(lang_entry.path().join("ProjectData.szs"));
		// 	} else {
		// 		for data_entry in fs::read_dir(lang_entry.path()).unwrap() {
		// 			for file_entry in fs::read_dir(data_entry.unwrap().path()).unwrap() {
		// 				match_szs(file_entry.unwrap().path());
		// 			}
		// 		}
		// 	}
		// }
	}
}
