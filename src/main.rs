use clap::Parser;

use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::event::{DeviceEvent, DeviceId, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowId};
use winit::{application::ApplicationHandler, event::MouseScrollDelta};

pub mod content;
use content::Engine;

pub mod utils;
use utils::{Location, Vertex, filer::Filer, print::*};
pub mod compiler;
use compiler::compile;

use crate::utils::lora::LoraCommandContext;

const RESOLUTION: f32 = 100.;

#[derive(Parser, Debug)]
#[command(name = "lora")]
#[command(
    about = "A rust-based framework for Lua games!",
    long_about = "A rust-based framework that allows you to create any game in Lua with the lora API!"
)]
pub struct Args {
    #[arg(
        short,
        long,
        help = "Enable test mode",
        long_help = "Enables testing for github actions.\nWhen enabled, exits at the end of lora.render() and will require all lora functions to be present in lua code."
    )]
    test: bool,

    #[arg(short, long, help = "Enable verbose output")]
    verbose: bool,

    #[arg(long)]
    devbug: bool,

    #[arg(long, conflicts_with = "filepath")]
    compile: Option<String>,

    filepath: Option<String>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, serde::Deserialize, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GPUView {
    pub time: [f32; 2],
    pub scale: [f32; 2],
    pub position: [f32; 2],
    pub rotation: [f32; 2],
}

impl GPUView {
    pub fn new() -> Self {
        Self {
            time: [0., 0.],
            scale: [1., 1.],
            position: [0., 0.],
            rotation: [0., 0.],
        }
    }
}

struct State {
    argus: Args,

    current_time: chrono::DateTime<chrono::Utc>,
    last_time: chrono::DateTime<chrono::Utc>,
    delta: chrono::TimeDelta,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    msaa_view: wgpu::TextureView,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    primitive_pipeline: wgpu::RenderPipeline,

    window: Arc<Window>,
    gpu_view: GPUView,
    gpu_view_buffer: wgpu::Buffer,
    gpu_view_bind_group: wgpu::BindGroup,

    texture_bind_layout: wgpu::BindGroupLayout,

    primitive_buffer: wgpu::Buffer,
    primitive_bind_group: wgpu::BindGroup,
    engine: Engine,
}

impl State {
    async fn new(window: Arc<Window>, argus: Args, filer: Filer) -> State {
        let mut delta = chrono::TimeDelta::new(0, 10_000_000).unwrap();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            backend_options: wgpu::BackendOptions::default(),
            display: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("WGPU Device and Adapter"),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .unwrap();

        let mut size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0].add_srgb_suffix();

        let mut gpu_view: GPUView = GPUView::new();
        gpu_view.scale = [
            size.width as f32 / RESOLUTION,
            size.height as f32 / RESOLUTION,
        ];

