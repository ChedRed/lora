use clap::Parser;
use crossbeam::{channel::{Receiver, Sender, bounded, unbounded}, select};
use std::{fs, process::exit, sync::Arc, thread::JoinHandle};
use rapier2d::prelude::*;
use winit::{application::ApplicationHandler, event::MouseScrollDelta};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, MouseButton, MouseScrollDelta::{LineDelta, PixelDelta}, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::SmolStr;
use winit::window::{Window, WindowId};
use wgpu::{naga::FastHashMap, util::DeviceExt};

pub mod content;
use content::{shape::LoriShape, spawner::LoriSpawner};
pub mod utils;
use crate::{content::{collider::{LoriCollider, LoriColliderRef}, shape::LoriShapeRef, spawner::{LoriObjectRef, LoriSpawnerRef}}, utils::{GPUPrimitives, Location, LoriToMainCall, LoriToMainCommand, MainToLoriCall, MainToLoriCommand, Primitive, Vertex, lori::Lori, print::*}};


#[derive(Parser, Debug)]
#[command(name = "lori")]
#[command(
    about="A rust-based framework for Lua games!",
    long_about="A rust-based framework that allows you to create any game in Lua with the lori API!")]
struct Args {
    #[arg(short, long,
        help="Enable test mode",
        long_help="Enables testing for github actions.\nWhen enabled, exits at the end of lori.render() and will require all lori functions to be present in lua code.")]
    test: bool,
    
    #[arg(short, long, help="Enable verbose output")]
    verbose: bool,

    #[arg(long)]
    devbug: bool,
    
    filepath: String,

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
    timestep: chrono::DateTime<chrono::Utc>,
    delta: chrono::TimeDelta,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    msaa_view: wgpu::TextureView,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    mouse: (f32, f32),
    keys: Vec<String>,
    render_pipeline: wgpu::RenderPipeline,
    primitive_pipeline: wgpu::RenderPipeline,

    lori_shapes: FastHashMap<u64, LoriShape>,
    shape_id: u64,
    lori_colliders: FastHashMap<u64, LoriCollider>,
    collider_id: u64,
    lori_spawners: FastHashMap<u64, LoriSpawner>,
    spawner_id: u64,
    primitives: Vec<Primitive>,

    window: Arc<Window>,
    gpu_view: GPUView,
    gpu_view_buffer: wgpu::Buffer,
    gpu_view_bind_group: wgpu::BindGroup,

    primitive_buffer: wgpu::Buffer,
    primitive_bind_group: wgpu::BindGroup,

    gravity: Vec2, // TODO: allow control from Lua
    integration_parameters: IntegrationParameters,
    physics: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    rigidbodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,

    lori_call: Sender<MainToLoriCall>,
    lori_back: Receiver<LoriToMainCall>,
    lori_cmd: Receiver<LoriToMainCommand>,
    lori_cmd_rev: Sender<LoriToMainCommand>,
    lori_rtrn: Sender<MainToLoriCommand>,
    lori_rtrn_rev: Receiver<MainToLoriCommand>,
    lori_handle: Option<JoinHandle<()>>,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let argus: Args = Args::parse();
        if argus.verbose {
            infoln("Verbose mode activated");
        }
        if argus.devbug {
            infoln("Debug mode activated");
        }

        let lua_code: String;
        match fs::read_to_string(&argus.filepath) {
            Ok(file) => {
                lua_code = file;
            }
            Err(e) => {
                errorln(&e);
                exit(e.raw_os_error().unwrap_or_default());
            }
        }
        
        let mouse: (f32, f32) = (0., 0.);
        let keys: Vec<String> = Vec::new();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            backend_options: wgpu::BackendOptions::default(),
            display: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();
        
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let (main_cmd, lori_cmd) = unbounded::<LoriToMainCommand>();
        let (lori_rtrn, main_rtrn) = unbounded::<MainToLoriCommand>();
        let (lori_call, main_call) = bounded::<MainToLoriCall>(0);
        let (main_back, lori_back) = bounded::<LoriToMainCall>(0);

        let lori_cmd_rev = main_cmd.clone();
        let lori_rtrn_rev = main_rtrn.clone();

        let mut lori: Lori = Lori::new(lua_code, argus.verbose, main_cmd, main_rtrn, main_call, main_back);
        let lori_handle = Some(std::thread::Builder::new()
            .name("lori".to_string())
            .spawn(move || { lori.begin(); }).unwrap());
        
        let lori_shapes: FastHashMap<u64, LoriShape> = FastHashMap::default();
        let shape_id: u64 = 0;
        let lori_colliders: FastHashMap<u64, LoriCollider> = FastHashMap::default();
        let collider_id: u64 = 0;
        let lori_spawners: FastHashMap<u64, LoriSpawner> = FastHashMap::default();
        let spawner_id: u64 = 0;

        let mut gpu_view: GPUView = GPUView::new();
        gpu_view.scale = [size.width as f32, size.height as f32];
        
        let gpu_view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Viewport Buffer"),
            contents: bytemuck::cast_slice(&[gpu_view]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let gpu_view_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Viewport Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });

        let gpu_view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Viewport Bind Group"),
            layout: &gpu_view_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu_view_buffer.as_entire_binding(),
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
        
        let raster_shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/main.wgsl").into());
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Layout for Primary Render Pipeline"),
            bind_group_layouts: &[Some(gpu_view_bind_layout).as_ref()],
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

        let primitives: Vec<Primitive> = Vec::with_capacity(200);

        let primitive_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Primitive Buffer"),
            size: ((12304)) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let primitive_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Primitives Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ]
        });
        
        let primitive_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primitives Bind Group"),
            layout: &primitive_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: primitive_buffer.as_entire_binding(),
            }]
        });
        
        let primitive_shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/prim.wgsl").into());
        let primitive_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Layout for Primary Render Pipeline"),
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

        let gravity = Vec2 { x: 0., y: 0. };
        let mut integration_parameters = IntegrationParameters::default();
        integration_parameters.dt = 1./100.;
        
        let physics = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = BroadPhaseBvh::new();
        let narrow_phase = NarrowPhase::new();
        let rigidbodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        let impulse_joints = ImpulseJointSet::new();
        let multibody_joints = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        let mut state = State {
            argus,

            current_time: chrono::Utc::now(),
            last_time: chrono::Utc::now(),
            timestep: chrono::Utc::now(),
            delta: chrono::TimeDelta::new(0, 10_000_000).unwrap(),
            surface,
            surface_format,
            msaa_view,
            device,
            queue,
            size,
            mouse,
            keys,
            render_pipeline,
            primitive_pipeline,

            lori_shapes,
            shape_id,
            lori_colliders,
            collider_id,
            lori_spawners,
            spawner_id,
            primitives,

            window,
            gpu_view,
            gpu_view_buffer,
            gpu_view_bind_group,

            primitive_buffer,
            primitive_bind_group,

            gravity,
            integration_parameters,
            physics,
            island_manager,
            broad_phase,
            narrow_phase,
            rigidbodies,
            colliders,
            impulse_joints,
            multibody_joints,
            ccd_solver,
            
            lori_call,
            lori_back,
            lori_cmd,
            lori_cmd_rev,
            lori_rtrn,
            lori_rtrn_rev,
            lori_handle,
        };

        _= state.lori_call.send(MainToLoriCall::Load);
        state.handle_lori_loop();
        
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
            view_formats: vec![self.surface_format.add_srgb_suffix()],
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
        self.gpu_view.scale = [size[0] as f32, size[1] as f32];
    }
    
    fn keyboard_inputs(&mut self, key: String, state: bool) {
        if state {
            self.keys.push(key.clone());
            _= self.lori_call.send(MainToLoriCall::Keypressed { code: key });
        } else {
            self.keys.retain(|k| k != &key);
            _= self.lori_call.send(MainToLoriCall::Keyreleased { code: key });
        }
        self.handle_lori_loop();
    }

    fn mouse_button_inputs(&mut self, button: MouseButton, state: bool) {
        let numerical_button: u32;
        match button {
            MouseButton::Left => {
                numerical_button = 1;
            }
            MouseButton::Right => {
                numerical_button = 2;
            }
            MouseButton::Middle => {
                numerical_button = 3;
            }
            MouseButton::Back => {
                numerical_button = 4;
            }
            MouseButton::Forward => {
                numerical_button = 5;
            }
            MouseButton::Other(num) => {
                numerical_button = (6+num) as u32;
            }
        }
        if state {
            _= self.lori_call.send(MainToLoriCall::Mousepressed { x: self.mouse.0, y: self.mouse.1, button: numerical_button });
        } else {
            _= self.lori_call.send(MainToLoriCall::Mousereleased { x: self.mouse.0, y: self.mouse.1, button: numerical_button });
        }
        self.handle_lori_loop();
    }

    fn mouse_movement_inputs(&mut self, motion: (f64, f64)) {
        let simple_motion: (f32, f32) = (motion.0 as f32, motion.1 as f32);
        _= self.lori_call.send(MainToLoriCall::MouseMoved { motion: simple_motion });
        self.handle_lori_loop();
    }

    fn mouse_scroll_inputs(&mut self, delta: MouseScrollDelta) {
        let simple_motion: (f32, f32);
        match delta {
            PixelDelta(position) => {
                simple_motion = (position.x as f32, position.y as f32);
            }
            LineDelta(x, y) => {
                simple_motion = (x, y);
            }
        }
        _= self.lori_call.send(MainToLoriCall::MouseScrolled { motion: simple_motion });
        self.handle_lori_loop();
    }

    fn render(&mut self) {
        self.current_time = chrono::Utc::now();

        while self.timestep < self.current_time {
            _= self.lori_call.send(MainToLoriCall::Update { delta: self.integration_parameters.dt });
            self.handle_lori_loop();
            
            self.physics.step(
                self.gravity,
                &self.integration_parameters,
                &mut self.island_manager,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.rigidbodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd_solver,
                &(),
                &(),
            );
            self.timestep += self.delta;
        }

        let surface_texture = self.surface.get_current_texture();
        
        let pretexture_view = match surface_texture {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            _ => return
        };
        let texture_view = pretexture_view.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_format.add_srgb_suffix()),
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

        renderpass.set_pipeline(&self.render_pipeline);
        
        self.gpu_view.time[0] = chrono::Utc::now().signed_duration_since(self.current_time).as_seconds_f32();
        self.gpu_view.time[1] = chrono::Utc::now().signed_duration_since(self.last_time).as_seconds_f32();
        self.queue.write_buffer(&self.gpu_view_buffer, 0, bytemuck::bytes_of(&[self.gpu_view]));

        renderpass.set_bind_group(0, &self.gpu_view_bind_group, &[]);
        
        for obj in self.lori_spawners.iter_mut() { // Draw each shape. TODO: move to separate function
            if obj.1.renderable() {
                if let Some(real_vertex_buffer) = &obj.1.vertex_buffer {
                    if let Some(real_location_buffer) = &obj.1.location_buffer {
                        if let Some(real_index_buffer) = &obj.1.index_buffer {
                            for item in obj.1.rigidhandles.iter() {
                                if let Some(body) = self.rigidbodies.get_mut(*item.1) {
                                    if let Some(loc) = obj.1.locations.get_mut(item.0) {
                                        let pose = body.position();
                                        let pos = pose.translation;
                                        let rot: f32 = pose.rotation.angle();
                                        loc.position = [pos.x, pos.y];
                                        loc.rotation = [rot, 0.];
                                        body.reset_forces(true);
                                    }
                                }
                            }
                            
                            if let Some(real_location_buffer) = &obj.1.location_buffer {
                                let locations: Vec<Location> = obj.1.locations.values().copied().collect();
                                self.queue.write_buffer(&real_location_buffer, 0, bytemuck::cast_slice(&locations));
                            }
                            renderpass.set_vertex_buffer(0, real_vertex_buffer.slice(..));
                            renderpass.set_vertex_buffer(1, real_location_buffer.slice(..));
                            renderpass.set_index_buffer(real_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                            renderpass.draw_indexed(0..obj.1.indices as u32, 0, 0..obj.1.locations.len() as _);
                        }
                    }
                }
            }
        }

        
        renderpass.set_pipeline(&self.primitive_pipeline);

        _= self.lori_call.send(MainToLoriCall::Render);
        self.handle_lori_loop();
        
        
        let mut primitive_box: GPUPrimitives = GPUPrimitives::from_vec(self.primitives.len() as u32, &self.primitives);
        primitive_box.scale = [self.size.width as f32, self.size.height as f32];
        self.primitives.clear();

        self.queue.write_buffer(&self.primitive_buffer, 0, &bytemuck::bytes_of(&[primitive_box]));
        renderpass.set_bind_group(0, &self.primitive_bind_group, &[]);
        renderpass.draw(0..3, 0..1); // TODO: Only do if any primitive drawing function is called
        
        drop(renderpass);

        
        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(pretexture_view);

        self.last_time = self.current_time;
    }

    fn handle_lori_commands(&mut self, v: LoriToMainCommand) {
        match v {
            LoriToMainCommand::SetWindowTitle { text } => {
                self.window.set_title(text.as_str());
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::SetWindowSize { w, h } => {
                _= self.window.request_inner_size(PhysicalSize { width: w, height: h });
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::SetWindowResizable { is } => {
                _= self.window.set_resizable(is);
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::SetGravity { x, y } => {
                self.gravity = Vec2 { x, y };
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::SetCameraPosition { x, y } => {
                self.gpu_view.position = [x, y];
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::GetWindowSize => {
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnGetWindowSize { w: self.size.width, h: self.size.height });
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            },
            LoriToMainCommand::GetKeyPressed { key } => {
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnKeyPressed { key: self.keys.contains(&key) });
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::GetCameraPosition => {
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnCameraPosition { x: self.gpu_view.position[0], y: self.gpu_view.position[1] });
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::NewShape { kind, w, h, color } => {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();
                if kind == "rectangle" {
                    vertices.push(Vertex { position: [0., 0.], uv: [0., 0.], color });
                    vertices.push(Vertex { position: [w, 0.], uv: [1., 0.], color });
                    vertices.push(Vertex { position: [0., h], uv: [0., 1.], color });
                    vertices.push(Vertex { position: [w, h], uv: [1., 1.], color });

                    indices.push(0);
                    indices.push(1);
                    indices.push(2);
                    indices.push(3);
                }
                self.lori_shapes.insert(self.shape_id, LoriShape::new(vertices, indices));
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnNewShape { shape: LoriShapeRef { uid: self.shape_id, tx: self.lori_cmd_rev.clone() } });
                self.shape_id += 1;
            }
            LoriToMainCommand::NewMesh { vertices, indices } => {
                sdbugln(format!("{}", vertices.len()));
                let mut new_vertices: Vec<Vertex> = Vec::new();
                for vertex in vertices {
                    new_vertices.push(Vertex { position: [vertex[0], vertex[1]], uv: [vertex[2], vertex[3]], color: [vertex[4], vertex[5], vertex[6], vertex[7]] });
                }
                
                self.lori_shapes.insert(self.shape_id, LoriShape::new(new_vertices, indices));
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnNewMesh { mesh: LoriShapeRef { uid: self.shape_id, tx: self.lori_cmd_rev.clone() } });
                self.shape_id += 1;
            }
            LoriToMainCommand::NewCollider { shape, collision } => {
                let real_shape: &LoriShape = self.lori_shapes.get(&shape.uid).unwrap();
                let vertices: Vec<Vertex> = real_shape.vertices.clone();
                let indices: Vec<u32> = real_shape.indices.clone();
                
                self.lori_colliders.insert(self.collider_id, LoriCollider::new(vertices, indices, collision));
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnNewCollider { collider: LoriColliderRef { uid: self.collider_id, tx: self.lori_cmd_rev.clone() } });
                self.collider_id += 1;
            }
            LoriToMainCommand::NewSpawner { shape, collider } => {
                let mut final_shape: Option<LoriShape> = None;
                let mut final_collider: Option<LoriCollider> = None;

                if let Some(real_shape) = shape {
                    final_shape = self.lori_shapes.get(&real_shape.uid).cloned();
                }
                if let Some(real_collider) = collider {
                    final_collider = self.lori_colliders.get(&real_collider.uid).cloned();
                }
                
                self.lori_spawners.insert(self.spawner_id, LoriSpawner::new(&self.device, final_shape, final_collider));
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnNewSpawner { spawner: LoriSpawnerRef { uid: self.spawner_id, tx: self.lori_cmd_rev.clone(), rx: self.lori_rtrn_rev.clone() } });
                self.spawner_id += 1;
            }
            LoriToMainCommand::DrawPrimitive { x, y, w, h, r, color, label } => {
                self.primitives.push(Primitive { xywh: [x, y, w, h], angle: r, label, _pad0: 0, _pad1: 0, color });
            }
            LoriToMainCommand::SpawnerSpawn { uid, x, y, r } => {
                let spawner: &mut LoriSpawner = self.lori_spawners.get_mut(&uid).unwrap();
                let ouid: u64 = spawner.spawn(x, y, r, &mut self.rigidbodies, &mut self.colliders);
                _= self.lori_rtrn.send(MainToLoriCommand::ReturnNewObject { object: LoriObjectRef { puid: uid, uid: ouid, tx: self.lori_cmd_rev.clone() } })
            }
            LoriToMainCommand::ObjectMove { puid, uid, x, y } => {
                let spawner: &LoriSpawner = self.lori_spawners.get(&puid).unwrap();
                let object: &Option<&RigidBodyHandle> = &spawner.rigidhandles.get(&uid);
                if let Some(real_object) = *object {
                    if let Some(body) = self.rigidbodies.get_mut(*real_object) {
                        body.apply_impulse(Vector2 { x, y }, true);
                    }
                }
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::ObjectPush { puid, uid, x, y } => {
                let spawner: &LoriSpawner = self.lori_spawners.get(&puid).unwrap();
                let object: &Option<&RigidBodyHandle> = &spawner.rigidhandles.get(&uid);
                if let Some(real_object) = *object {
                    if let Some(body) = self.rigidbodies.get_mut(*real_object) {
                        body.add_force(Vector2 { x, y }, true);
                    }
                }
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
            LoriToMainCommand::ObjectPull { puid, uid, x1, y1, x2, y2 } => {
                let spawner: &LoriSpawner = self.lori_spawners.get(&puid).unwrap();
                let object: &Option<&RigidBodyHandle> = &spawner.rigidhandles.get(&uid);
                if let Some(real_object) = *object {
                    if let Some(body) = self.rigidbodies.get_mut(*real_object) {
                        body.add_force_at_point(Vector2 { x: x1, y: y1 }, Vector2 { x: x2, y: y2 }, true);
                    }
                }
                _= self.lori_rtrn.send(MainToLoriCommand::Return);
            }
        }
    }
    
    fn handle_lori_loop(&mut self) {
        loop {
            select! {
                recv(self.lori_cmd) -> cmd => {
                    if let Ok(v) = cmd {
                        self.handle_lori_commands(v);
                    }
                }
                recv(self.lori_back) -> _ => {
                    break;
                }
            }
        }
    }

    fn exit(&mut self) {
        _= self.lori_call.send(MainToLoriCall::Exit);
        self.handle_lori_loop();
        if let Some(join_handle) = self.lori_handle.take() {
            _= join_handle.join();
        };
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());

        let state = pollster::block_on(State::new(window.clone()));
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
                let newtext: String = event.text.unwrap_or_else(|| SmolStr::new("NONE")).to_string();
                superstate.keyboard_inputs(newtext, event.state.is_pressed());
            }
            WindowEvent::MouseInput { state, button, .. } => {
                superstate.mouse_button_inputs(button, state.is_pressed());
            }
            WindowEvent::CursorMoved { position, .. } => {
                superstate.mouse = (position.x as f32, position.y as f32);
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
    let events = EventLoop::new().unwrap();
    events.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    match events.run_app(&mut app) {
        Ok(()) => infoln("Exited successfully."),
        Err(error) => serorln(format!("Exited with an error:\n {error:?}")),
    }
}
