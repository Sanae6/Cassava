/*
explicitly unsupported:
- hdr
- 3d textures
 */

use bitmatch::bitmatch;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use tracing::*;

use crate::nw::bntx::FormatInfo;

#[derive(Clone, Copy, Debug)]
struct Ise {
	bits: u8,
	trits: u8,
	quints: u8,
}
impl Ise {
	const fn table_rep(trits: u8, quints: u8, bits: u8) -> Ise {
		Ise {
			bits,
			quints,
			trits,
		}
	}

	const fn max_value(self) -> u8 {
		2u8.pow(self.bits as _) + 3u8.pow(self.trits as _) + 5u8.pow(self.quints as _)
	}
}

const fn calc_rho(rho: u8, high_precision: bool) -> Option<Ise> {
	match (rho, high_precision) {
		(0b000, _) | (0b001, _) => None,
		(0b010, false) => Some(Ise::table_rep(0, 0, 1)),
		(0b011, false) => Some(Ise::table_rep(1, 0, 0)),
		(0b100, false) => Some(Ise::table_rep(0, 0, 2)),
		(0b101, false) => Some(Ise::table_rep(0, 1, 0)),
		(0b110, false) => Some(Ise::table_rep(1, 0, 1)),
		(0b111, false) => Some(Ise::table_rep(0, 0, 3)),
		(0b010, true) => Some(Ise::table_rep(0, 1, 1)),
		(0b011, true) => Some(Ise::table_rep(1, 0, 2)),
		(0b100, true) => Some(Ise::table_rep(0, 0, 4)),
		(0b101, true) => Some(Ise::table_rep(0, 1, 2)),
		(0b110, true) => Some(Ise::table_rep(1, 0, 3)),
		(0b111, true) => Some(Ise::table_rep(0, 0, 5)),
		(8.., _) => unreachable!(),
	}
}

const fn mask(filled: u8, mask: u16) -> u16 {
	if filled != 0 { u16::MAX & mask } else { 0 }
}
#[derive(Debug)]
struct QuantInfo {
	ise_encoding: Ise,
	b_extractor: fn(u8) -> u16,
	c_value: u8,
}
fn calc_endpoint_info(remaining_bits: u8, cem_value_count: u8) -> QuantInfo {
	let mut i = 10u8;
	loop {
		let (trit_count, quint_count) = (!i & 1, i & 1);
		let bit_count = (i >> 1) + 1;
		let cem_bits = (cem_value_count * 8 * trit_count).div_ceil(5)
			+ (cem_value_count * 7 * quint_count).div_ceil(3)
			+ cem_value_count * bit_count;

		if cem_bits <= remaining_bits {
			return QuantInfo {
				ise_encoding: Ise::table_rep(trit_count, quint_count, bit_count),
				b_extractor: B_EXTRACTORS[i as usize],
				c_value: C_VALUES[i as usize],
			};
		}

		i -= 1;
	}
	const B_EXTRACTORS: [fn(u8) -> u16; 11] = [
		|_| 0,
		|_| 0,
		|v| mask(v & 1, 0b100010110),
		|v| mask(v & 1, 0b100001100),
		|v| mask(v & 1, 0b010000101) | mask(v & 2, 0b100001010),
		|v| mask(v & 1, 0b010000010) | mask(v & 2, 0b100000101),
		|v| mask(v & 1, 0b001000001) | mask(v & 2, 0b010000010) | mask(v & 4, 0b100000100),
		|v| mask(v & 1, 0b001000000) | mask(v & 2, 0b010000001) | mask(v & 4, 0b100000010),
		|v| {
			mask(v & 1, 0b000100000)
				| mask(v & 2, 0b001000000)
				| mask(v & 4, 0b010000001)
				| mask(v & 8, 0b100000010)
		},
		|v| {
			mask(v & 1, 0b000100000)
				| mask(v & 2, 0b001000000)
				| mask(v & 4, 0b010000000)
				| mask(v & 8, 0b100000001)
		},
		|v| {
			mask(v & 1, 0b000010000)
				| mask(v & 2, 0b000100000)
				| mask(v & 4, 0b001000000)
				| mask(v & 8, 0b010000000)
				| mask(v & 16, 0b100000001)
		},
	];
	const C_VALUES: [u8; 11] = [204, 113, 93, 54, 44, 26, 22, 13, 11, 6, 5];
}

