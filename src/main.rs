use bytemuck::{Pod, Zeroable};
use camera::{Camera, CameraController, CameraUniform, Projection};
use cgmath::{Deg, InnerSpace, Matrix3, Matrix4, Quaternion, Rotation3, Vector2, Vector3, Zero};
use light::{DrawLight, LightBundle, LightUniform};
// use math::vec::{Vec2, Vec3};
use model::{DrawModel, Model, ModelVertex, VertexBufferFormat};
use std::{
    iter,
    sync::OnceLock,
    time::{Duration, Instant},
};
use texture::Texture;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType,
    BufferUsages, CommandEncoderDescriptor, Device, Features, Limits, LoadOp, Operations,
    PipelineLayoutDescriptor, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, SamplerBindingType,
    ShaderModule, ShaderStages, StoreOp, Surface, SurfaceConfiguration, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexStepMode,
};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

mod camera;
mod light;
mod model;
mod pipeline;
mod terrain;
mod texture;

#[inline]
fn supported_backends() -> &'static Backends {
    static BACKENDS: OnceLock<Backends> = OnceLock::new();

    BACKENDS.get_or_init(|| Backends::VULKAN | Backends::DX12 | Backends::METAL)
}

// const INSTANCES_PER_ROW: u32 = 1;
const INSTANCES_PER_ROW: u32 = 10;
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

#[pollster::main]
async fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut graphics_state = GraphicsState::new(window).await;
    let mut previous_render_time = Instant::now();
    let target_frame_rate = 120;
    let frame_time = Duration::from_millis(1000) / target_frame_rate as u32;

    graphics_state.text_manager.update("ahoy sailor");

    event_loop
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::Suspended => target.exit(),
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                    ..
                } if graphics_state.mouse_pressed => {
                    graphics_state.camera_controller.handle_mouse(dx, dy);
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == graphics_state.window().id()
                    && !graphics_state.handle_input(event) =>
                {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => target.exit(),
                        // WindowEvent::KeyboardInput {
                        //     event:
                        //         KeyEvent {
                        //             state: ElementState::Released,
                        //             physical_key: PhysicalKey::Code(KeyCode::KeyP),
                        //             ..
                        //         },
                        //     ..
                        // } => graphics_state.toggle_wirefame(),
                        WindowEvent::Resized(size) => graphics_state.resize(*size),
                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            let dt = Instant::now() - previous_render_time;
                            previous_render_time = now;
                            graphics_state.update(dt);

                            match graphics_state.render() {
                                Ok(()) => {}
                                Err(wgpu::SurfaceError::Lost) => {
                                    println!("Error: Surface Lost");
                                    graphics_state.resize(graphics_state.size)
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                                Err(error) => eprintln!("{error:?}"),
                            }
                        }
                        _ => {}
                    }
                }

                _ => {}
            };

            let now = Instant::now();
            let dt = now - previous_render_time;

            if dt >= frame_time {
                graphics_state.window.request_redraw();
            }
        })
        .unwrap();
}

struct GraphicsState {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,

    wireframe: bool,

    model: Model,
    instance_buffer: Buffer,
    instances: Vec<Instance>,

    depth_texture: Texture,

    camera: Camera,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    camera_controller: CameraController,

    light_bundle: LightBundle,
    standard_render_pipeline: RenderPipeline,
    light_render_pipeline: RenderPipeline,

    text_manager: ui::TextManager,

    // pipelines: Vec<Pipeline>,
    mouse_pressed: bool,
}

