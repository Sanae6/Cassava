use bytemuck::cast;

use crate::texture::{bc4::BlockUnorm, bcn::channel_table_8_unorm};

pub fn unorm(input: &[u8], output: &mut [[u8; 8]], width: usize, height: usize) {
	let (block_width, block_height) = (width / 4, height / 4);
	for by in 0..block_height {
		for bx in 0..block_width {
			let range = |x, y| width * (4 * by + y) + bx * 4 + x;
			let block_pixels: [u8; 16] = cast::<[[u8; 4]; 4], _>(std::array::from_fn(|y| {
				input[range(0, y)..range(4, y)].try_into().unwrap()
			}));
			let texel: &mut BlockUnorm = bytemuck::cast_mut(&mut output[block_width * by + bx]);
			let (lowest, highest) = (
				*block_pixels.iter().min().unwrap(),
				*block_pixels.iter().max().unwrap(),
			);
			texel.red_0 = lowest;
			texel.red_1 = highest;
      // todo: check to see which side of the table are optimal
      // can omit blackish pixels from the min-max calculation for the lower interpolation table
			let table = channel_table_8_unorm(lowest, highest);
			let mut indices: u64 = 0;
			for index in (0..16).rev() {
				indices <<= 3;
				indices |= table
					.into_iter()
					.map(|value| value.abs_diff(block_pixels[index]))
					.enumerate()
					.min_by_key(|(_, value)| *value)
					.unwrap()
					.0 as u64;
			}
			texel.indices.copy_from_slice(&indices.to_le_bytes()[0..6]);
		}
	}
}
