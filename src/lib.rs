#![allow(unused_imports, dead_code, unused_variables, non_snake_case, deprecated, unused_assignments)]
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub signatures: Vec<String>,
    pub message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub header: Header,
    pub accountKeys: Vec<String>,
    pub recentBlockhash: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub numRequiredSignatures: i32,
    pub numReadonlySignedAccounts: i32,
    pub numReadonlyUnsignedAccounts: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Instruction {
    pub programIdIndex: i32,
    pub accounts: Vec<i32>,
    pub data: String,
    pub stackHeight: Option<i32>,
}

