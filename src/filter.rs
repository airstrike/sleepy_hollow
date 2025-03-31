//! A high-quality image filter that uses a Mitchell-Netravali cubic filter
//! for downsampling images with better quality than the built-in wgpu linear/
//! nearest filters.

use iced::mouse;
use iced::wgpu;
use iced::wgpu::util::DeviceExt;
use iced::widget::shader::{self, Viewport};
use iced::{ContentFit, Element, Fill, Rectangle, Size};

/// Utility function to create a filtered image element with the specified filter
pub fn filtered(image_data: Vec<u8>, image_size: Size<u32>, filter: Filter) -> Shader {
    Shader::new(image_data, image_size).filter(filter)
}

#[derive(Debug, Clone, Default, Copy, PartialEq)]
pub enum Filter {
    Cubic,
    #[default]
    Lanczos,
    Gaussian,
}

impl Filter {
    pub const ALL: [Filter; 3] = [Filter::Cubic, Filter::Lanczos, Filter::Gaussian];
    
    /// Returns the name of the filter as a string
    pub fn name(&self) -> &'static str {
        match self {
            Filter::Cubic => "cubic",
            Filter::Lanczos => "lanczos",
            Filter::Gaussian => "gaussian",
        }
    }
    
    /// Generates a label for a specific component with the filter name
    pub fn label(&self, component: &str) -> String {
        format!("{}_{}_filter", self.name(), component)
    }
    
    /// Returns the shader source code for this filter
    pub fn shader_source(&self) -> &'static str {
        match self {
            Filter::Cubic => include_str!("filter/cubic.wgsl"),
            Filter::Lanczos => include_str!("filter/lanczos.wgsl"),
            Filter::Gaussian => include_str!("filter/gaussian.wgsl"),
        }
    }
    
    /// Creates a shader module for this filter
    pub fn create_shader_module(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        // Log that we're creating a shader for a specific filter
        eprintln!("Creating shader module for filter: {:?}", self);
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&self.label("shader")),
            source: wgpu::ShaderSource::Wgsl(self.shader_source().into()),
        });
        
        // Log that we've created the shader
        eprintln!("Created shader module: {:?}", shader);
        
        shader
    }
}

impl std::fmt::Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A shader that applies a high-quality cubic filter for downsampling
pub struct Shader {
    image_data: Vec<u8>,
    image_size: Size<u32>,
    content_fit: ContentFit,
    filter: Filter,
}

impl Shader {
    pub fn new(image_data: Vec<u8>, image_size: Size<u32>) -> Self {
        Self {
            image_data,
            image_size,
            content_fit: ContentFit::Cover,
            filter: Default::default(),
        }
    }

    /// Set the content fit for the image
    pub fn content_fit(mut self, content_fit: ContentFit) -> Self {
        self.content_fit = content_fit;
        self
    }
    
    /// Set the filter to use for downsampling
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }
}

impl<Message> shader::Program<Message> for Shader {
    type State = ();
    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        eprintln!("Drawing shader with bounds: {bounds:?}");
        Primitive {
            image_data: self.image_data.clone(),
            image_size: self.image_size,
            content_fit: self.content_fit,
            filter: self.filter,
            bounds,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    image_data: Vec<u8>,
    image_size: Size<u32>,
    content_fit: ContentFit,
    filter: Filter,
    bounds: Rectangle,
}

// Define pipeline types for each filter to allow storing them separately in storage
struct CubicPipeline(Pipeline);
struct LanczosPipeline(Pipeline);
struct GaussianPipeline(Pipeline);

impl shader::Primitive for Primitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut shader::Storage,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        // Check if we have the requested filter's pipeline
        let has_pipeline = match self.filter {
            Filter::Cubic => storage.has::<CubicPipeline>(),
            Filter::Lanczos => storage.has::<LanczosPipeline>(),
            Filter::Gaussian => storage.has::<GaussianPipeline>(),
        };
        
        // Create the pipeline if it doesn't exist yet
        if !has_pipeline {
            eprintln!("Creating new pipeline for filter: {:?}", self.filter);
            
            let new_pipeline = Pipeline::new(
                self.filter,
                device,
                format,
                viewport.physical_size(),
            );
            
            // Store it with the appropriate wrapper type
            match self.filter {
                Filter::Cubic => storage.store(CubicPipeline(new_pipeline)),
                Filter::Lanczos => storage.store(LanczosPipeline(new_pipeline)),
                Filter::Gaussian => storage.store(GaussianPipeline(new_pipeline)),
            }
        }

        // Use actual bounds from the widget for proper target size
        let target_size = Size::new(bounds.width.round() as u32, bounds.height.round() as u32);

        // Get the appropriate pipeline based on the current filter
        let pipeline = match self.filter {
            Filter::Cubic => &mut storage.get_mut::<CubicPipeline>().unwrap().0,
            Filter::Lanczos => &mut storage.get_mut::<LanczosPipeline>().unwrap().0,
            Filter::Gaussian => &mut storage.get_mut::<GaussianPipeline>().unwrap().0,
        };
        
        eprintln!(
            "Preparing pipeline with:\n\
            - filter: {:?}\n\
            - image_size: {:?}\n\
            - target_size: {target_size:?}\n\
            - bounds: {:?}\n\
            - content_fit: {:?}\n\
            - viewport: {viewport:?}",
            self.filter, self.image_size, self.bounds, self.content_fit
        );
        