fn calc_weight_info(weight_bits: u8, weight_count: u8) -> QuantInfo {
	let mut i = 4u8;
	loop {
		let (trit_count, quint_count) = (!i & 1, i & 1);
		let bit_count = (i >> 1) + 1;
		let potential_weight_bits = (weight_count * 8 * trit_count).div_ceil(5)
			+ (weight_count * 7 * quint_count).div_ceil(3)
			+ weight_count * bit_count;

		if potential_weight_bits <= weight_bits {
			return QuantInfo {
				ise_encoding: Ise::table_rep(trit_count, quint_count, bit_count),
				b_extractor: B_EXTRACTORS[i as usize],
				c_value: C_VALUES[i as usize],
			};
		}

		info!("pot {potential_weight_bits} {weight_bits} {bit_count} {trit_count} {quint_count}");
		i -= 1;
	}
	const B_EXTRACTORS: [fn(u8) -> u16; 5] = [
		|_| 0,
		|_| 0,
		|v| mask(v & 1, 0b1000101),
		|v| mask(v & 1, 0b1000010),
		|v| mask(v & 1, 0b0100001) | mask(v & 2, 0b1000010),
	];
	const C_VALUES: [u8; 5] = [50, 28, 23, 13, 11];
}

type DecodedValues = heapless::Vec<(u8, u8), 5>;
fn decode_trits(value: u128, ise: Ise) -> DecodedValues {
	let tbits = value.extract_bits(0b0000_1000_0110_0001_0000_1100_0011_0000);
	let mut m = value.extract_bits(0b0000_0111_1001_1110_1111_0011_1100_1111);
	let mut trits = [0u8; 5];
	let c = if tbits.extract_bits(0b0001_1100) == 0b111 {
		trits[3] = 2;
		trits[4] = 2;
		tbits.extract_bits(0b1110_0011)
	} else {
		if tbits.extract_bits(0b0110_0000) == 0b11 {
			trits[3] = tbits.extract_bits(0b1000_0000) as _;
			trits[4] = 2;
		} else {
			trits[3] = tbits.extract_bits(0b0110_0000) as _;
			trits[4] = tbits.extract_bits(0b1000_0000) as _;
		}
		tbits.extract_bits(0b0001_1111)
	};

	if c.extract_bits(0b00011) == 0b11 {
		println!("0");
		trits[2] = 2;
		trits[1] = c.extract_bits(0b10000) as _;
		let c3 = c.extract_bits(0b01000);
		let c2 = c.extract_bits(0b00100);
		trits[0] = (c3 << 1 | c2 & !c3) as _;
	} else if c.extract_bits(0b11000) == 0b11 {
		println!("1");
		trits[2] = 2;
		trits[1] = 2;
		trits[0] = c.extract_bits(0b00011) as _;
	} else {
		trits[2] = c.extract_bits(0b10000) as _;
		trits[1] = c.extract_bits(0b01100) as _;
		let c1 = c.extract_bits(0b00010);
		let c0 = c.extract_bits(0b00001);
		trits[0] = (c1 << 1 | c0 & !c1) as _;
	}

	let bit_mask = (1 << ise.bits) - 1;
	let values = heapless::Vec::from_array(trits.map(|t| {
		let bits = m & bit_mask;
		m >>= ise.bits;
		(t, bits as u8)
	}));
	values
}

fn decode_quints(value: u128, ise: Ise) -> DecodedValues {
	let qbits = value.extract_bits(0b110_0001_1000_0111_0000);
	let mut m = value.extract_bits(0b001_1110_0111_1000_1111);

	let quints = if qbits.extract_bits(0b110_0110) == 0b0011 {
		let bits: [u8; 5] = std::array::from_fn(|i| qbits.extract_bits(1 << i) as _);
		let q2 = bits[0] << 2 | (bits[4] & !bits[0]) << 1 | bits[3] & !bits[0];
		[q2, 4, 4]
	} else {
		let (q2, c) = if qbits.extract_bits(0b00110) == 0b11 {
			(
				4,
				qbits.extract_bits(0b001_1000) << 3
					| (!qbits.extract_bits(0b110_0000) & 0b11) << 1
					| qbits.extract_bits(0b000_0001),
			)
		} else {
			(
				qbits.extract_bits(0b110_0000) as _,
				qbits.extract_bits(0b001_1111),
			)
		};

		let (q1, q0) = if c.extract_bits(0b00111) == 0b101 {
			(4, c.extract_bits(0b11000) as _)
		} else {
			(c.extract_bits(0b11000) as _, c.extract_bits(0b00111) as _)
		};
		[q2, q1, q0]
	};

	let bit_mask = (1 << ise.bits) - 1;
	let values = heapless::Vec::from_array(quints.map(|q| {
		let bits = m & bit_mask;
		m >>= ise.bits;
		(q, bits as u8)
	}));
	values
}

