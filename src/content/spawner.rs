use crossbeam::channel::{Receiver, Sender};
use mlua::{Table, UserData, UserDataMethods};
use rapier2d::{
    dynamics::{RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
    geometry::{ColliderBuilder, ColliderSet},
    math::Vec2,
    pipeline::ActiveEvents,
};

use crate::{
    content::{collider::LoraCollider, shape::LoraShape},
    utils::{Location, LoraToMainCommand, MainToLoraCommand},
};
use wgpu::{naga::FastHashMap, util::DeviceExt};

#[derive(Clone)]
pub struct LoraSpawnerRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraSpawnerRef {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.uuid));
    }
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("spawn", |_, this, (x, y, r)| {
            _ = this.tx.send(LoraToMainCommand::SpawnerSpawn {
                uuid: this.uuid,
                x,
                y,
                r,
            });
            let mut real_object: Option<LoraObjectRef> = None;
            match this.rx.recv().unwrap() {
                MainToLoraCommand::ReturnNewObject { object } => {
                    real_object = Some(object);
                }
                _ => {}
            }
            Ok(real_object)
        });
    }
}

#[derive(Clone)]
pub struct LoraObjectRef {
    pub parent_uuid: u128,
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraObjectRef {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.uuid));

        fields.add_field_function_get("world", |lua, this| {
            let that = this.borrow::<LoraObjectRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::ObjectGetWorld {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
            });
            let mut real_position: [f32; 2] = [0., 0.];
            match that.rx.recv().unwrap() {
                MainToLoraCommand::ReturnObjectGetWorld { position } => {
                    real_position = position;
                }
                _ => {}
            }

            let store = lua.create_table().unwrap();
            _ = store.raw_set("x", real_position[0]);
            _ = store.raw_set("y", real_position[1]);

            let mt = lua.create_table()?;

            _ = mt.raw_set("__index", store);

            let tx2 = that.tx.clone();
            let rx2 = that.rx.clone();

            mt.raw_set(
                "__newindex",
                lua.create_function(
                    // for SET
                    move |_, (this, key, value): (Table, String, f32)| {
                        this.raw_set(key.clone(), value)?;
                        let x = this.raw_get::<f32>("x").unwrap_or_default();
                        let y = this.raw_get::<f32>("y").unwrap_or_default();
                        _ = tx2.send(LoraToMainCommand::ObjectSetWorld {
                            parent_uuid: that.parent_uuid,
                            uuid: that.uuid,
                            x,
                            y,
                            c: key,
                        });
                        _ = rx2.recv();
                        Ok(())
                    },
                )?,
            )?;

            let pos = lua.create_table().unwrap();
            pos.set_metatable(Some(mt)).unwrap();

            Ok(pos)
        });

        fields.add_field_function_set("world", |_, this, nevw: Table| {
            let that = this.borrow::<LoraObjectRef>().unwrap();

            let x = nevw.raw_get::<f32>("x");
            let y = nevw.raw_get::<f32>("y");

            let mut c: String = String::new();

            if x.is_ok() {
                c += "x";
            }
            if y.is_ok() {
                c += "y";
            }

            _ = that.tx.send(LoraToMainCommand::ObjectSetWorld {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
                x: x.unwrap_or_default(),
                y: y.unwrap_or_default(),
                c,
            });
            _ = that.rx.recv();
            Ok(())
        });

        fields.add_field_function_get("position", |lua, this| {
            let that = this.borrow::<LoraObjectRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::ObjectGetPosition {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
            });
            let mut real_position: [f32; 2] = [0., 0.];
            match that.rx.recv().unwrap() {
                MainToLoraCommand::ReturnObjectGetPosition { position } => {
                    real_position = position;
                }
                _ => {}
            }

            let store = lua.create_table().unwrap();
            _ = store.raw_set("x", real_position[0]);
            _ = store.raw_set("y", real_position[1]);

            let mt = lua.create_table().unwrap();

            _ = mt.raw_set("__index", store);

            let tx2 = that.tx.clone();
            let rx2 = that.rx.clone();

            mt.raw_set(
                "__newindex",
                lua.create_function(
                    // for SET
                    move |_, (this, key, value): (Table, String, f32)| {
                        this.raw_set(key.clone(), value)?;
                        let x = this.raw_get::<f32>("x").unwrap_or_default();
                        let y = this.raw_get::<f32>("y").unwrap_or_default();
                        _ = tx2.send(LoraToMainCommand::ObjectSetPosition {
                            parent_uuid: that.parent_uuid,
                            uuid: that.uuid,
                            x,
                            y,
                            c: key,
                        });
                        _ = rx2.recv();
                        Ok(())
                    },
                )?,
            )?;

            let pos = lua.create_table().unwrap();
            pos.set_metatable(Some(mt)).unwrap();

            Ok(pos)
        });

        fields.add_field_function_set("position", |_, this, nevw: Table| {
            let that = this.borrow::<LoraObjectRef>().unwrap();

            let x = nevw.raw_get::<f32>("x");
            let y = nevw.raw_get::<f32>("y");

            let mut c: String = String::new();

            if x.is_ok() {
                c += "x";
            }
            if y.is_ok() {
                c += "y";
            }

            _ = that.tx.send(LoraToMainCommand::ObjectSetPosition {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
                x: x.unwrap_or_default(),
                y: y.unwrap_or_default(),
                c,
            });
            _ = that.rx.recv();
            Ok(())
        });

        fields.add_field_function_get("motion", |lua, this| {
            let that = this.borrow::<LoraObjectRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::ObjectGetMotion {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
            });
            let mut real_motion: [f32; 3] = [0., 0., 0.];
            match that.rx.recv().unwrap() {
                MainToLoraCommand::ReturnObjectGetMotion { motion } => {
                    real_motion = motion;
                }
                _ => {}
            }

            let store = lua.create_table().unwrap();
            _ = store.raw_set("x", real_motion[0]);
            _ = store.raw_set("y", real_motion[1]);
            _ = store.raw_set("r", real_motion[2]);

            let mt = lua.create_table()?;

            _ = mt.raw_set("__index", store);

            let tx2 = that.tx.clone();
            let rx2 = that.rx.clone();

            mt.raw_set(
                "__newindex",
                lua.create_function(
                    // for SET
                    move |_, (this, key, value): (Table, String, f32)| {
                        this.raw_set(key.clone(), value)?; // TODO: add tests to check key
                        let x = this.raw_get::<f32>("x").unwrap_or_default();
                        let y = this.raw_get::<f32>("y").unwrap_or_default();
                        let r = this.raw_get::<f32>("r").unwrap_or_default();
                        _ = tx2.send(LoraToMainCommand::ObjectSetMotion {
                            // TODO: Implement partial/component function
                            parent_uuid: that.parent_uuid,
                            uuid: that.uuid,
                            x,
                            y,
                            r,
                            c: key,
                        });
                        _ = rx2.recv();
                        Ok(())
                    },
                )?,
            )?;

            let pos = lua.create_table().unwrap();
            pos.set_metatable(Some(mt)).unwrap();

            Ok(pos)
        });

        fields.add_field_function_set("motion", |_, this, nevw: Table| {
            let that = this.borrow::<LoraObjectRef>().unwrap();
            let x = nevw.raw_get::<f32>("x");
            let y = nevw.raw_get::<f32>("y");
            let r = nevw.raw_get::<f32>("r");

            let mut c: String = String::new();

            if x.is_ok() {
                c += "x";
            }
            if y.is_ok() {
                c += "y";
            }
            if r.is_ok() {
                c += "r";
            }

            _ = that.tx.send(LoraToMainCommand::ObjectSetMotion {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
                x: x.unwrap_or_default(),
                y: y.unwrap_or_default(),
                r: r.unwrap_or_default(),
                c,
            });
            _ = that.rx.recv();
            Ok(())
        });

        fields.add_field_function_get("angle", |_, this| {
            let that = this.borrow::<LoraObjectRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::ObjectAngle {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
            });
            let mut real_angle: f32 = 0.;
            match that.rx.recv().unwrap() {
                MainToLoraCommand::ReturnObjectGetAngle { angle } => {
                    real_angle = angle;
                }
                _ => {}
            }
            Ok(real_angle)
        });

        fields.add_field_function_set("angle", |_, this, r| {
            let that = this.borrow::<LoraObjectRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::ObjectSetAngle {
                parent_uuid: that.parent_uuid,
                uuid: that.uuid,
                r,
            });
            _ = that.rx.recv();
            Ok(())
        });
    }
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.uuid));
        methods.add_method("impulse", |_, this, (x, y)| {
            _ = this.tx.send(LoraToMainCommand::ObjectImpulse {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
                x,
                y,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("force", |_, this, (x, y)| {
            _ = this.tx.send(LoraToMainCommand::ObjectForce {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
                x,
                y,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("shove", |_, this, (x1, y1, x2, y2)| {
            _ = this.tx.send(LoraToMainCommand::ObjectShove {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
                x1,
                y1,
                x2,
                y2,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("torque", |_, this, r| {
            _ = this.tx.send(LoraToMainCommand::ObjectTorque {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
                r,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("show", |_, this, ()| {
            _ = this.tx.send(LoraToMainCommand::ObjectShow {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("hide", |_, this, ()| {
            _ = this.tx.send(LoraToMainCommand::ObjectHide {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("enable", |_, this, ()| {
            _ = this.tx.send(LoraToMainCommand::ObjectEnable {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("disable", |_, this, ()| {
            _ = this.tx.send(LoraToMainCommand::ObjectDisable {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
            });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("toggle", |_, this, ()| {
            _ = this.tx.send(LoraToMainCommand::ObjectToggle {
                parent_uuid: this.parent_uuid,
                uuid: this.uuid,
            });
            _ = this.rx.recv();
            Ok(())
        });
    }
}

pub struct LoraSpawner {
    pub indices: u32,
    pub locations: FastHashMap<u128, Location>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub location_buffer: Option<wgpu::Buffer>,
    pub texture_bind_group: Option<wgpu::BindGroup>,
    render: bool,

    pub hull: Option<ColliderBuilder>,
    pub center: Option<(f32, f32)>,
    pub rigidhandles: FastHashMap<u128, RigidBodyHandle>,
    collision: String,
    collide: bool,

    pub status: FastHashMap<u128, bool>,
}

impl LoraSpawner {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        shape: Option<LoraShape>,
        collider: Option<LoraCollider>,
    ) -> Self {
        let mut points: Vec<Vec2> = Vec::new();
        let mut hull: Option<ColliderBuilder> = None;
        let mut center: Option<(f32, f32)> = None;
        let rigidhandles: FastHashMap<u128, RigidBodyHandle> = FastHashMap::default();
        let mut collide: bool = false;
        let mut collision: String = "static".to_string();
        if let Some(real_collider) = collider {
            collision = real_collider.collision;
            collide = true;
            for vertex in real_collider.vertices.iter() {
                points.push(Vec2 {
                    x: vertex.position[0],
                    y: vertex.position[1],
                })
            }

            hull = Some(
                ColliderBuilder::convex_hull(&points.clone().into_boxed_slice())
                    .unwrap()
                    .restitution(0.2)
                    .friction(0.2)
                    .density(5.)
                    .active_events(ActiveEvents::COLLISION_EVENTS),
            );

            let precenter = hull.clone().unwrap().build().mass_properties().local_com;
            center = Some((precenter.x, precenter.y));
        }

        let mut indices: u32 = 0;
        let locations: FastHashMap<u128, Location> = FastHashMap::default();
        let mut index_buffer: Option<wgpu::Buffer> = None;
        let mut vertex_buffer: Option<wgpu::Buffer> = None;
        let mut location_buffer: Option<wgpu::Buffer> = None;
        let mut texture_bind_group: Option<wgpu::BindGroup> = None;
        let mut render: bool = false;
        if let Some(real_shape) = shape {
            render = true;
            indices = real_shape.indices.len() as u32;
            index_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&real_shape.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
            );

            vertex_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&real_shape.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            );

            location_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Location Buffer"),
                size: (size_of::<Location>() * 200) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));

            let mut texture_size: (u32, u32) = (1, 1);

            if real_shape.texture_bytes.is_some() {
                texture_size = real_shape.texture_dimensions.unwrap();
            }

            let gpu_texture_size = wgpu::Extent3d {
                width: texture_size.0,
                height: texture_size.1,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Lora Texture"),
                size: gpu_texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let mut real_bytes: Vec<u8> = vec![255u8, 255u8, 255u8, 255u8];
            if real_shape.texture_bytes.is_some() {
                real_bytes = real_shape.texture_bytes.unwrap();
            }

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &real_bytes.into_boxed_slice(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * texture_size.0),
                    rows_per_image: Some(texture_size.1),
                },
                gpu_texture_size,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });

            texture_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Lora Texture Bind Group"),
                layout: &texture_layout,
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

        let status: FastHashMap<u128, bool> = FastHashMap::default();

        Self {
            indices,
            locations,
            vertex_buffer,
            index_buffer,
            location_buffer,
            texture_bind_group,
            render,

            hull,
            center,
            rigidhandles,
            collision,
            collide,

            status,
        }
    }

    pub fn spawn(
        &mut self,
        uuid: u128,
        x: f32,
        y: f32,
        rotation: f32,
        rigidbodies: &mut RigidBodySet,
        colliders: &mut ColliderSet,
    ) {
        if self.render {
            self.locations.insert(
                uuid,
                Location {
                    position: [x, y],
                    rotation: [rotation, 0.],
                },
            );
        }
        if self.collide {
            let rb: RigidBody;
            if self.collision == "static" {
                rb = RigidBodyBuilder::fixed()
                    .translation(Vec2 { x, y })
                    .rotation(rotation)
                    .user_data(uuid)
                    .build();
            } else if self.collision == "diaxial" {
                rb = RigidBodyBuilder::dynamic()
                    .translation(Vec2 { x, y })
                    .rotation(rotation)
                    .ccd_enabled(true)
                    .lock_rotations()
                    .user_data(uuid)
                    .build();
            } else {
                rb = RigidBodyBuilder::dynamic()
                    .translation(Vec2 { x, y })
                    .rotation(rotation)
                    .ccd_enabled(true)
                    .user_data(uuid)
                    .build();
            }
            let rb_handle = rigidbodies.insert(rb);
            self.rigidhandles.insert(uuid, rb_handle);

            if let Some(hullshape) = self.hull.as_mut() {
                colliders.insert_with_parent(hullshape.clone(), rb_handle, rigidbodies);
            }
        }

        self.status.insert(uuid, true);
    }

    pub fn renderable(&self) -> bool {
        self.render
    }

    pub fn collidable(&self) -> bool {
        self.collide
    }
}
