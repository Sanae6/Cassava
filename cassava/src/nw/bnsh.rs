use bitfield_struct::bitfield;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
	nw::{BinaryFileInfo, FileHeaderError, read_binary_file_header},
	util::{self, BinaryReader},
};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("failed to read file header"))]
	FileHeaderError {
		source: FileHeaderError,
	},
	UnsupportedVersion,
	#[snafu(display("expected magic to be 'BNTX\0\0\0\0'"))]
	IncorrectContainerMagic,
	#[snafu(display("invalid code type"))]
	InvalidCodeType {
		backtrace: snafu::Backtrace,
	},
	#[snafu(display("invalid source format"))]
	InvalidSourceFormat {
		backtrace: snafu::Backtrace,
	},
}

#[derive(Debug)]
pub struct ShaderContainer {
	pub file_info: BinaryFileInfo,
	pub target_api_type: u16,
	pub target_api_version: u16,
	pub target_code_type: u8,
	pub compiler_version: u32,
	pub low_level_compiler_version: u64,
	pub shader_variations: Vec<Variation>,
}

#[derive(Debug)]
pub struct Variation {
	pub source_program: Option<Program>,
	pub ir_program: Option<Program>,
	pub binary_program: Option<Program>,
}

#[derive(Debug)]
pub struct Program {
	pub flags: ShaderFlags,
	pub code_type: CodeType,
	pub source_format: SourceFormat,
	pub binary_format: u32,
	pub vertex_shader: Option<Shader>,
	pub hull_shader: Option<Shader>,
	pub domain_shader: Option<Shader>,
	pub geometry_shader: Option<Shader>,
	pub fragment_shader: Option<Shader>,
	pub compute_shader: Option<Shader>,
}

#[bitfield(u8)]
pub struct ShaderFlags {
	pub separation_enabled: bool,
	pub res_shader: bool,
	#[bits(6)]
	_reserved: u8,
}

#[derive(FromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeType {
	Binary,
	IntermediateRepresentation,
	Source,
	SourceArray,
}

#[derive(FromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
	Glsl,
	Hlsl,
}
pub struct Shader {
	pub control: Vec<u8>,
	pub data: Vec<u8>,
	pub scratch_memory_recommended: u32,
	pub scratch_memory_per_warp: u32,
	pub reflection: (),
}

impl std::fmt::Debug for Shader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Shader")
			.field("control", &self.control.len())
			.field("data", &self.data.len())
			.field(
				"scratch_memory_recommended",
				&self.scratch_memory_recommended,
			)
			.field("scratch_memory_per_warp", &self.scratch_memory_per_warp)
			.field("reflection", &self.reflection)
			.finish()
	}
}

impl ShaderContainer {
	pub fn read(reader: &mut BinaryReader) -> Result<ShaderContainer, Error> {
		let (file_info, header, mut reader) =
			read_binary_file_header(reader, Some(*b"BNSH\0\0\0\0")).context(FileHeaderSnafu)?;

		reader.seek(header.first_block_offset as usize)?;

		ensure!(
			reader.read_bytes()? == *b"grsc",
			IncorrectContainerMagicSnafu
		);

		reader.seek_rel(12)?;
		let target_api_type = reader.read_u16()?;
		let target_api_version = reader.read_u16()?;
		let target_code_type = reader.read_u8()?;
		reader.seek_rel(3)?;
		let compiler_version = reader.read_u32()?;
		let shader_variation_count = reader.read_u32()? as usize;
		let shader_variation_array = reader.read_u64()? as usize;
		let _shader_binary_pool = reader.read_u64()? as usize;
		let low_level_compiler_version = reader.read_u64()?;

		let shader_variations = reader.scope(shader_variation_array, |reader| {
			reader.read_vec(shader_variation_count, |reader, _| -> Result<_, Error> {
				let source_program = reader.read_u64()? as usize;
				let ir_program = reader.read_u64()? as usize;
				let binary_program = reader.read_u64()? as usize;
				reader.seek_rel(32 + 8)?;
				let read_program = |reader: &mut BinaryReader, program: usize| {
					(program != 0)
						.then(|| reader.scope(program, Self::read_program))
						.transpose()
				};
				let source_program = read_program(reader, source_program)?;
				let ir_program = read_program(reader, ir_program)?;
				let binary_program = read_program(reader, binary_program)?;
				Ok(Variation {
					source_program,
					ir_program,
					binary_program,
				})
			})
		})?;
		// println!("{file_info:#?} {header:?}");

		Ok(ShaderContainer {
			file_info,
			target_api_type,
			target_api_version,
			target_code_type,
			compiler_version,
			low_level_compiler_version,
			shader_variations,
		})
	}

