use std::io::Cursor;

use crossbeam::channel::{Receiver, Sender};
use mlua::{UserData, UserDataMethods};
use rodio::Decoder;

use crate::utils::{LoraToMainCommand, MainToLoraCommand};

#[derive(Clone)]
pub struct LoraSoundRef {
    pub uuid: u128,
    pub tx: Sender<LoraToMainCommand>,
    pub rx: Receiver<MainToLoraCommand>,
}

impl UserData for LoraSoundRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.uuid)
        });
        methods.add_method("play", |_, this, ()| {
            _= this.tx.send(LoraToMainCommand::SoundPlay { uuid: this.uuid });
            _= this.rx.recv();
            Ok(())
        });
    }
}

pub struct LoraSound {
    pub source: rodio::source::Buffered<Decoder<Cursor<Box<[u8]>>>>,
}

impl LoraSound {
    pub fn new(source: rodio::source::Buffered<Decoder<Cursor<Box<[u8]>>>>) -> Self {
        Self {
            source
        }
    }
}