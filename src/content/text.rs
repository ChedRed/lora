use crossbeam::channel::{Receiver, Sender};
use glyphon::{Attrs, Buffer, Color, Metrics, TextBounds, cosmic_text::Align};
use mlua::{Table, UserData};

use crate::utils::{LoraToMainCommand, MainToLoraCommand, font::FontGod};

pub struct LoraTextRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraTextRef {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.uuid));
        fields.add_field_function_get("bounds", |lua, this| {
            let that = this.borrow::<LoraTextRef>().unwrap();
            _ = that
                .tx
                .send(LoraToMainCommand::TextGetBounds { uuid: that.uuid });
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
                            parent_uuid: that.uuid,
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
        // fields.add_field_function_get("size", |lua, this| Ok(()));
        // fields.add_field_function_get("color", |lua, this| Ok(()));
    }
}

pub struct LoraText {
    pub buffer: Buffer,
    pub data: String,
    pub size: f32,
    pub color: Color,
    pub position: [f32; 2],
    pub bounds: Option<TextBounds>,
    pub wrap: u8,  // 0x000: none (undef), 0x001: word, 0x010: char, 0x011: both
    pub align: u8, // 0x000: left (undef), 0x001: center, 0x010: right, 0x011: justified, 0x100: end
}

impl LoraText {
    pub fn new(
        fontgod: &mut FontGod,
        data: String,
        size: f32,
        color: u32,
        position: [f32; 2],
        text_bounds: Option<[f32; 4]>,
        wrap: u8,
        align: u8,
    ) -> Self {
        let mut buffer = Buffer::new(&mut fontgod.fonts, Metrics::relative(size, 1.)); // TODO: Allow wrap and line height to be defined
        buffer.set_text(
            data.as_str(),
            &Attrs::new(),
            glyphon::Shaping::Advanced,
            Some(Align::Left),
        );

        let mut bounds = Some(TextBounds::default());
        if let Some(prebounds) = text_bounds {
            bounds = Some(TextBounds {
                left: prebounds[0] as i32,
                top: prebounds[1] as i32,
                right: prebounds[2] as i32,
                bottom: prebounds[3] as i32,
            })
        }

        let color = Color { 0: color };

        Self {
            buffer,
            data,
            position,
            bounds,
            size,
            color,
            wrap,
            align,
        }
    }
}
