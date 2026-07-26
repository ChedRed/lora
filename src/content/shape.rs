use crossbeam::channel::Sender;
use mlua::UserData;

use crate::utils::{LoraToMainCommand, Vertex};

#[derive(Clone)]
pub struct LoraShapeRef {
    pub uid: u64,
    pub tx: Sender<LoraToMainCommand>,
}

impl UserData for LoraShapeRef {}

#[derive(Clone)]
pub struct LoraShape {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl LoraShape {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
        }
    }
}