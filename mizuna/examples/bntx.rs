use std::{
	fs::{self, File},
	io::Cursor,
	path::PathBuf,
};

use clap::{Parser, Subcommand};
use ddsfile::{Dds, DxgiFormat, NewDxgiParams};
use image::codecs::dds::DdsDecoder;
use mizuna::{
	bntx::{self, ChannelFormat, TextureArchive, TypeFormat},
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
				let dims = texture.dimensions();
				let params = NewDxgiParams {
					width: dims.x,
					height: dims.y,
					depth: None,
					format: match texture.gfx.image_format {
						(ChannelFormat::BC1, TypeFormat::SRGB) => DxgiFormat::BC1_UNorm_sRGB,
						(ChannelFormat::BC4, TypeFormat::Unorm) => DxgiFormat::BC4_UNorm,
						(ChannelFormat::BC5, TypeFormat::Snorm) => DxgiFormat::BC5_SNorm,
						(channel_format, type_format) => todo!("{channel_format:?} {type_format:?}"),
					},
					mipmap_levels: Some(texture.gfx.mip_levels as u32),
					array_layers: Some(texture.gfx.array_layers),
					caps2: None,
					is_cubemap: false,
					resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
					alpha_mode: ddsfile::AlphaMode::Unknown,
				};
				let mut dxgi = Dds::new_dxgi(params).unwrap();
				dxgi.data = texture.image_data;
				dxgi
					.write(&mut File::create(format!("target/{name}.dds")).unwrap())
					.unwrap();
				let mut vec = Vec::new();
				dxgi.write(&mut vec).unwrap();
				let format = dxgi.get_dxgi_format().unwrap();
				if format == DxgiFormat::BC1_UNorm_sRGB {
					let body = image::load(&mut Cursor::new(vec), image::ImageFormat::Dds).unwrap();
					body.save(format!("target/{name}.png")).unwrap();
				} else {
					println!("skipping {format:?}");
				}
			}
			// let texture = texture_archive.textures;
		}
	}
	Ok(())
}
