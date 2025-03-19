use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::error::Error;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct QuoteResponse {
    // The structure may change; here's a simplified example.
    // Typically, there is a "data" or "routes" field containing route options.
    routes: Vec<Route>,
}

#[derive(Debug, Deserialize)]
struct Route {
    // For each route, Jupiter returns a transaction in base64 encoding.
    // It might be under a field named "tx" or similar.
    tx: String,
    // Other route details like price impact, slippage, etc.
    // priceImpactPct: String,
}
// server.rs


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Specify the socket path.
    
    let socket_path = "./mints.sock";


    if let Err(e) = get_swap_info("7CgVMjhT4M1VJidqeEqgCKA4anWE6YtZBHH2gXzkmoon").await {
        eprintln!("Error processing swap info: {}", e);
    }


    Ok(())
    /* 
    // Remove any existing socket at the same path.
    if Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }
    

    
    // Bind a UnixListener to the socket path.
    let listener = UnixListener::bind(socket_path)?;
    println!("Server listening on {}", socket_path);

    loop {
        // Accept an incoming connection.
        let (stream, addr) = listener.accept().await?;
        println!("New connection: {:?}", addr);

        // Spawn a new task for handling the connection.
        tokio::spawn(async move {
            let reader = BufReader::new(stream);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                println!("Received message: {}", line);
                
                tokio::spawn(async move {
                    if let Err(e) = get_swap_info(&line).await {
                        eprintln!("Error processing swap info: {}", e);
                    }
                });
            }
        });
    }
    */
}


async fn get_swap_info(target: &str) -> Result<(), Box<dyn Error>> {
    // Example parameters:
    // Input token: USDC mint address
    // Output token: the new coin's mint (for example, "2pnaH55CkbZFFVsMbB7cc5cN9LNn5XERhb2K9W1Fpump")
    // Amount: amount in smallest unit (e.g., for USDC with 6 decimals, 1 cent is 10,000 lamports)
    let input_mint = "So11111111111111111111111111111111111111112";  // USDC
    let output_mint = target; // Example target coin mint
    let amount = "1"; // 1 cent worth of USDC if 1 USDC = 1000000 units
    let slippage = "1"; // 1% slippage tolerance

    // Build the API endpoint URL
    let url = format!(
        "https://api.jup.ag/swap/v1/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&restrictIntermediateTokens=true",
        input_mint, output_mint, amount, slippage
    );

    println!("{}", url);
    // Create an HTTP client and request the quote.
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    println!("{:#?}", resp);
    let quote: QuoteResponse = resp.json().await?;

    // Check if we got at least one route.
    


    Ok(())
}
