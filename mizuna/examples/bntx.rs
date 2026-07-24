use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use image::{RgbImage, RgbaImage};
use mizuna::{
	nw::bntx::{self, ChannelFormat, TextureArchive, TypeFormat},
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
				println!(
					"{:?} {:?} {} {}",
					name, texture.gfx.image_format, texture.gfx.width, texture.gfx.height
				);
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
			}
		}
	}
	Ok(())
}