fn decode_ise<const MAX: usize>(
	mut data: u128,
	data_count: u8,
	info: QuantInfo,
	unquantise: fn(u16, u16, u16, u16) -> u8,
) -> heapless::Vec<u8, MAX> {
	info!("endpoint count {data_count}");
	match info.ise_encoding {
		ise @ Ise {
			trits: 1,
			quints: 0,
			bits: _,
		} => {
			let mut values: heapless::Vec<u8, MAX> = heapless::Vec::new();
			let block_count = data_count.div_ceil(5);
			let mut quant_values: heapless::Vec<(u8, u8), 64> = heapless::Vec::new();
			for _ in 0..block_count {
				let decoded = decode_trits(data, ise);
				data >>= 28;
				quant_values.extend(decoded);
			}
			quant_values.reverse();
			for _ in 0..data_count {
				let (d, bits) = quant_values.pop().unwrap();
				let a = 0b1_1111_1111 & !(bits as u16 & 0b1);
				let b = (info.b_extractor)(bits >> 1);
				let c = info.c_value as u16;
				let d = d as u16;

				// info!("unquantised {a:09b} {b:09b} {c} {d} {unq}");
				values
					.push(
						unquantise(a, b, c, d)
							.try_into()
							.expect("decoded trit was too large"),
					)
					.expect("too many colour endpoints");
			}
			values
		}
		ise @ Ise {
			trits: 0,
			quints: 1,
			bits: _,
		} => {
			let mut values: heapless::Vec<u8, MAX> = heapless::Vec::new();
			let block_count = data_count.div_ceil(3);
			let mut quant_values: heapless::Vec<(u8, u8), 64> = heapless::Vec::new();
			for _ in 0..block_count {
				let decoded = decode_quints(data, ise);
				data >>= 19;
				quant_values.extend(decoded);
			}
			for _ in 0..data_count {
				let (d, bits) = quant_values.pop().unwrap();
				let a = 0b1_1111_1111 & !(bits as u16 & 0b1);
				let b = (info.b_extractor)(bits >> 1);
				let c = info.c_value as u16;
				let d = d as u16;

				values
					.push(
						unquantise(a, b, c, d)
							.try_into()
							.expect("decoded quint was too large"),
					)
					.expect("too many colour endpoints");
			}
			values
		}
		_ => unreachable!(),
	}
}

#[derive(FromPrimitive, Clone, Copy, Debug)]
enum ColourEndpointMode {
	LowLumDirect = 0,
	LowLumBaseOffset = 1,
	HighLumLargeRange = 2,
	HighLumSmallRange = 3,
	LowLumAlphaDirect = 4,
	LowLumAlphaBaseOffset = 5,
	LowRgbBaseScale = 6,
	HighRgbBaseScale = 7,
	LowRgbDirect = 8,
	LowRgbBaseOffset = 9,
	LowRgbBaseScalePlusTwoA = 10,
	HighRgbDirect = 11,
	LowRgbaDirect = 12,
	LowRgbaBaseOffset = 13,
	HighRgbDirectLowAlpha = 14,
	HighRgbDirectHighAlpha = 15,
}

