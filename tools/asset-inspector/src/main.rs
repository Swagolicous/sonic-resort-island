use anyhow::{Context, Result, bail};
use std::{env, fs};

const USAGE: &str = "\
usage:
  asset-inspector <file>
  asset-inspector xcompress <file>
  asset-inspector ar <file>
  asset-inspector arl <file>
  asset-inspector pfi <file>
  asset-inspector pfd <file>
  asset-inspector mirage-mesh <file>
  asset-inspector mirage-instance <file>";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.as_slice() {
        [path] => inspect_auto(path),
        [command, path] => match command.as_str() {
            "xcompress" => inspect_xcompress(path),
            "ar" => inspect_ar(path),
            "arl" => inspect_arl(path),
            "pfi" => inspect_pfi(path),
            "pfd" => inspect_pfd(path),
            "mirage-mesh" => inspect_mirage_mesh(path),
            "mirage-instance" => inspect_mirage_instance(path),
            _ => bail!("{USAGE}"),
        },
        _ => bail!("{USAGE}"),
    }
}

fn read(path: &str) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("read {path}"))
}

fn inspect_auto(path: &str) -> Result<()> {
    let bytes = read(path)?;

    if bytes.len() >= unleashed_assets::xcompress::HEADER_SIZE {
        if let Ok(header) = unleashed_assets::xcompress::XCompressHeader::parse(&bytes) {
            print_xcompress_header(&header, bytes.len());
            return Ok(());
        }
    }

    if let Ok(archive) = unleashed_assets::ar::parse_archive_ref(&bytes) {
        print_ar(&bytes, &archive);
        return Ok(());
    }

    if let Ok(list) = unleashed_assets::ar::parse_archive_list(&bytes) {
        print_arl(&list);
        return Ok(());
    }

    if let Ok(index) = unleashed_assets::packed::parse_pfd(&bytes) {
        print_pfd(&index);
        return Ok(());
    }

    if let Ok(mesh) = unleashed_assets::mirage::parse_terrain_mesh(&bytes) {
        print_mirage_mesh(&mesh);
        return Ok(());
    }

    if let Ok(transform) = unleashed_assets::mirage::parse_terrain_instance_transform(&bytes) {
        print_mirage_instance(&transform);
        return Ok(());
    }

    println!("type=unknown bytes={}", bytes.len());
    Ok(())
}

fn inspect_xcompress(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let header = unleashed_assets::xcompress::XCompressHeader::parse(&bytes)
        .with_context(|| format!("parse XCompress {path}"))?;
    print_xcompress_header(&header, bytes.len());

    let decompressed = unleashed_assets::xcompress::decompress(&bytes)
        .with_context(|| format!("decompress XCompress {path}"))?;
    println!("decompressed_bytes={}", decompressed.len());
    Ok(())
}

fn print_xcompress_header(
    header: &unleashed_assets::xcompress::XCompressHeader,
    file_bytes: usize,
) {
    println!(
        "type=xcompress bytes={} window={:#x} compressed_block={} uncompressed_block={} compressed_size={} uncompressed_size={}",
        file_bytes,
        header.window_size,
        header.compressed_block_size,
        header.uncompressed_block_size,
        header.compressed_size,
        header.uncompressed_size
    );
}

fn inspect_ar(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let entries = unleashed_assets::ar::parse_archive_ref(&bytes)
        .with_context(|| format!("parse AR {path}"))?;
    print_ar(&bytes, &entries);
    Ok(())
}

fn print_ar(bytes: &[u8], entries: &[unleashed_assets::ar::ArchiveEntryRef<'_>]) {
    println!("type=ar bytes={} entries={}", bytes.len(), entries.len());
    let base = bytes.as_ptr() as usize;

    for (index, entry) in entries.iter().enumerate() {
        let data_offset = (entry.data.as_ptr() as usize).saturating_sub(base);
        println!(
            "entry={} data_offset={:#x} data_size={} hash={:#010x} timestamp={:#010x} name={:?}",
            index,
            data_offset,
            entry.data.len(),
            entry.hash,
            entry.timestamp,
            entry.name
        );
    }
}

fn inspect_arl(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let list = unleashed_assets::ar::parse_archive_list(&bytes)
        .with_context(|| format!("parse ARL {path}"))?;
    print_arl(&list);
    Ok(())
}

fn print_arl(list: &unleashed_assets::ar::ArchiveList<'_>) {
    println!(
        "type=arl archives={} names={}",
        list.archive_sizes.len(),
        list.names.len()
    );

    for (index, size) in list.archive_sizes.iter().enumerate() {
        println!("archive={} expected_bytes={}", index, size);
    }

    for (index, name) in list.names.iter().enumerate() {
        println!("name={} value={:?}", index, name);
    }
}

fn inspect_pfi(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let header = unleashed_assets::packed::parse_pfi_entry_header(&bytes)
        .with_context(|| format!("parse PFI entry header {path}"))?;

    println!(
        "type=pfi bytes={} raw_00={:#010x} raw_04={:#010x} raw_08={:#010x} raw_0c={:#010x} record_span={:#x} data_size={} data_offset={:#x} raw_1c={:#010x} raw_20={:#010x}",
        bytes.len(),
        header.raw_00,
        header.raw_04,
        header.raw_08,
        header.raw_0c,
        header.record_span,
        header.data_size,
        header.data_offset,
        header.raw_1c,
        header.raw_20
    );
    Ok(())
}

fn inspect_pfd(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let index =
        unleashed_assets::packed::parse_pfd(&bytes).with_context(|| format!("parse PFD {path}"))?;
    print_pfd(&index);
    Ok(())
}

fn print_pfd(index: &unleashed_assets::packed::PackedFileIndex<'_>) {
    println!(
        "type=pfd entries={} trailer_bytes={}",
        index.entries().len(),
        index.trailer().len()
    );

    for (entry_index, entry) in index.entries().iter().enumerate() {
        println!(
            "entry={} record_offset={:#x} record_span={:#x} payload_offset={:#x} data_size={} name={:?}",
            entry_index,
            entry.record_offset,
            entry.header.record_span,
            entry.payload_offset,
            entry.data.len(),
            entry.name
        );
    }
}

fn inspect_mirage_mesh(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let mesh = unleashed_assets::mirage::parse_terrain_mesh(&bytes)
        .with_context(|| format!("parse Mirage terrain mesh {path}"))?;
    print_mirage_mesh(&mesh);
    Ok(())
}

fn print_mirage_mesh(mesh: &unleashed_assets::mirage::TerrainMesh) {
    println!(
        "type=mirage-mesh vertices={} indices={} stride={} index_format={:?} topology={:?} primitive_restart={:?}",
        mesh.positions.len(),
        mesh.indices.len(),
        mesh.vertex_stride,
        mesh.index_format,
        mesh.topology,
        mesh.primitive_restart
    );
}

fn inspect_mirage_instance(path: &str) -> Result<()> {
    let bytes = read(path)?;
    let transform = unleashed_assets::mirage::parse_terrain_instance_transform(&bytes)
        .with_context(|| format!("parse Mirage terrain instance {path}"))?;
    print_mirage_instance(&transform);
    Ok(())
}

fn print_mirage_instance(transform: &unleashed_assets::mirage::InstanceTransform) {
    println!("type=mirage-instance");
    for (row, values) in transform.matrix.iter().enumerate() {
        println!(
            "row={} x={} y={} z={} w={}",
            row, values[0], values[1], values[2], values[3]
        );
    }
}
