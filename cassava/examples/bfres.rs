use bytemuck::cast_slice;
use cassava::{
	nw::{
		bfres::{self, ModelArchive, VertexAttribFormat},
		bntx::{ChannelFormat, TextureArchive, TextureInfo, TypeFormat},
	},
	texture::{bc1, bc3, bc4, bc5},
	util::{BinaryReader, ByteOrder},
};
use glam::Vec3;
use gltf::Semantic;
use gltf_json::{
	Asset,
	accessor::{self, ComponentType, Type},
	extras::Void,
	validation::Checked,
};
use image::{RgbImage, RgbaImage};
use serde_json::Number;
use std::{
	collections::{BTreeMap, HashMap},
	fs,
	path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Dump { in_path: PathBuf, out_dir: PathBuf },
}

#[snafu::report]
fn main() -> Result<(), bfres::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path, out_dir } => {
			let file = fs::read(in_path).unwrap();
			let model_archive = ModelArchive::read(&mut BinaryReader::new(&file, ByteOrder::Little))?;

			fs::create_dir_all(&out_dir).unwrap();

			let textures = if let Some(texture) = model_archive.embedded_files.get("textures.bntx") {
				fs::write(out_dir.join("textures.bntx"), texture).unwrap();

				let texture =
					TextureArchive::read(&mut BinaryReader::new(texture, ByteOrder::Little)).unwrap();
				texture
					.textures
					.iter()
					.map(|(name, texture)| (name.clone(), save_texture(&out_dir, name, texture)))
					.collect::<HashMap<_, _>>()
			} else {
				HashMap::new()
			};

			let mut root = gltf_json::Root {
				asset: Asset {
					generator: Some("Cassava".to_string()),
					..Default::default()
				},
				..Default::default()
			};
			let models = model_archive
				.models
				.iter()
				.map(|(model_name, model)| {
					let nodes = model
						.shapes
						.iter()
						.map(|(shape_name, shape)| {
							let attributes = shape
								.vertex_info
								.attributes
								.iter()
								.map(|(attrib_name, attribute)| {
									let semantic_name = match attrib_name.as_str() {
										"_p0" => Semantic::Positions,
										"_n0" => Semantic::Normals,
										"_t0" => Semantic::Tangents,
										"_u0" => Semantic::TexCoords(0),
										"_u1" => Semantic::TexCoords(1),
										"_u2" => Semantic::TexCoords(2),
										"_c0" => Semantic::Colors(0),
										"_i0" => Semantic::Joints(0),
										"_w0" => Semantic::Weights(0),
										_ => unimplemented!("{attrib_name}"),
									};

									let buffer = &shape.vertex_info.buffers[attribute.buffer_index as usize];

									let (min, max) = if let Semantic::Positions = semantic_name {
										let positions: &[Vec3] = cast_slice(&buffer.data);
										let mut min = Vec3::MAX;
										let mut max = Vec3::MIN;
										positions.iter().for_each(|value| {
											min = min.min(*value);
											max = max.max(*value);
										});
										let convert = |vec: Vec3| {
											gltf_json::Value::Array(
												vec
													.to_array()
													.map(|value| {
														gltf_json::Value::Number(Number::from_f64(value as _).unwrap())
													})
													.to_vec(),
											)
										};
										let min = convert(min);
										let max = convert(max);
										(Some(min), Some(max))
									} else {
										(None, None)
									};

									let (vertices, normalized, stride, component_type, type_) =
										read_vertices_for_gltf(&buffer.data, attribute.format, attrib_name);

									let _ = fs::create_dir(&out_dir.join("buffers/"));
									let path = format!("{model_name}_{shape_name}_vtx{attrib_name}");
									fs::write(out_dir.join("buffers").join(&path), &vertices).unwrap();

									let buffer = root.push(gltf_json::Buffer {
										byte_length: vertices.len().into(),
										name: None,
										uri: Some(format!("buffers/{path}")),
										extensions: None,
										extras: Void::default(),
									});

									let view = root.push(gltf_json::buffer::View {
										buffer,
										byte_length: vertices.len().into(),
										byte_offset: Some(0u64.into()),
										byte_stride: None,
										name: None,
										target: Some(Checked::Valid(gltf_json::buffer::Target::ArrayBuffer)),
										extensions: None,
										extras: Void::default(),
									});

									let buffer = root.push(gltf_json::Accessor {
										buffer_view: Some(view),
										byte_offset: Some((attribute.buffer_offset as u64).into()),
										count: (vertices.len() / stride as usize).into(),
										component_type: Checked::Valid(accessor::GenericComponentType(component_type)),
										extensions: None,
										extras: Void::default(),
										type_: Checked::Valid(type_),
										min,
										max,
										name: Some(attrib_name.clone()),
										normalized,
										sparse: None,
									});

									(Checked::Valid(semantic_name), buffer)
								})
								.collect::<BTreeMap<_, _>>();

							let primitives = shape
								.meshes
								.iter()
								.enumerate()
								.map(|(mesh_index, mesh)| {
									let path = format!("{model_name}_{shape_name}_mesh_idx{mesh_index}");

									// let indices: &[u16] = cast_slice(&mesh.index_buffer.data);
									// println!(
									// 	"{} {} {}",
									// 	indices.iter().min().unwrap(),
									// 	indices.iter().max().unwrap(),
									// 	mesh.index_buffer.data.len(),
									// );
									fs::write(out_dir.join("buffers").join(&path), &mesh.index_buffer.data).unwrap();
									let buffer = root.push(gltf_json::Buffer {
										byte_length: mesh.index_buffer.data.len().into(),
										name: None,
										uri: Some(format!("buffers/{path}")),
										extensions: None,
										extras: Void::default(),
									});

									let view = root.push(gltf_json::buffer::View {
										buffer,
										byte_length: mesh.index_buffer.data.len().into(),
										byte_offset: None,
										byte_stride: None,
										name: None,
										target: Some(Checked::Valid(
											gltf_json::buffer::Target::ElementArrayBuffer,
										)),
										extensions: None,
										extras: Void::default(),
									});

									let index_buffer = root.push(gltf_json::Accessor {
										buffer_view: Some(view),
										byte_offset: None,
										count: mesh.index_buffer.element_count().into(),
										component_type: Checked::Valid(accessor::GenericComponentType(
											match mesh.index_buffer.format {
												bfres::IndexFormat::U8 => ComponentType::U8,
												bfres::IndexFormat::U16 => ComponentType::U16,
												bfres::IndexFormat::U32 => ComponentType::U32,
											},
										)),
										extensions: None,
										extras: Void::default(),
										type_: Checked::Valid(Type::Scalar),
										min: None,
										max: None,
										name: None,
										normalized: false,
										sparse: None,
									});

									gltf_json::mesh::Primitive {
										attributes: attributes.clone(),
										extensions: Default::default(),
										extras: Void::default(),
										indices: Some(index_buffer),
										material: None,
										mode: Checked::Valid(match mesh.primitive_topology {
											bfres::PrimitiveTopology::Points => gltf_json::mesh::Mode::Points,
											bfres::PrimitiveTopology::Lines => gltf_json::mesh::Mode::Lines,
											bfres::PrimitiveTopology::LineStrip => gltf_json::mesh::Mode::LineStrip,
											bfres::PrimitiveTopology::Triangles => gltf_json::mesh::Mode::Triangles,
											bfres::PrimitiveTopology::TriangleStrip => {
												gltf_json::mesh::Mode::TriangleStrip
											}
											// bfres::PrimitiveTopology::LinesAdjacency => gltf_json::mesh::Mode::LineLoop,
											// bfres::PrimitiveTopology::LineStripAdjacency => gltf_json::mesh::Mode::Lines,
											// bfres::PrimitiveTopology::TrianglesAdjacency => gltf_json::mesh::Mode::Lines,
											// bfres::PrimitiveTopology::TriangleStripAdjacency => gltf_json::mesh::Mode::Lines,
											// bfres::PrimitiveTopology::Patches => gltf_json::mesh::Mode::,
											_ => unimplemented!(),
										}),
										targets: None,
									}
								})
								.collect::<Vec<_>>();

							let mesh = root.push(gltf_json::Mesh {
								extensions: None,
								extras: Void::default(),
								name: Some(shape_name.clone()),
								primitives: primitives,
								weights: None,
							});

							root.push(gltf_json::Node {
								name: Some(shape_name.clone()),
								mesh: Some(mesh),
								..Default::default()
							})
						})
						.collect::<Vec<_>>();
					root.push(gltf_json::Node {
						children: Some(nodes),
						..Default::default()
					})
				})
				.collect();
			root.push(gltf_json::Scene {
				name: Some(model_archive.file_name.clone()),
				nodes: models,
				extensions: Default::default(),
				extras: Void::default(),
			});

			fs::write(
				out_dir.join(format!("{}.gltf", model_archive.file_name)),
				gltf_json::serialize::to_vec(&root).unwrap(),
			)
			.unwrap();
			println!("{model_archive:#?}");
		}
	}
	Ok(())
}

