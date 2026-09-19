use crossbeam::channel::{Receiver, Sender};
use mlua::{Table, UserData, UserDataMethods};
use rapier2d::{
    dynamics::{RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
    geometry::{ColliderBuilder, ColliderSet},
    math::Vec2,
};

use crate::utils::{LoraToMainCommand, MainToLoraCommand};

#[derive(Clone)]
pub struct LoraBorderRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
    pub pos: Table,
}

impl UserData for LoraBorderRef {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.uuid));

        fields.add_field_function_get("position", |_, this| {
            let that = this.borrow::<LoraBorderRef>().unwrap();
            _ = that
                .tx
                .send(LoraToMainCommand::BorderPosition { uuid: that.uuid });
            let mut real_position: [f32; 2] = [0., 0.];

            match that.rx.recv().unwrap() {
                MainToLoraCommand::ReturnBorderGetPosition { position } => {
                    real_position = position;
                }
                _ => {}
            }

            _ = that.pos.raw_set("x", real_position[0]);
            _ = that.pos.raw_set("y", real_position[1]);
            Ok(that.pos.clone())
        });
        fields.add_field_function_set("position", |_, this, nevw: Table| {
            let that = this.borrow::<LoraBorderRef>().unwrap();
            println!(
                "{}, {}",
                nevw.raw_get::<f32>("x").unwrap(),
                nevw.raw_get::<f32>("y").unwrap()
            );
            _ = that.tx.send(LoraToMainCommand::BorderSetPosition {
                uuid: that.uuid,
                x: nevw.raw_get("x").unwrap(),
                y: nevw.raw_get("y").unwrap(),
            });
            _ = that.rx.recv();
            Ok(())
        });

        fields.add_field_function_get("angle", |_, this| {
            let that = this.borrow::<LoraBorderRef>().unwrap();
            _ = that
                .tx
                .send(LoraToMainCommand::BorderAngle { uuid: that.uuid });
            let mut real_angle: f32 = 0.;

            match that.rx.recv().unwrap() {
                MainToLoraCommand::ReturnBorderGetAngle { angle } => {
                    real_angle = angle;
                }
                _ => {}
            }

            Ok(real_angle)
        });
        fields.add_field_function_set("angle", |_, this, nevw: f32| {
            let that = this.borrow::<LoraBorderRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::BorderSetAngle {
                uuid: that.uuid,
                r: nevw,
            });
            _ = that.rx.recv();
            Ok(())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("enable", |_, this, ()| {
            _ = this
                .tx
                .send(LoraToMainCommand::BorderEnable { uuid: this.uuid });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("disable", |_, this, ()| {
            _ = this
                .tx
                .send(LoraToMainCommand::BorderDisable { uuid: this.uuid });
            _ = this.rx.recv();
            Ok(())
        });
        methods.add_method("toggle", |_, this, ()| {
            _ = this
                .tx
                .send(LoraToMainCommand::BorderToggle { uuid: this.uuid });
            _ = this.rx.recv();
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct LoraBorder {
    pub rigidhandle: RigidBodyHandle,
}

impl LoraBorder {
    pub fn new(
        uuid: u128,
        points: Vec<Vec2>,
        indices: Option<Vec<[u32; 2]>>,
        rigidbodies: &mut RigidBodySet,
        colliders: &mut ColliderSet,
    ) -> Self {
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