        let gpu_view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Viewport Buffer"),
            contents: bytemuck::cast_slice(&[gpu_view]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let gpu_view_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Viewport Bind Group Layout"),
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

        let gpu_view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Viewport Bind Group"),
            layout: &gpu_view_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: gpu_view_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Lora Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
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
            });

        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa color texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let raster_shader =
            device.create_shader_module(wgpu::include_wgsl!("./shaders/main.wgsl").into());
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Layout for Primary Render Pipeline"),
                bind_group_layouts: &[Some(&gpu_view_bind_layout), Some(&texture_bind_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primary Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &raster_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc()), Some(Location::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &raster_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // cull_mode: Some(wgpu::Face::Front),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let primitive_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Primitive Buffer"),
            size: (12304) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let primitive_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primitives Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let primitive_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primitives Bind Group"),
            layout: &primitive_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: primitive_buffer.as_entire_binding(),
            }],
        });

        let primitive_shader =
            device.create_shader_module(wgpu::include_wgsl!("./shaders/prim.wgsl").into());
        let primitive_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Layout for Primitive Render Pipeline"),
                bind_group_layouts: &[Some(primitive_bind_layout).as_ref()],
                immediate_size: 0,
            });

        let primitive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primary Render Pipeline"),
            layout: Some(&primitive_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &primitive_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &primitive_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // cull_mode: Some(wgpu::Face::Front),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let ctx = &mut LoraCommandContext {
            device: &device,
            queue: &queue,
            window: window.as_ref(),
            texture_bind_layout: &texture_bind_layout,
            gpu_view: &mut gpu_view,
            delta: &mut delta,
            size: &mut size,
        };
        let engine = Engine::new(filer, argus.verbose, ctx);

        let mut state = State {
            argus,

            current_time: chrono::Utc::now(),
            last_time: chrono::Utc::now(),
            delta,
            surface,
            surface_format,
            msaa_view,
            device,
            queue,
            size,
            render_pipeline,
            primitive_pipeline,

            window,
            gpu_view,
            gpu_view_buffer,
            gpu_view_bind_group,

            texture_bind_layout,

            primitive_buffer,
            primitive_bind_group,

            engine,
        };

        state.configure_surface();
        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&mut self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            color_space: wgpu::SurfaceColorSpace::default(),
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoNoVsync,
        };
        self.surface.configure(&self.device, &surface_config);

        let msaa_texture = &self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width: self.size.width,
                height: self.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.configure_surface();
        let size: [u32; 2] = [self.size.width, self.size.height];
        self.gpu_view.scale = [size[0] as f32 / RESOLUTION, size[1] as f32 / RESOLUTION];
    }

    fn keyboard_inputs(&mut self, key: String, state: bool) {
        let ctx = &mut LoraCommandContext {
            device: &self.device,
            queue: &self.queue,
            window: self.window.as_ref(),
            texture_bind_layout: &self.texture_bind_layout,
            gpu_view: &mut self.gpu_view,
            delta: &mut self.delta,
            size: &mut self.size,
        };
        self.engine.keyboard_inputs(state, key, ctx);
    }

    fn mouse_button_inputs(&mut self, button: MouseButton, state: bool) {
        let ctx = &mut LoraCommandContext {
            device: &self.device,
            queue: &self.queue,
            window: self.window.as_ref(),
            texture_bind_layout: &self.texture_bind_layout,
            gpu_view: &mut self.gpu_view,
            delta: &mut self.delta,
            size: &mut self.size,
        };
        self.engine.mouse_button_inputs(button, state, ctx);
    }

    fn mouse_movement_inputs(&mut self, motion: (f64, f64)) {
        let ctx = &mut LoraCommandContext {
            device: &self.device,
            queue: &self.queue,
            window: self.window.as_ref(),
            texture_bind_layout: &self.texture_bind_layout,
            gpu_view: &mut self.gpu_view,
            delta: &mut self.delta,
            size: &mut self.size,
        };
        self.engine.mouse_movement_inputs(motion, ctx);
    }

    fn mouse_scroll_inputs(&mut self, delta: MouseScrollDelta) {
        let ctx = &mut LoraCommandContext {
            device: &self.device,
            queue: &self.queue,
            window: self.window.as_ref(),
            texture_bind_layout: &self.texture_bind_layout,
            gpu_view: &mut self.gpu_view,
            delta: &mut self.delta,
            size: &mut self.size,
        };
        self.engine.mouse_scroll_inputs(delta, ctx);
    }

    fn render(&mut self) {
        self.current_time = chrono::Utc::now();

        let surface_texture = self.surface.get_current_texture();

        let pretexture_view = match surface_texture {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            _ => return,
        };
        let texture_view = pretexture_view
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa_view,
                depth_slice: None,
                resolve_target: Some(&texture_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        self.gpu_view.time[0] = chrono::Utc::now()
            .signed_duration_since(self.current_time)
            .as_seconds_f32();
        self.gpu_view.time[1] = chrono::Utc::now()
            .signed_duration_since(self.last_time)
            .as_seconds_f32();
        self.queue.write_buffer(
            &self.gpu_view_buffer,
            0,
            bytemuck::bytes_of(&[self.gpu_view]),
        );

        renderpass.set_pipeline(&self.render_pipeline);
        let ctx = &mut LoraCommandContext {
            device: &self.device,
            queue: &self.queue,
            window: self.window.as_ref(),
            texture_bind_layout: &self.texture_bind_layout,
            gpu_view: &mut self.gpu_view,
            delta: &mut self.delta,
            size: &mut self.size,
        };
        renderpass.set_bind_group(0, &self.gpu_view_bind_group, &[]);
        self.engine
            .object_prerender(&mut renderpass, self.current_time, ctx);

        renderpass.set_pipeline(&self.primitive_pipeline);
        renderpass.set_bind_group(0, &self.primitive_bind_group, &[]);
        self.engine.primitive_prerender(&self.primitive_buffer, ctx);

        renderpass.draw(0..3, 0..1);

        drop(renderpass);

        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(pretexture_view);

        self.last_time = self.current_time;
    }

    fn exit(&mut self) {
        let ctx = &mut LoraCommandContext {
            device: &self.device,
            queue: &self.queue,
            window: self.window.as_ref(),
            texture_bind_layout: &self.texture_bind_layout,
            gpu_view: &mut self.gpu_view,
            delta: &mut self.delta,
            size: &mut self.size,
        };
        self.engine.exit(ctx);
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let argus: Args = Args::parse();
        let filer: Filer = Filer::new(&argus.filepath);

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title(filer.read_name()))
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone(), argus, filer));
        self.state = Some(state);

        window.request_redraw();
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let superstate = self.state.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                infoln("Closing application by request...");
                superstate.exit();
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                superstate.render();
                superstate.get_window().request_redraw();
                if superstate.argus.test {
                    infoln("Closing application after successful test...");
                    superstate.exit();
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                superstate.resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let newtext: String = event
                    .key_without_modifiers()
                    .to_text()
                    .unwrap_or_else(|| "NONE")
                    .to_string();
                superstate.keyboard_inputs(newtext, event.state.is_pressed());
            }
            WindowEvent::MouseInput { state, button, .. } => {
                superstate.mouse_button_inputs(button, state.is_pressed());
            }
            WindowEvent::CursorMoved { position, .. } => {
                superstate.engine.mouse = (position.x as f32, position.y as f32);
            }
            _ => (),
        }
    }
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let superstate = self.state.as_mut().unwrap();

        match event {
            DeviceEvent::MouseMotion { delta } => {
                superstate.mouse_movement_inputs(delta);
            }
            DeviceEvent::MouseWheel { delta } => {
                superstate.mouse_scroll_inputs(delta);
            }
            _ => {}
        }
    }
}

fn main() {
    let argus: Args = Args::parse();
    if argus.compile.is_some() {
        compile(argus.compile.unwrap());
    } else {
        let events = EventLoop::new().unwrap();
        events.set_control_flow(ControlFlow::Poll);

        let mut app = App::default();
        match events.run_app(&mut app) {
            Ok(()) => infoln("Exited successfully."),
            Err(error) => serorln(format!("Exited with an error:\n {error:?}")),
        }
    }
}