fn read_vertices_for_gltf(
	vertices: &[u8],
	attrib: VertexAttribFormat,
	attrib_name: &str,
) -> (Vec<u8>, bool, usize, ComponentType, Type) {
	match attrib {
		VertexAttribFormat::Vec3F32 => (vertices.to_vec(), false, 12, ComponentType::F32, Type::Vec3),
		VertexAttribFormat::Vec2Unorm16 => (vertices.to_vec(), true, 4, ComponentType::U16, Type::Vec2),
		VertexAttribFormat::Vec4Snorm8 => {
			let vertices: &[i8] = cast_slice(&vertices);
			let mut out_vertices = Vec::with_capacity(vertices.len());
			out_vertices.extend(
				vertices
					.iter()
					.cloned()
					.map(|value| (value as f32 / 127.0).to_le_bytes())
					.flatten(),
			);

			(out_vertices, false, 16, ComponentType::F32, Type::Vec4)
		}
		VertexAttribFormat::Vec4Snorm10x3Unorm2 => {
			let vertices: &[[u8; 4]] = cast_slice(vertices);
			let mut out_vertices = Vec::with_capacity(vertices.len());
			out_vertices.extend(
				vertices
					.iter()
					.cloned()
					.map(|value| {
						let value = i32::from_le_bytes(value);
						let bytes: [u8; 12] = bytemuck::cast({
							let vec = Vec3::new(
								// sign extend each value
								((value << 22) >> 22) as f32 / 511.0,
								((value << 12) >> 22) as f32 / 511.0,
								((value << 2) >> 22) as f32 / 511.0,
							);
							vec.normalize().to_array().map(f32::to_le_bytes)
						});
						bytes
					})
					.flatten(),
			);

			(
				bytemuck::cast_vec(out_vertices),
				false,
				12,
				ComponentType::F32,
				Type::Vec3,
			)
		}
		VertexAttribFormat::Vec4Unorm8 => (vertices.to_vec(), true, 4, ComponentType::U8, Type::Vec4),
		_ => todo!("{attrib:?} for {attrib_name}"),
	}
}

fn save_texture(out_dir: &Path, name: &str, texture: &TextureInfo) -> PathBuf {
	fs::create_dir_all(out_dir.join("textures")).unwrap();
	println!(
		"{:?} {:?} {} {}",
		name, texture.gfx.image_format(), texture.gfx.width, texture.gfx.height
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
			let path = out_dir.join(format!("textures/{name}.png"));
			image.save(&path).unwrap();
			path
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
			let path = out_dir.join(format!("textures/{name}.png"));
			image.save(&path).unwrap();
			path
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
			let path = out_dir.join(format!("textures/{name}.png"));
			image.save(&path).unwrap();
			path
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
			let path = out_dir.join(format!("textures/{name}.png"));
			image.save(&path).unwrap();
			path
		}
		comb => {
			panic!("unsupported combination: {comb:?}")
		}
	}
}
