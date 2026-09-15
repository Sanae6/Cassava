use std::{collections::HashMap, fs, ops::Range, path::Path, sync::Weak};

use cassava::{
	nw::bfres::{self, ModelArchive, VertexAttribFormat, VertexBuffer},
	sarc,
	util::{BinaryReader, align_up},
	yaz0,
};
use cassava_project::stage::Transform;
use glam::Vec3;
use half::f16;
use itertools::{Itertools, TupleWindows};
use wgpu::{
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, BufferUsages, Device,
	IndexFormat, PipelineCache, PipelineLayoutDescriptor, Queue, RenderPass,
	RenderPipelineDescriptor, VertexState,
	util::{BufferInitDescriptor, DeviceExt},
	wgt::BufferDescriptor,
};
use zerocopy::{FromBytes, IntoBytes, little_endian};

pub struct Object {
	bfres: Option<bfres::ModelArchive>,
	shapes: Vec<RenderShape>,
	transform: Transform,
	transform_bind_group: BindGroup,
}

struct RenderShape {
	attributes: HashMap<String, Buffer>,
	meshes: Vec<RenderMesh>,
}

struct RenderMesh {
	index_buffer: Buffer,
	index_count: u32,
	base_vertex: u32,
	index_format: IndexFormat,
}

impl Object {
	pub fn new(
		object_path: &Path,
		device: &Device,
		queue: &Queue,
		bind_group_layout: &BindGroupLayout,
		transform: Transform,
	) -> Object {
		let buffer = device.create_buffer_init(&BufferInitDescriptor {
			label: None,
			contents: transform.into_mat4().as_bytes(),
			usage: BufferUsages::UNIFORM,
		});
		let transform_bind_group = device.create_bind_group(&BindGroupDescriptor {
			label: None,
			layout: bind_group_layout,
			entries: &[BindGroupEntry {
				binding: 0,
				resource: buffer.as_entire_binding(),
			}],
		});
		let stem = object_path
			.file_stem()
			.expect("no file stem?")
			.to_str()
			.unwrap();
		let Ok(object_data) = fs::read(object_path) else {
			return Object {
				bfres: None,
				shapes: Vec::new(),
				transform,
				transform_bind_group,
			};
		};
		let data = yaz0::decompress(&object_data).unwrap();
		let sarc = sarc::Reader::new(data.0).unwrap();
		let Some(bfres) = sarc
			.files()
			.find(|(name, _)| name.starts_with(stem) && name.ends_with(".bfres"))
			.map(|(_, data)| data)
		else {
			return Object {
				bfres: None,
				shapes: Vec::new(),
				transform,
				transform_bind_group,
			};
		};

		println!("loading model {stem}");
		let model = ModelArchive::read(&mut BinaryReader::new(bfres)).unwrap();
		let mut shapes = Vec::new();
		for (model_name, model) in &model.models {
			for (shape_name, shape) in &model.shapes {
				let mut attributes = HashMap::new();
				let vertex_count = shape.vertex_info.vertex_count;
				for (attrib_name, attrib) in &shape.vertex_info.attributes {
					let buffer: &VertexBuffer = &shape.vertex_info.buffers[attrib.buffer_index as usize];
					let vertex_starts = (0..vertex_count as usize)
						.map(|index| index * buffer.stride as usize + attrib.buffer_offset as usize);

					if (buffer.stride as usize) < attrib.format.element_size() {
						todo!("handle invalid buffers properly")
					}
					type LEF32 = little_endian::F32;
					let buffer = if attrib_name == "_p0" {
						let vertices = match attrib.format {
							VertexAttribFormat::Vec3F32 => vertex_starts
								.map(|start| {
									Vec3::from_array(
										<[LEF32; 3]>::read_from_bytes(&buffer.data[start..start + 12])
											.unwrap()
											.map(|val| val.get()),
									)
								})
								.collect_vec(),
							VertexAttribFormat::Vec4F16 => vertex_starts
								.map(|start| {
									Vec3::from_array(
										<[[u8; 2]; 3]>::read_from_bytes(&buffer.data[start..start + 6])
											.unwrap()
											.map(|array| f16::from_le_bytes(array).to_f32()),
									)
								})
								.collect_vec(),
							format => panic!("unknown format for positions {format:?}"),
						};
						device.create_buffer_init(&BufferInitDescriptor {
							label: None,
							contents: vertices.as_bytes(),
							usage: BufferUsages::VERTEX,
						})
					} else if attrib_name == "_n0" {
						let vertices = match attrib.format {
							VertexAttribFormat::Vec3F32 => vertex_starts
								.map(|start| {
									Vec3::from_array(
										<[LEF32; 3]>::read_from_bytes(&buffer.data[start..start + 12])
											.unwrap()
											.map(|val| val.get()),
									)
								})
								.collect_vec(),
							VertexAttribFormat::Vec4Snorm10x3Unorm2 => vertex_starts
								.map(|start| {
									let value = i32::from_le_bytes(buffer.data[start..start + 4].try_into().unwrap());
									let vec = Vec3::new(
										// sign extend each value
										((value << 22) >> 22) as f32 / 511.0,
										((value << 12) >> 22) as f32 / 511.0,
										((value << 2) >> 22) as f32 / 511.0,
									);
									vec.normalize_or_zero()
								})
								.collect_vec(),
							format => panic!("unknown format for normals {format:?}"),
						};
						device.create_buffer_init(&BufferInitDescriptor {
							label: None,
							contents: vertices.as_bytes(),
							usage: BufferUsages::VERTEX,
						})
					} else {
						continue;
					};
					attributes.insert(attrib_name.clone(), buffer);
				}

				let mut meshes = Vec::new();
				for mesh in &shape.meshes {
					let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
						label: Some(&format!("{model_name} {shape_name} index buffer")),
						contents: &mesh.index_buffer.data,
						usage: BufferUsages::VERTEX | BufferUsages::INDEX,
					});

					let bfres::PrimitiveTopology::Triangles = mesh.primitive_topology else {
						panic!("unexpected topology");
					};

					meshes.push(RenderMesh {
						index_buffer,
						index_count: mesh.index_buffer.element_count() as _,
						base_vertex: mesh.base_vertex,
						index_format: match mesh.index_buffer.format {
							bfres::IndexFormat::U8 => panic!(),
							bfres::IndexFormat::U16 => IndexFormat::Uint16,
							bfres::IndexFormat::U32 => IndexFormat::Uint32,
						},
					});
				}
				shapes.push(RenderShape { attributes, meshes })
			}
		}

		Object {
			bfres: Some(model),
			shapes,
			transform,
			transform_bind_group,
		}
	}

	pub fn draw(&self, pass: &mut RenderPass) {
		pass.set_bind_group(1, &self.transform_bind_group, &[]);
		for shape in &self.shapes {
			pass.set_vertex_buffer(0, shape.attributes.get("_p0").unwrap().slice(..));
			pass.set_vertex_buffer(1, shape.attributes.get("_n0").unwrap().slice(..));
			for mesh in &shape.meshes {
				pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
				pass.draw_indexed(0..mesh.index_count, mesh.base_vertex as _, 0..1)
			}
		}
	}
}
