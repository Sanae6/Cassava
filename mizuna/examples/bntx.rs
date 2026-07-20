use std::{
	fs::{self, File},
	io::Cursor,
	path::PathBuf,
};

use clap::{Parser, Subcommand};
use ddsfile::{Dds, DxgiFormat, NewDxgiParams};
use image::{Pixel, RgbImage, Rgba, RgbaImage};
use mizuna::{
	bntx::{self, ChannelFormat, TextureArchive, TypeFormat},
	texture::{bc1, bc3, bc4, bc5},
	util::{BinaryReader, ByteOrder},
};

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Dump { in_path: PathBuf },
}

#[snafu::report]
fn main() -> Result<(), bntx::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path } => {
			let file = fs::read(in_path).unwrap();
			let texture_archive =
				TextureArchive::read(&mut BinaryReader::new(&file, ByteOrder::Little)).unwrap();
			for (name, texture) in texture_archive.textures {
				// let ChannelFormat::BC3 = texture.gfx.image_format.0 else {
				// 	continue;
				// };
				println!(
					"{:?} {:?} {} {}",
					name, texture.gfx.image_format, texture.gfx.width, texture.gfx.height
				);
				let dims = texture.dimensions();
				// let (ChannelFormat::BC3 | ChannelFormat::BC1) = texture.gfx.image_format.0 else{
				match texture.gfx.image_format {
					(ChannelFormat::BC4, TypeFormat::Unorm) => {
						let mut output = vec![0u8; (texture.gfx.width * texture.gfx.height) as usize];
						bc4::decode_unorm(
							bytemuck::cast_slice(&texture.image_data),
							&mut output,
							texture.gfx.width as usize,
							texture.gfx.height as usize,
						);
						let image =
							image::GrayImage::from_vec(texture.gfx.width, texture.gfx.height, output).unwrap();
						image.save(format!("target/{name}.png")).unwrap();
					}
					(ChannelFormat::BC5, TypeFormat::Snorm) => {
						let mut output = vec![0u8; (texture.gfx.width * texture.gfx.height * 2) as usize];
						bc5::decode_snorm(
							bytemuck::cast_slice(&texture.image_data),
							bytemuck::cast_slice_mut(&mut output),
							texture.gfx.width as usize,
							texture.gfx.height as usize,
						);
						let mut output2 =
							Vec::<u8>::with_capacity((texture.gfx.width * texture.gfx.height * 4) as usize);
						for pixel in bytemuck::cast_slice::<_, [u8; 2]>(&output) {
							output2.extend(&[
								pixel[0].wrapping_add(0x80),
								pixel[1].wrapping_add(0x80),
								0,
								255,
							])
						}
						let image =
							image::RgbaImage::from_vec(texture.gfx.width, texture.gfx.height, output2).unwrap();
						image.save(format!("target/{name}.png")).unwrap();
					}
					(ChannelFormat::BC1, TypeFormat::SRGB) => {
						let mut output = vec![0u8; (texture.gfx.width * texture.gfx.height * 3) as usize];
						bc1::decode_rgb(
							bytemuck::cast_slice(&texture.image_data),
							bytemuck::cast_slice_mut(&mut output),
							texture.gfx.width as usize,
							texture.gfx.height as usize,
						);
						let image = RgbImage::from_vec(texture.gfx.width, texture.gfx.height, output).unwrap();
						image.save(format!("target/{name}.png")).unwrap();
					}
					(ChannelFormat::BC3, TypeFormat::SRGB | TypeFormat::Unorm) => {
						let mut output = vec![0u8; (texture.gfx.width * texture.gfx.height * 4) as usize];
						bc3::decode_rgb(
							bytemuck::cast_slice(&texture.image_data),
							bytemuck::cast_slice_mut(&mut output),
							texture.gfx.width as usize,
							texture.gfx.height as usize,
						);
						let image = RgbaImage::from_vec(texture.gfx.width, texture.gfx.height, output).unwrap();
						image.save(format!("target/{name}.png")).unwrap();
					}
					comb => {
						println!("unsupported combination: {comb:?}")
					}
				}
				// let params = NewDxgiParams {
				// 	width: dims.x,
				// 	height: dims.y,
				// 	depth: None,
				// 	format: match texture.gfx.image_format {
				// 		(ChannelFormat::BC1, TypeFormat::SRGB) => DxgiFormat::BC1_UNorm_sRGB,
				// 		(ChannelFormat::BC4, TypeFormat::Unorm) => DxgiFormat::BC4_UNorm,
				// 		(ChannelFormat::BC5, TypeFormat::Snorm) => DxgiFormat::BC5_SNorm,
				// 		(channel_format, type_format) => todo!("{channel_format:?} {type_format:?}"),
				// 	},
				// 	mipmap_levels: Some(texture.gfx.mip_levels as u32),
				// 	array_layers: Some(texture.gfx.array_layers),
				// 	caps2: None,
				// 	is_cubemap: false,
				// 	resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
				// 	alpha_mode: ddsfile::AlphaMode::Unknown,
				// };
				// let mut dxgi = Dds::new_dxgi(params).unwrap();
				// dxgi.data = texture.image_data;
				// dxgi
				// 	.write(&mut File::create(format!("target/{name}.dds")).unwrap())
				// 	.unwrap();
				// let mut vec = Vec::new();
				// dxgi.write(&mut vec).unwrap();
				// let format = dxgi.get_dxgi_format().unwrap();
				// if format == DxgiFormat::BC1_UNorm_sRGB {
				// 	let body = image::load(&mut Cursor::new(vec), image::ImageFormat::Dds).unwrap();
				// 	body.save(format!("target/{name}.png")).unwrap();
				// } else if format {
				// 	println!("skipping {format:?}");
				// }
			}
		}
	}
	Ok(())
}
