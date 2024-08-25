use std::io::stdout;

use chrono::Local;
use crossterm::{ExecutableCommand, style::{Color, Print, ResetColor, SetForegroundColor}};

pub fn form_logs(message: &str, info_message:&str, color: Color){
    let time_stamp = Local::now();
    let _ = stdout().execute(SetForegroundColor(color));
    println!("[{}][{}] {}", time_stamp,info_message, message);
    let _ = stdout().execute(ResetColor);
}

pub fn form_message(message: &str, color: Color){
    let _ = stdout().execute(SetForegroundColor(color));
    println!("{}", message);
    let _ = stdout().execute(ResetColor);
}