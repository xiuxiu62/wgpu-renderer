use bytemuck::{Pod, Zeroable};
use camera::{Camera, CameraController, CameraUniform, Projection};
use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rotation3, Vector3, Zero};
use math::vec::{Vec2, Vec3};
use model::{DrawModel, Model, ModelVertex, VertexBufferFormat};
use std::{
    iter,
    sync::OnceLock,
    time::{Duration, Instant},
};
use texture::Texture;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent, BlendState,
    Buffer, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompareFunction, DepthBiasState, Device, Face, Features,
    FragmentState, FrontFace, Limits, LoadOp, MultisampleState, Operations, PipelineLayout,
    PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderStages, StencilState, StoreOp, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexState, VertexStepMode,
};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

mod camera;
mod math;
mod model;
mod texture;

#[inline]
fn supported_backends() -> &'static Backends {
    static BACKENDS: OnceLock<Backends> = OnceLock::new();

    BACKENDS.get_or_init(|| Backends::VULKAN | Backends::DX12 | Backends::METAL)
}

const INSTANCES_PER_ROW: u32 = 10;
// const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(
//     INSTANCES_PER_ROW as f32 * 0.5,
//     0.0,
//     INSTANCES_PER_ROW as f32 * 0.5,
// );

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

// const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, /* padding */ 0];
// const VERTICES: &[Vertex] = &[
//     vertex!([-0.0868241, 0.49240386, 0.0], [0.4131759, 0.99240386]), // A
//     vertex!([-0.49513406, 0.06958647, 0.0], [0.0048659444, 0.56958647]), // B
//     vertex!([-0.21918549, -0.44939706, 0.0], [0.28081453, 0.05060294]), // C
//     vertex!([0.35966998, -0.3473291, 0.0], [0.85967, 0.1526709]),    // D
//     vertex!([0.44147372, 0.2347359, 0.0], [0.9414737, 0.7347359]),   // E
// ];

#[macro_export]
macro_rules! vertex {
    ([$x:expr, $y:expr, $z:expr], [$xt:expr, $yt:expr]) => {
        Vertex {
            position: vec3!($x, $y, $z),
            texture_coordinates: vec2!($xt, $yt),
        }
    };
}

