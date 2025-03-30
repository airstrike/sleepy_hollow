use iced::advanced::image::Bytes;
use iced::widget::{column, container, row, text};
use iced::{Element, Fill, Rectangle, Size};
use rand::Rng;

use crate::simulator;

#[derive(Debug, Clone)]
pub struct PngScreenshot {
    pub size: iced::Size<u32>,
    pub png_data: Vec<u8>,
    pub raw_data: Bytes,
}

// Helper function to create a styled text container
pub fn styled_text_container<'a, Message>(
    content: String,
    text_size: u16,
    color_hex: u32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(text(content).center().size(text_size as f32))
        .padding(10)
        .style(move |_| container::background(iced::color!(color_hex)))
        .center(Fill)
        .into()
}

// Create a sample document with variable layout
pub fn sample<'a>() -> (Element<'a, ()>, Size) {
    // fixed dimensions for our container
    let width = 1600_u32;
    let height = 900_u32;

    // Generate random grid dimensions
    let mut rng = rand::rng();
    let rows = rng.random_range(2..6);

    // Create grid items
    let mut grid_items = Vec::new();
    let mut cells = 0;

    for r in 0..rows {
        let mut row_items = Vec::new();
        let cols = rng.random_range(2..5);

        for c in 0..cols {
            cells += 1;
            // Generate random properties for this item
            let text_content = format!("Item ({},{})", r + 1, c + 1);
            let text_size = rng.random_range(16..32);

            // Generate a random color in blue/green range
            let r_val = rng.random_range(0..100);
            let g_val = rng.random_range(80..220);
            let b_val = rng.random_range(120..255);
            let color_hex = (r_val << 16) | (g_val << 8) | b_val;

            // Create a container with this text
            row_items.push(styled_text_container(text_content, text_size, color_hex));
        }

        // Add the row to our grid
        grid_items.push(
            row(row_items)
                .spacing(5)
                .padding(5)
                .height(Fill)
                .width(Fill)
                .into(),
        );
    }

    // Add a title at the top
    let title = container(
        text(format!("Sample Grid ({} x ~{})", rows, cells / rows))
            .center()
            .size(18),
    )
    .center_x(Fill);

    // Create the content with our grid
    let column_content = column![
        title,
        column(grid_items).spacing(5).width(Fill).height(Fill)
    ]
    .spacing(20)
    .padding(20);

    let content = container(column_content)
        .center_x(width)
        .center_y(height)
        .style(container::rounded_box);

    (content.into(), Size::new(width as f32, height as f32))
}

// Function that renders using an existing simulator
pub fn render(simulator: &mut simulator::Simulator) -> Result<PngScreenshot, String> {
    let (element, size) = sample();

    println!("Rendering sample document");

    // Take a screenshot with the element
    let scale_factor = 2.0;
    let screenshot = simulator.screenshot(element, size, scale_factor)?;

    // Account for the scale factor when cropping
    let scale_factor = screenshot.scale_factor as f32;
    let scaled_crop_rectangle = Rectangle {
        x: 0,
        y: 0,
        width: (size.width as f32 * scale_factor) as u32,
        height: (size.height as f32 * scale_factor) as u32,
    };

    println!(
        "Scale factor: {}, Original crop: {:?}, Scaled crop: {:?}",
        scale_factor, size, scaled_crop_rectangle
    );

    let screenshot = screenshot
        .crop(scaled_crop_rectangle)
        .map_err(|e| format!("Failed to crop screenshot: {:?}", e))?;

    let mut png_data = Vec::new();
    {
        let mut encoder =
            png::Encoder::new(&mut png_data, screenshot.size.width, screenshot.size.height);
        encoder.set_color(png::ColorType::Rgba);

        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;

        writer
            .write_image_data(&screenshot.bytes.to_vec())
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;

        writer
            .finish()
            .map_err(|e| format!("Failed to finish PNG encoding: {}", e))?;
    }

    // Return the PNG screenshot
    Ok(PngScreenshot {
        size: screenshot.size,
        png_data,
        raw_data: screenshot.bytes,
    })
}
