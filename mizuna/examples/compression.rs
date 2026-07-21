use std::path::PathBuf;

use clap::{Parser, Subcommand};
use image::{GrayAlphaImage, GrayImage, Pixel};
use mizuna::texture::bc4;

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Bc4Unorm { in_path: PathBuf, out_path: PathBuf },
}

fn main() {
	let args = Args::parse();
	match args.command {
		Commands::Bc4Unorm { in_path, out_path } => {
			let mut gray: GrayImage = image::open(in_path).unwrap().to_luma8();
			let mut output = vec![0u8; ((gray.width() / 4) * (gray.height() / 4) * 8) as usize];

			bc4::compress::unorm(
				gray.as_raw().as_slice(),
				bytemuck::cast_slice_mut(&mut output),
				gray.width() as _,
				gray.height() as _,
			);
			let (width, height) = (gray.width() as _, gray.height() as _);
			bc4::decode_unorm(
				bytemuck::cast_slice_mut(&mut output),
				&mut gray,
				width,
				height,
			);
      gray.save(out_path).unwrap();
		}
	}
}
