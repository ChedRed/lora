use crossbeam::channel::{Receiver, Sender};
use mlua::{UserData, UserDataMethods};
use rapier2d::{dynamics::{RigidBodyBuilder, RigidBodyHandle, RigidBodySet}, geometry::{ColliderBuilder, ColliderSet}, math::Vec2};

use crate::utils::{LoraToMainCommand, MainToLoraCommand};

#[derive(Clone)]
pub struct LoraBorderRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraBorderRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.uuid)
        });
        methods.add_method("set_position", |_, this, (x, y)| {
            _= this.tx.send(LoraToMainCommand::BorderSetPosition { uuid: this.uuid, x, y });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("set_angle", |_, this, r| {
            _= this.tx.send(LoraToMainCommand::BorderSetAngle { uuid: this.uuid, r });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("position", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::BorderPosition { uuid: this.uuid });
            let mut real_position: [f32; 2] = [0., 0.];
            while let Ok(cmd) = this.rx.recv() {
                match cmd {
                    MainToLoraCommand::ReturnBorderGetPosition { position } => {
                        real_position = position;
                        break;
                    }
                    _ => {}
                }
            }
            Ok((real_position[0], real_position[1]))
        });
        methods.add_method("angle", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::BorderAngle { uuid: this.uuid });
            let mut real_angle: f32 = 0.;
            while let Ok(cmd) = this.rx.recv() {
                match cmd {
                    MainToLoraCommand::ReturnBorderGetAngle { angle } => {
                        real_angle = angle;
                        break;
                    }
                    _ => {}
                }
            }
            Ok(real_angle)
        });
        methods.add_method("enable", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::BorderEnable { uuid: this.uuid });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("disable", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::BorderDisable { uuid: this.uuid });
            _= this.rx.recv();
            Ok(())
        });
        methods.add_method("toggle", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::BorderToggle { uuid: this.uuid });
            _= this.rx.recv();
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct LoraBorder {
    pub rigidhandle: RigidBodyHandle,
}

impl LoraBorder {
    pub fn new(uuid: u128, points: Vec<Vec2>, indices: Option<Vec<[u32; 2]>>, rigidbodies: &mut RigidBodySet, colliders: &mut ColliderSet) -> Self {
        let rb = RigidBodyBuilder::kinematic_position_based()
            .user_data(uuid)
            .build();
        
        let rigidhandle = rigidbodies.insert(rb);
        
        let collider = ColliderBuilder::polyline(points, indices).build();
        colliders.insert_with_parent(collider, rigidhandle, rigidbodies);
        
        Self {
            rigidhandle: rigidhandle,
        }
    }
}