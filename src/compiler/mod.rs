use std::{ffi::OsStr, fs, path::Path};

use crate::utils::print::errorln;

pub fn compile(filepath: String) {
    let (pathnames, _manifest, lua) = iterate_dir(&filepath);
    let mut bytes: Vec<u8> = Vec::new();

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

fn iterate_dir(path: &String) -> (Vec<String>, String, String) {
    let mut real_paths: Vec<String> = Vec::new();
    let mut manifest: Option<String> = None;
    let mut lua: Option<String> = None;
    
    
    for entry in fs::read_dir(path.clone()).unwrap() {
        let enry = entry.unwrap().path();
        if enry.is_dir() {
            let mut new_iteration = iterate_subdir(enry.to_str().unwrap().to_string(), path.clone());
            real_paths.append(&mut new_iteration);
        } else if enry.is_file() {
            let new_path = enry.strip_prefix(path.clone())
                .unwrap().to_str().unwrap().to_string();
            if enry.file_name() == Some(OsStr::new("lora.json")) {
                manifest = Some(new_path);
            } else if enry.file_name() == Some(OsStr::new("main.lua")) {
                lua = Some(new_path);
            } else {
                real_paths.push(new_path);
            }
        }
    };
    (real_paths, manifest.unwrap(), lua.unwrap())
}

fn iterate_subdir(path: String, prefix: String) -> Vec<String> {
    let mut real_paths: Vec<String> = Vec::new();
    
    for entry in fs::read_dir(path).unwrap() {
        let enry = entry.unwrap().path();
        if enry.is_dir() {
            let mut new_iteration = iterate_subdir(enry.to_str().unwrap().to_string(), prefix.clone());
            real_paths.append(&mut new_iteration);
        } else if enry.is_file() {
            real_paths.push(enry.strip_prefix(prefix.clone())
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