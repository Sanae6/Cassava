use bytemuck::Zeroable;

use crate::texture::bcn::color_table_4;

#[derive(bytemuck::Pod, Clone, Copy, Zeroable)]
#[repr(C)]
struct Block {
	color_0: [u8; 2],
	color_1: [u8; 2],
	indices: [u8; 4],
}

pub fn decode_rgb(input: &[[u8; 8]], output: &mut [[u8; 3]], width: usize, height: usize) {
	assert!(output.len() == width * height);
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let texel: Block = bytemuck::cast(input[block_width * by + bx]);
			let colors = color_table_4(texel.color_0, texel.color_1);

			let mut indices = u32::from_le_bytes(texel.indices);
			for y in 0..4 {
				for x in 0..4 {
					output[width * (by * 4 + y) + (bx * 4 + x)] = colors[(indices & 0b11) as usize];
					indices >>= 2;
				}
			}
		}
	}
}
