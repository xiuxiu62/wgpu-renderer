use cgmath::{InnerSpace, Matrix4, Point3, SquareMatrix, Vector3};
use math::vec::Vec3;
use wgpu::{util::DeviceExt, ShaderModule};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

pub mod math;

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

const VERTICES: &[Vertex] = &[
    vertex!([-0.0868241, 0.49240386, 0.0], [0.5, 0.0, 0.5]), // A
    vertex!([-0.49513406, 0.06958647, 0.0], [0.5, 0.0, 0.5]), // B
    vertex!([0.44147372, 0.2347359, 0.0], [0.5, 0.0, 0.5]),  // E
    vertex!([-0.49513406, 0.06958647, 0.0], [0.5, 0.0, 0.5]), // B
    vertex!([-0.21918549, -0.44939706, 0.0], [0.5, 0.0, 0.5]), // C
    vertex!([0.44147372, 0.2347359, 0.0], [0.5, 0.0, 0.5]),  // E
    vertex!([-0.21918549, -0.44939706, 0.0], [0.5, 0.0, 0.5]), // C
    vertex!([0.35966998, -0.3473291, 0.0], [0.5, 0.0, 0.5]), // D
    vertex!([0.44147372, 0.2347359, 0.0], [0.5, 0.0, 0.5]),  // E
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    0, 1, 4, 
    1, 2, 4, 
    2, 3, 4
];

#[macro_export]
macro_rules! vertex {
    ([$x:expr, $y:expr, $z:expr], [$r:expr, $g:expr, $b:expr]) => {
        Vertex {
            position: vec3!($x, $y, $z),
            color: vec3!($r, $g, $b),
        }
    };
}

#[pollster::main]
async fn main() {
    run().await;
}

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut graphics_state = GraphicsState::new(window).await;

    event_loop
        .run(move |event, target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == graphics_state.window().id() => {
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
                    WindowEvent::KeyboardInput { event, .. } => graphics_state.handle_input(event),
                    // WindowEvent::KeyboardInput {
                    //     event:
                    //         KeyEvent {
                    //             state: ElementState::Released,
                    //             physical_key: PhysicalKey::Code(KeyCode::KeyO),
                    //             ..
                    //         },
                    //     ..
                    // } => graphics_state.toggle_wireframe(),
                    WindowEvent::Resized(size) => graphics_state.resize(*size),
                    // WindowEvent::ScaleFactorChanged {
                    //     inner_size_writer: size,
                    //     ..
                    // } => graphics_state.resize(*size),
                    WindowEvent::RedrawRequested => {
                        graphics_state.update();
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
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,

    wireframe: bool,

    render_pipeline: wgpu::RenderPipeline,
    render_pipeline_layout: wgpu::PipelineLayout,
    shader: ShaderModule,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,

    index_buffer: wgpu::Buffer,
    index_count: u32,

    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
}

impl GraphicsState {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12 | wgpu::Backends::METAL,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window).unwrap() };
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .filter(|adapter| {
                // Check if this adapter supports our surface
                adapter.is_surface_supported(&surface)
            })
            .next()
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(wgpu::TextureFormat::is_srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes.first().cloned().unwrap(),
            alpha_mode: surface_caps.alpha_modes.first().cloned().unwrap(),
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let camera_controller = CameraController::new(0.2);

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/triangle.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::descriptor()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
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

            vertex_buffer,
            vertex_count: VERTICES.len() as u32,
            index_buffer,
            index_count: INDICES.len() as u32,

            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
        }
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
        }
    }

    fn handle_input(&mut self, event: &KeyEvent) {
        self.camera_controller.handle_input(event);
    }

    fn update(&mut self) {
        self.camera_controller.update(&mut self.camera);
        self.camera_uniform.update(&self.camera);
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
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw(0..self.vertex_count, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    // fn toggle_wireframe(&mut self) {
    //     self.wireframe = !self.wireframe;
    //     self.render_pipeline =
    //         self.device
    //             .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    //                 label: Some("Render Pipeline"),
    //                 layout: Some(&self.render_pipeline_layout),
    //                 vertex: wgpu::VertexState {
    //                     module: &self.shader,
    //                     entry_point: "vs_main",
    //                     buffers: &[],
    //                 },
    //                 fragment: Some(wgpu::FragmentState {
    //                     module: &self.shader,
    //                     entry_point: "fs_main",
    //                     targets: &[Some(wgpu::ColorTargetState {
    //                         format: self.config.format,
    //                         blend: Some(wgpu::BlendState::REPLACE),
    //                         write_mask: wgpu::ColorWrites::ALL,
    //                     })],
    //                 }),
    //                 primitive: wgpu::PrimitiveState {
    //                     topology: wgpu::PrimitiveTopology::TriangleList,
    //                     strip_index_format: None,
    //                     front_face: wgpu::FrontFace::Ccw,
    //                     cull_mode: Some(wgpu::Face::Back),
    //                     unclipped_depth: false,
    //                     polygon_mode: match self.wireframe {
    //                         true => wgpu::PolygonMode::Line,
    //                         false => wgpu::PolygonMode::Fill,
    //                     },

    //                     conservative: false,
    //                 },
    //                 depth_stencil: None,
    //                 multisample: wgpu::MultisampleState {
    //                     count: 1,
    //                     mask: !0,
    //                     alpha_to_coverage_enabled: false,
    //                 },
    //                 multiview: None,
    //             });
    // }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: Vec3,
    color: Vec3,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_projection: Matrix4::identity().into(),
        }
    }

    fn update(&mut self, camera: &Camera) {
        self.view_projection = camera.view_projection().into();
    }
}

