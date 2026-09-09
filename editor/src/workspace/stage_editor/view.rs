use std::sync::Arc;

use cassava_project::stage::Stage;
use glam::Mat4;
use wgpu::{
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
	BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState,
	ColorWrites, FragmentState, MultisampleState, PipelineCompilationOptions,
	PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
	RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureView, VertexAttribute,
	VertexBufferLayout, VertexFormat, VertexState, include_wgsl, wgt::CommandEncoderDescriptor,
};
use zerocopy::IntoBytes;

use crate::workspace::stage_editor::{Actions, camera::ViewCamera};

pub struct StageView {
	device: Arc<wgpu::Device>,
	queue: Arc<wgpu::Queue>,
	pipeline: RenderPipeline,
	camera: ViewCamera,
	proj_matrix: Mat4,
	uniform_buffer: Buffer,
	instance_buffer: Buffer,
	instances: Vec<Mat4>,
	bind_group: BindGroup,

	surface_width: u32,
	surface_height: u32,
	_surface_color_format: wgpu::TextureFormat,
}

impl StageView {
	pub fn new(
		device: Arc<wgpu::Device>,
		queue: Arc<wgpu::Queue>,
		surface_width: u32,
		surface_height: u32,
		surface_color_format: wgpu::TextureFormat,
		stage: &Stage,
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
		let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("stage view pipeline layout"),
			bind_group_layouts: &[Some(&bind_group_layout)],
			immediate_size: 0,
		});
		let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
			label: Some("stage view pipeline"),
			layout: Some(&pipeline_layout),
			vertex: VertexState {
				module: &wgsl,
				entry_point: Some("vtx_main"),
				compilation_options: PipelineCompilationOptions::default(),
				buffers: &[Some(VertexBufferLayout {
					array_stride: size_of::<Mat4>() as _,
					attributes: &[
						VertexAttribute {
							format: VertexFormat::Float32x4,
							offset: 16 * 0,
							shader_location: 0,
						},
						VertexAttribute {
							format: VertexFormat::Float32x4,
							offset: 16 * 2,
							shader_location: 1,
						},
						VertexAttribute {
							format: VertexFormat::Float32x4,
							offset: 16 * 2,
							shader_location: 2,
						},
						VertexAttribute {
							format: VertexFormat::Float32x4,
							offset: 16 * 3,
							shader_location: 3,
						},
					],
					step_mode: wgpu::VertexStepMode::Instance,
				})],
			},
			primitive: PrimitiveState::default(),
			depth_stencil: None,
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

		let mut instances = Vec::new();
		for (_, entity) in &stage.entities {
			instances.push(Mat4::from_translation(entity.transform.translation));
		}
		let instance_buffer = device.create_buffer(&BufferDescriptor {
			label: Some("stage view instance buffer"),
			usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
			size: instances.as_bytes().len() as _,
			mapped_at_creation: false,
		});
		queue.write_buffer(&instance_buffer, 0, instances.as_bytes());

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
			instance_buffer,
			instances,
			bind_group,
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
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
				multiview_mask: None,
			});
			pass.set_bind_group(0, &self.bind_group, &[]);
			pass.set_pipeline(&self.pipeline);
			pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
			pass.draw(0..3, 0..self.instances.len() as u32);
		}

		self.queue.write_buffer(
			&self.uniform_buffer,
			0,
			(self.proj_matrix * self.camera.calculate_matrix()).as_bytes(),
		);

		self.queue.submit([cmd.finish()]);
	}
}
