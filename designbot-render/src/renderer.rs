use designbot_core::{Canvas, DesignBotError};
use peniko::Fill;
use std::num::NonZeroUsize;
use vello::{
    kurbo::Affine,
    peniko::Color,
    util::RenderContext,
    AaConfig, Renderer as VelloRenderer, RendererOptions, Scene,
};

pub struct Renderer {
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn render_to_png(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        // Create wgpu instance
        let mut context = RenderContext::new();

        let device_id = pollster::block_on(context.device(None))
            .ok_or_else(|| DesignBotError::RenderError("No compatible device found".to_string()))?;

        let device_handle = &mut context.devices[device_id];
        let device = &device_handle.device;
        let queue = &device_handle.queue;

        // Create renderer
        let mut renderer = VelloRenderer::new(
            device,
            RendererOptions {
                surface_format: None,
                use_cpu: false,
                num_init_threads: NonZeroUsize::new(1),
                antialiasing_support: vello::AaSupport::all(),
            },
        )
        .map_err(|e| DesignBotError::RenderError(format!("Failed to create renderer: {}", e)))?;

        // Create scene from canvas
        let mut scene = Scene::new();

        // Set white background
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::WHITE,
            None,
            &kurbo::Rect::new(0.0, 0.0, self.width as f64, self.height as f64),
        );

        // Add all draw commands
        for command in canvas.commands() {
            use designbot_core::canvas::ShapeType;

            match command {
                designbot_core::canvas::DrawCommand::FillShape {
                    shape,
                    brush,
                    transform,
                } => match shape {
                    ShapeType::Rect(r) => {
                        scene.fill(Fill::NonZero, *transform, brush, None, r);
                    }
                    ShapeType::Circle(c) => {
                        scene.fill(Fill::NonZero, *transform, brush, None, c);
                    }
                    ShapeType::Line(l) => {
                        scene.fill(Fill::NonZero, *transform, brush, None, l);
                    }
                    ShapeType::Path(p) => {
                        scene.fill(Fill::NonZero, *transform, brush, None, p);
                    }
                },
                designbot_core::canvas::DrawCommand::StrokeShape {
                    shape,
                    brush,
                    stroke,
                    transform,
                } => match shape {
                    ShapeType::Rect(r) => {
                        scene.stroke(stroke, *transform, brush, None, r);
                    }
                    ShapeType::Circle(c) => {
                        scene.stroke(stroke, *transform, brush, None, c);
                    }
                    ShapeType::Line(l) => {
                        scene.stroke(stroke, *transform, brush, None, l);
                    }
                    ShapeType::Path(p) => {
                        scene.stroke(stroke, *transform, brush, None, p);
                    }
                },
            }
        }

        // Create render texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Render
        let render_params = vello::RenderParams {
            base_color: Color::WHITE,
            width: self.width,
            height: self.height,
            antialiasing_method: AaConfig::Area,
        };

        renderer
            .render_to_texture(device, queue, &scene, &view, &render_params)
            .map_err(|e| DesignBotError::RenderError(format!("Failed to render: {}", e)))?;

        // Read pixels
        let bytes_per_pixel = 4; // RGBA
        let unpadded_byte_width = self.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_byte_width = (unpadded_byte_width + align - 1) / align * align;
        let buffer_size = (padded_byte_width * self.height) as u64;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback encoder"),
        });

        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_byte_width),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        // Map the buffer and read pixels
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        device.poll(wgpu::Maintain::Wait);

        pollster::block_on(async {
            receiver.receive().await
        })
        .ok_or_else(|| DesignBotError::RenderError("Failed to receive buffer".to_string()))?
        .map_err(|e| DesignBotError::RenderError(format!("Failed to map buffer: {}", e)))?;

        let data = buffer_slice.get_mapped_range();

        // Remove padding from rows if necessary
        let mut rgba_data = Vec::with_capacity((self.width * self.height * bytes_per_pixel) as usize);
        for row in 0..self.height {
            let start = (row * padded_byte_width) as usize;
            let end = start + unpadded_byte_width as usize;
            rgba_data.extend_from_slice(&data[start..end]);
        }

        drop(data);
        buffer.unmap();

        // Save to PNG
        image::save_buffer(
            output_path,
            &rgba_data,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| DesignBotError::IOError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(())
    }
}
