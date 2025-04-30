## 🧾 HyperEvm S3 Block Downloader

This tool downloads EVM block files from an AWS S3 bucket.

## 🔧 Prerequisites
- AWS credentials with permissions to access the `aws s3` bucket 

## 📦 Installation

Clone the repository and build the project:

```bash
git clone https://github.com/Blueberryfi/hyperevm-s3-downloader.git
cd hyperevm-s3-downloader
cargo build --release
```

## 🚀 Usage
```bash
cargo run --release -- --start-block <start> --end-block <end> [--concurrency <N>] [--region <aws-region>]
```

### Example
```bash
cargo run --release -- --start-block 21348322 --end-block 21348332
```

## 📂 Output
Downloaded blocks will be saved under:
```bash
~/evm-blocks/<major>/<minor>/<block>.rmp.lz4
```
