use std::{fs, path::Path, sync::Arc};

use cassava_project::stage::Stage;
use glam::Mat4;
use wgpu::{
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
	BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState,
	ColorWrites, DepthBiasState, DepthStencilState, Extent3d, FragmentState, MultisampleState,
	Operations, PipelineCache, PipelineCacheDescriptor, PipelineCompilationOptions,
	PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment,
	RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
	ShaderStages, StencilState, Texture, TextureDimension, TextureFormat, TextureUsages, TextureView,
	VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
	wgt::{CommandEncoderDescriptor, TextureDescriptor, TextureViewDescriptor},
};
use zerocopy::IntoBytes;

use crate::workspace::stage_editor::{Actions, camera::ViewCamera, object::Object};

pub struct StageView {
	device: Arc<wgpu::Device>,
	queue: Arc<wgpu::Queue>,
	pipeline: RenderPipeline,
	camera: ViewCamera,
	proj_matrix: Mat4,
	uniform_buffer: Buffer,
	bind_group: BindGroup,
	depth_texture: Texture,
	objects: Vec<Object>,

	surface_width: u32,
	surface_height: u32,
	_surface_color_format: TextureFormat,
}

impl StageView {
	pub fn new(
		device: Arc<wgpu::Device>,
		queue: Arc<wgpu::Queue>,
		surface_width: u32,
		surface_height: u32,
		surface_color_format: TextureFormat,
		stage: &Stage,
		romfs_path: &Path,
	) -> Self {
		let wgsl = device.create_shader_module(include_wgsl!("./shader.wgsl"));

		let uniform_buffer = device.create_buffer(&BufferDescriptor {
			label: Some("stage view uniform buffer"),
			usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
			size: size_of::<Mat4>() as _,
			mapped_at_creation: false,
		});
		let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
			label: Some("stage view bind group layout"),
			entries: &[BindGroupLayoutEntry {
				binding: 0,
				visibility: ShaderStages::VERTEX,
				ty: BindingType::Buffer {
					ty: BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			}],
		});
		let bind_group = device.create_bind_group(&BindGroupDescriptor {
			label: Some("stage view bind group"),
			layout: &bind_group_layout,
			entries: &[BindGroupEntry {
				binding: 0,
				resource: uniform_buffer.as_entire_binding(),
			}],
		});
		let (pipeline_cache, cache_file) =
			if let Some(filename) = wgpu::util::pipeline_cache_key(&device.adapter_info()) {
				let cache_path = Path::new("/tmp").join(&filename);
				// If we failed to read the cache, for whatever reason, treat the data as lost.
				// In a real app, we'd probably avoid caching entirely unless the error was "file not found".
				let cache_data = std::fs::read(&cache_path).ok();
				let pipeline_cache = unsafe {
					device.create_pipeline_cache(&PipelineCacheDescriptor {
						data: cache_data.as_deref(),
						label: None,
						fallback: true,
					})
				};
				(Some(pipeline_cache), Some(cache_path))
			} else {
				(None, None)
			};

