use crossbeam::channel::Sender;
use mlua::{UserData, UserDataMethods};

use crate::utils::{LoraToMainCommand, Vertex};

#[derive(Clone)]
pub struct LoraColliderRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
}

impl UserData for LoraColliderRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.uuid)
        });
    }
}

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