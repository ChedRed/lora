use chrono::TimeDelta;
use clap::Parser;
use crossbeam::{channel::{Receiver, Sender, bounded}, select};
use std::sync::mpsc;

use std::{process::exit, sync::Arc, thread::JoinHandle};
use rapier2d::prelude::*;
use winit::{application::ApplicationHandler, event::MouseScrollDelta, platform::wayland::WindowAttributesExtWayland};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, MouseButton, MouseScrollDelta::{LineDelta, PixelDelta}, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::SmolStr;
use winit::window::{Window, WindowId};
use wgpu::{naga::FastHashMap, util::DeviceExt};

pub mod content;
use content::{border::{LoraBorder, LoraBorderRef}, shape::{LoraShape, LoraShapeRef}, collider::{LoraCollider, LoraColliderRef}, spawner::{LoraSpawner, LoraSpawnerRef, LoraObjectRef}};
pub mod utils;
use utils::{filer::Filer, GPUPrimitives, Location, LoraToMainCall, LoraToMainCommand, MainToLoraCall, MainToLoraCommand, Primitive, Vertex, lora::Lora, print::*, get_image};
pub mod compiler;
use compiler::compile;

const RESOLUTION: f32 = 100.;


#[derive(Parser, Debug)]
#[command(name = "lora")]
#[command(
    about="A rust-based framework for Lua games!",
    long_about="A rust-based framework that allows you to create any game in Lua with the lora API!")]
pub struct Args {
    #[arg(short, long,
        help="Enable test mode",
        long_help="Enables testing for github actions.\nWhen enabled, exits at the end of lora.render() and will require all lora functions to be present in lua code.")]
    test: bool,
    
    #[arg(short, long, help="Enable verbose output")]
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
    filer: Filer,
    
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

    lora_borders: FastHashMap<u128, LoraBorder>,
    lora_shapes: FastHashMap<u128, LoraShape>,
    lora_colliders: FastHashMap<u128, LoraCollider>,
    lora_spawners: FastHashMap<u128, LoraSpawner>,
    uuid: u128,
    primitives: Vec<Primitive>,

    window: Arc<Window>,
    gpu_view: GPUView,
    gpu_view_buffer: wgpu::Buffer,
    gpu_view_bind_group: wgpu::BindGroup,

    texture_bind_layout: wgpu::BindGroupLayout,

    primitive_buffer: wgpu::Buffer,
    primitive_bind_group: wgpu::BindGroup,

    gravity: Vec2,
    integration_parameters: IntegrationParameters,
    physics: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    rigidbodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    collision_recv: mpsc::Receiver<CollisionEvent>,
    _contact_recv: mpsc::Receiver<ContactForceEvent>,
    event_queue: ChannelEventCollector,
    ccd_solver: CCDSolver,

    lora_call: Sender<MainToLoraCall>,
    lora_back: Receiver<LoraToMainCall>,
    lora_cmd: Receiver<LoraToMainCommand>,
    lora_cmd_rev: Sender<LoraToMainCommand>,
    lora_rtrn: Sender<MainToLoraCommand>,
    lora_rtrn_rev: Receiver<MainToLoraCommand>,
    lora_handle: Option<JoinHandle<()>>,
}

