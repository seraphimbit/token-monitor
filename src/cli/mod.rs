use crossterm::{execute, terminal::{Clear, ClearType}};
use std::{error::Error, io::{stdin, stdout, Write}, process};

use crate::{logs, settings};
use crate::others;
use crate::monitor;

pub async fn start() -> Result<(), Box<dyn Error>>{ 
    if let Err(e) = execute!(stdout(), Clear(ClearType::All)) {
        logs::form_logs(&e.to_string(), "Error", crossterm::style::Color::Red);
        process::exit(1);
    }

    let art = others::design();
    logs::form_message(art, crossterm::style::Color::Grey);

    monitor::start().await?;
    Ok(())
}
