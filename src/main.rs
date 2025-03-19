use solana_client::{
    //rpc_client::RpcClient,
    nonblocking::rpc_client::RpcClient,
    pubsub_client::PubsubClient,
    rpc_config::RpcAccountInfoConfig,
    rpc_config::RpcTransactionLogsConfig,
    rpc_config::RpcTransactionLogsFilter,
    rpc_config::RpcTransactionConfig
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
   signature::Signature
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;
use tokio::signal;
use solana_account_decoder::UiAccountEncoding;
use solana_transaction_status::UiTransactionEncoding;
use solana_transaction_status::EncodedTransaction;
use solana_transaction_status::UiMessage;
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::UiCompiledInstruction;
use spl_token::state::Account;
use solana_sdk::program_pack::Pack; 
use solana_sdk::instruction::Instruction;
use solana_transaction_status::UiInstruction::Compiled;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Handle;
use std::thread;

use tokio::net::UnixStream;
use tokio::io::AsyncWriteExt;
use std::error::Error;

use chrono::{TimeZone, Utc};

use std::fmt;

use std::collections::HashSet;


#[tokio::main]
async fn main()-> Result<(), Box<dyn Error>> {
    // Define the WebSocket endpoint for Solana mainnet-beta.
    let socket_path = "./mints.sock";
    let mut stream = UnixStream::connect(socket_path).await?;

    let ws_url = "wss://api.mainnet-beta.solana.com/";
    let _my_addy = "DCHPQWDbvNEcwQRMXugBDCRA2req7MiKEdNV5T8Zz4sG";
    let _token_prog = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    // listen_user(_my_addy, ws_url).await;
    listen_mint(_token_prog, ws_url, stream).await;

    Ok(())
}

async fn listen_mint(public_key:&str, endpoint:&str, mut stream:UnixStream) {
    let config = RpcTransactionLogsConfig {
        commitment: Some(CommitmentConfig::finalized()),
    };
    let filter = RpcTransactionLogsFilter::Mentions(vec!(public_key.to_string()));

    let (mut client, receiver) =
        PubsubClient::logs_subscribe(&endpoint, filter, config)
        .expect("Failed to subscribe to program updates");

    println!("Subscribed to program updates. Waiting for events...");

    

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
    let stream = Arc::new(Mutex::new(stream));
    let seen = Arc::new(Mutex::new(HashSet::new()));
    // Spawn an asynchronous task to continuously process updates.
    tokio::spawn(async move {       
         
        for response in receiver {
            let log_entry = response.value; // log_entry is RpcLogsResponse
            let logs: Vec<String> = log_entry.logs;
            let signature_str = log_entry.signature;

            let signature = Signature::from_str(&signature_str)
                .expect("Failed to parse signature");
            
            let rpc_client = Arc::clone(&rpc_client); 
            let stream = Arc::clone(&stream); 
            let seen = Arc::clone(&seen);
            tokio::spawn(async move {
                let thread_id = thread::current().id();
                // println!("Current thread ID: {:?}", thread_id);
                let config = RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(CommitmentConfig::finalized()),
                    max_supported_transaction_version: Some(0),
                };
                        // Now you can use &signature where a &Signature is required
                for line in &logs {
                    // println!("{}", logs.len());
                    if line.contains("SetAuthority") {
                        
                        let mut seen = seen.lock().await;
                        if seen.contains(&signature) {
                            println!("Skipping duplicate transaction: {}", signature);
                            continue;
                        }
                        seen.insert(signature);
                        match rpc_client.get_transaction_with_config(&signature, config).await {
                            Ok(tx_details) => {
                                println!("found2");
                                match check_revoke(tx_details, &rpc_client).await {
                                    Ok(account) => {
                                        println!("mints : {:#?}", account.mint);
                                        
                                        let message = account.mint.to_string();
                                        
                                        let mut stream = stream.lock().await;
                                        if let Err(e) = stream.write_all(message.as_bytes()).await {
                                            eprintln!("Message Failed: {}", e);
                                        } else if let Err(e) = stream.write_all(b"\n").await {
                                            eprintln!("Message Failed: {}", e);
                                        } else { // Add a newline as a delimiter.
                                            println!("Sent message: {}", message);
                                        };
                                    }
                                    Err(e) => eprintln!("Error processing revoke: {}", e)
                                } 
                                
                            }
                            Err(e) => eprintln!("Error fetching transaction details: {}", e),
                        }
                        println!("HUH");
                        break;
                    } 
                }
            });
        }
    });
    // Wait until you send a termination signal (Ctrl+C) to close the connection.
    signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    println!("Termination signal received. Shutting down...");

    // Shutdown the client gracefully.
    client.shutdown().expect("Failed to shutdown the client");
}


