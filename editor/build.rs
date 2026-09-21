#[path = "./src/workspace/stage_editor/shaders/options.rs"]
mod options;
use std::{env, fs::File, io::BufWriter, path::Path};

use itertools::Itertools;
use wesl::{CompileOptions, Features};
use zerocopy::IntoBytes;

use crate::options::ShaderOptions;

fn main() {
	let mut map = phf_codegen::Map::new();
	const BINARY: [bool; 2] = [false, true];
	for (textured, vertex_colours) in BINARY.into_iter().cartesian_product(BINARY.into_iter()) {
		let mut features = Features::new();
		features.set("textured", textured);
		features.set("vertex_colours", vertex_colours);
		let output = wesl::Compiler::new(CompileOptions {
			features: features,
			..CompileOptions::default()
		})
		.compile("../wesl.toml")
		.inspect_err(|e| eprintln!("{e}")) // pretty-print errors
		.unwrap();

		let options = ShaderOptions {
			textured: { textured },
			vertex_colours: { vertex_colours },
		};
		map.entry(
			options,
			format!(
				"wgpu::ShaderModuleDescriptor {{
					label: Some(\"{options:?}\"),
					source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(\"{output}\")),
				}}"
			),
		);
	}

	use std::io::Write;
	let path = Path::new(&env::var("OUT_DIR").unwrap()).join("shader_options.rs");
	let mut file = BufWriter::new(File::create(&path).unwrap());
	write!(&mut file, "{}", map.build()).unwrap();
}
