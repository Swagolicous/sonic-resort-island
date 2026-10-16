use anyhow::{Context, Result};
use std::fs;
use unleashed_assets::{packed::parse_pfd, xcompress};

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| r"..\game\Packed\ActD_MykonosAct1\Stage.pfd".to_owned());

    let bytes = fs::read(&path).with_context(|| format!("read {path}"))?;
    let index = parse_pfd(&bytes).context("parse Stage.pfd")?;

    for entry in index
        .entries()
        .iter()
        .filter(|entry| entry.name.starts_with("tg-"))
        .take(4)
    {
        println!(
            "entry={} record={:#x} payload={:#x} bytes={:#x}",
            entry.name,
            entry.record_offset,
            entry.payload_offset,
            entry.data.len()
        );

        let prefix_len = entry.data.len().min(0x50);
        print!("prefix=");
        for byte in &entry.data[..prefix_len] {
            print!("{byte:02X}");
        }
        println!();

        if entry.data.len() <= 0x10 {
            println!("too short");
            continue;
        }

        match xcompress::XCompressHeader::parse(&entry.data[0x10..]) {
            Ok(header) => println!("header={header:?}"),
            Err(error) => {
                println!("header_error={error:#}");
                continue;
            }
        }

        match xcompress::decompress(&entry.data[0x10..]) {
            Ok(output) => println!(
                "decompressed={} first16={:02X?}",
                output.len(),
                &output[..output.len().min(16)]
            ),
            Err(error) => println!("decompress_error={error:#}"),
        }
    }

    Ok(())
}
