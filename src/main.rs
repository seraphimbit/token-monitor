#![allow(unused_imports, dead_code, unused_variables, non_snake_case, deprecated, unused_assignments, special_module_name)]

use std::{error::Error, thread::sleep, time::Duration};

mod others;
mod logs;
mod cli;
mod lib;
mod monitor;
mod settings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
     cli::start().await?;

    Ok(())
}
