use std::{collections::HashMap, fs, ops::Range, path::Path, sync::Weak};

use cassava::{
	byaml::Byml,
	nw::{
		bfres::{self, Material, ModelArchive, VertexAttribFormat, VertexBuffer, WrapMode},
		bntx::{ChannelFormat, FormatInfo, TextureArchive, TypeFormat},
	},
	sarc,
	util::{BinaryReader, align_up},
	yaz0,
};
use cassava_project::{
	smol_str::SmolStr,
	stage::{Transform, database},
};
use glam::Vec3;
use half::f16;
use itertools::{Itertools, TupleWindows};
use wgpu::{
	AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
	BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer, BufferUsages, ColorTargetState,
	ColorWrites, DepthBiasState, DepthStencilState, Device, Extent3d, FragmentState, IndexFormat,
	MultisampleState, Origin3d, PipelineCache, PipelineCompilationOptions, PipelineLayoutDescriptor,
	PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, SamplerDescriptor,
	ShaderStages, StencilState, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
	TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
	VertexStepMode,
	util::{BufferInitDescriptor, DeviceExt},
	wgt::{BufferDescriptor, TextureDescriptor, TextureViewDescriptor},
};
use zerocopy::{FromBytes, IntoBytes, little_endian};

use crate::workspace::stage_editor::{
	shaders::{ShaderOptions, get_shader_variant},
	view::PipelineContext,
};

pub struct Object {
	// todo: store skeletal information to play first frames of Wait animations
	shapes: Vec<RenderShape>,
	transform: Transform,
}

struct RenderShape {
	attributes: HashMap<AttributeKey, Buffer>,
	bind_group: BindGroup,
	pipeline: RenderPipeline,
	meshes: Vec<RenderMesh>,
	shader_options: ShaderOptions,
}

struct RenderMesh {
	index_buffer: Buffer,
	index_count: u32,
	base_vertex: u32,
	index_format: IndexFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AttributeKey {
	Position,
	Normal,
	Colour,
	Uv0,
	Uv1,
	Uv2,
	Tangents,
	Joints0,
	Weights0,
}

impl From<&str> for AttributeKey {
	fn from(value: &str) -> Self {
		match value.into() {
			"_p0" => AttributeKey::Position,
			"_n0" => AttributeKey::Normal,
			"_c0" => AttributeKey::Colour,
			"_u0" => AttributeKey::Uv0,
			"_u1" => AttributeKey::Uv1,
			"_u2" => AttributeKey::Uv2,
			"_t0" => AttributeKey::Tangents,
			"_i0" => AttributeKey::Joints0,
			"_w0" => AttributeKey::Weights0,
			attrib_name => panic!("unknown attribute type {attrib_name}"),
		}
	}
}

impl Object {
	fn shader_options(
		material: &Material,
		provided_attributes: &HashMap<AttributeKey, (u64, VertexFormat)>,
	) -> ShaderOptions {
		let shader_assign = material
			.shader_assign
			.as_ref()
			.expect("missing a shader assign on an object bfres?? how???");
		match shader_assign.shader_archive.as_str() {
			"alRenderMaterial" => {
				let textured = provided_attributes.contains_key(&AttributeKey::Uv0)
					&& material.samplers.contains_key("_a0");
				let vertex_colours = provided_attributes.contains_key(&AttributeKey::Colour);
				ShaderOptions {
					textured,
					vertex_colours,
				}
			}
			_ => panic!("unsupported material"), // maybe fall back to displaying in all purple
		}
	}

	fn bind_group_layout(device: &Device, shader_options: &ShaderOptions) -> BindGroupLayout {
		let mut entries = Vec::with_capacity(16);
		entries.push(BindGroupLayoutEntry {
			binding: 0,
			visibility: ShaderStages::VERTEX,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None,
			},
			count: None,
		});
		if shader_options.textured {
			entries.push(BindGroupLayoutEntry {
				binding: 1,
				visibility: ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Texture {
					sample_type: TextureSampleType::Float { filterable: true },
					view_dimension: TextureViewDimension::D2,
					multisampled: false,
				},
				count: None,
			});
			entries.push(BindGroupLayoutEntry {
				binding: 2,
				visibility: ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			})
		}
		device.create_bind_group_layout(&BindGroupLayoutDescriptor {
			label: Some("stage object bind group layout"),
			entries: &entries,
		})
	}

