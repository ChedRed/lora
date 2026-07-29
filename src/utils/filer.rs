use std::{ffi::OsStr, fs, path::{Path, PathBuf}, process::exit};

use serde_json::{Value, from_str};
use wgpu::naga::FastHashMap;

use crate::utils::print::{erorln, serorln};

pub struct Filer {
    name: String,
    id: String,
    lora: String,
    lora_files: FastHashMap<String, Vec<u8>>,
}

impl Filer {
    pub fn new(cwp: &Option<String>) -> Self {
        let name: String;
        let id: String;
        let lora: String;
        let lora_files: FastHashMap<String, Vec<u8>>;
        
        if let Some(real_cwp) = cwp {
            let real_path = Path::new(real_cwp);

            if real_path.is_dir() { // lora ./tests
                (name, id, lora, lora_files) = check_dir(real_path);
            } else if real_path.is_file() { // lora tests/<codefile>
                (name, id, lora, lora_files) = check_file(real_path);
            } else {
                erorln("Path provided is not a file or directory!");
                exit(4);
            }
        } else {
            (name, id, lora, lora_files) = check_dir(Path::new("."));
        }
        
        
        Self {
            name,
            id,
            lora,
            lora_files,
        }
    }

    pub fn read_name(&self) -> String {
        self.name.clone()
    }
    
    pub fn read_id(&self) -> String {
        self.id.clone()
    }

    pub fn read_code(&self) -> String {
        self.lora.clone()
    }
    
    pub fn read_file(&self, path: String) -> Option<&Vec<u8>> {
        self.lora_files.get(&path)
    }
}

fn check_dir(dir: &Path) -> (String, String, String, FastHashMap<String, Vec<u8>>) {
    let name: String;
    let id: String;
    let code: String;
    let files: FastHashMap<String, Vec<u8>>;
    
    let prefix = dir.to_str().unwrap().to_string();
    let folder_result = check_first_folder_type(&prefix);
    
    if folder_result == 1u8 {
        let parse_result = parse_first_folder(&prefix);
        name = parse_result.0;
        id = parse_result.1;
        code = parse_result.2;
        files = parse_result.3;
    } else if folder_result == 2u8 {
        let parse_result = parse_lora_folder(&prefix);
        name = parse_result.0;
        id = parse_result.1;
        code = parse_result.2;
        files = parse_result.3;
    } else {
        erorln("Folder provided does not contain main.lua or a .lora file!");
        exit(4);
    }

    (name, id, code, files)
}
// ../Resources/
// 
fn check_file(file: &Path) -> (String, String, String, FastHashMap<String, Vec<u8>>) {
    let name: String;
    let id: String;
    let code: String;
    let files: FastHashMap<String, Vec<u8>>;

    let file_result = check_code_type(file.to_path_buf());
    
    if file_result == 1u8 {
        let mut parse_path = file.parent().unwrap().to_str().unwrap().to_string();
        if parse_path.is_empty() {
            parse_path = ".".to_string();
        }
        let parse_result = parse_first_folder(&parse_path);
        name = parse_result.0;
        id = parse_result.1;
        code = parse_result.2;
        files = parse_result.3;
        
    } else if file_result == 2u8 {
        let parse_result = parse_lora(file.to_str().unwrap().to_string());
        name = parse_result.0;
        id = parse_result.1;
        code = parse_result.2;
        files = parse_result.3;
    } else {
        erorln("File provided is not main.lua or a .lora file!");
        exit(4);
    }

    (name, id, code, files)
}


fn check_first_folder_type(prefix: &String) -> u8 {
    for entry in Path::new(&prefix).read_dir().expect("Parsing the first folder failed!") {
        if let Ok(file) = entry {
             if file.path().is_file() {
                 let file_result = check_code_type(file.path());
                 if file_result != 0u8 {
                     return file_result;
                 }
             }
        }
    }
    0u8
}

pub fn check_code_type(filepath: PathBuf) -> u8 { // 0: NA, 1: main.lua, 2: *.lora
    if filepath.file_name() == Some(OsStr::new("main.lua")) {
        return 1u8;
    }
    if filepath.extension() == Some(OsStr::new("lora")) {
        return 2u8;
    }
    0u8
}