#[pollster::main]
async fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut graphics_state = GraphicsState::new(window).await;
    let mut previous_render_time = Instant::now();

    event_loop
        .run(move |event, target| match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                ..
            } if graphics_state.mouse_pressed => {
                graphics_state.camera_controller.handle_mouse(dx, dy)
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

                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Released,
                                physical_key: PhysicalKey::Code(KeyCode::KeyP),
                                ..
                            },
                        ..
                    } => graphics_state.toggle_wirefame(),
                    WindowEvent::Resized(size) => graphics_state.resize(*size),
                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let dt = Instant::now() - previous_render_time;
                        previous_render_time = now;
                        graphics_state.update(dt);

                        match graphics_state.render() {
                            Ok(()) => {}
                            Err(wgpu::SurfaceError::Lost) => {
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

    render_pipeline: RenderPipeline,
    render_pipeline_layout: PipelineLayout,
    shader: ShaderModule,

    model: Model,
    instance_buffer: Buffer,
    instances: Vec<Instance>,

    depth_texture: Texture,

    camera: Camera,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    mouse_pressed: bool,
}

impl GraphicsState {
    async fn new(window: Window) -> Self {
        let (surface, size, device, queue, config) = Self::initialize_surface(&window).await;
        // let (_texture_bind_group, texture_bind_group_layout) =
        //     Self::intialize_texture(&device, &queue);
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

        // let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Vertex Buffer"),
        //     contents: bytemuck::cast_slice(VERTICES),
        //     usage: wgpu::BufferUsages::VERTEX,
        // });
        // let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Index Buffer"),
        //     contents: bytemuck::cast_slice(INDICES),
        //     usage: wgpu::BufferUsages::INDEX,
        // });

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/standard.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[ModelVertex::descriptor(), RawInstance::descriptor()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,

            wireframe: false,

            render_pipeline,
            render_pipeline_layout,
            shader,

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
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
    }

    // fn intialize_texture(device: &Device, queue: &Queue) -> (BindGroup, BindGroupLayout) {
    //     // fn intialize_texture(device: &Device, queue: &Queue) -> BindGroupLayout {
    //     let bytes = include_bytes!("../assets/happy-tree.png");
    //     let image = image::load_from_memory(bytes).unwrap();
    //     let rgba = image.to_rgba8();
    //     let dimensions = image.dimensions();
    //     let size = Extent3d {
    //         width: dimensions.0,
    //         height: dimensions.1,
    //         depth_or_array_layers: 1,
    //     };
    //     let texture = device.create_texture(&TextureDescriptor {
    //         label: Some("Diffuse Texture"),
    //         size,
    //         mip_level_count: 1,
    //         sample_count: 1,
    //         dimension: TextureDimension::D2,
    //         format: TextureFormat::Rgba8UnormSrgb,
    //         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    //         view_formats: &[],
    //     });

    //     queue.write_texture(
    //         ImageCopyTexture {
    //             texture: &texture,
    //             mip_level: 0,
    //             origin: Origin3d::ZERO,
    //             aspect: TextureAspect::All,
    //         },
    //         &rgba,
    //         ImageDataLayout {
    //             offset: 0,
    //             bytes_per_row: Some(4 * dimensions.0),
    //             rows_per_image: Some(dimensions.1),
    //         },
    //         size,
    //     );

    //     let view = texture.create_view(&TextureViewDescriptor::default());
    //     let sampler = device.create_sampler(&SamplerDescriptor {
    //         address_mode_u: AddressMode::ClampToEdge,
    //         address_mode_v: AddressMode::ClampToEdge,
    //         address_mode_w: AddressMode::ClampToEdge,
    //         mag_filter: FilterMode::Linear,
    //         min_filter: FilterMode::Nearest,
    //         mipmap_filter: FilterMode::Nearest,
    //         ..Default::default()
    //     });

    //     let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    //         label: Some("Texture bind group layout"),
    //         entries: &[
    //             BindGroupLayoutEntry {
    //                 binding: 0,
    //                 visibility: ShaderStages::FRAGMENT,
    //                 ty: BindingType::Texture {
    //                     sample_type: TextureSampleType::Float { filterable: true },
    //                     view_dimension: TextureViewDimension::D2,
    //                     multisampled: false,
    //                 },
    //                 count: None,
    //             },
    //             BindGroupLayoutEntry {
    //                 binding: 1,
    //                 visibility: ShaderStages::FRAGMENT,
    //                 ty: BindingType::Sampler(SamplerBindingType::Filtering),
    //                 count: None,
    //             },
    //         ],
    //     });
    //     let bind_group = device.create_bind_group(&BindGroupDescriptor {
    //         label: Some("Texture bind group"),
    //         layout: &bind_group_layout,
    //         entries: &[
    //             wgpu::BindGroupEntry {
    //                 binding: 0,
    //                 resource: BindingResource::TextureView(&view),
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 1,
    //                 resource: BindingResource::Sampler(&sampler),
    //             },
    //         ],
    //     });

    //     (bind_group, bind_group_layout)
    //     // bind_group_layout
    // }

    fn initialize_instances(device: &Device) -> (Buffer, Vec<Instance>) {
        const SPACE_BETWEEN: f32 = 3.0;

        let instances = (0..INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - INSTANCES_PER_ROW as f32 / 2.0);

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
        let camera = Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = CameraController::new(4.0, 0.4);
        let camera_uniform = CameraUniform::new(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
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
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
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
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw_model_instanced(
                &self.model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
            );
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn toggle_wirefame(&mut self) {
        self.wireframe = !self.wireframe;
        let topology = match self.wireframe {
            true => PrimitiveTopology::LineList,
            false => PrimitiveTopology::TriangleList,
        };

        let vertex = VertexState {
            module: &self.shader,
            entry_point: "vs_main",
            buffers: &[ModelVertex::descriptor(), RawInstance::descriptor()],
        };

        let fragment_targets = [Some(ColorTargetState {
            format: self.config.format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];
        let fragment = Some(FragmentState {
            module: &self.shader,
            entry_point: "fs_main",
            targets: &fragment_targets,
        });

        let primitive = PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        };

        let multisample = MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        self.render_pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Render pipeline"),
                layout: Some(&self.render_pipeline_layout),
                vertex,
                fragment,
                primitive,
                depth_stencil: None,
                multisample,
                multiview: None,
            });
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct PrimitiveVertex {
    position: Vec3,
    texture_coordinates: Vec2,
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
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RawInstance {
    model: [[f32; 4]; 4],
}

impl VertexBufferFormat for RawInstance {
    type Attributes = [VertexAttribute; 4];
    const ATTRIBUTES: Self::Attributes = vertex_attr_array![
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
    ];

    fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