impl GraphicsState {
    async fn new(window: Window) -> Self {
        let (surface, size, device, queue, config) = Self::initialize_surface(&window).await;
        let texture_bind_group_layout = Self::initialize_texture(&device);
        let (instance_buffer, instances) = Self::initialize_instances(&device);
        let (
            camera,
            projection,
            camera_uniform,
            camera_controller,
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
        ) = Self::initialize_camera(&device, &config);
        let depth_texture = Texture::create_depth_texture(&device, &config);
        let model =
            model::resource::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
                .unwrap();

        let light_bundle =
            LightUniform::new(vec3!(2.0, 2.0, 2.0), vec3!(1.0, 1.0, 1.0)).prepared(&device);

        let standard_render_pipeline = {
            let shader =
                device.create_shader_module(wgpu::include_wgsl!("../shaders/standard.wgsl"));

            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Standard render pipeline layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &light_bundle.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            Self::create_render_pipeline(
                Some("Standard pipeline"),
                &device,
                &layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[model::ModelVertex::descriptor(), RawInstance::descriptor()],
                &shader,
                None,
            )
        };

        let light_render_pipeline = {
            let shader = device.create_shader_module(include_wgsl!("../shaders/light.wgsl"));
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Light pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bundle.bind_group_layout],
                push_constant_ranges: &[],
            });

            Self::create_render_pipeline(
                Some("Lighting pipeline"),
                &device,
                &layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[ModelVertex::descriptor()],
                &shader,
                None,
            )
        };

        let text_manager = ui::TextManager::new(&device, &queue, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,

            wireframe: false,

            model,
            instance_buffer,
            instances,

            depth_texture,

            camera,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,

            light_bundle,

            text_manager,

            standard_render_pipeline,
            light_render_pipeline,
            // pipelines: vec![],
            mouse_pressed: false,
        }
    }

    async fn initialize_surface(
        window: &Window,
    ) -> (
        Surface,
        PhysicalSize<u32>,
        Device,
        Queue,
        SurfaceConfiguration,
    ) {
        let size = window.inner_size();
        let backends = *supported_backends();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window).unwrap() };
        let adapter = instance
            .enumerate_adapters(backends)
            .map(|adapter| {
                println!("Discovered device: {}", adapter.get_info().name);

                adapter
            })
            .filter(|adapter| adapter.is_surface_supported(&surface))
            .next()
            .unwrap();
        println!("Selected device: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: Features::empty(),
                    limits: Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(TextureFormat::is_srgb)
            .next()
            .unwrap_or(surface_capabilities.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes.first().cloned().unwrap(),
            alpha_mode: surface_capabilities.alpha_modes.first().cloned().unwrap(),
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        (surface, size, device, queue, config)
    }

    fn initialize_texture(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Texture bind group layout"),
        })
    }

    fn initialize_instances(device: &Device) -> (Buffer, Vec<Instance>) {
        const SPACE_BETWEEN: f32 = 3.0;

        let instances = (0..INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - INSTANCES_PER_ROW as f32 / 2.0);

                    // Single centered model
                    // let position = Vector3::new(0.0, 0.0, 0.0);
                    // let rotation = Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), Deg(180.0));

                    // Many dispersed models
                    let position = Vector3::new(x, 0.0, z);
                    let rotation = if position.is_zero() {
                        Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
                    } else {
                        Quaternion::from_axis_angle(position.normalize(), Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data: Vec<RawInstance> = instances.iter().map(Instance::raw).collect();
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: BufferUsages::VERTEX,
        });

        (instance_buffer, instances)
    }

    fn initialize_camera(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> (
        Camera,
        Projection,
        CameraUniform,
        CameraController,
        Buffer,
        BindGroupLayout,
        BindGroup,
    ) {
        let speed = 8.0;
        let sensitivity = 1.0;

        let camera = Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = CameraController::new(speed, sensitivity);
        let camera_uniform = CameraUniform::new(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        (
            camera,
            projection,
            camera_uniform,
            camera_controller,
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
        )
    }

    fn create_render_pipeline(
        label: Option<&str>,
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: &ShaderModule,
        topology: Option<PrimitiveTopology>,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label,
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: topology.unwrap_or(PrimitiveTopology::TriangleList),
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = Texture::create_depth_texture(&self.device, &self.config);
            self.text_manager.resize(&self.config);
        }
    }

    fn handle_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => return self.camera_controller.handle_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => self.camera_controller.handle_scroll(delta),
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => self.mouse_pressed = state.is_pressed(),
            _ => return false,
        }

        true
    }

    fn update(&mut self, dt: Duration) {
        self.camera_controller.update(&mut self.camera, dt);
        self.camera_uniform.update(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera_uniform),
        );
        self.light_bundle.update(&self.queue);
        self.text_manager.resize(&self.config);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(CLEAR_COLOR),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.model,
                &self.camera_bind_group,
                &self.light_bundle.bind_group,
            );

            render_pass.set_pipeline(&self.standard_render_pipeline);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw_model_instanced(
                &self.model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
                &self.light_bundle.bind_group,
            );
        }

        self.text_manager
            .render(&self.device, &self.queue, &self.config, &mut encoder, &view);

        self.queue.submit(iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    // fn toggle_wirefame(&mut self) {
    //     self.wireframe = !self.wireframe;
    //     let topology = match self.wireframe {
    //         true => PrimitiveTopology::LineList,
    //         false => PrimitiveTopology::TriangleList,
    //     };

    //     let vertex = VertexState {
    //         module: &self.shader,
    //         entry_point: "vs_main",
    //         buffers: &[ModelVertex::descriptor(), RawInstance::descriptor()],
    //     };

    //     let fragment_targets = [Some(ColorTargetState {
    //         format: self.config.format,
    //         blend: Some(BlendState::REPLACE),
    //         write_mask: ColorWrites::ALL,
    //     })];
    //     let fragment = Some(FragmentState {
    //         module: &self.shader,
    //         entry_point: "fs_main",
    //         targets: &fragment_targets,
    //     });

    //     let primitive = PrimitiveState {
    //         topology,
    //         strip_index_format: None,
    //         front_face: FrontFace::Ccw,
    //         cull_mode: Some(Face::Back),
    //         unclipped_depth: false,
    //         polygon_mode: PolygonMode::Fill,
    //         conservative: false,
    //     };

    //     let multisample = MultisampleState {
    //         count: 1,
    //         mask: !0,
    //         alpha_to_coverage_enabled: false,
    //     };

    //     self.render_pipeline = self
    //         .device
    //         .create_render_pipeline(&RenderPipelineDescriptor {
    //             label: Some("Render pipeline"),
    //             layout: Some(&self.render_pipeline_layout),
    //             vertex,
    //             fragment,
    //             primitive,
    //             depth_stencil: None,
    //             multisample,
    //             multiview: None,
    //         });
    // }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct PrimitiveVertex {
    position: Vector3<f32>,
    texture_coordinates: Vector2<f32>,
}

