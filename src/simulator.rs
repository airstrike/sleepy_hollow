//! Simulate `iced` user interfaces and take screenshots of them
use iced::advanced::renderer;
use iced::advanced::renderer::Headless;
use iced::theme::Base;
use iced::{self, Element, Size, mouse, window};
use iced_runtime::{UserInterface, user_interface};
use std::borrow::Cow;
use std::env;

use iced::Theme;

/// A simulator that can take screenshots of `iced`` user interfaces
pub struct Simulator<R = iced::Renderer>
where
    R: renderer::Renderer + Headless + Send,
{
    renderer: R,
    cursor: mouse::Cursor,
    theme: Theme,
}

impl<R> Simulator<R>
where
    R: renderer::Renderer + Headless + Send,
{
    pub fn new() -> Self
    where
        R: Headless,
    {
        Self::with_size(iced::Settings::default())
    }

    pub fn with_size(settings: iced::Settings) -> Self
    where
        R: Headless,
    {
        let default_font = match settings.default_font {
            iced::Font::DEFAULT => iced::Font::with_name("Fira Sans"),
            _ => settings.default_font,
        };

        for font in settings.fonts {
            load_font(font).expect("Font must be valid");
        }

        let renderer = {
            let backend = env::var("ICED_BACKEND").ok();

            iced::futures::executor::block_on(R::new(
                default_font,
                settings.default_text_size,
                backend.as_deref(),
            ))
            .expect("Create new headless renderer")
        };

        Simulator {
            renderer,
            cursor: mouse::Cursor::Unavailable,
            theme: Theme::default(),
        }
    }

    // Takes a screenshot of the given element
    pub fn screenshot<'a, Message>(
        &mut self,
        element: impl Into<Element<'a, Message, Theme, R>>,
        size: impl Into<Size>,
        scale_factor: f32,
    ) -> Result<window::Screenshot, String>
    where
        Message: 'a,
    {
        let base = self.theme.base();
        let size = size.into();

        // build a UI just for this screenshot
        let mut ui = UserInterface::build(
            element,
            size * scale_factor,
            user_interface::Cache::default(),
            &mut self.renderer,
        );

        let _ = ui.draw(
            &mut self.renderer,
            &self.theme,
            &renderer::Style {
                text_color: base.text_color,
            },
            self.cursor,
        );

        let physical_size = Size::new(
            (size.width * scale_factor).round() as u32,
            (size.height * scale_factor).round() as u32,
        );

        let rgba = self
            .renderer
            .screenshot(physical_size, scale_factor, base.background_color);

        Ok(window::Screenshot::new(
            rgba,
            physical_size,
            f64::from(scale_factor),
        ))
    }
}

fn load_font(font: impl Into<Cow<'static, [u8]>>) -> Result<(), String> {
    iced::advanced::graphics::text::font_system()
        .write()
        .expect("Write to font system")
        .load_font(font.into());

    Ok(())
}
