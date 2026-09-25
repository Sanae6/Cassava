mod options;
pub use options::ShaderOptions;

pub const SHADER_OPTIONS: phf::Map<ShaderOptions, wgpu::ShaderModuleDescriptor> =
	include!(concat!(env!("OUT_DIR"), "/shader_options.rs"));

pub fn get_shader_variant(options: &ShaderOptions) -> wgpu::ShaderModuleDescriptor<'static> {
	SHADER_OPTIONS.get(&options).unwrap().clone()
}
