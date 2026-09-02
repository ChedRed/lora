use colored::Colorize;
use std::io::Error;

fn print(prefix: String, text: &'static str) {
    println!("{}  {}", prefix, text);
}

fn sprint(prefix: String, text: String) {
    println!("{}  {}", prefix, text);
}

fn eprint(prefix: String, text: &'static str) {
    eprintln!("{}  {}", prefix, text);
}

fn esprint(prefix: String, text: String) {
    eprintln!("{}  {}", prefix, text);
}

pub fn infoln(text: &'static str) {
    print(format!("{}", " ".black().on_green().bold()), text);
}

pub fn warnln(text: &'static str) {
    print(format!("{}", " ".black().on_yellow().bold()), text);
}

pub fn erorln(text: &'static str) {
    eprint(format!("{}", " ".black().on_bright_red().bold()), text);
}

pub fn errorln(err: &Error) {
    esprint(
        format!("{}", " ".black().on_bright_red().bold()),
        err.to_string(),
    );
}

pub fn serorln(err: String) {
    esprint(format!("{}", " ".black().on_bright_red().bold()), err);
}

pub fn dbugln(text: &'static str) {
    print(format!("{}", " ".black().on_purple().bold()), text);
}

pub fn sdbugln(text: String) {
    sprint(format!("{}", " ".black().on_purple().bold()), text);
}

pub fn vbosln(text: &'static str) {
    print(format!("{}", " ".black().on_blue().bold()), text);
}