	fn read_program(reader: &mut BinaryReader) -> Result<Program, Error> {
		let flags = ShaderFlags::from_bits(reader.read_u8()?);
		let code_type = CodeType::from_u8(reader.read_u8()?).context(InvalidCodeTypeSnafu)?;
		let source_format =
			SourceFormat::from_u8(reader.read_u8()?).context(InvalidSourceFormatSnafu)?;
		reader.seek_rel(1)?;
		let binary_format = reader.read_u32()?;
		let vertex_shader = reader.read_u64()? as usize;
		let hull_shader = reader.read_u64()? as usize;
		let domain_shader = reader.read_u64()? as usize;
		let geometry_shader = reader.read_u64()? as usize;
		let fragment_shader = reader.read_u64()? as usize;
		let compute_shader = reader.read_u64()? as usize;
		reader.seek_rel(40)?;
		let _obj_size = reader.read_u32()?;
		reader.seek_rel(4)?;
		let _obj = reader.read_u64()?;
		let _parent_variation = reader.read_u64()?;
		let reflection_data = reader.read_u64()?;
		let compiler_reflection_data = reader.read_u64()?;
		// todo: fetch reflection data and pass it to shaders

		let read_shader = |reader: &mut BinaryReader, shader: usize| {
			(shader != 0)
				.then(|| reader.scope(shader, Self::read_shader))
				.transpose()
		};
		let vertex_shader = read_shader(reader, vertex_shader)?;
		let hull_shader = read_shader(reader, hull_shader)?;
		let domain_shader = read_shader(reader, domain_shader)?;
		let geometry_shader = read_shader(reader, geometry_shader)?;
		let fragment_shader = read_shader(reader, fragment_shader)?;
		let compute_shader = read_shader(reader, compute_shader)?;

		Ok(Program {
			flags,
			code_type,
			source_format,
			binary_format,
			vertex_shader,
			hull_shader,
			domain_shader,
			geometry_shader,
			fragment_shader,
			compute_shader,
		})
	}

	fn read_shader(reader: &mut BinaryReader) -> Result<Shader, Error> {
		let _runtime_data_address = reader.read_u64()?;
		let control = reader.read_u64()? as usize;
		let data = reader.read_u64()? as usize;
		let control_size = reader.read_u32()? as usize;
		let data_size = reader.read_u32()? as usize;
		let decomposed_control = reader.read_u64()? as usize;
		let scratch_memory_recommended = reader.read_u32()?;
		let scratch_memory_per_warp = reader.read_u32()?;
		let mut control = reader.scope(control, |reader| {
			reader.read_slice(control_size).map(<[_]>::to_vec)
		})?;
		let data = reader.scope(data, |reader| {
			reader.read_slice(data_size).map(<[_]>::to_vec)
		})?;

		if decomposed_control != 0 {
			reader.scope(decomposed_control, |reader| -> Result<_, util::Error> {
				let metadata = reader.read_u64()? as usize;
				let assembly = reader.read_u64()? as usize;
				let specialization = reader.read_u64()? as usize;
				let pragma = reader.read_u64()? as usize;
				let locals = reader.read_u64()? as usize;
				let uniform_64 = reader.read_u64()? as usize;
				reader.subfile_scope(metadata, |reader| {
					reader.seek(20)?;
					let asm_offset = reader.read_u32()? as usize;
					let asm_size = reader.read_u32()? as usize;
					let locals_size = reader.read_u32()? as usize;
					let locals_offset = reader.read_u32()? as usize;
					reader.seek(1776)?;
					let specialization_offset = reader.read_u32()? as usize;
					let specialization_size = reader.read_u32()? as usize;
					reader.seek(1992)?;
					let pragma_offset = reader.read_u32()? as usize;
					let pragma_size = reader.read_u32()? as usize;
					reader.seek(2024)?;
					let uniform_64_offset = reader.read_u32()? as usize;
					let uniform_64_size = reader.read_u32()? as usize;

					let range = |offset: usize, size: usize| offset..offset + size;
					control[range(0, 20)].copy_from_slice(reader.read_slice_at(metadata, 20)?);
					control[range(asm_offset, asm_size)]
						.copy_from_slice(reader.read_slice_at(assembly, asm_size)?);
					control[range(locals_offset, locals_size)]
						.copy_from_slice(reader.read_slice_at(locals, locals_size)?);
					control[range(specialization_offset, specialization_size)]
						.copy_from_slice(reader.read_slice_at(specialization, specialization_size)?);
					control[range(pragma_offset, pragma_size)]
						.copy_from_slice(reader.read_slice_at(pragma, pragma_size)?);
					control[range(uniform_64_offset, uniform_64_size)]
						.copy_from_slice(reader.read_slice_at(uniform_64, uniform_64_size)?);
					Ok(())
				})
			})?;
		}

		Ok(Shader {
			control,
			data,
			scratch_memory_recommended,
			scratch_memory_per_warp,
			reflection: (),
		})
	}
}
