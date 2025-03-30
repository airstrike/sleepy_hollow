mod shader;

use iced::{Element, Size};

/// A high-quality image filter that uses a Mitchell-Netravali cubic filter
/// for downsampling images with better quality than the default Linear filter
pub struct CubicFilter {
    shader: shader::Shader,
}

impl CubicFilter {
    /// Create a new cubic filter using raw RGBA data
    pub fn new(image_data: Vec<u8>, image_size: Size<u32>, target_size: Size<u32>) -> Self {
        Self {
            shader: shader::Shader::new(image_data, image_size, target_size),
        }
    }
}

impl<'a, Message> From<CubicFilter> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(filter: CubicFilter) -> Self {
        iced::widget::shader(filter.shader).into()
    }
}

/// Utility function to create a cubic filtered image element
pub fn cubic_filtered_image<Message>(
    image_data: Vec<u8>,
    image_size: Size<u32>,
    target_size: Size<u32>,
) -> Element<'static, Message>
where
    Message: 'static,
{
    CubicFilter::new(image_data, image_size, target_size).into()
}
