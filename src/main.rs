use aws_sdk_s3::Client;
use aws_sdk_s3::config::Region;
use aws_config::BehaviorVersion;
use std::fs;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::io::AsyncWriteExt;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    start_block: u64,

    #[arg(long)]
    end_block: u64,

    #[arg(long, default_value_t = 10)]
    concurrency: usize,

    #[arg(long, default_value = "ap-northeast-1")]
    region: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start_block = args.start_block;
    let end_block = args.end_block;
    let concurrency = args.concurrency;
    let region = args.region;

    let home_dir = env::var("HOME")?;
    let download_dir = format!("{}/evm-blocks", home_dir);

    fs::create_dir_all(&download_dir)?;

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .load()
        .await;
    let client = Arc::new(Client::new(&config));

    println!("Downloading blocks {} to {}...", start_block, end_block);

    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut tasks = Vec::new();

    for block in start_block..=end_block {
        let major_dir = (block / 1000000) * 1000000;
        let minor_dir = (block / 1000) * 1000;

        let dir_path = format!("{}/{}/{}", download_dir, major_dir, minor_dir);
        fs::create_dir_all(&dir_path)?;

        let s3_key = format!("{}/{}/{}.rmp.lz4", major_dir, minor_dir, block);
        let local_path = format!("{}/{}.rmp.lz4", dir_path, block);

        if Path::new(&local_path).exists() {
            println!("Block {} already exists, skipping", block);
            continue;
        }

        let client = client.clone();
        let semaphore = semaphore.clone();
        let local_path_clone = local_path.clone();

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            match download_block(&client, "hl-testnet-evm-blocks", &s3_key, &local_path_clone).await {
                Ok(_) => println!("Downloaded block {}", block),
                Err(e) => {
                    let user_friendly_error = extract_error_message(&e);
                    eprintln!("Failed to download block {}: {}", block, user_friendly_error);
                },
            }
        });

        tasks.push(task);
    }

    futures::future::join_all(tasks).await;

    println!("Processing complete.");
    Ok(())
}

fn extract_error_message(err: &Box<dyn std::error::Error>) -> String {
    let err_string = format!("{:?}", err);

    if err_string.contains("NoSuchKey") {
        "Block does not exist in the S3 bucket".to_string()
    } else if err_string.contains("PermanentRedirect") {
        if let Some(start) = err_string.find("Endpoint>") {
            if let Some(end) = err_string.find("</Endpoint") {
                let endpoint = &err_string[start+9..end];
                format!("Wrong region. Use the region from: {}", endpoint)
            } else {
                "Wrong region for this S3 bucket".to_string()
            }
        } else {
            "Wrong region for this S3 bucket".to_string()
        }
    } else if err_string.contains("AccessDenied") {
        "Access denied. Check your AWS credentials and permissions".to_string()
    } else if err_string.contains("timeout") {
        "Request timed out. The network might be slow or unstable".to_string()
    } else {

        format!("{}", err)
    }
}

async fn download_block(
    client: &Client,
    bucket: &str,
    key: &str,
    local_path: &str
) -> Result<(), Box<dyn std::error::Error>> {

    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .request_payer(aws_sdk_s3::types::RequestPayer::Requester)
        .send()
        .await?;

    let data = resp.body.collect().await?;
    let bytes = data.into_bytes();

    let mut file = tokio::fs::File::create(local_path).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;

    Ok(())
}