use std::error::Error;
use serde::{Serialize, Deserialize};
use serde_json::{from_str, json};
use std::{collections::HashMap, fs, io::Cursor};
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
   pub ws:String,
   pub rpc:String,
   pub raydium_settings:RaydiumSettings,
   pub pumpfun_settings:PumpfunSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RaydiumSettings {
    pub raydium_program_id:String,
    pub work:bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PumpfunSettings {
    pub pumpfun_program_id:String,
    pub work:bool
}

pub fn setup() -> Result<Settings, Box<dyn Error>>{
    //Change this settings.json file to your own settings file location
    let file_path = "./src/settings.json";
    let user = read_json_file(&file_path)?;

    Ok(user)
}

fn read_json_file(path: &str) -> Result<Settings, Box<dyn Error>> {
    let file_content = fs::read_to_string(path)?;
    let data = from_str::<Settings>(&file_content)?;
    
    Ok(data)
}
