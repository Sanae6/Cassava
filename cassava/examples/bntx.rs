use std::{fs, path::PathBuf};

use cassava::{
	nw::bntx::{self, ChannelFormat, FormatInfo, TextureArchive, TypeFormat},
	texture::{astc, bc1, bc3, bc4, bc5},
	util::{BinaryReader, ByteOrder},
};
use clap::{Parser, Subcommand};
use image::{ImageBuffer, RgbImage, Rgba, RgbaImage};

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	List {
		in_path: PathBuf,
	},
	Dump {
		in_path: PathBuf,
		#[arg(short = 't')]
		texture_name: Option<String>,
	},
}

#[snafu::report]
fn main() -> Result<(), bntx::Error> {
	tracing_subscriber::fmt().init();
	let args = Args::parse();
	match args.command {
		Commands::List { in_path } => {
			let file = fs::read(in_path).unwrap();
			let texture_archive = TextureArchive::read(&mut BinaryReader::new(&file)).unwrap();
			for (name, texture) in texture_archive.textures {
				println!(
					"{name} {}x{} {:?}",
					texture.gfx.width,
					texture.gfx.height,
					texture.gfx.image_format(),
				)
			}
		}
		Commands::Dump {
			in_path,
			texture_name,
		} => {
			let file = fs::read(in_path).unwrap();
			let mut texture_archive = TextureArchive::read(&mut BinaryReader::new(&file)).unwrap();
			if let Some(filter_name) = texture_name {
				texture_archive.textures.retain(|k, _| k == &filter_name);
				if texture_archive.textures.len() == 0 {
					println!("no texture named {filter_name}");
				}
			}
			for (name, texture) in texture_archive.textures {
				println!(
					"{:?} {:?} {} {}",
					name,
					texture.gfx.image_format(),
					texture.gfx.width,
					texture.gfx.height
				);
				match texture.gfx.image_format() {
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
					(format, TypeFormat::SRGB) if format.is_astc() => {
						let mut output = vec![0u16; (texture.gfx.width * texture.gfx.height * 4) as usize];
						let format_info = FormatInfo::from_image_format(format);
						astc::decode_srgb(
							bytemuck::cast_slice(&texture.image_data),
							bytemuck::cast_slice_mut(&mut output),
							format_info,
							texture.gfx.width as usize,
							texture.gfx.height as usize,
						);
						let image = ImageBuffer::<Rgba<u16>, Vec<u16>>::from_vec(
							texture.gfx.width,
							texture.gfx.height,
							output,
						)
						.unwrap();
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
