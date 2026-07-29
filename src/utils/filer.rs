use std::{ffi::OsStr, fs, path::{Path, PathBuf}, process::exit};

use wgpu::naga::FastHashMap;

use crate::utils::print::erorln;

pub struct Filer {
    lora: String,
    lora_files: FastHashMap<String, Vec<u8>>,
}

impl Filer {
    pub fn new(cwp: &Option<String>) -> Self {
        let lora: String;
        let lora_files: FastHashMap<String, Vec<u8>>;
        
        if let Some(real_cwp) = cwp {
            let real_path = Path::new(real_cwp);

            if real_path.is_dir() { // lora ./tests
                (lora, lora_files) = check_dir(real_path);
            } else if real_path.is_file() { // lora tests/<codefile>
                (lora, lora_files) = check_file(real_path);
            } else {
                erorln("Path provided could not be decrypted!");
                exit(4);
            }
        } else {
            (lora, lora_files) = check_dir(Path::new("."));
        }
        
        
        Self {
            lora: lora,
            lora_files: lora_files,
        }
    }

    pub fn read_code(&self) -> String {
        self.lora.clone()
    }
    
    pub fn read_file(&self, path: String) -> Option<&Vec<u8>> {
        self.lora_files.get(&path)
    }
}

fn check_dir(dir: &Path) -> (String, FastHashMap<String, Vec<u8>>) {
    let code: String;
    let files: FastHashMap<String, Vec<u8>>;
    
    let prefix = dir.to_str().unwrap().to_string();
    let folder_result = check_first_folder_type(&prefix);
    
    if folder_result == 1u8 {
        let parse_result = parse_first_folder(&prefix);
        code = parse_result.0;
        files = parse_result.1;
    } else if folder_result == 2u8 {
        let parse_result = parse_lora_folder(&prefix);
        code = parse_result.0;
        files = parse_result.1;
    } else {
        erorln("Folder provided does not contain main.lua or a .lora file!");
        exit(4);
    }

    (code, files)
}
// ../Resources/
// 
fn check_file(file: &Path) -> (String, FastHashMap<String, Vec<u8>>) {
    let code: String;
    let files: FastHashMap<String, Vec<u8>>;

    let file_result = check_code_type(file.to_path_buf());
    
    if file_result == 1u8 {
        let mut parse_path = file.parent().unwrap().to_str().unwrap().to_string();
        if parse_path.is_empty() {
            parse_path = ".".to_string();
        }
        let parse_result = parse_first_folder(&parse_path);
        code = parse_result.0;
        files = parse_result.1;
        
    } else if file_result == 2u8 {
        let parse_result = parse_lora(file.to_str().unwrap().to_string());
        code = parse_result.0;
        files = parse_result.1;
    } else {
        erorln("File provided is not main.lua or a .lora file!");
        exit(4);
    }

    (code, files)
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

fn parse_first_folder(prefix: &String) -> (String, FastHashMap<String, Vec<u8>>) {
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
                } else if current_filepath.file_name() != Some(OsStr::new("lora.json")) {
                    files.insert(current_filepath.to_str().unwrap().to_string(), fs::read(early_filepath).unwrap());
                }
            }
        }
    }
    (code.unwrap(), files)
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

fn parse_lora_folder(path: &String) -> (String, FastHashMap<String, Vec<u8>>) {
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

fn parse_lora(path: String) -> (String, FastHashMap<String, Vec<u8>>) {
    let mut lorafile = fs::read(path).unwrap();
    let code_size = read_u64(&mut lorafile);
    let code = read_string(&mut lorafile, code_size);
    let mut files: FastHashMap<String, Vec<u8>> = FastHashMap::default();
    while !lorafile.is_empty() {
        let path_size =  read_u32(&mut lorafile) as u64;
        let path = std::str::from_utf8(&read_bytes(&mut lorafile, path_size).into_boxed_slice()).unwrap().to_string();

        let data_size = read_u64(&mut lorafile) as u64;
        let data = read_bytes(&mut lorafile, data_size);
        
        files.insert(path, data);
    }

    (code, files)
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