	fn pipeline(
		context: &PipelineContext,
		material: &Material,
		attributes: HashMap<AttributeKey, (u64, VertexFormat)>,
	) -> (RenderPipeline, BindGroupLayout, ShaderOptions) {
		let shader_options = Self::shader_options(material, &attributes);
		let wgsl = context
			.device
			.create_shader_module(get_shader_variant(&shader_options));
		let bind_group_layout = Self::bind_group_layout(&context.device, &shader_options);
		let pipeline_layout = context
			.device
			.create_pipeline_layout(&PipelineLayoutDescriptor {
				label: Some("object pipeline layout"),
				bind_group_layouts: &[Some(context.proj_bind_layout), Some(&bind_group_layout)],
				immediate_size: 0,
			});
		let textured = shader_options.textured.then(|| {
			[VertexAttribute {
				format: attributes[&AttributeKey::Uv0].1.clone(),
				offset: 0,
				shader_location: 1,
			}]
		});
		let vertex_colours = shader_options.vertex_colours.then(|| {
			[VertexAttribute {
				format: attributes[&AttributeKey::Colour].1.clone(),
				offset: 0,
				shader_location: 2,
			}]
		});

		let pipeline = context
			.device
			.create_render_pipeline(&RenderPipelineDescriptor {
				label: Some("object pipeline"),
				layout: Some(&pipeline_layout),
				vertex: VertexState {
					module: &wgsl,
					entry_point: Some("vtx_main"),
					compilation_options: PipelineCompilationOptions::default(),
					buffers: &[
						Some(VertexBufferLayout {
							array_stride: attributes[&AttributeKey::Position].0,
							step_mode: VertexStepMode::Vertex,
							attributes: &[VertexAttribute {
								format: attributes[&AttributeKey::Position].1,
								offset: 0,
								shader_location: 0,
							}],
						}),
						textured.as_ref().map(|attrib| VertexBufferLayout {
							array_stride: attributes[&AttributeKey::Uv0].0,
							step_mode: VertexStepMode::Vertex,
							attributes: attrib,
						}),
						vertex_colours.as_ref().map(|attrib| VertexBufferLayout {
							array_stride: attributes[&AttributeKey::Colour].0,
							step_mode: VertexStepMode::Vertex,
							attributes: attrib,
						}),
						Some(VertexBufferLayout {
							array_stride: 12,
							step_mode: VertexStepMode::Vertex,
							attributes: &[VertexAttribute {
								format: VertexFormat::Float32x3,
								offset: 0,
								shader_location: 3,
							}],
						}),
					],
				},
				primitive: PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleList,
					strip_index_format: None,
					front_face: wgpu::FrontFace::Ccw,
					cull_mode: Some(wgpu::Face::Back),
					unclipped_depth: false,
					polygon_mode: wgpu::PolygonMode::Fill,
					conservative: false,
				},
				depth_stencil: Some(context.depth_stencil_state.clone()),
				multisample: MultisampleState::default(),
				fragment: Some(FragmentState {
					module: &wgsl,
					entry_point: Some("frag_main"),
					compilation_options: PipelineCompilationOptions::default(),
					targets: &[Some(context.colour_target_state.clone())],
				}),
				multiview_mask: None,
				cache: context.pipeline_cache,
			});

