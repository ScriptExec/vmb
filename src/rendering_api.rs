use clap::ValueEnum;

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum RenderingAPI {
	#[value(name = "vk")]
	Vulkan,
	#[value(name = "dx12")]
	DirectX12,
	// Metal,
	// #[value(name = "gl")]
	// OpenGL3,
	// OpenGL3_Es,
	// OpenGL3_Angle,
}

impl RenderingAPI {
	pub fn as_driver_name(self) -> &'static str {
		match self {
			RenderingAPI::Vulkan => "vulkan",
			RenderingAPI::DirectX12 => "d3d12",
			// RenderingAPI::Metal => "metal",
			// RenderingAPI::OpenGL3 => "opengl3",
			// RenderingAPI::OpenGL3_Es => "opengl3_es",
			// RenderingAPI::OpenGL3_Angle => "opengl3_angle",
		}
	}
}