fn endpoint_interpretation(
	mode: ColourEndpointMode,
	colours: heapless::Vec<u8, 8>,
) -> [[u8; 4]; 2] {
	let v = colours;
	fn blue_contract(values: [u8; 4]) -> [u8; 4] {
		[
			((values[0] as u16 + values[2] as u16) >> 1) as u8,
			((values[1] as u16 + values[2] as u16) >> 1) as u8,
			values[2],
			values[3],
		]
	}
	match mode {
		ColourEndpointMode::LowLumDirect => {
			let (c0, c1) = (v[0], v[1]);
			[[c0, c0, c0, 0xFF], [c1, c1, c1, 0xFF]]
		}
		ColourEndpointMode::LowLumBaseOffset => todo!(),
		ColourEndpointMode::LowLumAlphaDirect => todo!(),
		ColourEndpointMode::LowLumAlphaBaseOffset => todo!(),
		ColourEndpointMode::LowRgbBaseScale => [[0; 4]; 2],
		ColourEndpointMode::LowRgbDirect => {
			let s0 = v[0] as u16 + v[2] as u16 + v[4] as u16;
			let s1 = v[1] as u16 + v[3] as u16 + v[5] as u16;
			let endpoints = [[v[0], v[2], v[4], 0xFF], [v[1], v[3], v[5], 0xFF]];
			if s1 >= s0 {
				endpoints
			} else {
				endpoints.map(blue_contract)
			}
		}
		ColourEndpointMode::LowRgbBaseOffset => [[0; 4]; 2],
		ColourEndpointMode::LowRgbBaseScalePlusTwoA => todo!(),
		ColourEndpointMode::LowRgbaDirect => todo!(),
		ColourEndpointMode::LowRgbaBaseOffset => todo!(),
		ColourEndpointMode::HighLumLargeRange => unimplemented!(),
		ColourEndpointMode::HighLumSmallRange => unimplemented!(),
		ColourEndpointMode::HighRgbBaseScale => unimplemented!(),
		ColourEndpointMode::HighRgbDirect => unimplemented!(),
		ColourEndpointMode::HighRgbDirectLowAlpha => unimplemented!(),
		ColourEndpointMode::HighRgbDirectHighAlpha => unimplemented!(),
	}
}

