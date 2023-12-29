use wgpu::{
    Device, PipelineLayout, PrimitiveTopology, RenderPipeline, ShaderModule, TextureFormat,
    VertexBufferLayout,
};

pub struct Pipeline {
    shader: ShaderModule,
    layout: PipelineLayout,
    inner: RenderPipeline,
}

pub struct PipelineOptions<'a> {
    label: Option<&'a str>,
    device: &'a Device,
    layout: PipelineLayout,
    color_format: TextureFormat,
    depth_format: Option<TextureFormat>,
    vertex_layouts: &'a [VertexBufferLayout<'a>],
    shader: ShaderModule,
    topology: Option<PrimitiveTopology>,
}

impl Pipeline {
    pub fn new<'a>(
        PipelineOptions {
            label,
            device,
            layout,
            color_format,
            depth_format,
            vertex_layouts,
            shader,
            topology,
        }: PipelineOptions<'a>,
    ) -> Self {
        let inner = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label,
            layout: Some(&layout),
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
        });

        Self {
            shader,
            layout,
            inner,
        }
    }
}