fn parse_first_folder(prefix: &String) -> (String, String, String, FastHashMap<String, Vec<u8>>) {
    let mut name: String = "Lora App".to_string();
    let mut id: String = "red.ched.lora".to_string();
    let mut code: Option<String> = None;
    let mut files: FastHashMap<String, Vec<u8>> = FastHashMap::default();

    for entry in Path::new(prefix).read_dir().expect("Parsing the first folder failed!") {
        if let Ok(real_entry) = entry {
            if real_entry.path().is_dir() {
                files.extend(parse_subfolder(prefix, &real_entry.path().to_str().unwrap().to_string()));
            } else if real_entry.path().is_file() {
                let early_filepath = real_entry.path();
                let current_filepath = early_filepath.strip_prefix(prefix).unwrap();
                if current_filepath.file_name() == Some(OsStr::new("main.lua")) {
                    code = Some(fs::read_to_string(real_entry.path()).unwrap());
                } else if current_filepath.file_name() == Some(OsStr::new("lora.json")) {
                    let manifest: Value = from_str(fs::read_to_string(real_entry.path()).unwrap().as_str()).unwrap();
                    let file_schema = from_str(include_str!("../schema/schema.json")).unwrap();
                    let schema = jsonschema::Validator::new(&file_schema).unwrap();
                    
                    match schema.validate(&manifest) {
                        Ok(()) => {
                            let name_string = manifest["name"].to_string();
                            name = name_string[1..name_string.len() - 1].to_string();
            
                            let id_string = manifest["id"].to_string();
                            id = id_string[1..id_string.len() - 1].to_string();
                        }
                        Err(e) => {
                            serorln(e.to_string());
                            exit(5);
                        }
                    }
                    
                } else {
                    files.insert(current_filepath.to_str().unwrap().to_string(), fs::read(early_filepath).unwrap());
                }
            }
        }
    }
    (name, id, code.unwrap(), files)
}

fn parse_subfolder(prefix: &String, path: &String) -> FastHashMap<String, Vec<u8>> {
    let mut files: FastHashMap<String, Vec<u8>> = FastHashMap::default();

    for entry in Path::new(path).read_dir().expect("Parsing the first folder failed!") {
        if let Ok(real_entry) = entry {
            if real_entry.path().is_dir() {
                files.extend(parse_subfolder(prefix, &real_entry.path().to_str().unwrap().to_string()));
            } else if real_entry.path().is_file() {
                let early_filepath = real_entry.path();
                let current_filepath = early_filepath.strip_prefix(prefix).unwrap();
                files.insert(current_filepath.to_str().unwrap().to_string(), fs::read(early_filepath).unwrap());
            }
        }
    }
    files
}

fn parse_lora_folder(path: &String) -> (String, String, String, FastHashMap<String, Vec<u8>>) {
    for entry in Path::new(&path).read_dir().expect("Parsing the first folder failed!") {
        if let Ok(file) = entry {
             if file.path().is_file() {
                 let file_result = check_code_type(file.path());
                 if file_result == 2u8 {
                     return parse_lora(file.path().to_str().unwrap().to_string());
                 }
             }
        }
    }
    erorln(".lora file not found!");
    exit(404);
}

fn parse_lora(path: String) -> (String, String, String, FastHashMap<String, Vec<u8>>) {
    let mut lorafile = fs::read(path).unwrap();
    let name_size = read_u32(&mut lorafile);
    let name = read_string(&mut lorafile, name_size as u64);
    let id_size = read_u32(&mut lorafile);
    let id = read_string(&mut lorafile, id_size as u64);
    
    let code_size = read_u64(&mut lorafile);
    let code = read_string(&mut lorafile, code_size);
    let mut files: FastHashMap<String, Vec<u8>> = FastHashMap::default();
    while !lorafile.is_empty() {
        let path_size =  read_u32(&mut lorafile) as u64;
        let path = read_string(&mut lorafile, path_size);

        let data_size = read_u64(&mut lorafile) as u64;
        let data = read_bytes(&mut lorafile, data_size);
        
        files.insert(path, data);
    }

    (name, id, code, files)
}

fn read_u64(data: &mut Vec<u8>) -> u64 {
    let rval: u64 = u64::from_be_bytes(data[..8].try_into().unwrap());
    data.drain(0..8);
    rval
}

fn read_u32(data: &mut Vec<u8>) -> u32 {
    let rval: u32 = u32::from_be_bytes(data[..4].try_into().unwrap());
    data.drain(0..4);
    rval
}

fn read_bytes(data: &mut Vec<u8>, size: u64) -> Vec<u8> {
    let rval: Vec<u8> = data[..size as usize].to_vec();
    data.drain(0..size as usize);
    rval
}

fn read_string(data: &mut Vec<u8>, size: u64) -> String {
    std::str::from_utf8(&read_bytes(data, size).into_boxed_slice()).unwrap().to_string()
}