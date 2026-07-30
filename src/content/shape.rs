use crossbeam::channel::Sender;
use mlua::{UserData, UserDataMethods};

use crate::utils::{LoraToMainCommand, Vertex};

#[derive(Clone)]
pub struct LoraShapeRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
}

impl UserData for LoraShapeRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.uuid)
        });
    }
}

#[derive(Clone)]
pub struct LoraShape {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture_bytes: Option<Vec<u8>>,
    pub texture_dimensions: Option<(u32, u32)>,
}

impl LoraShape {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>, texture_bytes: Option<Vec<u8>>, texture_dimensions: Option<(u32, u32)>) -> Self {
        Self {
            vertices,
            indices,
            texture_bytes,
            texture_dimensions,
        }
    }
}