#[derive(Debug)]
enum TransactionError {
    NotJsonTransaction,
    DataNotRaw,
    MissingMeta,
    NoMints,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for TransactionError {} 

async fn check_revoke (transaction:EncodedConfirmedTransactionWithStatusMeta, rpc_client:&RpcClient) -> Result<Account, TransactionError> {
    // println!("{:#?}",transaction);
    //println!("A");
    
    if let Some(time) = transaction.block_time {
        let datetime = Utc.timestamp_opt(time, 0);
        println!("{:#?}", datetime);
    }
    

    println!("time is {:#?}", transaction.block_time);
    let json_value = match &transaction.transaction.transaction {
        EncodedTransaction::Json(ref json_value) => json_value,
        _ => return Err(TransactionError::NotJsonTransaction),
    };

    let message = match &json_value.message {
        UiMessage::Raw(ref message) => message,
        _ => return Err(TransactionError::DataNotRaw),
    };

   
    let acc_keys = &message.account_keys;

    let meta = transaction.transaction.meta.as_ref().ok_or(TransactionError::MissingMeta)?;
    let mut writable_keys: &Vec<String> = &Vec::new();
    let mut readonly_keys: &Vec<String> = &Vec::new();

    match &meta.loaded_addresses {
        OptionSerializer::Some(addys) => {
            writable_keys = &addys.writable;
            readonly_keys = &addys.readonly;
        },
        OptionSerializer::Skip => {
            
        }
        OptionSerializer::None => {
            return Err(TransactionError::MissingMeta);
        },
    }

    let total_capacity = acc_keys.len() + writable_keys.len() + readonly_keys.len();
    let mut all_keys = Vec::with_capacity(total_capacity);
    all_keys.extend_from_slice(acc_keys);
    all_keys.extend_from_slice(writable_keys);
    all_keys.extend_from_slice(readonly_keys);

    // println!("{:#?}", all_keys);

    let mut inner = Vec::new();
    match &meta.inner_instructions {
        OptionSerializer::Some(inner_inst) => {
            for list_inst in inner_inst {
                for ui_instruction in &list_inst.instructions {
                    if let Compiled(ref ui_instr) = ui_instruction {
                        inner.push(ui_instr);
                    };
                }
            }
        },
        OptionSerializer::Skip => {
            
        }
        OptionSerializer::None => {
            return Err(TransactionError::MissingMeta);
        },
    }
    
    for inst in &message.instructions {
        inner.push(inst);
    }


    for instruction in &inner {
        let index = instruction.program_id_index as usize;

        if all_keys[index] == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {   
            let bytes = instruction.data.as_bytes();
            //println!("{:#?}", bytes);
            if bytes[0] - ('0' as u8) == 6 {
                match get_mint_address(&rpc_client, &all_keys[instruction.accounts[0] as usize]).await {
                    Ok(acc) => {
                        return Ok(acc)
                    }
                    Err(_e) => {
                        //eprintln!("Error fetching mint address: {}", _e);
                    }
                }
            }
        }
        //println!("{:#?}", acc_keys[instruction.program_id_index as usize]);
    } 
    //println!("{:#?}", transaction);
    Err(TransactionError::NoMints)
}

#[derive(Debug)]
enum AccountFetchError {
    InvalidPubkey(String),
    RpcError(String),
    UnpackError(String),
}

impl std::fmt::Display for AccountFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountFetchError::InvalidPubkey(msg) => write!(f, "Invalid pubkey: {}", msg),
            AccountFetchError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            AccountFetchError::UnpackError(msg) => write!(f, "Unpack error: {}", msg),
        }
    }
}

impl std::error::Error for AccountFetchError {}

// Function to get the account data using the RPC client.
async fn get_mint_address(rpc_client: &RpcClient, token_account: &str) -> Result<Account, AccountFetchError> {
    let token_account_pubkey = Pubkey::from_str(token_account)
        .map_err(|e| AccountFetchError::InvalidPubkey(e.to_string()))?;

    let account_data = rpc_client.get_account_data(&token_account_pubkey).await
        .map_err(|e| AccountFetchError::RpcError(e.to_string()))?;

    let token_account_info = Account::unpack(&account_data)
        .map_err(|e| AccountFetchError::UnpackError(e.to_string()))?;

    Ok(token_account_info)
}

