pub mod border;
pub mod collider;
pub mod shape;
pub mod sound;
pub mod spawner;

use chrono::TimeDelta;

use rapier2d::{prelude::*, utils::PoseOps};

use crossbeam::{
    channel::{Receiver, Sender, bounded},
    select,
};
use rapier2d::{
    dynamics::{
        CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet,
        RigidBodySet,
    },
    geometry::{BroadPhaseBvh, ColliderSet, CollisionEvent, ContactForceEvent, NarrowPhase},
    math::Vec2,
    pipeline::{ChannelEventCollector, PhysicsPipeline},
};
use rodio::{Decoder, MixerDeviceSink, Source};
use std::{
    io::Cursor,
    sync::{Arc, mpsc},
    thread::JoinHandle,
};
use wgpu::naga::FastHashMap;

use winit::event::{
    MouseButton,
    MouseScrollDelta::{self, LineDelta, PixelDelta},
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    GPUView, RESOLUTION,
    content::{
        border::{LoraBorder, LoraBorderRef},
        collider::{LoraCollider, LoraColliderRef},
        shape::{LoraShape, LoraShapeRef},
        sound::{LoraSound, LoraSoundRef},
        spawner::{LoraObjectRef, LoraSpawner, LoraSpawnerRef},
    },
    utils::{
        GPUPrimitives, Location, LoraToMainCall, LoraToMainCommand, MainToLoraCall,
        MainToLoraCommand, Primitive, Vertex,
        filer::Filer,
        get_image,
        lora::{Lora, LoraCommandContext},
    },
};

pub struct Engine {
    filer: Filer,

    timestep: chrono::DateTime<chrono::Utc>,

    pub sampler: wgpu::Sampler,

    pub mouse: (f32, f32),
    pub keys: Vec<String>,

    pub lora_call: Sender<MainToLoraCall>,
    pub lora_back: Receiver<LoraToMainCall>,
    pub lora_cmd: Receiver<LoraToMainCommand>,
    pub lora_cmd_rev: Sender<LoraToMainCommand>,
    pub lora_rtrn: Sender<MainToLoraCommand>,
    pub lora_rtrn_rev: Receiver<MainToLoraCommand>,
    pub lora_handle: Option<JoinHandle<()>>,

    pub sink: MixerDeviceSink,
    pub primitives: Vec<Primitive>,

    pub lora_borders: FastHashMap<u128, LoraBorder>,
    pub lora_shapes: FastHashMap<u128, LoraShape>,
    pub lora_colliders: FastHashMap<u128, LoraCollider>,
    pub lora_spawners: FastHashMap<u128, LoraSpawner>,
    pub lora_sounds: FastHashMap<u128, LoraSound>,
    pub uuid: u128,

    pub gravity: Vec2,
    pub integration_parameters: IntegrationParameters,
    pub physics: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseBvh,
    pub narrow_phase: NarrowPhase,
    pub rigidbodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub collision_recv: mpsc::Receiver<CollisionEvent>,
    pub _contact_recv: mpsc::Receiver<ContactForceEvent>,
    pub event_queue: ChannelEventCollector,
    pub ccd_solver: CCDSolver,
}

