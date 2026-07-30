use std::process::exit;

use crossbeam::channel::{Receiver, Sender};
use mlua::{Function, UserDataRef};

use crate::{content::{border::LoraBorderRef, collider::LoraColliderRef, shape::LoraShapeRef, spawner::LoraSpawnerRef}, utils::{LoraToMainCall, LoraToMainCommand, MainToLoraCall, MainToLoraCommand, print::{serorln, vbosln}}};

pub struct Lora {
    _lua: mlua::Lua,
    main_call: Receiver<MainToLoraCall>,
    main_back: Sender<LoraToMainCall>,

    lora_load: Option<mlua::Function>,
    lora_keypressed: Option<mlua::Function>,
    lora_keyreleased: Option<mlua::Function>,
    lora_mousepressed: Option<mlua::Function>,
    lora_mousereleased: Option<mlua::Function>,
    lora_mousemoved: Option<mlua::Function>,
    lora_mousescrolled: Option<mlua::Function>,
    lora_collision: Option<mlua::Function>,
    lora_update: Option<mlua::Function>,
    lora_render: Option<mlua::Function>,
    lora_exit: Option<mlua::Function>,
}

impl Lora {
    pub fn new(code: String, verbose: bool, main_cmd: Sender<LoraToMainCommand>, main_rtrn: Receiver<MainToLoraCommand>, main_call: Receiver<MainToLoraCall>, main_back: Sender<LoraToMainCall>) -> Self {
        let _lua = mlua::Lua::new();
        let lora = _lua.create_table().unwrap();
    
        let set = _lua.create_table().unwrap();
        let set_physics = _lua.create_table().unwrap();
        _= set_physics.set("gravity", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.set.physics.gravity
            move |_, (x, y)| {
                _= tx.send(LoraToMainCommand::SetPhysicsGravity { x, y });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        _= set_physics.set("hertz", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.set.physics.hertz
            move |_, hz| {
                _= tx.send(LoraToMainCommand::SetPhysicsHertz { hz });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        let set_window = _lua.create_table().unwrap();
        _= set_window.set("title", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.set.window.title
            move |_, text| {
                _= tx.send(LoraToMainCommand::SetWindowTitle { text });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        _= set_window.set("size", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.set.window.size
            move |_, (w, h)| {
                _= tx.send(LoraToMainCommand::SetWindowSize { w, h });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        _= set_window.set("resizable", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.set.window.resizable
            move |_, is| {
                _= tx.send(LoraToMainCommand::SetWindowResizable { is });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        let set_camera = _lua.create_table().unwrap();
        _= set_camera.set("position", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.set.camera.position
            move |_, (x, y)| {
                _= tx.send(LoraToMainCommand::SetCameraPosition { x, y });
                _= rx.recv();
                Ok(())
            }
        }).unwrap());
        
        let get = _lua.create_table().unwrap();
        let get_window = _lua.create_table().unwrap();
        _= get_window.set("size", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.get.window.size
            move |_, ()| {
                let mut nw: u32 = 0;
                let mut nh: u32 = 0;
                _= tx.try_send(LoraToMainCommand::GetWindowSize);
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnGetWindowSize { w, h } => {
                        nw = w;
                        nh = h;
                    }
                    _ => {}
                }
                Ok((nw, nh))
            }
        }).unwrap());
        let get_key = _lua.create_table().unwrap();
        _= get_key.set("state", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.get.key.state
            move |_, key| {
                let mut pressed: bool = false;
                _= tx.try_send(LoraToMainCommand::GetKeyPressed { key });
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnKeyPressed { key } => {
                        pressed = key;
                    }
                    _ => {}
                }
                Ok(pressed)
            }
        }).unwrap());
        let get_camera = _lua.create_table().unwrap();
        _= get_camera.set("position", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.get.camera.position
            move |_, ()| {
                let mut position: [f32; 2] = [0., 0.];
                _= tx.try_send(LoraToMainCommand::GetCameraPosition);
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnCameraPosition { x, y } => {
                        position = [x, y];
                    }
                    _ => {}
                }
                Ok(position)
            }
        }).unwrap());
        