		(pipeline, bind_group_layout, shader_options)
	}

	pub fn load_model_archive(
		object_data: &Path,
		object_name: &str,
	) -> Option<(ModelArchive, Option<TextureArchive>, Option<SmolStr>)> {
		let object_path = object_data.join(format!("{object_name}.szs"));
		let data = fs::read(object_path).ok()?;
		let data = yaz0::decompress(&data).unwrap();
		let sarc = sarc::Reader::new(data.0).unwrap();
		let bfres = sarc
			.files()
			.find(|(name, _)| name.starts_with(object_name) && name.ends_with(".bfres"))
			.map(|(_, data)| data)?;

		println!("loading model {object_name}");
		let model = ModelArchive::read(&mut BinaryReader::new(bfres)).ok()?;
		let external_texture_archive = if let Some(data) = sarc.get_file("InitModel.byml") {
			let byml = Byml::read(&mut BinaryReader::new(data)).unwrap();
			byml.as_element().get_key_string("TextureArc")
		} else {
			None
		};
		let texture_archive = model
			.embedded_files
			.get("textures.bntx")
			.map(|bntx| TextureArchive::read(&mut BinaryReader::new(bntx)).unwrap());

		Some((model, texture_archive, external_texture_archive))
	}

	pub fn load_external_texture_archive(object_data: &Path, name: &str) -> Option<TextureArchive> {
		let texture_path = object_data.join(format!("{name}.szs"));
		let data = fs::read(texture_path).ok()?;
		let data = yaz0::decompress(&data).unwrap();
		let sarc = sarc::Reader::new(data.0).unwrap();
		println!("{:?}", sarc.filenames().collect::<Vec<_>>());
		let bfres = sarc
			.files()
			.find(|(name, _)| name.starts_with(name) && name.ends_with(".bfres"))
			.map(|(_, data)| data)?;
		println!("awesome");
		let model = ModelArchive::read(&mut BinaryReader::new(bfres)).unwrap();
		Some(
			TextureArchive::read(&mut BinaryReader::new(
				&model.embedded_files["textures.bntx"],
			))
			.unwrap(),
		)
	}

	pub fn new(
		model: &ModelArchive,
		context: &PipelineContext,
		transform: Transform,
		texture_archive: Option<&TextureArchive>,
		external_texture_archive: Option<&TextureArchive>,
	) -> Object {
		let transform_buffer = context.device.create_buffer_init(&BufferInitDescriptor {
			label: None,
			contents: transform.into_mat4().as_bytes(),
			usage: BufferUsages::UNIFORM,
		});
		let mut shapes = Vec::new();
		for (model_name, model) in &model.models {
			for (shape_name, shape) in &model.shapes {
				if shape_name.contains("Caustic") {
					// temporarily removes visual suffering in CapWorldTowerStage
					// todo: transparency is an issue that needs to be resolved
					continue;
				}
				let mut attributes = HashMap::new();
				let mut attribute_formats = HashMap::new();
				let vertex_count = shape.vertex_info.vertex_count;
				for (attrib_name, attrib) in &shape.vertex_info.attributes {
					let buffer: &VertexBuffer = &shape.vertex_info.buffers[attrib.buffer_index as usize];
					let vertex_starts = (0..vertex_count as usize)
						.map(|index| index * buffer.stride as usize + attrib.buffer_offset as usize);

					if (buffer.stride as usize) < attrib.format.element_size() {
						panic!("invalid buffer")
					}
					let key = AttributeKey::from(attrib_name.as_str());
					let (buffer, stride, format) = match key {
						AttributeKey::Position => {
							let (vertices, stride, format) = match attrib.format {
								VertexAttribFormat::Vec3F32 => (
									vertex_starts
										.map(|start| buffer.data[start..start + 12].into_iter().cloned())
										.flatten()
										.collect_vec(),
									12,
									VertexFormat::Float32x3,
								),
								VertexAttribFormat::Vec4F16 => (
									vertex_starts
										.map(|start| buffer.data[start..start + 8].into_iter().cloned())
										.flatten()
										.collect_vec(),
									8,
									VertexFormat::Float16x4,
								),
								format => panic!("unknown format for positions {format:?}"),
							};
							(
								context.device.create_buffer_init(&BufferInitDescriptor {
									label: None,
									contents: vertices.as_bytes(),
									usage: BufferUsages::VERTEX,
								}),
								stride,
								format,
							)
						}
						AttributeKey::Normal => {
							let vertices = match attrib.format {
								VertexAttribFormat::Vec3F32 => vertex_starts
									.map(|start| buffer.data[start..start + 12].into_iter().cloned())
									.flatten()
									.collect_vec(),
								VertexAttribFormat::Vec4Snorm10x3Unorm2 => vertex_starts
									.map(|start| {
										let value =
											i32::from_le_bytes(buffer.data[start..start + 4].try_into().unwrap());
										let vec = Vec3::new(
											// sign extend each value
											((value << 22) >> 22) as f32 / 511.0,
											((value << 12) >> 22) as f32 / 511.0,
											((value << 2) >> 22) as f32 / 511.0,
										);
										vec.normalize_or_zero().to_array().map(|v| v.to_le_bytes())
									})
									.flatten()
									.flatten()
									.collect_vec(),
								format => panic!("unknown format for normals {format:?}"),
							};
							(
								context.device.create_buffer_init(&BufferInitDescriptor {
									label: None,
									contents: vertices.as_bytes(),
									usage: BufferUsages::VERTEX,
								}),
								12,
								VertexFormat::Float32x3,
							)
						}
						AttributeKey::Colour => {
							let (vertices, stride, format) = match attrib.format {
								VertexAttribFormat::Vec4Unorm8 => (
									vertex_starts
										.map(|start| buffer.data[start..start + 4].into_iter().cloned())
										.flatten()
										.collect_vec(),
									4,
									VertexFormat::Unorm8x4,
								),
								VertexAttribFormat::Vec4F16 => (
									vertex_starts
										.map(|start| buffer.data[start..start + 8].into_iter().cloned())
										.flatten()
										.collect_vec(),
									8,
									VertexFormat::Float16x4,
								),

								format => panic!("unknown format for vertex colours {format:?}"),
							};

							(
								context.device.create_buffer_init(&BufferInitDescriptor {
									label: None,
									contents: vertices.as_bytes(),
									usage: BufferUsages::VERTEX,
								}),
								stride,
								format,
							)
						}
						AttributeKey::Uv0 => {
							let (vertices, stride, format) = match attrib.format {
								VertexAttribFormat::Vec2Unorm16 | VertexAttribFormat::Vec2Snorm16 => (
									vertex_starts
										.map(|start| buffer.data[start + 0..start + 4].into_iter().cloned())
										.flatten()
										.collect_vec(),
									4,
									match attrib.format {
										VertexAttribFormat::Vec2Unorm16 => VertexFormat::Unorm16x2,
										VertexAttribFormat::Vec2Snorm16 => VertexFormat::Snorm16x2,
										_ => unreachable!(),
									},
								),
								VertexAttribFormat::Vec2F16 => (
									vertex_starts
										.map(|start| {
											let data =
												<[[u8; 2]; 2]>::read_from_bytes(&buffer.data[start + 0..start + 4])
													.unwrap();
											data
										})
										.map(|array| {
											array
												.map(f16::from_le_bytes)
												.map(f16::to_f32)
												.map(f32::to_le_bytes)
										})
										.flatten()
										.flatten()
										.collect_vec(),
									8,
									VertexFormat::Float32x2,
								),
								VertexAttribFormat::Vec2Unorm8 => (
									vertex_starts
										.map(|start| [buffer.data[start], buffer.data[start + 1], 0, 0])
										.flatten()
										.collect_vec(),
									4, // stride of 4 because wgpu wants things "aligned to 4 bytes" or something
									VertexFormat::Unorm8x2,
								),
								VertexAttribFormat::Vec2F32 => (
									vertex_starts
										.map(|start| buffer.data[start..start + 8].into_iter().cloned())
										.flatten()
										.collect_vec(),
									8,
									VertexFormat::Float32x2,
								),

								format => panic!("unknown format for uvs {format:?}"),
							};

							(
								context.device.create_buffer_init(&BufferInitDescriptor {
									label: None,
									contents: vertices.as_bytes(),
									usage: BufferUsages::VERTEX,
								}),
								stride,
								format,
							)
						}
						_ => continue,
					};
					attributes.insert(key, buffer);
					attribute_formats.insert(key, (stride, format));
				}

				let mut meshes = Vec::new();
				for mesh in &shape.meshes {
					let index_buffer = context.device.create_buffer_init(&BufferInitDescriptor {
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

				let (pipeline, bind_group_layout, shader_options) =
					Self::pipeline(&context, &shape.material, attribute_formats);

				let mut entries = Vec::new();
				entries.push(BindGroupEntry {
					binding: 0,
					resource: transform_buffer.as_entire_binding(),
				});
				let mut view = None;
				let mut texture_wgpu = None;
				let mut sampler_wgpu = None;
				if shader_options.textured {
					let sampler = &shape.material.samplers["_a0"];

					let texture_name =
						&shape.material.texture_names[shape.material.samplers.get_index_of("_a0").unwrap()];
					let texture = texture_archive
						.and_then(|arc| arc.textures.get(texture_name.as_str()))
						.or_else(|| {
							external_texture_archive.and_then(|arc| arc.textures.get(texture_name.as_str()))
						})
						.unwrap_or_else(|| panic!("missing texture :( {texture_name}"));
					let texture_format = match texture.gfx.image_format() {
						(ChannelFormat::BC1, TypeFormat::SRGB) => TextureFormat::Bc1RgbaUnormSrgb,
						(ChannelFormat::BC3, TypeFormat::SRGB) => TextureFormat::Bc3RgbaUnormSrgb,
						(ChannelFormat::BC4, TypeFormat::Unorm) => TextureFormat::Bc4RUnorm,
						(ChannelFormat::R8G8B8A8, TypeFormat::SRGB) => TextureFormat::Rgba8UnormSrgb,
						(chan, ty) => unimplemented!("{chan:?} {ty:?}"),
					};
					let convert_wrap_mode = |wrap_mode: WrapMode| match wrap_mode {
						WrapMode::Repeat => AddressMode::Repeat,
						WrapMode::MirrorRepeat => AddressMode::MirrorRepeat,
						WrapMode::ClampToEdge => AddressMode::ClampToEdge,
						WrapMode::ClampToBorder => AddressMode::ClampToBorder,
						WrapMode::MirrorClampToBorder => panic!("oh, i sure hope nobody is using this!"),
					};
					let sampler = sampler_wgpu.insert(
						context.device.create_sampler(&SamplerDescriptor {
							label: None,
							address_mode_u: convert_wrap_mode(sampler.wrap_modes[0]),
							address_mode_v: convert_wrap_mode(sampler.wrap_modes[1]),
							address_mode_w: convert_wrap_mode(sampler.wrap_modes[2]),
							mag_filter: match sampler.expand_filter {
								bfres::FilterMode::Points => wgpu::FilterMode::Nearest,
								bfres::FilterMode::Linear => wgpu::FilterMode::Linear,
							},
							min_filter: match sampler.shrink_filter {
								bfres::FilterMode::Points => wgpu::FilterMode::Nearest,
								bfres::FilterMode::Linear => wgpu::FilterMode::Linear,
							},
							mipmap_filter: match sampler.mip_filter {
								bfres::MipFilterMode::None => wgpu::MipmapFilterMode::Linear, // what is this??
								bfres::MipFilterMode::Points => wgpu::MipmapFilterMode::Nearest,
								bfres::MipFilterMode::Linear => wgpu::MipmapFilterMode::Linear,
							},
							lod_min_clamp: sampler.min_lod,
							lod_max_clamp: sampler.max_lod,
							compare: (texture.image_dimension == 2).then(|| match sampler.compare_op {
								bfres::CompareOp::Never => wgpu::CompareFunction::Never,
								bfres::CompareOp::LessThan => wgpu::CompareFunction::Less,
								bfres::CompareOp::Equal => wgpu::CompareFunction::Equal,
								bfres::CompareOp::LessThanEqual => wgpu::CompareFunction::LessEqual,
								bfres::CompareOp::GreaterThan => wgpu::CompareFunction::Greater,
								bfres::CompareOp::NotEqual => wgpu::CompareFunction::NotEqual,
								bfres::CompareOp::GreaterThanEqual => wgpu::CompareFunction::GreaterEqual,
								bfres::CompareOp::Always => wgpu::CompareFunction::Always,
							}),
							anisotropy_clamp: sampler
								.anisotropic_level
								.try_into()
								.expect("anisotropic level greater than 256..."),
							border_color: match sampler.border_color {
								bfres::BorderColor::White => Some(wgpu::SamplerBorderColor::OpaqueWhite),
								bfres::BorderColor::TransparentBlack => {
									Some(wgpu::SamplerBorderColor::TransparentBlack)
								}
								bfres::BorderColor::Black => Some(wgpu::SamplerBorderColor::OpaqueBlack),
							},
						}),
					);
					entries.push(BindGroupEntry {
						binding: 2,
						resource: wgpu::BindingResource::Sampler(sampler),
					});
					let extents = Extent3d {
						width: texture.gfx.width,
						height: texture.gfx.height,
						depth_or_array_layers: texture.gfx.depth * texture.gfx.array_layers,
					};
					let dimension = match texture.image_dimension {
						0 => TextureDimension::D1,
						1 => TextureDimension::D2,
						2 => TextureDimension::D3,
						dim => panic!("texture {texture_name} has a bad image dimension {dim}"),
					};
					let texture_wgpu =
						texture_wgpu.insert(context.device.create_texture(&TextureDescriptor {
							label: None,
							size: extents,
							mip_level_count: texture.mip_offsets.len() as _,
							sample_count: texture.gfx.sample_count as _,
							dimension,
							format: texture_format,
							usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
							view_formats: &[texture_format],
						}));

					pub fn mip_level_size(extents: Extent3d, level: u32, dim: TextureDimension) -> Extent3d {
						Extent3d {
							width: u32::max(1, extents.width >> level),
							height: match dim {
								TextureDimension::D1 => 1,
								_ => u32::max(1, extents.height >> level),
							},
							depth_or_array_layers: match dim {
								TextureDimension::D1 => 1,
								TextureDimension::D2 => extents.depth_or_array_layers,
								TextureDimension::D3 => u32::max(1, extents.depth_or_array_layers >> level),
							},
						}
					}
					let format =
						FormatInfo::from_image_format(texture.gfx.channel_format, texture.gfx.type_format);
					println!("texture: {texture_name} {:?}", texture.gfx.image_format());
					for (mip_level, mip_offset) in texture.mip_offsets.iter().cloned().enumerate() {
						let extents = mip_level_size(extents, mip_level as _, dimension);
						let bpr = ((extents.width * format.bytes_per_pixel) / format.block_dim.height.get());

						// wgpu no likey :( need to move this sooner so i can report the correct number of mips
						if extents.width % format.block_dim.width.get() != 0
							|| extents.height % format.block_dim.height.get() != 0
						{
							break;
						}
						// println!(
						// 	"{:?} {} {} {} {mip_offset}",
						// 	extents,
						// 	bpr,
						// 	format.block_dim.width.get(),
						// 	format.bytes_per_pixel
						// );
						context.queue.write_texture(
							wgpu::TexelCopyTextureInfo {
								texture: texture_wgpu,
								mip_level: mip_level as _,
								origin: Origin3d::ZERO,
								aspect: wgpu::TextureAspect::All,
							},
							&texture.image_data[mip_offset..],
							wgpu::TexelCopyBufferLayout {
								offset: 0,
								bytes_per_row: Some(bpr),
								rows_per_image: None,
							},
							extents,
						);
					}

					let view = view.insert(texture_wgpu.create_view(&TextureViewDescriptor::default()));
					entries.push(BindGroupEntry {
						binding: 1,
						resource: wgpu::BindingResource::TextureView(view),
					});
				}

				let bind_group = context.device.create_bind_group(&BindGroupDescriptor {
					label: None,
					layout: &bind_group_layout,
					entries: &entries,
				});

				shapes.push(RenderShape {
					attributes,
					meshes,
					bind_group,
					pipeline,
					shader_options,
				})
			}
		}

		Object { shapes, transform }
	}

	pub fn draw(&self, pass: &mut RenderPass) {
		for shape in &self.shapes {
			pass.set_pipeline(&shape.pipeline);
			pass.set_bind_group(1, &shape.bind_group, &[]);
			let mut apply_vertex_buffer = |slot: u32, key: AttributeKey| {
				let attrib = shape.attributes.get(&key).unwrap();
				pass.set_vertex_buffer(slot, attrib.slice(..));
			};
			apply_vertex_buffer(0, AttributeKey::Position);
			if shape.shader_options.textured {
				apply_vertex_buffer(1, AttributeKey::Uv0);
			}
			if shape.shader_options.vertex_colours {
				apply_vertex_buffer(2, AttributeKey::Colour);
			}
			apply_vertex_buffer(3, AttributeKey::Normal);
			for mesh in &shape.meshes {
				pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
				pass.draw_indexed(0..mesh.index_count, mesh.base_vertex as _, 0..1)
			}
		}
	}
}
