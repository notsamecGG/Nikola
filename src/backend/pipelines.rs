use std::rc::Rc;
use wgpu::util::DeviceExt;

use crate::backend::Shader;

use crate::backend::state::*;
use crate::backend::binding;
use crate::backend::Size;

use super::FORMAT;


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    pub fn new(data: [f32; 5]) -> Self {
        Vertex {
            position: data[0..3].try_into().unwrap(),
            uv: data[3..5].try_into().unwrap(),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ],
        }
    }
}

pub struct Rect {
    vertices: [Vertex; 4],
    indices: [u16; 6]
}

const RECT: Rect = Rect {
    vertices: [
        Vertex { position: [ 1.,  1., 0.], uv: [1., 1.]},
        Vertex { position: [-1., -1., 0.], uv: [0., 0.]},
        Vertex { position: [ 1., -1., 0.], uv: [1., 0.]},
        Vertex { position: [-1.,  1., 0.], uv: [0., 1.]},
    ],
    indices: [
        0, 1, 2,
        0, 3, 1
    ],
};

pub struct RenderPipeline {
    texture: binding::Texture,
    _vertex: Shader,
    fragment: Shader,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    pipeline: wgpu::RenderPipeline,
    state: Rc<StateData>,
}

impl RenderPipeline {
    pub fn new(state: &State, vertex: Shader, mut fragment: Shader) -> Self {
        // setup the inputs
            // setup generic inputs
        let texture = state.create_texture(
            state.size, 
            wgpu::TextureUsages::STORAGE_BINDING | 
            wgpu::TextureUsages::TEXTURE_BINDING, 
            binding::Access::Both,
            false
        );

            // segup specific inputs
        fragment.create_sampler();

           // setup exceptional inputs
        let vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&RECT.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&RECT.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // bind the generic inputs
        fragment.add_entry(Box::new(texture.get_view(None)));

        // setup the pipeline 
        let layout = state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: None, 
            bind_group_layouts: &[
                fragment.get_layout().unwrap(),
            ], 
            push_constant_ranges: &[]
        });
        let pipeline = state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
            label: None, 
            layout: Some(&layout), 
            vertex: wgpu::VertexState { 
                module: vertex.get_module(), 
                entry_point: vertex.entry_point, 
                buffers: &[Vertex::desc()] // vertex description
            }, 
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // todo
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None, 
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(
                wgpu::FragmentState { 
                    module: fragment.get_module(), 
                    entry_point: fragment.entry_point, 
                    targets: &[Some(
                        wgpu::ColorTargetState {
                            format: FORMAT, // todo FORMAT?
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        }
                    )]
                },
            ),
            multiview: None,
        });

        RenderPipeline { 
            texture, 
            _vertex: vertex, 
            fragment, 
            vertex_buffer, 
            index_buffer, 
            pipeline, 
            state: state.get_state() 
        }
    }

    /// Get a handle to the render texture
    pub fn get_texture(&self, access: binding::Access, is_storage: bool) -> binding::Texture {
        self.texture.get_view(Some((access, binding::Dimension::D2, is_storage)))
    }

    /// Creates render pass with instructions to clear display in place
    fn begin_render_pass<'a>(encoder: &'a mut wgpu::CommandEncoder, view: &'a wgpu::TextureView) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { 
            label: Some("Render pipeline pass"), 
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment { 
                    view, 
                    resolve_target: None, 
                    ops: wgpu::Operations { 
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true, 
                    } 
                }),
            ], 
            depth_stencil_attachment: None,
        })
    }

    /// !!! Not fully implemented, may cause bugs (bind group missalignments)
    fn _resize(&mut self, size: Size<u32>) {
        let usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
        let new_texture = self.state.create_raw_texture(size, usage);

        unsafe {
            self.texture.swap_texture(new_texture);
        }

        self.fragment.refresh_binding();
        // todo: implement dynamic update of pipeline and its layout
    }

    /// Plot input texture onto the surface
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> { 
        let output = self.state.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render pipeline command encoder"),
        });

        {
            let mut render_pass = RenderPipeline::begin_render_pass(&mut encoder, &view);

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, self.fragment.get_bind_group().unwrap(), &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..2);
        }

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn render_with_ui(&mut self, renderer: &mut imgui_wgpu::Renderer, draw_data: &imgui::DrawData) -> Result<(), wgpu::SurfaceError> { 
        let output = self.state.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render pipeline command encoder"),
        });

        {
            let mut render_pass = RenderPipeline::begin_render_pass(&mut encoder, &view);

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, self.fragment.get_bind_group().unwrap(), &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..2);

            renderer
                .render(draw_data, &self.state.queue, &self.state.device, &mut render_pass)
                .expect("Rendering UI failed");
        }

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}



pub struct ComputePipeline {
    state: Rc<StateData>,
    pipeline: wgpu::ComputePipeline,

    shader: Shader,

    workgroup_size: Size<u32>, // size of single work group
    workgroups: Option<Size<u32>>, // work groups count
    size: Size<u32>,
    _size_z: Option<u32>,
}

impl ComputePipeline {
    pub fn new(state: &State, mut shader: Shader, size: Size<u32>, workgroup_size: Option<Size<u32>>) -> Self {
        let workgroup_size = workgroup_size.unwrap_or(Size { width: 8u32, height: 8u32 });

        let layout = state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: None, 
            bind_group_layouts: &[
                shader.get_layout().unwrap()
            ],
            push_constant_ranges: &[]
        });

        let pipeline = state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            module: shader.get_module(),
            entry_point: shader.entry_point,
        });

        let mut result = ComputePipeline { 
            state: state.get_state(), 
            pipeline, 
            shader, 
            workgroup_size,
            workgroups: None,
            size, 
            _size_z: None 
        };
        result.compute_workgroups();

        result
    }

    /// Regenerate the binding layout and pipeline
    fn refresh_binding(&mut self) {
        self.shader.refresh_binding();

        let layout = self.state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                self.shader.get_layout().unwrap()
            ],
            push_constant_ranges: &[]
        });
        let pipeline = self.state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { 
            label: None, 
            layout: Some(&layout), 
            module: self.shader.get_module(), 
            entry_point: self.shader.entry_point 
        });

        self.pipeline = pipeline;
    }

    /// Get the count of workgroups needed to be dispatched
    fn compute_workgroups(&mut self) {
        let workgroups = self.size.fit_other(self.workgroup_size);

        self.workgroups = Some(workgroups);
    }

    /// Resize size of this pipeline, ! keep in mind if you are using this pipeline to 
    /// render to texture you need to resize the texture first
    pub fn resize(&mut self, size: Size<u32>) { 
       self.size = size; 
       self.compute_workgroups();
       self.shader.refresh_binding();
       self.refresh_binding();
       // todo: implement TextureComputePipelines
    }

    /// Swap shader resources and refresh pipeline binding
    pub fn swap_resources(&mut self, first: usize, second: usize) {
        match self.shader.swap_resources(first, second) {
            Ok(_) => { self.refresh_binding() },
            Err(err) => eprintln!("{:?}", err),
        }
    }

    /// Execute the shader
    pub fn execute(&mut self) {
        let encoder = self.start_execute();
        self.state.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Start execution
    pub fn start_execute(&mut self) -> wgpu::CommandEncoder {
        let bind_group = self.shader.get_bind_group().unwrap();
        let mut encoder = self.state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { 
                label: None
            });

            let workgroups = self.workgroups.unwrap_or_else(|| {
                self.size.fit_other(self.workgroup_size)
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups.width, workgroups.height, 1);
        }

        encoder
    }
}