impl VertexBufferFormat for PrimitiveVertex {
    type Attributes = [VertexAttribute; 2];
    const ATTRIBUTES: Self::Attributes = vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2
    ];

    fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct Instance {
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
}

impl Instance {
    fn raw(&self) -> RawInstance {
        RawInstance {
            model: (Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)).into(),
            normal: Matrix3::from(self.rotation).into(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RawInstance {
    model: Matrix4<f32>,
    normal: Matrix3<f32>,
}

impl VertexBufferFormat for RawInstance {
    type Attributes = [VertexAttribute; 7];
    const ATTRIBUTES: Self::Attributes = vertex_attr_array![
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x3,
        10 => Float32x3,
        11 => Float32x3,
    ];

    fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

mod ui {
    use std::iter;

    use glyphon::{
        Attrs, Buffer, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
        TextArea, TextAtlas, TextBounds, TextRenderer,
    };
    use wgpu::{
        CommandEncoder, CommandEncoderDescriptor, Device, LoadOp, MultisampleState, Operations,
        Queue, RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor, StoreOp, Surface, SurfaceConfiguration, TextureView,
        TextureViewDescriptor,
    };

    pub struct TextManager {
        font_system: FontSystem,
        cache: SwashCache,
        pub atlas: TextAtlas,
        pub renderer: TextRenderer,
        buffer: Buffer,
    }

    impl TextManager {
        const DEFAULT_FONT: &'static [u8] = include_bytes!("../res/Inter-Bold.ttf");
        const SCALE: f32 = 0.5;

        pub fn new(device: &Device, queue: &Queue, config: &SurfaceConfiguration) -> Self {
            let mut font_system = FontSystem::new();
            let cache = SwashCache::new();
            let mut atlas = TextAtlas::new(device, queue, config.format);
            let renderer = TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
            let mut buffer = Buffer::new(&mut font_system, Metrics::new(30.0, 42.0));

            let width = config.width as f32 * Self::SCALE;
            let height = config.height as f32 * Self::SCALE;

            buffer.set_size(&mut font_system, width, height);
            buffer.set_text(
                &mut font_system,
                "ahoy there",
                Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );
            buffer.shape_until_scroll(&mut font_system);

            Self {
                font_system,
                cache,
                atlas,
                renderer,
                buffer,
            }
        }

        pub fn update(&mut self, message: &str) {
            self.buffer.set_text(
                &mut self.font_system,
                message,
                Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );
        }

        pub fn resize(&mut self, config: &SurfaceConfiguration) {
            self.buffer.set_size(
                &mut self.font_system,
                config.width as f32 * Self::SCALE,
                config.height as f32 * Self::SCALE,
            );
        }

        pub fn render(
            &mut self,
            device: &Device,
            queue: &Queue,
            config: &SurfaceConfiguration,
            encoder: &mut CommandEncoder,
            view: &TextureView,
        ) {
            self.renderer
                .prepare(
                    device,
                    queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    Resolution {
                        width: config.width,
                        height: config.height,
                    },
                    [TextArea {
                        buffer: &self.buffer,
                        left: 10.0,
                        top: 10.0,
                        scale: 1.0,
                        bounds: TextBounds {
                            left: 0,
                            top: 0,
                            right: 600,
                            bottom: 160,
                        },
                        default_color: Color::rgb(255, 255, 255),
                    }],
                    &mut self.cache,
                )
                .unwrap();

            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                self.renderer.render(&self.atlas, &mut pass).unwrap();
            }

            self.atlas.trim();
        }
    }
}

mod util;
