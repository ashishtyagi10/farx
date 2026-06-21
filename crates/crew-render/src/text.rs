use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

use crate::gpu::Gpu;

pub struct TextLayer {
    font_system: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffer: Buffer,
}

impl TextLayer {
    pub fn new(gpu: &Gpu) -> Self {
        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();
        let cache = Cache::new(&gpu.device);
        let viewport = Viewport::new(&gpu.device, &cache);
        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.format);
        let renderer = TextRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));
        buffer.set_size(
            &mut font_system,
            Some(gpu.config.width as f32),
            Some(gpu.config.height as f32),
        );
        buffer.set_text(
            &mut font_system,
            "crew ready",
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
            None,
        );
        Self {
            font_system,
            swash,
            viewport,
            atlas,
            renderer,
            buffer,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.buffer.set_text(
            &mut self.font_system,
            text,
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
            None,
        );
    }

    pub fn prepare(&mut self, gpu: &Gpu) {
        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu.config.width,
                height: gpu.config.height,
            },
        );
        let area = TextArea {
            buffer: &self.buffer,
            left: 8.0,
            top: 8.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: gpu.config.width as i32,
                bottom: gpu.config.height as i32,
            },
            default_color: Color::rgb(220, 220, 220),
            custom_glyphs: &[],
        };
        self.renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [area],
                &mut self.swash,
            )
            .expect("glyphon prepare failed");
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("glyphon render failed");
    }
}
