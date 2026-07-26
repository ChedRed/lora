use std::process::exit;

use crossbeam::channel::{Receiver, Sender};
use mlua::{Function, UserDataRef};

use crate::{content::{collider::LoriColliderRef, shape::LoriShapeRef, spawner::LoriSpawnerRef}, utils::{LoriToMainCall, LoriToMainCommand, MainToLoriCall, MainToLoriCommand, print::{dbugln, serorln, vbosln}}};

pub struct Lori {
    _lua: mlua::Lua,
    main_call: Receiver<MainToLoriCall>,
    main_back: Sender<LoriToMainCall>,

    lori_load: Option<mlua::Function>,
    lori_keypressed: Option<mlua::Function>,
    lori_keyreleased: Option<mlua::Function>,
    lori_mousepressed: Option<mlua::Function>,
    lori_mousereleased: Option<mlua::Function>,
    lori_mousemoved: Option<mlua::Function>,
    lori_mousescrolled: Option<mlua::Function>,
    lori_update: Option<mlua::Function>,
    lori_render: Option<mlua::Function>,
    lori_exit: Option<mlua::Function>,
}

impl Lori {
    pub fn new(code: String, verbose: bool, main_cmd: Sender<LoriToMainCommand>, main_rtrn: Receiver<MainToLoriCommand>, main_call: Receiver<MainToLoriCall>, main_back: Sender<LoriToMainCall>) -> Self {
        let _lua = mlua::Lua::new();
        let lori = _lua.create_table().unwrap();
    
        let set = _lua.create_table().unwrap();
        let set_physics = _lua.create_table().unwrap();
        _= set_physics.set("gravity", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.set.physics.gravity
            move |_, (x, y)| {
                _= tx.send(LoriToMainCommand::SetPhysicsGravity { x, y });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        _= set_physics.set("hertz", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.set.physics.hertz
            move |_, hz| {
                _= tx.send(LoriToMainCommand::SetPhysicsHertz { hz });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        let set_window = _lua.create_table().unwrap();
        _= set_window.set("title", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.set.window.title
            move |_, text| {
                _= tx.send(LoriToMainCommand::SetWindowTitle { text });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        _= set_window.set("size", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.set.window.size
            move |_, (w, h)| {
                _= tx.send(LoriToMainCommand::SetWindowSize { w, h });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        _= set_window.set("resizable", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.set.window.resizable
            move |_, is| {
                _= tx.send(LoriToMainCommand::SetWindowResizable { is });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        let set_camera = _lua.create_table().unwrap();
        _= set_camera.set("position", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.set.camera.position
            move |_, (x, y)| {
                _= tx.send(LoriToMainCommand::SetCameraPosition { x, y });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        
        let get = _lua.create_table().unwrap();
        let get_window = _lua.create_table().unwrap();
        _= get_window.set("size", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.get.window.size
            move |_, ()| {
                let mut nw: u32 = 0;
                let mut nh: u32 = 0;
                _= tx.try_send(LoriToMainCommand::GetWindowSize);
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        MainToLoriCommand::ReturnGetWindowSize { w, h } => {
                            nw = w;
                            nh = h;
                            break;
                        }
                        _ => {}
                    }
                }
                Ok((nw, nh))
            }
        }).unwrap());
        let get_key = _lua.create_table().unwrap();
        _= get_key.set("state", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.get.key.state
            move |_, key| {
                let mut pressed: bool = false;
                _= tx.try_send(LoriToMainCommand::GetKeyPressed { key });
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        MainToLoriCommand::ReturnKeyPressed { key } => {
                            pressed = key;
                            break;
                        }
                        _ => {}
                    }
                }
                Ok(pressed)
            }
        }).unwrap());
        let get_camera = _lua.create_table().unwrap();
        _= get_camera.set("position", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.get.camera.position
            move |_, ()| {
                let mut position: [f32; 2] = [0., 0.];
                _= tx.try_send(LoriToMainCommand::GetCameraPosition);
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        MainToLoriCommand::ReturnCameraPosition { x, y } => {
                            position = [x, y];
                            break;
                        }
                        _ => {}
                    }
                }
                Ok(position)
            }
        }).unwrap());
        
        let new = _lua.create_table().unwrap();
        _= new.set("shape", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.new.shape
            move |_, (kind, w, h, color)| {
                _= tx.send(LoriToMainCommand::NewShape { kind, w, h, color });
                let mut new_shape: Option<LoriShapeRef> = None;
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        MainToLoriCommand::ReturnNewShape { shape } => {
                            new_shape = Some(shape);
                            break;
                        }
                        _ => {}
                    }
                }
                
                Ok(new_shape)
            }
        }).unwrap());
        _= new.set("mesh", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.new.mesh
            move |_, (vertices, indices): (Vec<[f32; 8]>, Vec<u32>)| {
                dbugln("eaeae");
                _= tx.send(LoriToMainCommand::NewMesh { vertices: vertices.clone(), indices });
                let mut new_shape: Option<LoriShapeRef> = None;
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        MainToLoriCommand::ReturnNewMesh { mesh } => {
                            new_shape = Some(mesh);
                            break;
                        }
                        _ => {}
                    }
                }
                
                Ok(new_shape)
            }
        }).unwrap());
        _= new.set("collider", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.new.collider
            move |_, (shape, collision): (UserDataRef<LoriShapeRef>, String)| {
                _= tx.send(LoriToMainCommand::NewCollider { shape: shape.clone(), collision });
                let mut new_collider: Option<LoriColliderRef> = None;
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        MainToLoriCommand::ReturnNewCollider { collider } => {
                            new_collider = Some(collider);
                            break;
                        }
                        _ => {}
                    }
                }
                
                Ok(new_collider)
            }
        }).unwrap());
        _= new.set("spawner", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.new.spawner
            move |_, (shape, collider): (Option<UserDataRef<LoriShapeRef>>, Option<UserDataRef<LoriColliderRef>>)| {
            _= tx.send(LoriToMainCommand::NewSpawner { shape: shape.as_deref().cloned(), collider: collider.as_deref().cloned() });
            let mut new_spawner: Option<LoriSpawnerRef> = None;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    MainToLoriCommand::ReturnNewSpawner { spawner } => {
                        new_spawner = Some(spawner);
                        break;
                    }
                    _ => {}
                }
            }
            
            Ok(new_spawner)
        }
        }).unwrap());
        
        let draw = _lua.create_table().unwrap();
        _= draw.set("rect", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.draw.rect
            move |_, (x, y, w, h, r, color)| {
            _= tx.send(LoriToMainCommand::DrawPrimitive { x, y, w, h, r, color, label: 0 });
            _= rx.recv();
            Ok(())
        }
        }).unwrap());
        _= draw.set("circle", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.draw.circle
            move |_, (x, y, r, color)| {
            _= tx.send(LoriToMainCommand::DrawPrimitive { x, y, w: 0., h: 0., r, color, label: 1 });
            _= rx.recv();
            Ok(())
        }
        }).unwrap());
        _= draw.set("line", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lori.draw.line
            move |_, (x1, y1, x2, y2, r, color)| {
            _= tx.send(LoriToMainCommand::DrawPrimitive { x: x1, y: y1, w: x2, h: y2, r, color, label: 2 });
            _= rx.recv();
            Ok(())
        }
        }).unwrap());

        _= set.set("window", set_window);
        _= set.set("physics", set_physics);
        _= set.set("camera", set_camera);
        
        _= get.set("key", get_key);
        _= get.set("camera", get_camera);
        _= get.set("window", get_window);
        
        _= lori.set("set", set);
        _= lori.set("get", get);
        _= lori.set("draw", draw);
        _= lori.set("new", new);
        
        _= _lua.globals().set("lori", lori.clone());
        match _lua.load(code).exec() {
            Ok(()) => {
                if verbose {
                    vbosln("Successfully loaded code");
                }
            }
            Err(e)=> {
                serorln(e.to_string());
                exit(3);
            }
        }
        let lhk: mlua::Table = _lua.globals().get("lori").unwrap();

        let mut lori_load: Option<mlua::Function> = None;
        let mut lori_keypressed: Option<mlua::Function> = None; 
        let mut lori_keyreleased: Option<mlua::Function> = None; 
        let mut lori_mousepressed: Option<mlua::Function> = None; 
        let mut lori_mousereleased: Option<mlua::Function> = None; 
        let mut lori_mousemoved: Option<mlua::Function> = None; 
        let mut lori_mousescrolled: Option<mlua::Function> = None; 
        let mut lori_update: Option<mlua::Function> = None; 
        let mut lori_render: Option<mlua::Function> = None;
        let mut lori_exit: Option<mlua::Function> = None;
        
        match lhk.get("load") {
            Ok(func) => {
                lori_load = func;
                if verbose {
                    vbosln("Loaded function 'Load'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("keypressed") {
            Ok(func) => {
                lori_keypressed = Some(func);
                if verbose {
                    vbosln("Loaded function 'KeyPressed'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("keyreleased") {
            Ok(func) => {
                lori_keyreleased = Some(func);
                if verbose {
                    vbosln("Loaded function 'KeyReleased'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousepressed") {
            Ok(func) => {
                lori_mousepressed = Some(func);
                if verbose {
                    vbosln("Loaded function 'MousePressed'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousereleased") {
            Ok(func) => {
                lori_mousereleased = Some(func);
                if verbose {
                    vbosln("Loaded function 'MouseReleased'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousemoved") {
            Ok(func) => {
                lori_mousemoved = Some(func);
                if verbose {
                    vbosln("Loaded function 'MouseMoved'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousescrolled") {
            Ok(func) => {
                lori_mousescrolled = Some(func);
                if verbose {
                    vbosln("Loaded function 'MouseScrolled'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("update") {
            Ok(func) => {
                lori_update = Some(func);
                if verbose {
                    vbosln("Loaded function 'Update'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("render") {
            Ok(func) => {
                lori_render = Some(func);
                if verbose {
                    vbosln("Loaded function 'Render'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("exit") {
            Ok(func) => {
                lori_exit = Some(func);
                if verbose {
                    vbosln("Loaded function 'Exit'");
                }
            }
            _ => {}
        }

        
        Self {
            _lua,
            main_call,
            main_back,

            lori_load,
            lori_keypressed,
            lori_keyreleased,
            lori_mousepressed,
            lori_mousereleased,
            lori_mousemoved,
            lori_mousescrolled,
            lori_update,
            lori_render,
            lori_exit,
        }
    }

    pub fn begin(&mut self) {
        while let Ok(cmd) = self.main_call.recv() {
            match cmd {
                MainToLoriCall::Load => {
                    match &self.lori_load {
                        Some(func) => {
                            _= func.call::<()>(());
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Load);
                }
                MainToLoriCall::Keypressed { code } => {
                    match &self.lori_keypressed {
                        Some(func) => {
                            _= func.call::<()>(code);
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Keypressed);
                }
                MainToLoriCall::Keyreleased { code } => {
                    match &self.lori_keyreleased {
                        Some(func) => {
                            _= func.call::<()>(code);
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Keyreleased);
                }
                MainToLoriCall::Mousepressed { x, y, button } => {
                    match &self.lori_mousepressed {
                        Some(func) => {
                            _= func.call::<()>((x, y, button));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Mousepressed);
                }
                MainToLoriCall::Mousereleased { x, y, button } => {
                    match &self.lori_mousereleased {
                        Some(func) => {
                            _= func.call::<()>((x, y, button));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Mousereleased);
                }
                MainToLoriCall::MouseMoved { motion } => {
                    match &self.lori_mousemoved {
                        Some(func) => {
                            _= func.call::<()>((motion.0, motion.1));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::MouseMoved);
                }
                MainToLoriCall::MouseScrolled { motion } => {
                    match &self.lori_mousescrolled {
                        Some(func) => {
                            _= func.call::<()>((motion.0, motion.1));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::MouseScrolled);
                }
                MainToLoriCall::Update { delta } => {
                    match &self.lori_update {
                        Some(func) => {
                            _= func.call::<()>(delta);
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Draw);
                }
                MainToLoriCall::Render => {
                    match &self.lori_render {
                        Some(func) => {
                            _= func.call::<()>(());
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Render);
                }
                MainToLoriCall::Exit => {
                    match &self.lori_exit {
                        Some(func) => {
                            _= func.call::<()>(());
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoriToMainCall::Exit);
                    break;
                }
            }
        }
    }
}