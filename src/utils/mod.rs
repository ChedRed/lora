pub mod transform;
pub mod lora;
pub mod print;

use std::path;

use image::EncodableLayout;
use transform::Vector2;

use crate::{content::{collider::LoraColliderRef, shape::LoraShapeRef, spawner::{LoraObjectRef, LoraSpawnerRef}}};

pub enum LoraToMainCommand {
    SetWindowTitle {
        text: String,
    },
    SetWindowSize {
        w: u32,
        h: u32,
    },
    SetWindowResizable {
        is: bool,
    },
    SetPhysicsGravity {
        x: f32,
        y: f32,
    },
    SetPhysicsHertz {
        hz: f64,
    },
    SetCameraPosition {
        x: f32,
        y: f32,
    },
    GetWindowSize,
    GetKeyPressed {
        key: String,
    },
    GetCameraPosition,
    NewImage {
        image: String,
        scale: f32,
    },
    NewShape {
        kind: String,
        w: f32,
        h: f32,
        color: [f32; 4],
    },
    NewMesh {
        vertices: Vec<[f32; 8]>,
        indices: Vec<u32>,
    },
    NewCollider {
        shape: LoraShapeRef,
        collision: String,
    },
    NewSpawner {
        shape: Option<LoraShapeRef>,
        collider: Option<LoraColliderRef>,
    },
    DrawPrimitive {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        color: [f32; 4],
        label: u32,
    },
    SpawnerSpawn {
        uid: u64,
        x: f32,
        y: f32,
        r: f32,
    },
    ObjectSetPosition {
        puid: u64,
        uid: u64,
        x: f32,
        y: f32,
    },
    ObjectSetMotion {
        puid: u64,
        uid: u64,
        x: f32,
        y: f32,
    },
    ObjectSetAngle {
        puid: u64,
        uid: u64,
        r: f32,
    },
    ObjectGetPosition {
        puid: u64,
        uid: u64,
    },
    ObjectGetMotion {
        puid: u64,
        uid: u64,
    },
    ObjectGetAngle {
        puid: u64,
        uid: u64,
    },
    ObjectImpulse {
        puid: u64,
        uid: u64,
        x: f32,
        y: f32,
    },
    ObjectAddForce {
        puid: u64,
        uid: u64,
        x: f32,
        y: f32,
    },
    ObjectAddWorldForce {
        puid: u64,
        uid: u64,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    ObjectAddTorque {
        puid: u64,
        uid: u64,
        r: f32,
    },
    ObjectEnable {
        puid: u64,
        uid: u64,
    },
    ObjectDisable {
        puid: u64,
        uid: u64,
    },
    ObjectToggle {
        puid: u64,
        uid: u64,
    },
}

pub enum MainToLoraCommand {
    ReturnGetWindowSize {
        w: u32,
        h: u32,
    },
    ReturnKeyPressed {
        key: bool,
    },
    ReturnCameraPosition {
        x: f32,
        y: f32,
    },
    ReturnNewImage {
        image: LoraShapeRef,
    },
    ReturnNewShape {
        shape: LoraShapeRef,
    },
    ReturnNewMesh {
        mesh: LoraShapeRef,
    },
    ReturnNewCollider {
        collider: LoraColliderRef,
    },
    ReturnNewSpawner {
        spawner: LoraSpawnerRef,
    },
    ReturnNewObject {
        object: LoraObjectRef,
    },
    ReturnObjectGetPosition {
        position: [f32; 2],
    },
    ReturnObjectGetMotion {
        motion: [f32; 2],
    },
    ReturnObjectGetAngle {
        angle: f32,
    },
    Return,
}

pub enum MainToLoraCall {
    Load,
    Keypressed {
        code: String,
    },
    Keyreleased {
        code: String,
    },
    Mousepressed {
        x: f32,
        y: f32,
        button: u32,
    },
    Mousereleased {
        x: f32,
        y: f32,
        button: u32,
    },
    MouseMoved {
        motion: (f32, f32),
    },
    MouseScrolled {
        motion: (f32, f32),
    },
    Update {
        delta: f32,
    },
    Render,
    Exit
}

pub enum LoraToMainCall {
    Load,
    Keypressed,
    Keyreleased,
    Mousepressed,
    Mousereleased,
    MouseMoved,
    MouseScrolled,
    Draw,
    Render,
    GetWindowSize,
    Exit
}

pub struct Xy {
    pub position: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, serde::Deserialize, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Location {
    pub position: [f32; 2],
    pub rotation: [f32; 2],
}

impl Location {
    pub fn new() -> Self {
        Self {
            position: [0., 0.],
            rotation: [0., 0.],
        }
    }
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Location>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}


#[derive(Clone, Copy)]
pub struct Displacement {
    pub position: Vector2,
    pub velocity: Vector2,
    pub rotation: f32,
}

impl Displacement {
    pub fn new() -> Self {
        Self {
            position: Vector2::new(),
            velocity: Vector2::new(),
            rotation: 0.,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Primitive {
    pub xywh: [f32; 4],
    pub angle: f32,
    pub label: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub color: [f32; 4],
}

impl Primitive {
    pub fn new() -> Self {
        Self {
            xywh: [0., 0., 0., 0.],
            angle: 0.,
            label: 0,
            _pad0: 0,
            _pad1: 0,
            color: [0., 0., 0., 0.],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GPUPrimitives {
    pub count: u32,
    pub _pad: u32,
    pub scale: [f32; 2],
    pub data: [Primitive; 256],
}

impl GPUPrimitives {
    pub fn from_vec(size: u32, data: &[Primitive]) -> Self {
        let mut primitives = GPUPrimitives {
            count: size,
            _pad: 0,
            scale: [0., 0.],
            data: [Primitive { xywh: [0., 0., 0., 0.], angle: 0., label: 0, _pad0: 0, _pad1: 0, color: [0., 0., 0., 0.]}; 256],
        };

        for (i, p) in data.iter().take(256).enumerate() {
            primitives.data[i] = *p;
        }
    
        primitives
    }
}

pub fn get_image(prefix: String, image: String) -> (Vec<u8>, (u32, u32)) {
    let img = image::ImageReader::open(path::Path::new(&prefix).join(&image)).unwrap().decode().unwrap();
    let real_img = img.as_rgba8().unwrap();

    let dimensions = real_img.dimensions();
    let bytes = real_img.as_bytes();

    let mut real_bytes: Vec<u8> = Vec::new();
    for byte in bytes {
        real_bytes.push(*byte);
    }
    
    (real_bytes, dimensions)
}