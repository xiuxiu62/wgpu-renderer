use glyphon::{Buffer, Color, FontSystem, Metrics, SwashCache, TextAtlas, TextRenderer};
use wgpu::{
    CommandEncoderDescriptor, Device, LoadOp, MultisampleState, Operations, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDescriptor, StoreOp, Surface, SurfaceConfiguration,
    SurfaceTexture, TextureView, TextureViewDescriptor,
};

const DEFAULT_FONT: &'static [u8] = include_bytes!("../res/DejaVuSans.ttf");
const SCALE: f32 = 0.7;

pub struct Console {
    font_system: FontSystem,
    cache: SwashCache,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffer: Buffer,
    // width: f32,
    // height: f32,
}

impl Console {
    pub fn new() {
        let temp = egui::Context::default();
    }

    // pub fn new(device: &wgpu::Device, queue: &Queue, config: &SurfaceConfiguration) -> Self {
    //     let mut font_system = FontSystem::new();
    //     let cache = SwashCache::new();
    //     let mut atlas = TextAtlas::new(device, queue, config.format);
    //     let text_renderer =
    //         TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
    //     let mut buffer = Buffer::new(&mut font_system, Metrics::new(30.0, 42.0));

    //     let physical_width = config.width as f32 * SCALE;
    //     let physical_height = config.height as f32 * SCALE;

    //     buffer.set_size(&mut font_system, physical_width, physical_height);
    //     buffer.set_text(
    //         &mut font_system,
    //         "ahoy there",
    //         glyphon::Attrs::new().family(glyphon::Family::SansSerif),
    //         glyphon::Shaping::Advanced,
    //     );
    //     buffer.shape_until_scroll(&mut font_system);

    //     Self {
    //         font_system,
    //         cache,
    //         atlas,
    //         renderer: text_renderer,
    //         buffer,
    //         // width: physical_width,
    //         // height: physical_height,
    //     }
    // }

    // pub fn render(
    //     &mut self,
    //     device: &Device,
    //     queue: &Queue,
    //     surface: &Surface,
    //     config: &SurfaceConfiguration,
    // ) {
    //     self.renderer
    //         .prepare(
    //             &device,
    //             &queue,
    //             &mut self.font_system,
    //             &mut self.atlas,
    //             glyphon::Resolution {
    //                 width: config.width,
    //                 height: config.height,
    //             },
    //             [glyphon::TextArea {
    //                 buffer: &self.buffer,
    //                 left: 10.0,
    //                 top: 10.0,
    //                 scale: 1.0,
    //                 bounds: glyphon::TextBounds {
    //                     left: 0,
    //                     top: 0,
    //                     right: 600,
    //                     bottom: 160,
    //                 },
    //                 default_color: Color::rgb(255, 0, 255),
    //             }],
    //             &mut self.cache,
    //         )
    //         .unwrap();

    //     let frame = surface.get_current_texture().unwrap();
    //     let view = frame.texture.create_view(&TextureViewDescriptor::default());
    //     let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    //     {
    //         let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
    //             label: Some("[Console] render pass"),
    //             // color_attachments: &[Some(RenderPassColorAttachment {
    //             //     view: &view,
    //             //     resolve_target: None,
    //             //     ops: Operations {
    //             //         // load: LoadOp::Clear(wgpu::Color {
    //             //         //     r: 1.0,
    //             //         //     g: 1.0,
    //             //         //     b: 1.0,
    //             //         //     a: 0.2,
    //             //         // }),
    //             //         load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
    //             //         store: StoreOp::Store,
    //             //     },
    //             // })],
    //             color_attachments: &[Some(RenderPassColorAttachment {
    //                 view: &view,
    //                 resolve_target: None,
    //                 ops: Operations {
    //                     // load: LoadOp::Load,
    //                     // load: LoadOp::Clear(wgpu::Color::RED),
    //                     load: LoadOp::Clear(wgpu::Color {
    //                         r: 0.0,
    //                         g: 1.0,
    //                         b: 1.0,
    //                         a: -1.0,
    //                     }),
    //                     store: StoreOp::Store,
    //                 },
    //             })],
    //             depth_stencil_attachment: None,
    //             timestamp_writes: None,
    //             occlusion_query_set: None,
    //         });

    //         self.renderer.render(&self.atlas, &mut pass).unwrap();
    //     }

    //     queue.submit(Some(encoder.finish()));
    //     frame.present();

    //     self.atlas.trim();
    // }

    // pub fn render<'pass>(
    //     &'pass mut self,
    //     frame: &SurfaceTexture,
    //     mut pass: &mut RenderPass<'pass>,
    //     device: &Device,
    //     queue: &Queue,
    //     surface: &Surface,
    //     config: &SurfaceConfiguration,
    // ) {
    //     self.renderer
    //         .prepare(
    //             &device,
    //             &queue,
    //             &mut self.font_system,
    //             &mut self.atlas,
    //             glyphon::Resolution {
    //                 width: config.width,
    //                 height: config.height,
    //             },
    //             [glyphon::TextArea {
    //                 buffer: &self.buffer,
    //                 left: 10.0,
    //                 top: 10.0,
    //                 scale: 1.0,
    //                 bounds: glyphon::TextBounds {
    //                     left: 0,
    //                     top: 0,
    //                     right: 600,
    //                     bottom: 160,
    //                 },
    //                 default_color: Color::rgb(255, 255, 255),
    //             }],
    //             &mut self.cache,
    //         )
    //         .unwrap();

    //     self.renderer.render(&self.atlas, &mut pass).unwrap();

    //     // let frame = surface.get_current_texture().unwrap();
    //     // let view = frame.texture.create_view(&TextureViewDescriptor::default());
    //     // let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    //     {
    //         // let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
    //         //     label: Some("[Console] render pass"),
    //         //     // color_attachments: &[Some(RenderPassColorAttachment {
    //         //     //     view: &view,
    //         //     //     resolve_target: None,
    //         //     //     ops: Operations {
    //         //     //         // load: LoadOp::Clear(wgpu::Color {
    //         //     //         //     r: 1.0,
    //         //     //         //     g: 1.0,
    //         //     //         //     b: 1.0,
    //         //     //         //     a: 0.2,
    //         //     //         // }),
    //         //     //         load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
    //         //     //         store: StoreOp::Store,
    //         //     //     },
    //         //     // })],
    //         //     color_attachments: &[Some(RenderPassColorAttachment {
    //         //         view: &view,
    //         //         resolve_target: None,
    //         //         ops: Operations {
    //         //             load: LoadOp::Load,
    //         //             // load: LoadOp::Clear(wgpu::Color::RED),
    //         //             store: StoreOp::Store,
    //         //         },
    //         //     })],
    //         //     depth_stencil_attachment: None,
    //         //     timestamp_writes: None,
    //         //     occlusion_query_set: None,
    //         // });
    //     }

    //     // queue.submit(Some(encoder.finish()));
    //     // frame.present();

    //     // self.atlas.trim();
    // }
}
