use crossbeam::channel::Sender;
use glyphon::{Buffer, Color, Metrics, TextBounds};
use mlua::UserData;

use crate::utils::{LoraToMainCommand, font::FontGod};

pub struct LoraTextRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
}

impl UserData for LoraTextRef {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.uuid));
        fields.add_field_function_get("bounds", |lua, this| {
            let that = this.borrow::<LoraTextRef>().unwrap();
            _ = that.tx.send(LoraToMainCommand::TextGetBounds {
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
        fields.add_field_function_get("size", |lua, this| {
            Ok
        });
        fields.add_field_function_get("color", |lua, this| {
            Ok
        });
    }
}

pub struct LoraText {
    pub buffer: Buffer,
    pub position: [f32; 2],
    pub bounds: TextBounds,
    pub color: Color,
}

impl LoraText {
    pub fn new(fontgod: &mut FontGod, text_bounds: [f32; 4], font_size: f32, color: u32) -> Self {

        let buffer = Buffer::new(&mut fontgod.fonts, Metrics::relative(font_size, 1.)); // TODO: Allow wrap and line height to be defined

        let position: [f32; 2] = [text_bounds[0], text_bounds[1]];
        let bounds = TextBounds::default();

        let color = Color {0: color};

        Self {
            buffer,
            position,
            bounds,
            color,
        }
    }
}
