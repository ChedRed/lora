use crossbeam::channel::Sender;
use mlua::UserData;

use crate::utils::{LoraToMainCommand, Vertex};

#[derive(Clone)]
pub struct LoraColliderRef {
    pub uid: u64,
    pub tx: Sender<LoraToMainCommand>,
}

impl UserData for LoraColliderRef {}

#[derive(Clone)]
pub struct LoraCollider {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub collision: String,
}

impl LoraCollider {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>, collision: String) -> Self {
        Self {
            vertices,
            indices,
            collision,
        }
    }
}