		let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("stage view pipeline layout"),
			bind_group_layouts: &[Some(&bind_group_layout), Some(&bind_group_layout)],
			immediate_size: 0,
		});
		let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
			label: Some("stage view pipeline"),
			layout: Some(&pipeline_layout),
			vertex: VertexState {
				module: &wgsl,
				entry_point: Some("vtx_main"),
				compilation_options: PipelineCompilationOptions::default(),
				buffers: &[
					Some(VertexBufferLayout {
						array_stride: 12,
						step_mode: VertexStepMode::Vertex,
						attributes: &[VertexAttribute {
							format: VertexFormat::Float32x3,
							offset: 0,
							shader_location: 0,
						}],
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
			depth_stencil: Some(DepthStencilState {
				format: TextureFormat::Depth32Float,
				depth_write_enabled: Some(true),
				depth_compare: Some(wgpu::CompareFunction::Less),
				stencil: StencilState::default(),
				bias: DepthBiasState::default(),
			}),
			multisample: MultisampleState::default(),
			fragment: Some(FragmentState {
				module: &wgsl,
				entry_point: Some("frag_main"),
				compilation_options: PipelineCompilationOptions::default(),
				targets: &[Some(ColorTargetState {
					format: surface_color_format,
					blend: None,
					write_mask: ColorWrites::COLOR,
				})],
			}),
			multiview_mask: None,
			cache: None,
		});

		let depth_texture = device.create_texture(&TextureDescriptor {
			label: Some("stage view depth texture"),
			size: Extent3d {
				width: surface_width,
				height: surface_height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: TextureDimension::D2,
			format: TextureFormat::Depth32Float,
			usage: TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[TextureFormat::Depth32Float],
		});

		if let (Some(pipeline_cache), Some(cache_file)) = (pipeline_cache, cache_file) {
			let data = pipeline_cache.get_data();
			if let Some(data) = data {
				let temp_file = cache_file.with_extension("temp");
				std::fs::write(&temp_file, &data).unwrap();
				std::fs::rename(&temp_file, &cache_file).unwrap();
			}
		}

		let mut objects = Vec::new();
		let object_data = romfs_path.join("ObjectData");
		for (_, entity) in &stage.entities {
			// let object_name = entity.object_name;
			let object_path = object_data.join(format!("{}.szs", entity.object_name));
			let object = Object::new(
				&object_path,
				&device,
				&queue,
				&bind_group_layout,
				entity.transform,
			);
			objects.push(object);
		}

		Self {
			device,
			queue,
			pipeline,
			camera: ViewCamera::new(),
			proj_matrix: glam::camera::rh::proj::directx::perspective(
				45.0,
				surface_width as f32 / surface_height as f32,
				0.1,
				10000.0,
			),
			uniform_buffer,
			objects,
			bind_group,
			depth_texture,
			surface_width,
			surface_height,
			_surface_color_format: surface_color_format,
		}
	}

	pub fn try_resize(&mut self, new_width: u32, new_height: u32) {
		if self.surface_width == new_width && self.surface_height == new_height {
			return;
		}

		self.surface_width = new_width;
		self.surface_height = new_height;
		self.proj_matrix = glam::camera::rh::proj::directx::perspective(
			45.0,
			new_width as f32 / new_height as f32,
			0.1,
			50000.0,
		);
		self.depth_texture = self.device.create_texture(&TextureDescriptor {
			label: Some("stage view depth texture"),
			size: Extent3d {
				width: new_width,
				height: new_height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: TextureDimension::D2,
			format: TextureFormat::Depth32Float,
			usage: TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[TextureFormat::Depth32Float],
		})
	}

	pub fn update(&mut self, actions: &Actions) {
		// if !actions.held(StageAction::Lock) {
		// 	return;
		// }
		self.camera.update(actions);
	}

	pub fn render(&mut self, view: TextureView) {
		let mut cmd = self
			.device
			.create_command_encoder(&CommandEncoderDescriptor { label: Some("lol") });

		{
			let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
				label: Some("awesome render pass"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: &view,
					depth_slice: None,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(Color::BLACK),
						store: wgpu::StoreOp::Store,
					},
				})],
				depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
					view: &self.depth_texture.create_view(&TextureViewDescriptor {
						..Default::default()
					}),
					depth_ops: Some(Operations {
						load: wgpu::LoadOp::Clear(1.0),
						store: wgpu::StoreOp::Store,
					}),
					stencil_ops: None,
				}),
				timestamp_writes: None,
				occlusion_query_set: None,
				multiview_mask: None,
			});
			pass.set_bind_group(0, &self.bind_group, &[]);
			pass.set_pipeline(&self.pipeline);

			for object in &self.objects {
				object.draw(&mut pass);
			}
		}

		self.queue.write_buffer(
			&self.uniform_buffer,
			0,
			(self.proj_matrix * self.camera.calculate_matrix()).as_bytes(),
		);

		self.queue.submit([cmd.finish()]);
	}
}