impl Engine {
    pub fn new(device: &wgpu::Device, filer: Filer, verbose: bool) -> Self {
        let lua_code: String = filer.read_code();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let mouse: (f32, f32) = (0., 0.);
        let keys: Vec<String> = Vec::new();

        let (main_cmd, lora_cmd) = bounded::<LoraToMainCommand>(1);
        let (lora_rtrn, main_rtrn) = bounded::<MainToLoraCommand>(1);
        let (lora_call, main_call) = bounded::<MainToLoraCall>(0);
        let (main_back, lora_back) = bounded::<LoraToMainCall>(0);

        let lora_cmd_rev = main_cmd.clone();
        let lora_rtrn_rev = main_rtrn.clone();

        let mut lora: Lora =
            Lora::new(lua_code, verbose, main_cmd, main_rtrn, main_call, main_back);
        let lora_handle = Some(
            std::thread::Builder::new()
                .name("lora".to_string())
                .spawn(move || {
                    lora.begin();
                })
                .unwrap(),
        );

        let mut sink: MixerDeviceSink = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        sink.log_on_drop(false);
        let primitives: Vec<Primitive> = Vec::with_capacity(200);

        let lora_borders: FastHashMap<u128, LoraBorder> = FastHashMap::default();
        let lora_shapes: FastHashMap<u128, LoraShape> = FastHashMap::default();
        let lora_colliders: FastHashMap<u128, LoraCollider> = FastHashMap::default();
        let lora_spawners: FastHashMap<u128, LoraSpawner> = FastHashMap::default();
        let lora_sounds: FastHashMap<u128, LoraSound> = FastHashMap::default();
        let uuid: u128 = 0;

        let gravity = Vec2 { x: 0., y: 0. };
        let mut integration_parameters = IntegrationParameters::default();
        integration_parameters.dt = 1. / 100.;

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

        let mut engine = Engine {
            filer,

            timestep: chrono::Utc::now(),

            sampler,

            mouse,
            keys,

            lora_call,
            lora_back,
            lora_cmd,
            lora_cmd_rev,
            lora_rtrn,
            lora_rtrn_rev,
            lora_handle,

            sink,
            primitives,

            lora_borders,
            lora_shapes,
            lora_colliders,
            lora_spawners,
            lora_sounds,
            uuid,

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
        };

        _ = engine.lora_call.send(MainToLoraCall::Load);
        engine.handle_lora_loop();

        engine
    }

    pub fn keyboard_inputs(&mut self, state: bool, key: String) {
        if state {
            self.keys.push(key.clone());
            _ = self
                .lora_call
                .send(MainToLoraCall::Keypressed { code: key });
        } else {
            self.keys.retain(|k| k != &key);
            _ = self
                .lora_call
                .send(MainToLoraCall::Keyreleased { code: key });
        }
        self.handle_lora_loop();
    }

