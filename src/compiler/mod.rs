use std::{ffi::OsStr, fs, path::Path, process::exit};

use serde_json::from_str;

use crate::utils::print::{errorln, serorln};

pub fn compile(filepath: String) {
    let (pathnames, premanifest, lua) = iterate_dir(&filepath);
    let mut bytes: Vec<u8> = Vec::new();

    let mut name: String = "Lora App".to_string();
    let mut id: String = "red.ched.lora".to_string();

    
    if let Some(real_premanifest) = premanifest {
        let real_manifest = from_str(real_premanifest.as_str()).unwrap();
        
        let file_schema = from_str(include_str!("../schema/schema.json")).unwrap();
        let schema = jsonschema::Validator::new(&file_schema).unwrap();
        
        match schema.validate(&real_manifest) {
            Ok(()) => {
                let name_string = real_manifest["name"].to_string();
                name = name_string[1..name_string.len() - 1].to_string();

                let id_string = real_manifest["id"].to_string();
                id = id_string[1..id_string.len() - 1].to_string();
                println!("{}", name);
            }
            Err(e) => {
                serorln(e.to_string());
                exit(5);
            }
        }
    }
    
    write_string(&mut bytes, &name);
    write_string(&mut bytes, &id);
    write_file(&mut bytes, &lua, &filepath);

    for path in pathnames {
        write_string(&mut bytes, &path);
        write_file(&mut bytes, &path, &filepath);
    }

    match fs::write("output/app.lora", bytes) { // TODO: Add output dir param
        Ok(()) => {}
        Err(e) => { errorln(&e); }
    }
}

fn iterate_dir(path: &String) -> (Vec<String>, Option<String>, String) {
    let mut real_paths: Vec<String> = Vec::new();
    let mut manifest: Option<String> = None;
    let mut lua: Option<String> = None;
    
    
    for entry in fs::read_dir(path).unwrap() {
        let enry = entry.unwrap().path();
        if enry.is_dir() {
            let mut new_iteration = iterate_subdir(&enry.to_str().unwrap().to_string(), path);
            real_paths.append(&mut new_iteration);
        } else if enry.is_file() {
            let new_path = enry.strip_prefix(path)
                .unwrap().to_str().unwrap().to_string();
            if enry.file_name() == Some(OsStr::new("lora.json")) {
                manifest = Some(fs::read_to_string(enry).unwrap());
            } else if enry.file_name() == Some(OsStr::new("main.lua")) {
                lua = Some(new_path);
            } else {
                real_paths.push(new_path);
            }
        }
    };
    (real_paths, manifest, lua.unwrap())
}

fn iterate_subdir(path: &String, prefix: &String) -> Vec<String> {
    let mut real_paths: Vec<String> = Vec::new();
    
    for entry in fs::read_dir(path).unwrap() {
        let enry = entry.unwrap().path();
        if enry.is_dir() {
            let mut new_iteration = iterate_subdir(&enry.to_str().unwrap().to_string(), prefix);
            real_paths.append(&mut new_iteration);
        } else if enry.is_file() {
            real_paths.push(enry.strip_prefix(prefix)
                .unwrap().to_str().unwrap().to_string());
        }
    };
    real_paths
}

fn write_u32(bytes: &mut Vec<u8>, input: u32) {
    let bytes_input = input.to_be_bytes();
    for byte in bytes_input {
        bytes.push(byte);
    }
}

fn write_u64(bytes: &mut Vec<u8>, input: u64) {
    let bytes_input = input.to_be_bytes();
    for byte in bytes_input {
        bytes.push(byte);
    }
}

fn write_string(bytes: &mut Vec<u8>, input: &String) {
    write_u32(bytes, input.len() as u32);
    let mut bytes_input = input.clone().into_bytes();
    bytes.append(&mut bytes_input);
}

fn write_file(bytes: &mut Vec<u8>, input: &String, prefix: &String) {
    let mut bytes_input = fs::read(Path::new(prefix).join(input)).unwrap();
    write_u64(bytes, bytes_input.len() as u64);
    bytes.append(&mut bytes_input);
}