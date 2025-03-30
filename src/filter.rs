//! A high-quality image filter that uses a Mitchell-Netravali cubic filter
//! for downsampling images with better quality than the default Linear filter

mod shader;

use iced::{Element, Fill, Size};

impl<'a, Message> From<shader::Shader> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(shader: shader::Shader) -> Self {
        iced::widget::shader(shader).width(Fill).height(Fill).into()
    }
}

/// Utility function to create a cubic filtered image element
pub fn cubic<Message>(
    image_data: Vec<u8>,
    image_size: Size<u32>,
    target_size: Size<u32>,
) -> Element<'static, Message>
where
    Message: 'static,
{
    shader::Shader::new(image_data, image_size, target_size).into()
}
