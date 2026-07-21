use bytemuck::Zeroable;

use crate::texture::bcn::{bc3_colour_table_4, channel_table_8_unorm};

#[derive(bytemuck::Pod, Clone, Copy, Zeroable)]
#[repr(C)]
struct Block {
	alpha_0: u8,
	alpha_1: u8,
	alpha_indices: [u8; 6],
	colour_0: [u8; 2],
	colour_1: [u8; 2],
	colour_indices: [u8; 4],
}

pub fn decode_rgb(input: &[[u8; 16]], output: &mut [[u8; 4]], width: usize, height: usize) {
	assert!(output.len() == width * height);
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let texel: Block = bytemuck::cast(input[block_width * by + bx]);
			let alpha = channel_table_8_unorm(texel.alpha_0, texel.alpha_1);
			let alpha_indices =
				std::array::from_fn(|byte| texel.alpha_indices.get(byte).cloned().unwrap_or(0));
			let mut alpha_indices = u64::from_le_bytes(alpha_indices);
			let colours = bc3_colour_table_4(texel.colour_0, texel.colour_1);
			let mut colour_indices = u32::from_le_bytes(texel.colour_indices);
			for y in 0..4 {
				for x in 0..4 {
					let colours = colours[(colour_indices & 0b11) as usize];
					output[width * (by * 4 + y) + (bx * 4 + x)][0] = colours[0];
					output[width * (by * 4 + y) + (bx * 4 + x)][1] = colours[1];
					output[width * (by * 4 + y) + (bx * 4 + x)][2] = colours[2];
					output[width * (by * 4 + y) + (bx * 4 + x)][3] = alpha[(alpha_indices & 0b111) as usize];
					colour_indices >>= 2;
					alpha_indices >>= 3;
				}
			}
		}
	}
}