    pub fn mouse_button_inputs(&mut self, button: MouseButton, state: bool) {
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
                numerical_button = (6 + num) as u32;
            }
        }
        if state {
            _ = self.lora_call.send(MainToLoraCall::Mousepressed {
                x: self.mouse.0,
                y: self.mouse.1,
                button: numerical_button,
            });
        } else {
            _ = self.lora_call.send(MainToLoraCall::Mousereleased {
                x: self.mouse.0,
                y: self.mouse.1,
                button: numerical_button,
            });
        }
        self.handle_lora_loop();
    }

    pub fn mouse_movement_inputs(&mut self, motion: (f64, f64)) {
        let simple_motion: (f32, f32) = (motion.0 as f32, motion.1 as f32);
        _ = self.lora_call.send(MainToLoraCall::MouseMoved {
            motion: simple_motion,
        });
        self.handle_lora_loop();
    }

    pub fn mouse_scroll_inputs(&mut self, delta: MouseScrollDelta) {
        let simple_motion: (f32, f32);
        match delta {
            PixelDelta(position) => {
                simple_motion = (position.x as f32, position.y as f32);
            }
            LineDelta(x, y) => {
                simple_motion = (x, y);
            }
        }
        _ = self.lora_call.send(MainToLoraCall::MouseScrolled {
            motion: simple_motion,
        });
        self.handle_lora_loop();
    }

    pub fn object_prerender(
        &mut self,
        queue: &wgpu::Queue,
        renderpass: &mut wgpu::RenderPass,
        current_time: chrono::DateTime<chrono::Utc>,
        delta: &mut chrono::TimeDelta,

        device: &wgpu::Device,
        window: &Arc<Window>,
        texture_bind_layout: &wgpu::BindGroupLayout,
        gpu_view: &mut GPUView,
        filer: &Filer,
        size: winit::dpi::PhysicalSize<u32>,
    ) {
        while self.timestep < current_time {
            _ = self.lora_call.send(MainToLoraCall::Update {
                delta: self.integration_parameters.dt,
            });
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
                        let one = self
                            .rigidbodies
                            .get(self.colliders.get(collider1).unwrap().parent().unwrap())
                            .unwrap()
                            .user_data;
                        let two = self
                            .rigidbodies
                            .get(self.colliders.get(collider2).unwrap().parent().unwrap())
                            .unwrap()
                            .user_data;
                        _ = self.lora_call.send(MainToLoraCall::Collision { one, two });
                        self.handle_lora_loop(
                            device,
                            queue,
                            window,
                            texture_bind_layout,
                            gpu_view,
                            delta,
                            size,
                        );
                    }
                    CollisionEvent::Stopped(_collider1, _collider2, _flags) => {}
                }
            }

            self.timestep += delta;
        }

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

                            let locations: Vec<Location> =
                                obj.1.locations.values().copied().collect();
                            queue.write_buffer(
                                &real_location_buffer,
                                0,
                                bytemuck::cast_slice(&locations),
                            );
                            renderpass.set_vertex_buffer(0, real_vertex_buffer.slice(..));
                            renderpass.set_vertex_buffer(1, real_location_buffer.slice(..));

                            if let Some(bindgroup) = &obj.1.texture_bind_group {
                                renderpass.set_bind_group(1, bindgroup, &[]);
                            }
                            renderpass.set_index_buffer(
                                real_index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            renderpass.draw_indexed(
                                0..obj.1.indices as u32,
                                0,
                                0..obj.1.locations.len() as _,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn primitive_prerender(
        &mut self,
        queue: &wgpu::Queue,
        primitive_buffer: &wgpu::Buffer,
        size: winit::dpi::PhysicalSize<u32>,
    ) {
        _ = self.lora_call.send(MainToLoraCall::Render);
        self.handle_lora_loop();

        let mut primitive_box: GPUPrimitives =
            GPUPrimitives::from_vec(self.primitives.len() as u32, &self.primitives);
        primitive_box.scale = [size.width as f32, size.height as f32];
        self.primitives.clear();

        queue.write_buffer(primitive_buffer, 0, &bytemuck::bytes_of(&[primitive_box]));
    }

    pub fn exit(&mut self) {
        _ = self.lora_call.send(MainToLoraCall::Exit);
        self.handle_lora_loop();
        if let Some(join_handle) = self.lora_handle.take() {
            _ = join_handle.join();
        };
    }

    fn handle_lora_commands(&mut self, v: LoraToMainCommand, ctx: LoraCommandContext) {
        match v {
            LoraToMainCommand::SetWindowTitle { text } => {
                ctx.window.set_title(text.as_str());
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetWindowSize { w, h } => {
                _ = ctx.window.request_inner_size(PhysicalSize {
                    width: w,
                    height: h,
                });
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetWindowResizable { is } => {
                _ = ctx.window.set_resizable(is);
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetPhysicsGravity { x, y } => {
                self.gravity = Vec2 { x, y };
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetPhysicsHertz { hz } => {
                let pre_delta: f64 = 1f64 / hz;
                *ctx.delta = TimeDelta::seconds(pre_delta.trunc() as i64)
                    + TimeDelta::nanoseconds((pre_delta.fract() * 1_000_000_000.0) as i64);
                self.integration_parameters.dt = pre_delta as f32;
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SetCameraPosition { x, y } => {
                ctx.gpu_view.position = [x / RESOLUTION, y / RESOLUTION];
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::GetWindowSize => {
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnGetWindowSize {
                    w: ctx.size.width,
                    h: ctx.size.height,
                });
            }
            LoraToMainCommand::GetKeyPressed { key } => {
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnKeyPressed {
                    key: self.keys.contains(&key),
                });
            }
            LoraToMainCommand::GetCameraPosition => {
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnCameraPosition {
                        x: ctx.gpu_view.position[0] * RESOLUTION,
                        y: ctx.gpu_view.position[1] / RESOLUTION,
                    });
            }
            LoraToMainCommand::NewBorder { points, indices } => {
                let mut vertices: Vec<Vec2> = Vec::new();
                for point in points {
                    vertices.push(Vec2 {
                        x: point[0] / RESOLUTION,
                        y: point[1] / RESOLUTION,
                    });
                }

                self.lora_borders.insert(
                    self.uuid,
                    LoraBorder::new(
                        self.uuid,
                        vertices,
                        indices,
                        &mut self.rigidbodies,
                        &mut self.colliders,
                    ),
                );
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewBorder {
                    border: LoraBorderRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                        rx: self.lora_rtrn_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::NewImage { image, scale } => {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                let new_scale = scale / RESOLUTION;
                let (image_bytes, image_scale) = get_image(self.filer.read_file(image).unwrap());
                vertices.push(Vertex {
                    position: [0., 0.],
                    uv: [0., 0.],
                    color: [1., 1., 1., 1.],
                });
                vertices.push(Vertex {
                    position: [image_scale.0 as f32 * new_scale, 0.],
                    uv: [1., 0.],
                    color: [1., 1., 1., 1.],
                });
                vertices.push(Vertex {
                    position: [0., image_scale.1 as f32 * new_scale],
                    uv: [0., 1.],
                    color: [1., 1., 1., 1.],
                });
                vertices.push(Vertex {
                    position: [
                        image_scale.0 as f32 * new_scale,
                        image_scale.1 as f32 * new_scale,
                    ],
                    uv: [1., 1.],
                    color: [1., 1., 1., 1.],
                });

                indices.push(0);
                indices.push(1);
                indices.push(2);
                indices.push(3);

                self.lora_shapes.insert(
                    self.uuid,
                    LoraShape::new(vertices, indices, Some(image_bytes), Some(image_scale)),
                );
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewImage {
                    image: LoraShapeRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::NewShape { kind, w, h, color } => {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();
                if kind == "rectangle" {
                    vertices.push(Vertex {
                        position: [0., 0.],
                        uv: [0., 0.],
                        color,
                    });
                    vertices.push(Vertex {
                        position: [w / RESOLUTION, 0.],
                        uv: [1., 0.],
                        color,
                    });
                    vertices.push(Vertex {
                        position: [0., h / RESOLUTION],
                        uv: [0., 1.],
                        color,
                    });
                    vertices.push(Vertex {
                        position: [w / RESOLUTION, h / RESOLUTION],
                        uv: [1., 1.],
                        color,
                    });

                    indices.push(0);
                    indices.push(1);
                    indices.push(2);
                    indices.push(3);
                } else if kind == "triangle" {
                    vertices.push(Vertex {
                        position: [0., 0.],
                        uv: [0., 0.],
                        color,
                    });
                    vertices.push(Vertex {
                        position: [w / RESOLUTION, 0.],
                        uv: [1., 0.],
                        color,
                    });
                    vertices.push(Vertex {
                        position: [0., h / RESOLUTION],
                        uv: [0., 1.],
                        color,
                    });

                    indices.push(0);
                    indices.push(1);
                    indices.push(2);
                }
                self.lora_shapes
                    .insert(self.uuid, LoraShape::new(vertices, indices, None, None));
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewShape {
                    shape: LoraShapeRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::NewMesh { vertices, indices } => {
                let mut new_vertices: Vec<Vertex> = Vec::new();
                for vertex in vertices {
                    new_vertices.push(Vertex {
                        position: [vertex[0] / RESOLUTION, vertex[1] / RESOLUTION],
                        uv: [vertex[2], vertex[3]],
                        color: [vertex[4], vertex[5], vertex[6], vertex[7]],
                    });
                }

                self.lora_shapes
                    .insert(self.uuid, LoraShape::new(new_vertices, indices, None, None));
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewMesh {
                    mesh: LoraShapeRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::NewCollider { shape, collision } => {
                let real_shape: &LoraShape = self.lora_shapes.get(&shape.uuid).unwrap();
                let vertices: Vec<Vertex> = real_shape.vertices.clone();
                let indices: Vec<u32> = real_shape.indices.clone();

                self.lora_colliders
                    .insert(self.uuid, LoraCollider::new(vertices, indices, collision));
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewCollider {
                    collider: LoraColliderRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                    },
                });
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

                self.lora_spawners.insert(
                    self.uuid,
                    LoraSpawner::new(
                        &ctx.device,
                        &ctx.queue,
                        &self.sampler,
                        &ctx.texture_bind_layout,
                        final_shape,
                        final_collider,
                    ),
                );
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewSpawner {
                    spawner: LoraSpawnerRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                        rx: self.lora_rtrn_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::NewSound { sound } => {
                let sound = self.filer.read_file(sound).unwrap();
                let sound_cursor = Cursor::new(sound.clone().into_boxed_slice());
                let source = Decoder::try_from(sound_cursor).unwrap().buffered();

                self.lora_sounds.insert(self.uuid, LoraSound::new(source));
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewSound {
                    sound: LoraSoundRef {
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                        rx: self.lora_rtrn_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::DrawPrimitive {
                x,
                y,
                w,
                h,
                r,
                color,
                label,
            } => {
                self.primitives.push(Primitive {
                    xywh: [x, y, w, h],
                    angle: r,
                    label,
                    _pad0: 0,
                    _pad1: 0,
                    color,
                });
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SpawnerSpawn { uuid, x, y, r } => {
                let spawner: &mut LoraSpawner = self.lora_spawners.get_mut(&uuid).unwrap();
                let center = spawner.center.unwrap();

                spawner.spawn(
                    self.uuid,
                    x / RESOLUTION - center.0,
                    y / RESOLUTION - center.1,
                    r.to_radians(),
                    &mut self.rigidbodies,
                    &mut self.colliders,
                );
                _ = self.lora_rtrn.send(MainToLoraCommand::ReturnNewObject {
                    object: LoraObjectRef {
                        parent_uuid: uuid,
                        uuid: self.uuid,
                        tx: self.lora_cmd_rev.clone(),
                        rx: self.lora_rtrn_rev.clone(),
                    },
                });
                self.uuid += 1;
            }
            LoraToMainCommand::BorderSetPosition { uuid, x, y } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_next_kinematic_translation(Vec2 {
                    x: x / RESOLUTION,
                    y: y / RESOLUTION,
                });
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderSetAngle { uuid, r } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_next_kinematic_rotation(Rot2 {
                    re: r.to_radians().cos(),
                    im: r.to_radians().sin(),
                });
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderPosition { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                let preposition = body.next_position().translation();
                let position: [f32; 2] = [preposition.x * RESOLUTION, preposition.y * RESOLUTION];
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnBorderGetPosition { position });
            }
            LoraToMainCommand::BorderAngle { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                let angle = body.next_position().rotation().angle();
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnBorderGetAngle { angle });
            }
            LoraToMainCommand::BorderEnable { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_enabled(true);
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderDisable { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_enabled(false);
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::BorderToggle { uuid } => {
                let border: &mut LoraBorder = self.lora_borders.get_mut(&uuid).unwrap();
                let body = self.rigidbodies.get_mut(border.rigidhandle).unwrap();
                body.set_enabled(!body.is_enabled());
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetWorld {
                parent_uuid,
                uuid,
                x,
                y,
                c,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();

                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let oldpos = body.translation();

                    let mut n_x = x;
                    let mut n_y = y;

                    if !c.contains("x") {
                        n_x = oldpos.x * RESOLUTION;
                    }
                    if !c.contains("y") {
                        n_y = oldpos.y * RESOLUTION;
                    }

                    body.set_translation(
                        Vector2 {
                            x: n_x / RESOLUTION,
                            y: n_y / RESOLUTION,
                        },
                        true,
                    );
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetPosition {
                parent_uuid,
                uuid,
                x,
                y,
                c,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_position = body.center_of_mass();
                    let pre_translation = body.translation();

                    let mut n_x = x;
                    let mut n_y = y;

                    if !c.contains("x") {
                        n_x = body.center_of_mass().x * RESOLUTION;
                    }
                    if !c.contains("y") {
                        n_y = body.center_of_mass().y * RESOLUTION;
                    }

                    body.set_translation(
                        Vector2 {
                            x: n_x / RESOLUTION - pre_position.x + pre_translation.x,
                            y: n_y / RESOLUTION - pre_position.y + pre_translation.y,
                        },
                        true,
                    );
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetMotion {
                parent_uuid,
                uuid,
                x,
                y,
                r,
                c,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let mut n_x = x;
                    let mut n_y = y;
                    let mut n_r = r;

                    if !c.contains("x") {
                        n_x = body.linvel().x;
                    }
                    if !c.contains("y") {
                        n_y = body.linvel().y;
                    }
                    if !c.contains("r") {
                        n_r = body.angvel();
                    }

                    body.set_linvel(Vector2 { x: n_x, y: n_y }, true);
                    body.set_angvel(n_r, true);
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectSetAngle {
                parent_uuid,
                uuid,
                r,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_rotation(
                        Rot2 {
                            re: r.to_radians().cos(),
                            im: r.to_radians().sin(),
                        },
                        true,
                    );
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectGetWorld { parent_uuid, uuid } => {
                let mut position: [f32; 2] = [0., 0.];
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_position = body.translation();
                    position[0] = pre_position.x * RESOLUTION;
                    position[1] = pre_position.y * RESOLUTION;
                }
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnObjectGetWorld { position });
            }
            LoraToMainCommand::ObjectGetPosition { parent_uuid, uuid } => {
                let mut position: [f32; 2] = [0., 0.];
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_position = body.center_of_mass();
                    position[0] = pre_position.x * RESOLUTION;
                    position[1] = pre_position.y * RESOLUTION;
                }
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnObjectGetPosition { position });
            }
            LoraToMainCommand::ObjectGetMotion { parent_uuid, uuid } => {
                let mut motion: [f32; 3] = [0., 0., 0.];
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    let pre_motion = body.linvel();
                    let pre_motionang = body.angvel();
                    motion[0] = pre_motion.x;
                    motion[1] = pre_motion.y;
                    motion[2] = pre_motionang;
                }
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnObjectGetMotion { motion });
            }
            LoraToMainCommand::ObjectAngle { parent_uuid, uuid } => {
                let mut angle: f32 = 0.;
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    angle = body.rotation().angle().to_degrees();
                }
                _ = self
                    .lora_rtrn
                    .send(MainToLoraCommand::ReturnObjectGetAngle { angle });
            }
            LoraToMainCommand::ObjectImpulse {
                parent_uuid,
                uuid,
                x,
                y,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.apply_impulse(Vector2 { x, y }, true);
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectForce {
                parent_uuid,
                uuid,
                x,
                y,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.add_force(Vector2 { x, y }, true);
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectShove {
                parent_uuid,
                uuid,
                x1,
                y1,
                x2,
                y2,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.add_force_at_point(
                        Vector2 { x: x1, y: y1 },
                        Vector2 { x: x2, y: y2 },
                        true,
                    );
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectTorque {
                parent_uuid,
                uuid,
                r,
            } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.add_torque(r, true);
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectShow { parent_uuid, uuid } => {
                let spawner: &mut LoraSpawner = self.lora_spawners.get_mut(&parent_uuid).unwrap();
                let status: &mut bool = spawner.status.get_mut(&uuid).unwrap();
                *status = true;
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectHide { parent_uuid, uuid } => {
                let spawner: &mut LoraSpawner = self.lora_spawners.get_mut(&parent_uuid).unwrap();
                let status: &mut bool = spawner.status.get_mut(&uuid).unwrap();
                *status = false;
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectEnable { parent_uuid, uuid } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_enabled(true);
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectDisable { parent_uuid, uuid } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_enabled(false);
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::ObjectToggle { parent_uuid, uuid } => {
                let spawner: &LoraSpawner = self.lora_spawners.get(&parent_uuid).unwrap();
                let object: &RigidBodyHandle = spawner.rigidhandles.get(&uuid).unwrap();
                if let Some(body) = self.rigidbodies.get_mut(*object) {
                    body.set_enabled(!body.is_enabled());
                }
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
            LoraToMainCommand::SoundPlay { uuid } => {
                let sound: &LoraSound = self.lora_sounds.get(&uuid).unwrap();
                self.sink.mixer().add(sound.source.clone());
                _ = self.lora_rtrn.send(MainToLoraCommand::Return);
            }
        }
    }

    fn handle_lora_loop(&mut self, ctx: LoraCommandContext<'_>) {
        loop {
            select! {
                recv(self.lora_cmd) -> cmd => {
                    if let Ok(v) = cmd {
                        self.handle_lora_commands(v, &ctx);
                    }
                }
                recv(self.lora_back) -> _ => {
                    break;
                }
            }
        }
    }
}
