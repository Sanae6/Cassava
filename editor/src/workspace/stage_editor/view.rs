use std::{collections::HashMap, fs, path::Path, sync::Arc};

use cassava::nw::{bfres::ModelArchive, bntx::TextureArchive};
use cassava_project::{smol_str::SmolStr, stage::Stage};
use glam::Mat4;
use wgpu::{
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
	BindGroupLayoutEntry, BindingType, BlendComponent, BlendState, Buffer, BufferBindingType,
	BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, DepthBiasState,
	DepthStencilState, Device, Extent3d, FragmentState, MultisampleState, Operations, PipelineCache,
	PipelineCacheDescriptor, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
	Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
	RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, Texture, TextureDimension,
	TextureFormat, TextureUsages, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat,
	VertexState, VertexStepMode, include_wgsl,
	wgt::{CommandEncoderDescriptor, TextureDescriptor, TextureViewDescriptor},
};
use zerocopy::IntoBytes;

use crate::workspace::stage_editor::{
	Actions,
	camera::ViewCamera,
	object::Object,
	shaders::{SHADER_OPTIONS, ShaderOptions},
};

pub struct StageView {
	device: Arc<wgpu::Device>,
	queue: Arc<wgpu::Queue>,
	camera: ViewCamera,
	proj_matrix: Mat4,
	proj_uniform: Buffer,
	proj_bind_group: BindGroup,
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
		let uniform_buffer = device.create_buffer(&BufferDescriptor {
			label: Some("stage view uniform buffer"),
			usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
			size: size_of::<Mat4>() as _,
			mapped_at_creation: false,
		});
		let proj_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
			layout: &proj_bind_layout,
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

		if let (Some(pipeline_cache), Some(cache_file)) = (&pipeline_cache, cache_file) {
			let data = pipeline_cache.get_data();
			if let Some(data) = data {
				let temp_file = cache_file.with_extension("temp");
				std::fs::write(&temp_file, &data).unwrap();
				std::fs::rename(&temp_file, &cache_file).unwrap();
			}
		}

		let mut objects = Vec::new();
		let object_data = romfs_path.join("ObjectData");
		let context = PipelineContext {
			depth_stencil_state: DepthStencilState {
				format: TextureFormat::Depth32Float,
				depth_write_enabled: Some(true),
				depth_compare: Some(wgpu::CompareFunction::LessEqual),
				stencil: StencilState::default(),
				bias: DepthBiasState::default(),
			},
			colour_target_state: ColorTargetState {
				format: surface_color_format,
				blend: Some(BlendState {
					color: BlendComponent {
						src_factor: wgpu::BlendFactor::SrcAlpha,
						dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
						operation: wgpu::BlendOperation::Add,
					},
					alpha: BlendComponent {
						src_factor: wgpu::BlendFactor::SrcAlpha,
						dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
						operation: wgpu::BlendOperation::Add,
					},
				}),
				write_mask: ColorWrites::COLOR,
			},
			device: &device,
			queue: &queue,
			proj_bind_layout: &proj_bind_layout,
			pipeline_cache: pipeline_cache.as_ref(),
		};
		let mut model_cache: HashMap<SmolStr, (ModelArchive, Option<TextureArchive>, Option<SmolStr>)> =
			HashMap::new();
		let mut texture_arc_cache = HashMap::new();
		for (_, entity) in &stage.entities {
			let object_path = object_data.join(format!("{}.szs", entity.object_name));

			let (model, texture_archive, external_texture_archive) =
				if let Some(all) = model_cache.get(&entity.object_name) {
					all
				} else if let Some((model, texture_archive, external_texture_archive)) =
					Object::load_model_archive(&object_data, &entity.object_name)
				{
					model_cache.insert(
						entity.object_name.clone(),
						(model, texture_archive, external_texture_archive),
					);
					model_cache.get(&entity.object_name).unwrap()
				} else {
					continue;
				};

			let external_texture_archive = external_texture_archive.as_ref().and_then(|name| {
				if let Some(texture_arc) = texture_arc_cache.get(name) {
					Some(texture_arc)
				} else if let Some(texture_arc) = Object::load_external_texture_archive(&object_data, &name)
				{
					texture_arc_cache.insert(entity.object_name.clone(), texture_arc);
					texture_arc_cache.get(&entity.object_name)
				} else {
					None
				}
			});

			let object = Object::new(
				model,
				&context,
				entity.transform,
				texture_archive.as_ref(),
				external_texture_archive,
			);
			objects.push(object);
		}

		Self {
			device,
			queue,
			camera: ViewCamera::new(),
			proj_matrix: Self::projection_matrix(surface_width as f32 / surface_height as f32),
			proj_uniform: uniform_buffer,
			objects,
			proj_bind_group: bind_group,
			depth_texture,
			surface_width,
			surface_height,
			_surface_color_format: surface_color_format,
		}
	}

	fn projection_matrix(aspect_ratio: f32) -> Mat4 {
		glam::camera::rh::proj::directx::perspective(45.0, aspect_ratio, 10.0, 100000.0)
	}

	pub fn try_resize(&mut self, new_width: u32, new_height: u32) {
		if self.surface_width == new_width && self.surface_height == new_height {
			return;
		}

		self.surface_width = new_width;
		self.surface_height = new_height;
		self.proj_matrix = Self::projection_matrix(new_width as f32 / new_height as f32);
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
		self.camera.update(actions);
	}

	pub fn render(&mut self, view: TextureView) {
		self.queue.write_buffer(
			&self.proj_uniform,
			0,
			(self.proj_matrix * self.camera.calculate_matrix()).as_bytes(),
		);

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
			pass.set_bind_group(0, &self.proj_bind_group, &[]);

			for object in &self.objects {
				object.draw(&mut pass);
			}
		}

		self.queue.submit([cmd.finish()]);
	}
}

pub struct PipelineContext<'a> {
	pub depth_stencil_state: DepthStencilState,
	pub colour_target_state: ColorTargetState,
	pub device: &'a Device,
	pub queue: &'a Queue,
	pub proj_bind_layout: &'a BindGroupLayout,
	pub pipeline_cache: Option<&'a PipelineCache>,
}
