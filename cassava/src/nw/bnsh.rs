
#[derive(Debug)]
pub struct ShaderFile {
  target_api_type: u16,
  target_api_version: u16,
  target_code_type: u8,
  compiler_version: u32,
  shader_variations: Vec<ShaderVariation>,
}

#[derive(Debug)]
pub struct ShaderVariation {
  
}
