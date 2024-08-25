use core::str;
use std::{collections::HashMap, error::Error,  str::FromStr, sync::Arc, vec};
use std::process;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use serde_json::{from_str, json};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{borsh, bs58, commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use tungstenite::{connect, Message};
use crossterm::style::{self, Color};
use tokio::{join, task, time::{sleep, Duration}};
use byteorder::{ByteOrder, LittleEndian};
use tokio::sync::mpsc::{
    Sender, 
    Receiver, 
    channel
};

use crate::{ lib::Transaction, logs, settings::{self, Settings}};

pub async fn start() -> Result<(), Box<dyn Error>> {
    let settings = settings::setup()?;
    let atomic_settings = Arc::new(Mutex::new(settings.clone()));
    let mut valid_dexes_workers = vec![];

    let mut threads = vec![];
    let (tx, mut rx): (Sender<WebSocketResponse>, Receiver<WebSocketResponse>) = channel(1);

    if settings.ws.is_empty() || settings.rpc.is_empty() {
        logs::form_logs("WS or RPC is empty", "INFO", style::Color::Red);
        
        process::exit(0); 
    }

    if settings.raydium_settings.work == false && settings.pumpfun_settings.work == false{
        logs::form_logs("Both work are false choose one action to monitor in the settings file", "INFO", style::Color::Red);
        
        process::exit(0);
    }

    if settings.raydium_settings.work && settings.pumpfun_settings.work{
        logs::form_logs("Choose one action at a time", "INFO", style::Color::Red);
        
        process::exit(0);
    }

    if settings.raydium_settings.work {
        logs::form_logs("RAYDIUM LOADED","INFO", style::Color::Yellow);
        valid_dexes_workers.push(settings.raydium_settings.raydium_program_id.clone());
    }else if settings.pumpfun_settings.work {
        logs::form_logs("PUMPFUN LOADED","INFO", style::Color::Yellow);
        valid_dexes_workers.push(settings.pumpfun_settings.pumpfun_program_id.clone());  
    }

    let first_task = task::spawn(async move {
        for dex_id in valid_dexes_workers {
            let tx = tx.clone();
            let settings = atomic_settings.lock().await.clone();
            let _ = monitor(tx, &dex_id, settings).await;
        }
    });

    let second_task = task::spawn(async move {
       loop {
            let data = match rx.recv().await {
                Some(data) => data,
                None => {
                    logs::form_logs("No data received, check RPC", "INFO", style::Color::Red);
                    break;
                }
            };
            
            let settings = settings.clone();
            let _ = data.start_filtering(settings).await;
        }
    });

    threads.push(first_task);
    threads.push(second_task);

    for task in threads {
        let task_result = join!(task);
       
        if let (Err(e),) = task_result {
            logs::form_logs("Erron in tasks", "INFO", style::Color::Red);
        }
    }

   Ok(())
}

pub async fn monitor(
    sender:Sender<WebSocketResponse>, 
    program_id:&String, 
    settings: Settings
)-> Result<(), Box<dyn Error>> {
    let ( mut socket, response) = connect(settings.ws)?;
    logs::form_logs("CONNECTING TO WS", "INFO", style::Color::Cyan);

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            {
                "mentions": [program_id]
            },
            {
                "commitment": "confirmed"
            }
        ]
    });

    let serialize_payload = serde_json::to_string(&payload)?;
    socket.send(Message::Text(serialize_payload))?;
   
    loop {
     let json = socket.read()?;

     if json.is_close() {
         continue;
     }  

     let data =  match from_str::<WebSocketResponse>(&json.to_string()) {
            Ok(data) => data,
            Err(e) => continue
    };
     
    sender.send(data).await?;

    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebSocketResponse {
    pub jsonrpc: String,
    pub method: String,
    pub params:  WebSocketParams,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebSocketParams {
    pub result: WebSocketResult,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebSocketResult {
    pub context: Context,
    pub value: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Context {
    pub slot: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Value {
    pub signature: String,
    #[serde(default)]  // Use default value if err is missing or null
    pub err: Option<InstructionError>,
    pub logs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstructionError {
    #[serde(rename = "InstructionError")]
    pub error_data: (i32, HashMap<String, i32>),
}

impl WebSocketResponse {
    pub async  fn start_filtering(&self, settings:Settings) -> Result<(), Box<dyn Error>>{
        let mut retries = 5;
        let mut signatures = vec![];
        let rpc = load_client(&settings.rpc.to_string());

        match settings {
            Settings{
                raydium_settings, 
                pumpfun_settings, 
                ..
            } => {
                if raydium_settings.work {
                    let signatures_raydium = self.raydium_filters_conditions()?;
                    signatures.push(signatures_raydium);
                }else if pumpfun_settings.work {
                    let signatures_pumpfun = self.pumpfun_filters_conditions()?;
                    signatures.push(signatures_pumpfun);
                }
            }
        }

        for txs in &signatures {
            let transaction_meta = self.get_tx(&txs, &rpc, &mut retries).await?;
            let token_accounts = self.serialize_tx(&transaction_meta).await?;
            self.print_statements_manager(txs, token_accounts); 
        }

        signatures.pop();         
        Ok(())
    }

    async fn get_tx(
        &self, 
        signature: &String, 
        rpc: &RpcClient,
        retries: &mut i32
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, Box<dyn Error>>{
        let sig = Signature::from_str(&signature)?;
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };
    
        let txs_result = match rpc.get_transaction_with_config(&sig, config){
            Ok(txs) =>return  Ok(txs),
            Err(e) => {
                logs::form_logs("Retrying getting transaction....","INFO", style::Color::Yellow);
                *retries += 1;
                if *retries == 5 {
                    return Err("Max retries reached".into());   
                }

                let recursive_call = Box::pin(self.get_tx(signature, rpc ,retries,)).await?;

                return Ok(recursive_call);
            },
        };
    }

    async fn serialize_tx(&self, txs: &EncodedConfirmedTransactionWithStatusMeta) -> Result<Vec<String>, Box<dyn Error>>{
        let serialize_tx = serde_json::to_string(&txs.transaction.transaction)?;
        let tx_deser = from_str::<Transaction>(&serialize_tx)?;
        let account_keys = tx_deser.message.accountKeys;
     
        Ok(account_keys)
    }

    fn pumpfun_filters_conditions(&self) -> Result<String, Box<dyn Error>> {
        self.filter(
            "Program 11111111111111111111111111111111 success",
            &["Program log: Instruction: InitializeMint"],
        )
    }

    fn raydium_filters_conditions(&self) -> Result<String, Box<dyn Error>> {
        self.filter(
            "Program 11111111111111111111111111111111 success",
            &[
                "Program log: Instruction: SyncNative",
                "Program log: Instruction: InitializeMint",
            ],
        )
    }

    fn filter(
        &self,
        success_condition: &str,
        conditions: &[&str],
    ) -> Result<String, Box<dyn Error>> {
        let mut success = false;
        let mut condition_flags = vec![false; conditions.len()];

        for d in &self.params.result.value.logs {
            let black_list = self.black_list_logs(d);

            if !black_list {
                break;
            }

            if d == success_condition {
                success = true;
            }

            for (i, condition) in conditions.iter().enumerate() {
                if d.starts_with(condition) {
                    condition_flags[i] = true;
                }
            }
        }

        if self.params.result.value.signature.to_string() == "1111111111111111111111111111111111111111111111111111111111111111" {
            return Err("Illegal  signature".into());
        }

        if success && condition_flags.iter().all(|&flag| flag) {
            return Ok(self.params.result.value.signature.to_string());
        } else {
            return Err("Transaction not successful".into());
        }
    }

    fn black_list_logs(&self, d: &String) -> bool {
        let filters = vec![
            "Program log: Instruction: MintNft",
            "Program log: Instruction: MintV",
            "Program whirL",
            "Program data",
            "Program log: Instruction: ReleaseInitViaHubV",
            "Program log: Instruction: Buy"
        ];
    
        for filter in filters {
            if d.starts_with(filter) {
                return false;
            }
        }
    
        true
    }

    fn print_statements_manager(
        &self, 
        signature: &String, 
        token_accounts: Vec<String>
    ){
        println!("------------------------Signature------------------------");
        println!("Signatures: {:} ", signature);
        println!("-------------------------Accounts------------------------");
        println!("Token Account size:{:?}", token_accounts.len());
        
        for (index, account) in token_accounts.iter().enumerate() {
            println!("Account {:?}: {:?}", index, account);  
        }

        println!("---------------------------------------------------------");
        println!("\n");
    }
    
}

pub fn load_client(url: &String) -> RpcClient {
    let rpc_client = RpcClient::new(url);
    rpc_client
}