impl State {
    async fn new(window: Arc<Window>, argus: Args, filer: Filer) -> State {        
        let lua_code: String = filer.read_code();
        
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
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("WGPU Device and Adapter"),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            ..wgpu::DeviceDescriptor::default()
        }).await.unwrap();
        
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let (main_cmd, lora_cmd) = bounded::<LoraToMainCommand>(1);
        let (lora_rtrn, main_rtrn) = bounded::<MainToLoraCommand>(1);
        let (lora_call, main_call) = bounded::<MainToLoraCall>(0);
        let (main_back, lora_back) = bounded::<LoraToMainCall>(0);

        let lora_cmd_rev = main_cmd.clone();
        let lora_rtrn_rev = main_rtrn.clone();

        let mut lora: Lora = Lora::new(lua_code, argus.verbose, main_cmd, main_rtrn, main_call, main_back);
        let lora_handle = Some(std::thread::Builder::new()
            .name("lora".to_string())
            .spawn(move || { lora.begin(); }).unwrap());

        let lora_borders: FastHashMap<u128, LoraBorder> = FastHashMap::default();
        let lora_shapes: FastHashMap<u128, LoraShape> = FastHashMap::default();
        let lora_colliders: FastHashMap<u128, LoraCollider> = FastHashMap::default();
        let lora_spawners: FastHashMap<u128, LoraSpawner> = FastHashMap::default();
        let uuid: u128 = 0;

        let mut gpu_view: GPUView = GPUView::new();
        gpu_view.scale = [size.width as f32 / RESOLUTION, size.height as f32 / RESOLUTION];
        
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

        let texture_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        
        let raster_shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/main.wgsl").into());
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
        let (collision_send, collision_recv) = mpsc::channel::<CollisionEvent>();
        let (contact_send, contact_recv) = mpsc::channel::<ContactForceEvent>();
        let event_queue = ChannelEventCollector::new(collision_send, contact_send);
        let ccd_solver = CCDSolver::new();

        let mut state = State {
            argus,
            filer,
            
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

            lora_borders,
            lora_shapes,
            lora_colliders,
            lora_spawners,
            uuid,
            primitives,

            window,
            gpu_view,
            gpu_view_buffer,
            gpu_view_bind_group,

            texture_bind_layout,

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
            collision_recv,
            _contact_recv: contact_recv,
            event_queue,
            ccd_solver,
            
            lora_call,
            lora_back,
            lora_cmd,
            lora_cmd_rev,
            lora_rtrn,
            lora_rtrn_rev,
            lora_handle,
        };

        _= state.lora_call.send(MainToLoraCall::Load);
        state.handle_lora_loop();
        
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
        self.gpu_view.scale = [size[0] as f32  / RESOLUTION, size[1] as f32  / RESOLUTION];
    }
    
    fn keyboard_inputs(&mut self, key: String, state: bool) {
        if state {
            self.keys.push(key.clone());
            _= self.lora_call.send(MainToLoraCall::Keypressed { code: key });
        } else {
            self.keys.retain(|k| k != &key);
            _= self.lora_call.send(MainToLoraCall::Keyreleased { code: key });
        }
        self.handle_lora_loop();
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
            _= self.lora_call.send(MainToLoraCall::Mousepressed { x: self.mouse.0, y: self.mouse.1, button: numerical_button });
        } else {
            _= self.lora_call.send(MainToLoraCall::Mousereleased { x: self.mouse.0, y: self.mouse.1, button: numerical_button });
        }
        self.handle_lora_loop();
    }

    fn mouse_movement_inputs(&mut self, motion: (f64, f64)) {
        let simple_motion: (f32, f32) = (motion.0 as f32, motion.1 as f32);
        _= self.lora_call.send(MainToLoraCall::MouseMoved { motion: simple_motion });
        self.handle_lora_loop();
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
        _= self.lora_call.send(MainToLoraCall::MouseScrolled { motion: simple_motion });
        self.handle_lora_loop();
    }

    fn render(&mut self) {
        self.current_time = chrono::Utc::now();

        while self.timestep < self.current_time {
            _= self.lora_call.send(MainToLoraCall::Update { delta: self.integration_parameters.dt });
            self.handle_lora_loop();
            
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
                &mut self.event_queue,
            );

            while let Ok(event) = self.collision_recv.try_recv() {
                match event {
                    CollisionEvent::Started(collider1, collider2, _flags) => {
                        let one = self.rigidbodies.get(self.colliders.get(collider1).unwrap().parent().unwrap()).unwrap().user_data;
                        let two = self.rigidbodies.get(self.colliders.get(collider2).unwrap().parent().unwrap()).unwrap().user_data;
                        _= self.lora_call.send(MainToLoraCall::Collision { one, two });
                        self.handle_lora_loop();
                    }
                    CollisionEvent::Stopped(_collider1, _collider2, _flags) => {}
                }
            }

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
        
        for obj in self.lora_spawners.iter_mut() {
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
                                        body.reset_torques(true);
                                    }
                                }
                            }
                            
                            let locations: Vec<Location> = obj.1.locations.values().copied().collect();
                            self.queue.write_buffer(&real_location_buffer, 0, bytemuck::cast_slice(&locations));
                            renderpass.set_vertex_buffer(0, real_vertex_buffer.slice(..));
                            renderpass.set_vertex_buffer(1, real_location_buffer.slice(..));

                            if let Some(bindgroup) = &obj.1.texture_bind_group {
                                renderpass.set_bind_group(1, bindgroup, &[]);
                            }
                            renderpass.set_index_buffer(real_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                            renderpass.draw_indexed(0..obj.1.indices as u32, 0, 0..obj.1.locations.len() as _);
                            
                        }
                    }
                }
            }
        }

        
        renderpass.set_pipeline(&self.primitive_pipeline);

        _= self.lora_call.send(MainToLoraCall::Render);
        self.handle_lora_loop();
        
        
        let mut primitive_box: GPUPrimitives = GPUPrimitives::from_vec(self.primitives.len() as u32, &self.primitives);
        primitive_box.scale = [self.size.width as f32, self.size.height as f32];
        self.primitives.clear();

        self.queue.write_buffer(&self.primitive_buffer, 0, &bytemuck::bytes_of(&[primitive_box]));
        renderpass.set_bind_group(0, &self.primitive_bind_group, &[]);
        renderpass.draw(0..3, 0..1);
        
        drop(renderpass);

        
        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(pretexture_view);

        self.last_time = self.current_time;
    }
    
    fn handle_lora_commands(&mut self, v: LoraToMainCommand) {
        match v {
            LoraToMainCommand::SetWindowTitle { text } => {
                self.window.set_title(text.as_str());
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetWindowSize { w, h } => {
                _= self.window.request_inner_size(PhysicalSize { width: w, height: h });
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetWindowResizable { is } => {
                _= self.window.set_resizable(is);
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetPhysicsGravity { x, y } => {
                self.gravity = Vec2 { x, y };
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetPhysicsHertz { hz } => {
                let pre_delta: f64 = 1f64/hz;
                self.delta = TimeDelta::seconds(pre_delta.trunc() as i64) + TimeDelta::nanoseconds((pre_delta.fract() * 1_000_000_000.0) as i64);
                self.integration_parameters.dt = pre_delta as f32;
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetCameraPosition { x, y } => {
                self.gpu_view.position = [x / RESOLUTION, y / RESOLUTION];
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::GetWindowSize => {
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnGetWindowSize { w: self.size.width, h: self.size.height });
            },
            LoraToMainCommand::GetKeyPressed { key } => {
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnKeyPressed { key: self.keys.contains(&key) });
            }
            LoraToMainCommand::GetCameraPosition => {
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnCameraPosition { x: self.gpu_view.position[0] * RESOLUTION, y: self.gpu_view.position[1] / RESOLUTION });
            }
            LoraToMainCommand::NewBorder { points, indices } => {
                let mut vertices: Vec<Vec2> = Vec::new();
                for point in points {
                    vertices.push(Vec2 { x: point[0] / RESOLUTION, y: point[1] / RESOLUTION });
                }
                
                self.lora_borders.insert(self.uuid, LoraBorder::new(self.uuid, vertices, indices, &mut self.rigidbodies, &mut self.colliders));
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewBorder { border: LoraBorderRef { uuid: self.uuid, tx: self.lora_cmd_rev.clone(), rx: self.lora_rtrn_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::NewImage { image, scale } => {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                let new_scale = scale / RESOLUTION;
                let (image_bytes, image_scale) = get_image(self.filer.read_file(image).unwrap());
                vertices.push(Vertex { position: [0., 0.], uv: [0., 0.], color: [1., 1., 1., 1.] });
                vertices.push(Vertex { position: [image_scale.0 as f32 * new_scale, 0.], uv: [1., 0.], color: [1., 1., 1., 1.] });
                vertices.push(Vertex { position: [0., image_scale.1 as f32 * new_scale], uv: [0., 1.], color: [1., 1., 1., 1.] });
                vertices.push(Vertex { position: [image_scale.0 as f32 * new_scale, image_scale.1 as f32 * new_scale], uv: [1., 1.], color: [1., 1., 1., 1.] });

                indices.push(0);
                indices.push(1);
                indices.push(2);
                indices.push(3);

                
                self.lora_shapes.insert(self.uuid, LoraShape::new(vertices, indices, Some(image_bytes), Some(image_scale)));
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewImage { image: LoraShapeRef { uuid: self.uuid, tx: self.lora_cmd_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::NewShape { kind, w, h, color } => {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();
                if kind == "rectangle" {
                    vertices.push(Vertex { position: [0., 0.], uv: [0., 0.], color });
                    vertices.push(Vertex { position: [w / RESOLUTION, 0.], uv: [1., 0.], color });
                    vertices.push(Vertex { position: [0., h / RESOLUTION], uv: [0., 1.], color });
                    vertices.push(Vertex { position: [w / RESOLUTION, h / RESOLUTION], uv: [1., 1.], color });

                    indices.push(0);
                    indices.push(1);
                    indices.push(2);
                    indices.push(3);
                } else if kind == "triangle" {
                    vertices.push(Vertex { position: [0., 0.], uv: [0., 0.], color });
                    vertices.push(Vertex { position: [w / RESOLUTION, 0.], uv: [1., 0.], color });
                    vertices.push(Vertex { position: [0., h / RESOLUTION], uv: [0., 1.], color });

                    indices.push(0);
                    indices.push(1);
                    indices.push(2);
                }
                self.lora_shapes.insert(self.uuid, LoraShape::new(vertices, indices, None, None));
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewShape { shape: LoraShapeRef { uuid: self.uuid, tx: self.lora_cmd_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::NewMesh { vertices, indices } => {
                let mut new_vertices: Vec<Vertex> = Vec::new();
                for vertex in vertices {
                    new_vertices.push(Vertex { position: [vertex[0] / RESOLUTION, vertex[1] / RESOLUTION], uv: [vertex[2], vertex[3]], color: [vertex[4], vertex[5], vertex[6], vertex[7]] });
                }
                
                self.lora_shapes.insert(self.uuid, LoraShape::new(new_vertices, indices, None, None));
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewMesh { mesh: LoraShapeRef { uuid: self.uuid, tx: self.lora_cmd_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::NewCollider { shape, collision } => {
                let real_shape: &LoraShape = self.lora_shapes.get(&shape.uuid).unwrap();
                let vertices: Vec<Vertex> = real_shape.vertices.clone();
                let indices: Vec<u32> = real_shape.indices.clone();
                
                self.lora_colliders.insert(self.uuid, LoraCollider::new(vertices, indices, collision));
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewCollider { collider: LoraColliderRef { uuid: self.uuid, tx: self.lora_cmd_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::NewSpawner { shape, collider } => {
                let mut final_shape: Option<LoraShape> = None;
                let mut final_collider: Option<LoraCollider> = None;

                if let Some(real_shape) = shape {
                    final_shape = self.lora_shapes.get(&real_shape.uuid).cloned();
                }
                if let Some(real_collider) = collider {
                    final_collider = self.lora_colliders.get(&real_collider.uuid).cloned();
                }
                
                self.lora_spawners.insert(self.uuid, LoraSpawner::new(&self.device, &self.queue, &self.texture_bind_layout, final_shape, final_collider));
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewSpawner { spawner: LoraSpawnerRef { uuid: self.uuid, tx: self.lora_cmd_rev.clone(), rx: self.lora_rtrn_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::DrawPrimitive { x, y, w, h, r, color, label } => {
                self.primitives.push(Primitive { xywh: [x, y, w, h], angle: r, label, _pad0: 0, _pad1: 0, color });
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SpawnerSpawn { uuid, x, y, r } => {
                let spawner: &mut LoraSpawner = self.lora_spawners.get_mut(&uuid).unwrap();
                spawner.spawn(self.uuid, x / RESOLUTION, y / RESOLUTION, r.to_radians(), &mut self.rigidbodies, &mut self.colliders);
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnNewObject { object: LoraObjectRef { parent_uuid: uuid, uuid: self.uuid, tx: self.lora_cmd_rev.clone(), rx: self.lora_rtrn_rev.clone() } });
                self.uuid += 1;
            }
            LoraToMainCommand::BorderSetPosition { uuid, x, y } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_next_kinematic_translation(Vec2 { x: x / RESOLUTION, y: y / RESOLUTION });
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
                
            }
            LoraToMainCommand::BorderSetAngle { uuid, r } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_next_kinematic_rotation(Rot2 { re: r.to_radians().cos(), im: r.to_radians().sin() });
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderGetPosition { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                let preposition = body.translation();
                let position: [f32; 2] = [preposition.x * RESOLUTION, preposition.y * RESOLUTION];
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnBorderGetPosition { position });
            }
            LoraToMainCommand::BorderGetAngle { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                let angle = body.rotation().angle();
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnBorderGetAngle { angle });
            }
            LoraToMainCommand::BorderEnable { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_enabled(true);
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderDisable { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_enabled(false);
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderToggle { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_enabled(!body.is_enabled());
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetPosition { parent_uuid, uuid, x, y } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_translation(Vector2 { x: x / RESOLUTION, y: y / RESOLUTION }, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetMotion { parent_uuid, uuid, x, y } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_linvel(Vector2 { x, y }, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetAngle { parent_uuid, uuid, r } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_rotation(Rot2 { re: r.to_radians().cos(), im: r.to_radians().sin() }, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectGetPosition { parent_uuid, uuid } => {
                let mut position: [f32; 2] = [0., 0.];
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_position = body.translation();
                    position[0] = pre_position.x * RESOLUTION;
                    position[1] = pre_position.y * RESOLUTION;
                }
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnObjectGetPosition { position });
            }
            LoraToMainCommand::ObjectGetCenter { uuid } => {
                let position: [f32; 2];
                let spawner: &LoraSpawner = self.lora_spawners.get(&uuid).unwrap();
                let preposition = spawner.center.unwrap();
                position = [preposition.0, preposition.1];
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnObjectGetCenter { position });
            }
            LoraToMainCommand::ObjectGetWorldCenter { parent_uuid, uuid } => {
                let mut position: [f32; 2] = [0., 0.];
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_position = body.center_of_mass();
                    position[0] = pre_position.x * RESOLUTION;
                    position[1] = pre_position.y * RESOLUTION;
                }
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnObjectGetWorldCenter { position });
            }
            LoraToMainCommand::ObjectGetMotion { parent_uuid, uuid } => {
                let mut motion: [f32; 2] = [0., 0.];
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_motion = body.linvel();
                    motion[0] = pre_motion.x;
                    motion[1] = pre_motion.y;
                }
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnObjectGetMotion { motion });
            }
            LoraToMainCommand::ObjectGetAngle { parent_uuid, uuid } => {
                let mut angle: f32 = 0.;
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    angle = body.rotation().angle().to_degrees();
                }
                _= self.lora_rtrn.send(MainToLoraCommand::ReturnObjectGetAngle { angle });
            }
            LoraToMainCommand::ObjectImpulse { parent_uuid, uuid, x, y } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.apply_impulse(Vector2 { x, y }, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectAddForce { parent_uuid, uuid, x, y } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.add_force(Vector2 { x, y }, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectAddWorldForce { parent_uuid, uuid, x1, y1, x2, y2 } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.add_force_at_point(Vector2 { x: x1, y: y1 }, Vector2 { x: x2, y: y2 }, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectAddTorque { parent_uuid, uuid, r } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.add_torque(r, true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectShow { parent_uuid, uuid } => {
                let spawner: &mut LoraSpawner = self.lora_spawners.get_mut(&parent_uuid).unwrap();
                let status: &mut (bool, bool) = spawner.status.get_mut(&uuid).unwrap();
                status.0 = true;
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectHide { parent_uuid, uuid } => {
                let spawner: &mut LoraSpawner = self.lora_spawners.get_mut(&parent_uuid).unwrap();
                let status: &mut (bool, bool) = spawner.status.get_mut(&uuid).unwrap();
                status.0 = false;
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectEnable { parent_uuid, uuid } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_enabled(true);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectDisable { parent_uuid, uuid } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_enabled(false);
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectToggle { parent_uuid, uuid } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_enabled(!body.is_enabled());
                }
                _= self.lora_rtrn.send(MainToLoraCommand::Return);
            }
        }
    }
    
    fn handle_lora_loop(&mut self) {
        loop {
            select! {
                recv(self.lora_cmd) -> cmd => {
                    if let Ok(v) = cmd {
                        self.handle_lora_commands(v);
                    }
                }
                recv(self.lora_back) -> _ => {
                    break;
                }
            }
        }
    }

    fn exit(&mut self) {
        _= self.lora_call.send(MainToLoraCall::Exit);
        self.handle_lora_loop();
        if let Some(join_handle) = self.lora_handle.take() {
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
        let argus: Args = Args::parse();
        let filer: Filer = Filer::new(&argus.filepath);
        
        let window = Arc::new(event_loop.create_window(Window::default_attributes()
            .with_title(filer.read_name())
            .with_name(filer.read_id(), filer.read_id())).unwrap());

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
    let argus: Args = Args::parse();
    if argus.compile.is_some() {
        compile(argus.compile.unwrap());
        exit(0);
    }
    
    let events = EventLoop::new().unwrap();
    events.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    match events.run_app(&mut app) {
        Ok(()) => infoln("Exited successfully."),
        Err(error) => serorln(format!("Exited with an error:\n {error:?}")),
    }
}