        let new = _lua.create_table().unwrap();
        _= new.set("border", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone();
            move |_, (points, indices)| {
                _= tx.send(LoraToMainCommand::NewBorder { points, indices });
                let mut new_border: Option<LoraBorderRef> = None;
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnNewBorder { border } => {
                        new_border = Some(border);
                    }
                    _ => {} // TODO: Error on fail here
                }
                Ok(new_border)
            }
        }).unwrap());
        _= new.set("image", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.new.shape
            move |_, (image, scale)| {
                _= tx.send(LoraToMainCommand::NewImage { image, scale });
                let mut new_image: Option<LoraShapeRef> = None;
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnNewImage { image } => {
                        new_image = Some(image);
                    }
                    _ => {}
                }
                
                Ok(new_image)
            }
        }).unwrap());
        _= new.set("shape", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.new.shape
            move |_, (kind, w, h, color)| {
                _= tx.send(LoraToMainCommand::NewShape { kind, w, h, color });
                let mut new_shape: Option<LoraShapeRef> = None;
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnNewShape { shape } => {
                        new_shape = Some(shape);
                    }
                    _ => {}
                }
                
                Ok(new_shape)
            }
        }).unwrap());
        _= new.set("mesh", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.new.mesh
            move |_, (vertices, indices): (Vec<[f32; 8]>, Vec<u32>)| {
                _= tx.send(LoraToMainCommand::NewMesh { vertices: vertices.clone(), indices });
                let mut new_mesh: Option<LoraShapeRef> = None;
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnNewMesh { mesh } => {
                        new_mesh = Some(mesh);
                    }
                    _ => {}
                }
                
                Ok(new_mesh)
            }
        }).unwrap());
        _= new.set("collider", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.new.collider
            move |_, (shape, collision): (UserDataRef<LoraShapeRef>, String)| {
                _= tx.send(LoraToMainCommand::NewCollider { shape: shape.clone(), collision });
                let mut new_collider: Option<LoraColliderRef> = None;
                match rx.recv().unwrap() {
                    MainToLoraCommand::ReturnNewCollider { collider } => {
                        new_collider = Some(collider);
                    }
                    _ => {}
                }
                
                Ok(new_collider)
            }
        }).unwrap());
        _= new.set("spawner", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.new.spawner
            move |_, (shape, collider): (Option<UserDataRef<LoraShapeRef>>, Option<UserDataRef<LoraColliderRef>>)| {
            _= tx.send(LoraToMainCommand::NewSpawner { shape: shape.as_deref().cloned(), collider: collider.as_deref().cloned() });
            let mut new_spawner: Option<LoraSpawnerRef> = None;
            match rx.recv().unwrap() {
                MainToLoraCommand::ReturnNewSpawner { spawner } => {
                    new_spawner = Some(spawner);
                }
                _ => {}
            }
            
            Ok(new_spawner)
        }
        }).unwrap());
        
        let draw = _lua.create_table().unwrap();
        _= draw.set("rect", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.draw.rect
            move |_, (x, y, w, h, r, color): (f32, f32, f32, f32, f32, [f32; 4])| {
            _= tx.send(LoraToMainCommand::DrawPrimitive { x, y, w, h, r: r.to_radians(), color, label: 0 });
            _= rx.recv();
            Ok(())
        }
        }).unwrap());
        _= draw.set("circle", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.draw.circle
            move |_, (x, y, r, color): (f32, f32, f32, [f32; 4])| {
            _= tx.send(LoraToMainCommand::DrawPrimitive { x, y, w: 0., h: 0., r, color, label: 1 });
            _= rx.recv();
            Ok(())
        }
        }).unwrap());
        _= draw.set("line", _lua.create_function({let tx = main_cmd.clone(); let rx = main_rtrn.clone(); // lora.draw.line
            move |_, (x1, y1, x2, y2, r, color): (f32, f32, f32, f32, f32, [f32; 4])| {
            _= tx.send(LoraToMainCommand::DrawPrimitive { x: x1, y: y1, w: x2, h: y2, r: r.to_radians(), color, label: 2 });
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
        
        _= lora.set("set", set);
        _= lora.set("get", get);
        _= lora.set("draw", draw);
        _= lora.set("new", new);
        
        _= _lua.globals().set("lora", lora.clone());
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
        let lhk: mlua::Table = _lua.globals().get("lora").unwrap();

        let mut lora_load: Option<mlua::Function> = None;
        let mut lora_keypressed: Option<mlua::Function> = None; 
        let mut lora_keyreleased: Option<mlua::Function> = None; 
        let mut lora_mousepressed: Option<mlua::Function> = None; 
        let mut lora_mousereleased: Option<mlua::Function> = None; 
        let mut lora_mousemoved: Option<mlua::Function> = None; 
        let mut lora_mousescrolled: Option<mlua::Function> = None;
        let mut lora_collision: Option<mlua::Function> = None;
        let mut lora_update: Option<mlua::Function> = None;
        let mut lora_render: Option<mlua::Function> = None;
        let mut lora_exit: Option<mlua::Function> = None;
        
        match lhk.get("load") {
            Ok(func) => {
                lora_load = func;
                if verbose {
                    vbosln("Loaded function 'Load'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("keypressed") {
            Ok(func) => {
                lora_keypressed = Some(func);
                if verbose {
                    vbosln("Loaded function 'KeyPressed'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("keyreleased") {
            Ok(func) => {
                lora_keyreleased = Some(func);
                if verbose {
                    vbosln("Loaded function 'KeyReleased'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousepressed") {
            Ok(func) => {
                lora_mousepressed = Some(func);
                if verbose {
                    vbosln("Loaded function 'MousePressed'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousereleased") {
            Ok(func) => {
                lora_mousereleased = Some(func);
                if verbose {
                    vbosln("Loaded function 'MouseReleased'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousemoved") {
            Ok(func) => {
                lora_mousemoved = Some(func);
                if verbose {
                    vbosln("Loaded function 'MouseMoved'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("mousescrolled") {
            Ok(func) => {
                lora_mousescrolled = Some(func);
                if verbose {
                    vbosln("Loaded function 'MouseScrolled'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("collision") {
            Ok(func) => {
                lora_collision = Some(func);
                if verbose {
                    vbosln("Loaded function 'collision'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("update") {
            Ok(func) => {
                lora_update = Some(func);
                if verbose {
                    vbosln("Loaded function 'Update'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("render") {
            Ok(func) => {
                lora_render = Some(func);
                if verbose {
                    vbosln("Loaded function 'Render'");
                }
            }
            _ => {}
        }
        match lhk.get::<Function>("exit") {
            Ok(func) => {
                lora_exit = Some(func);
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

            lora_load,
            lora_keypressed,
            lora_keyreleased,
            lora_mousepressed,
            lora_mousereleased,
            lora_mousemoved,
            lora_mousescrolled,
            lora_collision,
            lora_update,
            lora_render,
            lora_exit,
        }
    }

    pub fn begin(&mut self) {
        while let Ok(cmd) = self.main_call.recv() {
            match cmd {
                MainToLoraCall::Load => {
                    match &self.lora_load {
                        Some(func) => {
                            _= func.call::<()>(());
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Load);
                }
                MainToLoraCall::Keypressed { code } => {
                    match &self.lora_keypressed {
                        Some(func) => {
                            _= func.call::<()>(code);
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Keypressed);
                }
                MainToLoraCall::Keyreleased { code } => {
                    match &self.lora_keyreleased {
                        Some(func) => {
                            _= func.call::<()>(code);
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Keyreleased);
                }
                MainToLoraCall::Mousepressed { x, y, button } => {
                    match &self.lora_mousepressed {
                        Some(func) => {
                            _= func.call::<()>((x, y, button));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Mousepressed);
                }
                MainToLoraCall::Mousereleased { x, y, button } => {
                    match &self.lora_mousereleased {
                        Some(func) => {
                            _= func.call::<()>((x, y, button));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Mousereleased);
                }
                MainToLoraCall::MouseMoved { motion } => {
                    match &self.lora_mousemoved {
                        Some(func) => {
                            _= func.call::<()>((motion.0, motion.1));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::MouseMoved);
                }
                MainToLoraCall::MouseScrolled { motion } => {
                    match &self.lora_mousescrolled {
                        Some(func) => {
                            _= func.call::<()>((motion.0, motion.1));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::MouseScrolled);
                }
                MainToLoraCall::Collision { one, two } => {
                    match &self.lora_collision {
                        Some(func) => {
                            _= func.call::<()>((one, two));
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Collision);
                }
                MainToLoraCall::Update { delta } => {
                    match &self.lora_update {
                        Some(func) => {
                            _= func.call::<()>(delta);
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Update);
                }
                MainToLoraCall::Render => {
                    match &self.lora_render {
                        Some(func) => {
                            _= func.call::<()>(());
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Render);
                }
                MainToLoraCall::Exit => {
                    match &self.lora_exit {
                        Some(func) => {
                            _= func.call::<()>(());
                        }
                        _ => {}
                    }
                    _= self.main_back.send(LoraToMainCall::Exit);
                    break;
                }
            }
        }
    }
}