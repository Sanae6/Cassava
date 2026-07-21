use bytemuck::{Zeroable, cast};

use crate::texture::bcn::{channel_table_8_snorm, channel_table_8_unorm};

#[derive(bytemuck::Pod, Clone, Copy, Zeroable)]
#[repr(C)]
struct BlockUnorm {
	pub red_0: u8,
	pub red_1: u8,
	pub red_indices: [u8; 6],
	pub green_0: u8,
	pub green_1: u8,
	pub green_indices: [u8; 6],
}

pub fn decode_unorm(input: &[[u8; 16]], output: &mut [[u8; 2]], width: usize, height: usize) {
	assert!(output.len() == width * height);
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let texel: BlockUnorm = cast(input[block_width * by + bx]);
			let red_colours = channel_table_8_unorm(texel.red_0, texel.red_1);
			let green_colours = channel_table_8_unorm(texel.green_0, texel.green_1);
			let red_indices =
				std::array::from_fn(|byte| texel.red_indices.get(byte).cloned().unwrap_or(0));
			let green_indices =
				std::array::from_fn(|byte| texel.green_indices.get(byte).cloned().unwrap_or(0));
			let (mut red_indices, mut green_indices) = (
				u64::from_le_bytes(red_indices),
				u64::from_le_bytes(green_indices),
			);
			for y in 0..4 {
				for x in 0..4 {
					output[width * (by * 4 + y) + (bx * 4 + x)][0] =
						red_colours[(red_indices & 0b111) as usize];
					output[width * (by * 4 + y) + (bx * 4 + x)][1] =
						green_colours[(green_indices & 0b111) as usize];
					red_indices >>= 3;
					green_indices >>= 3;
				}
			}
		}
	}
}

#[derive(bytemuck::Pod, Clone, Copy, Zeroable)]
#[repr(C)]
struct BlockSnorm {
	pub red_0: i8,
	pub red_1: i8,
	pub red_indices: [u8; 6],
	pub green_0: i8,
	pub green_1: i8,
	pub green_indices: [u8; 6],
}

pub fn decode_snorm(input: &[[u8; 16]], output: &mut [[i8; 2]], width: usize, height: usize) {
	assert!(output.len() == width * height);
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let texel: BlockSnorm = cast(input[block_width * by + bx]);
			let red_colours = channel_table_8_snorm(texel.red_0, texel.red_1);
			let green_colours = channel_table_8_snorm(texel.green_0, texel.green_1);
			let red_indices =
				std::array::from_fn(|byte| texel.red_indices.get(byte).cloned().unwrap_or(0));
			let green_indices =
				std::array::from_fn(|byte| texel.green_indices.get(byte).cloned().unwrap_or(0));
			let (mut red_indices, mut green_indices) = (
				u64::from_le_bytes(red_indices),
				u64::from_le_bytes(green_indices),
			);
			for y in 0..4 {
				for x in 0..4 {
					output[width * (by * 4 + y) + (bx * 4 + x)][0] =
						red_colours[(red_indices & 0b111) as usize];
					output[width * (by * 4 + y) + (bx * 4 + x)][1] =
						green_colours[(green_indices & 0b111) as usize];
					red_indices >>= 3;
					green_indices >>= 3;
				}
			}
		}
	}
}
