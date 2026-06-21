use glyphon::{
    Buffer, Cache, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport, Wrap,
};

use crate::gpu::Gpu;
use crate::quads::{Quad, QuadLayer};

use crate::celltext::{build_rich_text, probe_cell_width};

// Default terminal background colour.
const DEFAULT_BG: (u8, u8, u8) = (10, 10, 18);

/// A single terminal cell to be rendered.
pub struct CellView {
    pub col: u16,
    pub row: u16,
    pub c: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub bold: bool,
    pub italic: bool,
}

/// Grid dimensions inferred from the window size and font metrics.
pub struct GridMetrics {
    pub cell_w: f32,
    pub cell_h: f32,
    pub cols: u16,
    pub rows: u16,
}

/// Renders a terminal grid: per-cell background quads + per-cell colored text.
pub struct CellGrid {
    font_system: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffer: Buffer,
    quad_layer: QuadLayer,
    cell_w: f32,
    cell_h: f32,
}

impl CellGrid {
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

        const FONT_SIZE: f32 = 16.0;
        const LINE_HEIGHT: f32 = 20.0;

        let mut buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));
        buffer.set_wrap(&mut font_system, Wrap::None);
        buffer.set_size(
            &mut font_system,
            Some(gpu.config.width as f32),
            Some(gpu.config.height as f32),
        );

        let cell_w = probe_cell_width(&mut buffer, &mut font_system, FONT_SIZE);
        let cell_h = LINE_HEIGHT;
        let quad_layer = QuadLayer::new(&gpu.device, gpu.format);

        Self {
            font_system,
            swash,
            viewport,
            atlas,
            renderer,
            buffer,
            quad_layer,
            cell_w,
            cell_h,
        }
    }

    /// Returns the monospace cell size `(width, height)` in pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_w, self.cell_h)
    }

    /// Update the text buffer's layout bounds to match the new surface size.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.buffer
            .set_size(&mut self.font_system, Some(width), Some(height));
    }

    /// Upload cell data: builds background quads + rich-text foreground.
    /// `gpu` is needed to upload quad instance data to the GPU.
    pub fn set_cells(&mut self, gpu: &Gpu, cells: &[CellView], metrics: &GridMetrics) {
        let quads: Vec<Quad> = cells
            .iter()
            .filter(|c| c.bg != DEFAULT_BG)
            .map(|c| Quad {
                x: f32::from(c.col) * metrics.cell_w,
                y: f32::from(c.row) * metrics.cell_h,
                w: metrics.cell_w,
                h: metrics.cell_h,
                color: [
                    c.bg.0 as f32 / 255.0,
                    c.bg.1 as f32 / 255.0,
                    c.bg.2 as f32 / 255.0,
                    1.0,
                ],
            })
            .collect();
        self.quad_layer.set_quads(&gpu.device, &quads);
        build_rich_text(&mut self.buffer, &mut self.font_system, cells, metrics);
    }

    /// Update viewports and prepare GPU uploads.
    pub fn prepare(&mut self, gpu: &Gpu) {
        self.quad_layer.set_viewport(
            &gpu.queue,
            gpu.config.width as f32,
            gpu.config.height as f32,
        );
        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu.config.width,
                height: gpu.config.height,
            },
        );
        let area = TextArea {
            buffer: &self.buffer,
            left: 0.0,
            top: 0.0,
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

    /// Draw backgrounds then text into the active render pass.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.quad_layer.draw(pass);
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("glyphon render failed");
    }
}
