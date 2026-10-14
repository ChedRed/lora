use glyphon::{Cache, FontSystem, SwashCache, TextAtlas, TextRenderer, Viewport};
use wgpu::TextureFormat;

pub struct FontGod {
     pub cache: Cache,
     pub fonts: FontSystem,
     pub atlas: TextAtlas,
     pub swash: SwashCache,
     pub viewport: Viewport,
     pub renderer: TextRenderer,
}

impl FontGod {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let cache = Cache::new(device);
        let fonts = FontSystem::new();
        let swash = SwashCache::new();
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let viewport = Viewport::new(device, &cache);
        let renderer = TextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);

        Self {
            cache,
            fonts,
            swash,
            atlas,
            viewport,
            renderer,
        }
    }
}
