use iced::mouse;
use iced::wgpu;
use iced::wgpu::util::DeviceExt;
use iced::widget::shader::{self, Viewport};
use iced::{ContentFit, Point, Rectangle, Size};

/// A shader that applies a high-quality cubic filter for downsampling
pub struct Shader {
    image_data: Vec<u8>,
    image_size: Size<u32>,
    target_size: Size<u32>,
    content_fit: ContentFit,
}

impl Shader {
    pub fn new(image_data: Vec<u8>, image_size: Size<u32>, target_size: Size<u32>) -> Self {
        Self {
            image_data,
            image_size,
            target_size,
            content_fit: ContentFit::Cover,
        }
    }

    /// Set the content fit for the image
    pub fn content_fit(mut self, content_fit: ContentFit) -> Self {
        self.content_fit = content_fit;
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
            target_size: self.target_size,
            content_fit: self.content_fit,
            bounds,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    image_data: Vec<u8>,
    image_size: Size<u32>,
    target_size: Size<u32>,
    content_fit: ContentFit,
    bounds: Rectangle,
}

impl shader::Primitive for Primitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut shader::Storage,
        _bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        if !storage.has::<Pipeline>() {
            storage.store(Pipeline::new(device, format, viewport.physical_size()));
        }

        let pipeline = storage.get_mut::<Pipeline>().unwrap();
        pipeline.prepare(
            device,
            queue,
            &self.image_data,
            self.image_size,
            self.target_size,
        );
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let pipeline = storage.get::<Pipeline>().unwrap();

        // Scale factors for shader
        let scale_x = self.image_size.width as f32 / self.target_size.width as f32;
        let scale_y = self.image_size.height as f32 / self.target_size.height as f32;

        pipeline.render(
            encoder,
            target,
            clip_bounds,
            self.bounds,
            scale_x,
            scale_y,
            self.content_fit,
        );
    }
}

struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
}

impl Pipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, _viewport: Size<u32>) -> Self {
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cubic_filter_bind_group_layout"),
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

        // Create shader
        let shader_source = include_str!("cubic.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cubic_filter_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cubic_filter_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // For the RenderPipelineDescriptor, add the cache field
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cubic_filter_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(), // Add this
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(), // Add this
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
            cache: None, // Add this
        });

        // Create vertex buffer for full-screen quad
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cubic_filter_vertex_buffer"),
            contents: bytemuck::cast_slice(&[
                -1.0f32, -1.0, // bottom-left
                1.0, -1.0, // bottom-right
                -1.0, 1.0, // top-left
                1.0, 1.0, // top-right
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create uniform buffer for texture dimensions and scale
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cubic_filter_uniform_buffer"),
            size: 16, // 4 f32 values: width, height, scale_x, scale_y
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cubic_filter_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            texture: None,
            texture_view: None,
            sampler,
            bind_group: None,
            vertex_buffer,
            uniform_buffer,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image_data: &[u8],
        image_size: Size<u32>,
        target_size: Size<u32>,
    ) {
        // Create the texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cubic_filter_texture"),
            size: wgpu::Extent3d {
                width: image_size.width,
                height: image_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
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

        // Update uniform buffer with texture dimensions and scale
        let scale_x = image_size.width as f32 / target_size.width as f32;
        let scale_y = image_size.height as f32 / target_size.height as f32;
        let uniforms = [
            image_size.width as f32,
            image_size.height as f32,
            scale_x,
            scale_y,
        ];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cubic_filter_bind_group"),
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
        scale_x: f32,
        scale_y: f32,
        content_fit: ContentFit,
    ) {
        if let Some(bind_group) = &self.bind_group {
            // Calculate image and target sizes
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

            // Determine actual rendering area by intersecting with clip bounds
            let render_bounds = if let Some(intersection) =
                fitted_bounds.intersection(&Rectangle::new(
                    Point::new(clip_bounds.x as f32, clip_bounds.y as f32),
                    Size::new(clip_bounds.width as f32, clip_bounds.height as f32),
                )) {
                Rectangle {
                    x: intersection.x.round() as u32,
                    y: intersection.y.round() as u32,
                    width: intersection.width.round() as u32,
                    height: intersection.height.round() as u32,
                }
            } else {
                return; // Nothing to render if no intersection
            };

            // Begin render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cubic_filter_render_pass"),
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

            // Set scissor rectangle for clipping to the fitted bounds
            render_pass.set_scissor_rect(
                render_bounds.x,
                render_bounds.y,
                render_bounds.width,
                render_bounds.height,
            );

            // Draw the full-screen quad (4 vertices in a triangle strip)
            render_pass.draw(0..4, 0..1);
        }
    }
}
