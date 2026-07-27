use crossbeam::channel::{Sender, Receiver};
use mlua::{UserData, UserDataMethods};
use rapier2d::{dynamics::{RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet}, geometry::{ColliderBuilder, ColliderSet}, math::Vec2};

use wgpu::{naga::FastHashMap, util::DeviceExt};
use crate::{content::{collider::LoraCollider, shape::LoraShape}, utils::{Location, LoraToMainCommand, MainToLoraCommand}};


#[derive(Clone)]
pub struct LoraSpawnerRef {
    pub uid: u64,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraSpawnerRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("spawn", |_, this, (x, y, r)| {
            _= this.tx.send(LoraToMainCommand::SpawnerSpawn { uid: this.uid, x, y, r });
            let mut real_object: Option<LoraObjectRef> = None;
            while let Ok(cmd) = this.rx.recv() {
                match cmd {
                    MainToLoraCommand::ReturnNewObject { object } => {
                        real_object = Some(object);
                        break;
                    }
                    _ => {}
                }
            }
            Ok(real_object)
        });
    }
}

#[derive(Clone)]
pub struct LoraObjectRef {
    pub puid: u64,
    pub uid: u64,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraObjectRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("set_position", |_, this, (x, y)| {
            _= this.tx.send(LoraToMainCommand::ObjectSetPosition { puid: this.puid, uid: this.uid, x, y });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("set_motion", |_, this, (x, y)| {
            _= this.tx.send(LoraToMainCommand::ObjectSetMotion { puid: this.puid, uid: this.uid, x, y });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("set_angle", |_, this, r| {
            _= this.tx.send(LoraToMainCommand::ObjectSetAngle { puid: this.puid, uid: this.uid, r });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("get_position", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::ObjectGetPosition { puid: this.puid, uid: this.uid });
            let mut real_position: [f32; 2] = [0., 0.];
            while let Ok(cmd) = this.rx.recv() {
                match cmd {
                    MainToLoraCommand::ReturnObjectGetPosition { position } => {
                        real_position = position;
                        break;
                    }
                    _ => {}
                }
            }
            Ok(real_position)
        });
        methods.add_method("get_motion", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::ObjectGetMotion { puid: this.puid, uid: this.uid });
            let mut real_motion: [f32; 2] = [0., 0.];
            while let Ok(cmd) = this.rx.recv() {
                match cmd {
                    MainToLoraCommand::ReturnObjectGetMotion { motion } => {
                        real_motion = motion;
                        break;
                    }
                    _ => {}
                }
            }
            Ok(real_motion)
        });
        methods.add_method("get_angle", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::ObjectGetAngle { puid: this.puid, uid: this.uid });
            let mut real_angle: f32 = 0.;
            while let Ok(cmd) = this.rx.recv() {
                match cmd {
                    MainToLoraCommand::ReturnObjectGetAngle { angle } => {
                        real_angle = angle;
                        break;
                    }
                    _ => {}
                }
            }
            Ok(real_angle)
        });
        methods.add_method("impulse", |_, this, (x, y)| {
            _= this.tx.send(LoraToMainCommand::ObjectImpulse { puid: this.puid, uid: this.uid, x, y });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("add_force", |_, this, (x, y)| {
            _= this.tx.send(LoraToMainCommand::ObjectAddForce { puid: this.puid, uid: this.uid, x, y });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("add_world_force", |_, this, (x1, y1, x2, y2)| {
            _= this.tx.send(LoraToMainCommand::ObjectAddWorldForce { puid: this.puid, uid: this.uid, x1, y1, x2, y2 });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("add_torque", |_, this, r| {
            _= this.tx.send(LoraToMainCommand::ObjectAddTorque { puid: this.puid, uid: this.uid, r });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("enable", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::ObjectEnable { puid: this.puid, uid: this.uid });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("disable", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::ObjectDisable { puid: this.puid, uid: this.uid });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("toggle", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::ObjectToggle { puid: this.puid, uid: this.uid });
            _= this.rx.recv();
            Ok(())
        });
    }
}

pub struct LoraSpawner {
    count: u64,
    
    pub indices: u32,
    pub locations: FastHashMap<u64, Location>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub location_buffer: Option<wgpu::Buffer>,
    pub texture_bind_group: Option<wgpu::BindGroup>,
    render: bool,
    
    pub hull: Option<ColliderBuilder>,
    pub rigidhandles: FastHashMap<u64, RigidBodyHandle>,
    collision: String,
    collide: bool,
}

impl LoraSpawner {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, shape: Option<LoraShape>, collider: Option<LoraCollider>) -> Self {
        let count: u64 = 0;
        
        let mut points: Vec<Vec2> = Vec::new();
        let mut hull: Option<ColliderBuilder> = None;
        let rigidhandles: FastHashMap<u64, RigidBodyHandle> = FastHashMap::default();
        let mut collide: bool = false;
        let mut collision: String = "static".to_string();
        if let Some(real_collider) = collider {
            collision = real_collider.collision;
            collide = true;
            for vertex in real_collider.vertices.iter() {
                points.push(Vec2{
                    x: vertex.position[0],
                    y: vertex.position[1],
                })
            }

            hull = Some(ColliderBuilder::convex_hull(&points.clone().into_boxed_slice()).unwrap()
                .restitution(0.2)
                .friction(0.2)
                .density(0.001)); // TODO: Make it accessible via Lua
        }

        let mut indices: u32 = 0;
        let locations: FastHashMap<u64, Location> = FastHashMap::default();
        let mut index_buffer: Option<wgpu::Buffer> = None;
        let mut vertex_buffer: Option<wgpu::Buffer> = None;
        let mut location_buffer: Option<wgpu::Buffer> = None;
        let mut texture_bind_group: Option<wgpu::BindGroup> = None;
        let mut render: bool = false;
        if let Some(real_shape) = shape {
            render = true;
            indices = real_shape.indices.len() as u32;
            index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&real_shape.indices),
                usage: wgpu::BufferUsages::INDEX,
            }));
            
            vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&real_shape.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }));
            
            location_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { // TODO: Replace 200 with a reasonable number
                label: Some("Location Buffer"),
                size: (size_of::<Location>() * 200) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));

            if real_shape.texture_bytes.is_some() {
                let texture_size = wgpu::Extent3d {
                    width: real_shape.texture_dimensions.unwrap().0,
                    height: real_shape.texture_dimensions.unwrap().1,
                    depth_or_array_layers: 1,
                };
                
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Lora Texture"),
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &real_shape.texture_bytes.unwrap().into_boxed_slice(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * real_shape.texture_dimensions.unwrap().0),
                        rows_per_image: Some(real_shape.texture_dimensions.unwrap().1),
                    },
                    texture_size,
                );

                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor { // TODO: Make it owned by Main
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                    ..Default::default()
                });

                let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

                texture_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Lora Texture Bind Group"),
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                }));
            }
        }

        
        Self {
            count,
            
            indices,
            locations,
            vertex_buffer,
            index_buffer,
            location_buffer,
            texture_bind_group,
            render,
            
            hull,
            rigidhandles,
            collision,
            collide,
        }
    }

    pub fn spawn(&mut self, x: f32, y: f32, rotation: f32, rigidbodies: &mut RigidBodySet, colliders: &mut ColliderSet) -> u64 {
        if self.render {
            self.locations.insert(self.count, Location {position: [x, y], rotation: [rotation, 0.]});
        }
        if self.collide {
            let rb: RigidBody;
            if self.collision == "static" {
                rb = RigidBodyBuilder::fixed()
                    .translation(Vec2 { x, y })
                    .rotation(rotation)
                    .build();
            } else if self.collision == "diaxial" {
                rb = RigidBodyBuilder::dynamic()
                    .translation(Vec2 { x, y })
                    .rotation(rotation)
                    .ccd_enabled(true)
                    .lock_rotations()
                    .build();
            } else {
                rb = RigidBodyBuilder::dynamic()
                    .translation(Vec2 { x, y })
                    .rotation(rotation)
                    .ccd_enabled(true)
                    .build();
            }
            let rb_handle = rigidbodies.insert(rb);
            self.rigidhandles.insert(self.count, rb_handle);
    
            if let Some(hullshape) = self.hull.as_mut() {
                colliders.insert_with_parent(hullshape.clone(), rb_handle, rigidbodies);
            }
        }
        self.count += 1;
        return self.count - 1;
    }

    pub fn renderable(&self) -> bool {
        self.render
    }

    pub fn collidable(&self) -> bool {
        self.collide
    }
}