#[bitmatch]
fn decode<O: Copy>(
	input: &[[u8; 16]],
	output: &mut [O],
	info: FormatInfo,
	width: usize,
	height: usize,
	srgb: bool,
	colour_decoder: fn(colour: [u16; 4]) -> O,
) {
	assert!(output.len() == width * height);
	let block_width = width / info.block_width();
	let block_height = height / info.block_height();
	for by in 0..block_height {
		'block: for bx in 0..block_width {
			let block = bytemuck::cast(input[block_width * by + bx]);
			let block = u128::from_le_bytes(block);
			let block_mode = block & 0x7ff;

			macro_rules! report_error {
				() => {{
					for y in 0..info.block_width() {
						for x in 0..info.block_height() {
							output[width * (by * info.block_height() + y) + (bx * info.block_width() + x)] =
								colour_decoder([u16::MAX, 0, u16::MAX, u16::MAX]);
						}
					}
					continue 'block;
				}};
			}
			if block_mode & 0x1ff == 0b1_1111_1100 {
				// void extent block
				// don't care about other blocks, just deal with each one by one
				// so we're skipping the min/max extent stuff
				let colours = block >> 64;
				let colours = [
					((colours >> 00) & 0xFFFF) as u16,
					((colours >> 16) & 0xFFFF) as u16,
					((colours >> 32) & 0xFFFF) as u16,
					((colours >> 48) & 0xFFFF) as u16,
				];
				let colours = colour_decoder(colours);
				for y in 0..info.block_width() {
					for x in 0..info.block_height() {
						output[width * (by * info.block_height() + y) + (bx * info.block_width() + x)] =
							colours;
					}
				}
				continue 'block;
			}
			info!("block mode: {:011b}", block_mode & 0b111_1111_1111);
			let (weight_grid_width, weight_grid_height, rho, dual_plane, precision) = #[bitmatch]
			match block_mode {
				"dpwwhhr00rr" => (w + 4, h + 2, r, d, p),
				"dpwwhhr01rr" => (w + 8, h + 2, r, d, p),
				"dphhwwr10rr" => (w + 2, h + 8, r, d, p),
				"dp0hwwr11rr" => (w + 2, h + 6, r, d, p),
				"dp1whhr11rr" => (w + 2, h + 2, r, d, p),
				"dp00hhrrr00" => (12, h + 2, r, d, p),
				"dp01wwrrr00" => (w + 2, 12, r, d, p),
				"dp1100rrr00" => (6, 10, r, d, p),
				"hh10wwrrr00" => (w + 6, h + 6, r, 0, 0),
				_ => {
					warn!(
						"invalid block mode encoding {:011b}",
						block_mode & 0b111_1111_1111
					);
					report_error!()
				}
			};
			let (dual_plane, high_precision) = (dual_plane != 0, precision != 0);
			let rho = ((rho << 1) & 0b0110) | ((rho & 0b100) >> 2);
			let Some(rho) = calc_rho(rho as _, high_precision) else {
				warn!("cooked rho value");
				report_error!();
			};
			let partition_count = (block >> 11) & 0b11;
			// Data format spec 1.1: 18.22
			info!("weight grid wxh: {weight_grid_width}x{weight_grid_height}");
			let mut weight_count = (weight_grid_width * weight_grid_height) as u8;
			let weight_grid_dim = [weight_grid_width as u8, weight_grid_height as u8, 1];
			let mut config_bits = 17;
			if dual_plane {
				weight_count *= 2;
				config_bits += 2;
			}
			if partition_count == 0 {
				// single partition mode
				let cem_class = (block >> 15) & 0b11;
				let cem = ColourEndpointMode::from_u128((block >> 13) & 0b1111).unwrap();

				{
					let weight_bits = (weight_count * 8 * rho.trits as u8).div_ceil(5)
						+ (weight_count * 7 * rho.quints as u8).div_ceil(3)
						+ weight_count * rho.bits as u8;
					info!(
						"weight: {weight_count} {dual_plane} {rho:?} {}",
						weight_bits
					);
					let remaining_bits = 128 - config_bits - weight_bits;
					info!("bits: {}", remaining_bits);
					let endpoint_mod = (block >> config_bits) & 0xF;
					let colour_endpoint_region = block >> (config_bits + 4);
					let weights_region = block >> config_bits;
					let cem_pair_count = cem_class + 1;
					info!("cem pair count {cem_pair_count}");
					let endpoint_info = calc_endpoint_info(remaining_bits as _, cem_pair_count as u8 * 2);
					info!("endpoint info: {endpoint_info:?}");
					let endpoint_values = decode_ise(
						colour_endpoint_region,
						cem_pair_count as u8 * 2,
						endpoint_info,
						|a, b, c, d| {
							let unq = d * c + b;
							let unq = unq ^ a;
							let unq = (a & 0x80) | unq >> 2;
							unq.try_into().expect("msg")
						},
					);
					let endpoints = endpoint_interpretation(cem, endpoint_values)
						.map(|a| a.map(|v| (v as u16) << 8 | 0x80));

					let colours = colour_decoder(endpoints[0]);
					let weights_info = calc_weight_info(weight_bits as u8, weight_count as u8);
					let weights_region = weights_region.reverse_bits();
					let weight_values =
						decode_ise::<24>(weights_region, weight_count, weights_info, |a, b, c, d| {
							let unq = d * c + b;
							let unq = unq ^ a;
							let unq = (a & 0x20) | unq >> 2;
							(if unq == 63 { 64 } else { unq }).try_into().expect("msg")
						});

					let mut index = 0;
					let coordinates = [info.block_width(), info.block_height()].map(|block_size| {
						let block_size = block_size as u16;
						println!("{block_size}");
						let scalar = (1024 + block_size / 2) / (block_size - 1);
						let coord = block_size * scalar;
						let grid = (coord * (weight_grid_dim[index] as u16 - 1) + 32) >> 6;
						let integral = grid >> 4;
						let fractional = grid & 0xF;
						index += 1;
						(integral, fractional)
					});

					// for y in 0..info.block_width() {
					// 	for x in 0..info.block_height() {
					// 		output[width * (by * info.block_height() + y) + (bx * info.block_width() + x)] =
					// 			colours;
					// 	}
					// }
					// continue 'block;
				}

				info!("{bx}x{by} {cem:?} {dual_plane} {rho:?} {}", rho.max_value());
				// report_error!()
			} else {
				// multi-partition mode
				let partition_count = partition_count + 1;
				{
					// Data format spec 1.1: 18.22
					let mut config_bits = 17;
					if dual_plane {
						config_bits += 2;
					}
					// let weight_bits = (weight_count * 8 * rho.quints as u32).div_ceil(5)
					// 	+ (weight_count * 8 * rho.trits as u32).div_ceil(3)
					// 	+ weight_count * rho.bits as u32;
					// let remaining_bits = 128 - config_bits - weight_bits;
					// let extra_cem_bits = match partition_count {
					// 	2 => 2,
					// 	3 => 5,
					// 	4 => 8,
					// 	_ => unreachable!(),
					// };
					// let cem_pair_count = cem_class + 1 + count_bits(extra_cem_bits);
				}
				warn!("texture uses multi-partition mode: {partition_count}");
				report_error!();
			}
		}
	}
}

pub fn decode_srgb(
	input: &[[u8; 16]],
	output: &mut [[u16; 4]],
	info: FormatInfo,
	width: usize,
	height: usize,
) {
	decode(input, output, info, width, height, true, |value| value)
}
