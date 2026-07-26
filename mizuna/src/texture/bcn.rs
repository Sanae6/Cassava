use bitfield_struct::bitfield;
use bytemuck::Zeroable;

pub fn channel_table_8_unorm(c_0: u8, c_1: u8) -> [u8; 8] {
	let (c_0, c_1) = (c_0 as u16, c_1 as u16);
	if c_0 > c_1 {
		let c_2 = (6 * c_0 + 1 * c_1) / 7;
		let c_3 = (5 * c_0 + 2 * c_1) / 7;
		let c_4 = (4 * c_0 + 3 * c_1) / 7;
		let c_5 = (3 * c_0 + 4 * c_1) / 7;
		let c_6 = (2 * c_0 + 5 * c_1) / 7;
		let c_7 = (1 * c_0 + 6 * c_1) / 7;
		[c_0, c_1, c_2, c_3, c_4, c_5, c_6, c_7].map(|value| value as u8)
	} else {
		let c_2 = (4 * c_0 + 1 * c_1) / 5;
		let c_3 = (3 * c_0 + 2 * c_1) / 5;
		let c_4 = (2 * c_0 + 3 * c_1) / 5;
		let c_5 = (1 * c_0 + 4 * c_1) / 5;
		let c_6 = 0;
		let c_7 = 255;
		[c_0, c_1, c_2, c_3, c_4, c_5, c_6, c_7].map(|value| value as u8)
	}
}

pub fn channel_table_8_snorm(c_0: i8, c_1: i8) -> [i8; 8] {
	let (c_0, c_1) = (c_0 as i16, c_1 as i16);
	if c_0 > c_1 {
		let c_2 = (6 * c_0 + 1 * c_1) / 7;
		let c_3 = (5 * c_0 + 2 * c_1) / 7;
		let c_4 = (4 * c_0 + 3 * c_1) / 7;
		let c_5 = (3 * c_0 + 4 * c_1) / 7;
		let c_6 = (2 * c_0 + 5 * c_1) / 7;
		let c_7 = (1 * c_0 + 6 * c_1) / 7;
		[c_0, c_1, c_2, c_3, c_4, c_5, c_6, c_7]
			.map(|value| i16::clamp(value, i8::MIN as i16, i8::MAX as i16) as i8)
	} else {
		let c_2 = (4 * c_0 + 1 * c_1) / 5;
		let c_3 = (3 * c_0 + 2 * c_1) / 5;
		let c_4 = (2 * c_0 + 3 * c_1) / 5;
		let c_5 = (1 * c_0 + 4 * c_1) / 5;
		let c_6 = 0;
		let c_7 = 255;
		[c_0, c_1, c_2, c_3, c_4, c_5, c_6, c_7]
			.map(|value| i16::clamp(value, i8::MIN as i16, i8::MAX as i16) as i8)
	}
}

#[bitfield(u16)]
#[derive(bytemuck::Pod, Zeroable)]
struct Rgb565 {
	#[bits(5)]
	b: u8,
	#[bits(6)]
	g: u8,
	#[bits(5)]
	r: u8,
}

impl Rgb565 {
	pub fn components(self) -> [u16; 3] {
		// https://stackoverflow.com/a/9069480
		[
			(self.r() as u16 * 527 + 23) >> 6,
			(self.g() as u16 * 259 + 33) >> 6,
			(self.b() as u16 * 527 + 23) >> 6,
		]
	}
}

pub fn bc3_color_table_4(color_0: [u8; 2], color_1: [u8; 2]) -> [[u8; 3]; 4] {
	if u16::from_le_bytes(color_0) <= u16::from_le_bytes(color_1) {
		color_table_4(color_1, color_0)
	} else {
		color_table_4(color_0, color_1)
	}
}

pub fn color_table_4(color_0: [u8; 2], color_1: [u8; 2]) -> [[u8; 3]; 4] {
	let color_0 = u16::from_le_bytes(color_0);
	let color_1 = u16::from_le_bytes(color_1);
	let comp_0 = Rgb565::from_bits(color_0).components();
	let comp_1 = Rgb565::from_bits(color_1).components();
	if color_0 > color_1 {
		[
			comp_0,
			comp_1,
			std::array::from_fn(|i| (2 * comp_0[i] + 1 * comp_1[i]) / 3),
			std::array::from_fn(|i| (1 * comp_0[i] + 2 * comp_1[i]) / 3),
		]
	} else {
		[
			comp_0,
			comp_1,
			std::array::from_fn(|i| (comp_0[i] + comp_1[i]) / 2),
			[0, 0, 0],
		]
	}
	.map(|color| color.map(|value| value as u8))
}