        pipeline.prepare(
            device,
            queue,
            &self.image_data,
            self.image_size,
            target_size,
            self.bounds,
            self.content_fit,
        );
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        // Get the appropriate pipeline based on the current filter
        let pipeline = match self.filter {
            Filter::Cubic => &storage.get::<CubicPipeline>().unwrap().0,
            Filter::Lanczos => &storage.get::<LanczosPipeline>().unwrap().0,
            Filter::Gaussian => &storage.get::<GaussianPipeline>().unwrap().0,
        };

        pipeline.render(encoder, target, clip_bounds, self.bounds, self.content_fit);
    }
}

struct Pipeline {
    filter: Filter,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    target_size: Size<u32>,
}

impl Pipeline {
    pub fn new(
        filter: Filter,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        viewport: Size<u32>,
    ) -> Self {
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&filter.label("bind_group_layout")),
            entries: &[
                // Texture binding
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler binding
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniform buffer for texture dimensions and scale
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create shader using the Filter's method
        let shader = filter.create_shader_module(device);

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&filter.label("pipeline_layout")),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create vertex buffer for full-screen quad with positions and UVs
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&filter.label("vertex_buffer")),
            contents: bytemuck::cast_slice(&[
                // Positions       // UVs (these aren't actually used since they're hardcoded in the shader)
                -1.0f32, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // For the RenderPipelineDescriptor, add the cache field
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&filter.label("pipeline")),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create uniform buffer for texture dimensions and scale
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&filter.label("uniform_buffer")),
            size: 16, // 4 f32 values: width, height, scale_x, scale_y
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&filter.label("sampler")),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            filter,
            pipeline,
            bind_group_layout,
            texture: None,
            texture_view: None,
            sampler,
            bind_group: None,
            vertex_buffer,
            uniform_buffer,
            target_size: viewport,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image_data: &[u8],
        image_size: Size<u32>,
        target_size: Size<u32>,
        bounds: Rectangle,
        content_fit: ContentFit,
    ) {
        // Store the target size for later use in render()
        self.target_size = target_size;

        // Create the texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&self.filter.label("texture")),
            size: wgpu::Extent3d {
                width: image_size.width,
                height: image_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create texture view
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Write the image data to the texture
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image_size.width),
                rows_per_image: Some(image_size.height),
            },
            wgpu::Extent3d {
                width: image_size.width,
                height: image_size.height,
                depth_or_array_layers: 1,
            },
        );

        // Calculate fitted image size based on content_fit
        let image_size_f32 = Size::new(image_size.width as f32, image_size.height as f32);
        let bounds_size = bounds.size();
        let fitted_size = content_fit.fit(image_size_f32, bounds_size);

        // Calculate actual scale factors based on the fitted size
        let actual_scale_x = image_size_f32.width / fitted_size.width;
        let actual_scale_y = image_size_f32.height / fitted_size.height;

        eprintln!(
            "Image scaling factors: scale_x={}, scale_y={}",
            actual_scale_x, actual_scale_y
        );

        // Update the uniform buffer with correct scaling factors
        let uniforms = [
            image_size.width as f32,
            image_size.height as f32,
            actual_scale_x,
            actual_scale_y,
        ];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&self.filter.label("bind_group")),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        self.texture = Some(texture);
        self.texture_view = Some(texture_view);
        self.bind_group = Some(bind_group);
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
        bounds: Rectangle,
        content_fit: ContentFit,
    ) {
        if let Some(bind_group) = &self.bind_group {
            // Calculate image size
            let image_size = Size::new(
                self.texture.as_ref().unwrap().size().width as f32,
                self.texture.as_ref().unwrap().size().height as f32,
            );

            // Apply ContentFit to determine the actual rendering size
            let fitted_size = content_fit.fit(image_size, bounds.size());

            // Calculate position to center the image within bounds
            let x = bounds.x + (bounds.width - fitted_size.width) / 2.0;
            let y = bounds.y + (bounds.height - fitted_size.height) / 2.0;

            // Create rectangle for the fitted image
            let fitted_bounds = Rectangle {
                x,
                y,
                width: fitted_size.width,
                height: fitted_size.height,
            };

            // Convert fitted bounds to viewport-space units
            let render_bounds = Rectangle {
                x: fitted_bounds.x.round() as u32,
                y: fitted_bounds.y.round() as u32,
                width: fitted_bounds.width.round() as u32,
                height: fitted_bounds.height.round() as u32,
            };

            // Begin render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&self.filter.label("render_pass")),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set up the pipeline and resources
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // Debug all bounds:
            eprintln!(
                "Rendering shader with:\n\
                - clip_bounds: {clip_bounds:?}\n\
                - bounds: {bounds:?}\n\
                - fitted_bounds: {fitted_bounds:?}\n\
                - render_bounds: {render_bounds:?}\n\
            "
            );

            // Set scissor rectangle to the bounds of our widget
            render_pass.set_scissor_rect(
                render_bounds.x,
                render_bounds.y,
                render_bounds.width,
                render_bounds.height,
            );

            // Set viewport to match the render bounds
            // This is crucial - it maps the normalized device coordinates from
            // the shader to the correct screen position
            render_pass.set_viewport(
                render_bounds.x as f32,
                render_bounds.y as f32,
                render_bounds.width as f32,
                render_bounds.height as f32,
                0.0,
                1.0,
            );

            // Draw the full-screen quad (4 vertices in a triangle strip)
            render_pass.draw(0..4, 0..1);
        }
    }
}

impl<'a, Message> From<Shader> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(shader: Shader) -> Self {
        iced::widget::shader(shader).width(Fill).height(Fill).into()
    }
}