struct Camera {
    eye: Point3<f32>,
    target: Point3<f32>,
    up: Vector3<f32>,
    aspect: f32,
    fovy: f32,
    z_near: f32,
    z_far: f32,
}

impl Camera {
    fn view_projection(&self) -> Matrix4<f32> {
        let view = Matrix4::look_to_rh(
            self.eye,
            Vector3::new(self.target.x, self.target.y, self.target.z),
            self.up,
        );
        let projection =
            cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.z_near, self.z_far);

        view * projection * OPENGL_TO_WGPU_MATRIX
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn handle_input(&mut self, event: &KeyEvent) {
        match event {
            KeyEvent {
                state: ElementState::Pressed,
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                ..
            } => self.is_forward_pressed = true,

            KeyEvent {
                state: ElementState::Pressed,
                physical_key: PhysicalKey::Code(KeyCode::KeyS),
                ..
            } => self.is_forward_pressed = true,
            KeyEvent {
                state: ElementState::Pressed,
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                ..
            } => self.is_forward_pressed = true,
            KeyEvent {
                state: ElementState::Pressed,
                physical_key: PhysicalKey::Code(KeyCode::KeyD),
                ..
            } => self.is_forward_pressed = true,
            KeyEvent {
                state: ElementState::Released,
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                ..
            } => self.is_forward_pressed = false,
            KeyEvent {
                state: ElementState::Released,
                physical_key: PhysicalKey::Code(KeyCode::KeyS),
                ..
            } => self.is_backward_pressed = false,
            KeyEvent {
                state: ElementState::Released,
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                ..
            } => self.is_left_pressed = false,
            KeyEvent {
                state: ElementState::Released,
                physical_key: PhysicalKey::Code(KeyCode::KeyD),
                ..
            } => self.is_right_pressed = false,
            _ => {}
        }
    }

    pub fn update(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_normalized = forward.normalize();
        let forward_magnitude = forward.magnitude();

        if self.is_forward_pressed && forward_magnitude > self.speed {
            camera.eye += forward_normalized * self.speed;
        }

        if self.is_backward_pressed {
            camera.eye -= forward_normalized * self.speed;
        }

        let right = forward_normalized.cross(camera.up);
        let forward = camera.target - camera.eye;
        let forward_magnitude = forward.magnitude();

        if self.is_right_pressed {
            camera.eye =
                camera.target - (forward + right * self.speed).normalize() * forward_magnitude;
        }

        if self.is_left_pressed {
            camera.eye =
                camera.target - (forward - right * self.speed).normalize() * forward_magnitude;
        }
    }
}
