pub mod compress;

use bytemuck::Zeroable;

use crate::texture::bcn::{channel_table_8_snorm, channel_table_8_unorm};

#[derive(bytemuck::Pod, Clone, Copy, Zeroable)]
#[repr(C)]
struct BlockUnorm {
	pub red_0: u8,
	pub red_1: u8,
	pub indices: [u8; 6],
}

pub fn decode_unorm(input: &[[u8; 8]], output: &mut [u8], width: usize, height: usize) {
	assert!(output.len() == width * height);
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let texel: BlockUnorm = bytemuck::cast(input[block_width * by + bx]);
			let colors = channel_table_8_unorm(texel.red_0, texel.red_1);
			let indices = std::array::from_fn(|byte| texel.indices.get(byte).cloned().unwrap_or(0));
			let mut indices = u64::from_le_bytes(indices);
			for y in 0..4 {
				for x in 0..4 {
					output[width * (by * 4 + y) + (bx * 4 + x)] = colors[(indices & 0b111) as usize];
					indices >>= 3;
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
	pub indices: [u8; 6],
}
pub fn decode_snorm(input: &[[u8; 8]], output: &mut [i8], width: usize, height: usize) {
	assert!(output.len() == width * height);
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let texel: BlockSnorm = bytemuck::cast(input[block_width * by + bx]);
			let colors = channel_table_8_snorm(texel.red_0, texel.red_1);
			let indices = std::array::from_fn(|byte| texel.indices.get(byte).cloned().unwrap_or(0));
			let mut indices = u64::from_le_bytes(indices);
			for y in 0..4 {
				for x in 0..4 {
					output[width * (by * 4 + y) + (bx * 4 + x)] = colors[(indices & 0b111) as usize];
					indices >>= 3;
				}
			}
		}
	}
}
