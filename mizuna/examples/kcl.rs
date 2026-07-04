use std::{collections::BTreeMap, fs, io::Cursor, path::PathBuf, process::exit};

use clap::{Parser, Subcommand};
use gltf::{Semantic, accessor::DataType, mesh::Mode};
use gltf_json::{
	Accessor, Index, Mesh, Node, Scene,
	accessor::{ComponentType, GenericComponentType, Type},
	extras::Void,
	mesh::Primitive,
	validation::Checked,
};
use mizuna::kcl::{self, Kcl, ModelOctree, PrismOctree};
use snafu::ErrorCompat;

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Dump { in_path: PathBuf },
	Model { in_path: PathBuf, out_path: PathBuf },
}

#[snafu::report]
fn main() -> Result<(), kcl::Error> {
	let args = Args::parse();

	match args.command {
		Commands::Dump { in_path } => {
			println!("{in_path:?}");
			let data = fs::read(in_path)?;
			let Kcl {
				models,
				octree,
				bounds_min,
				bounds_max,
			} = kcl::Kcl::read(&data)?;
			traverse_model(&octree, 0);
			for model in models {
				println!("model");
				// model.
				// for octree in model.octree {
				// println!("  tree");
				// traverse_prisms(&octree, 2);
				// }
			}
		}
		Commands::Model { in_path, out_path } => {
			let data = fs::read(in_path)?;
			let kcl = kcl::Kcl::read(&data)?;

			let mut root = gltf_json::Root::default();
			let mesh = root.push(Mesh {
				name: Some("Octree".to_owned()),
				primitives: [Primitive {
					mode: Checked::Valid(Mode::Lines),
					attributes: {
						let mut map: BTreeMap<Checked<Semantic>, Index<Accessor>> = BTreeMap::new();
						map.insert(
							Checked::Valid(Semantic::Positions),
							root.push(Accessor {
								buffer_view: todo!(),
								byte_offset: None,
								count: (0u64).into(),
								component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
								extensions: None,
								extras: Void::default(),
								type_: Checked::Valid(Type::Vec3),
								min: None,
								max: None,
								name: Some("Prim".to_owned()),
								normalized: false,
								sparse: None,
							}),
						);
						map
					},
					extensions: None,
					extras: Void::default(),
					indices: None,
					material: None,
					targets: None,
				}]
				.to_vec(),
				extensions: None,
				extras: Void::default(),
				weights: None,
			});
			let node = root.push(Node {
				mesh: Some(mesh),
				..Default::default()
			});
			root.push(Scene {
				nodes: [node].to_vec(),
				extensions: None,
				extras: Void::default(),
				name: None,
			});

			gltf_json::serialize::to_string_pretty(&root).unwrap();
		}
	}

	Ok(())
}

fn traverse_model(octree: &ModelOctree, depth: usize) {
	for _ in 0..depth {
		print!("  ");
	}
	match octree {
		ModelOctree::Tree(subtree) => {
			println!("subtree:");
			for node in subtree.iter() {
				traverse_model(node, depth + 1);
			}
		}
		ModelOctree::ModelRef(model) => {
			println!("model: {model}");
		}
		ModelOctree::Empty => {
			println!("nothing")
		}
	}
}

fn traverse_prisms(octree: &PrismOctree, depth: usize) {
	for _ in 0..depth {
		print!("  ");
	}
	match octree {
		PrismOctree::Tree(tree) => {
			println!("tree");
			for node in tree.iter() {
				traverse_prisms(node, depth + 1);
			}
		}
		PrismOctree::Leaf { some_u16, prisms } => {
			println!("leaf {some_u16} {}", prisms.len());
		}
	}
}
