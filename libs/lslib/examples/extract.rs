//! Extracts a package (`.lsv` savegame or `.pak`) into a directory.
//!
//! ```text
//! cargo run -p lslib --example extract -- <package> <output dir> [--dump]
//! ```
//!
//! With `--dump`, every `.lsf` resource also gets a readable `<name>.lsf.txt`
//! tree next to it.

use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use lslib::lsf::LsfFile;
use lslib::package::Package;
use lslib::resource::{AttributeValue, Node, uuid_string};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dump = args.iter().any(|a| a == "--dump");
    let paths: Vec<&String> = args.iter().filter(|a| *a != "--dump").collect();
    let [package, out] = paths[..] else {
        eprintln!("usage: extract <package> <output dir> [--dump]");
        return ExitCode::FAILURE;
    };
    match run(Path::new(package), Path::new(out), dump) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(package_path: &Path, out: &Path, dump: bool) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(package_path)?;
    let package = Package::read(&bytes)?;
    println!(
        "{}: {:?}, {} files",
        package_path.display(),
        package.version(),
        package.files().len()
    );

    for file in package.files() {
        if file.is_deletion() {
            println!("  skip  {} (deletion marker)", file.name);
            continue;
        }
        let target = out.join(safe_relative(&file.name)?);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = package.contents(file)?;
        std::fs::write(&target, &contents)?;
        println!(
            "  {:>10}  {:?}  {}",
            contents.len(),
            file.flags.method()?,
            file.name
        );

        if dump && file.name.to_ascii_lowercase().ends_with(".lsf") {
            match LsfFile::read(&contents) {
                Ok(lsf) => {
                    let mut text = format!(
                        "{:?}\nengine {:?}\n\n",
                        lsf.format, lsf.resource.engine_version
                    );
                    for region in &lsf.resource.regions {
                        dump_node(&mut text, region, 0);
                    }
                    let mut dump_path = target.into_os_string();
                    dump_path.push(".txt");
                    std::fs::write(dump_path, text)?;
                }
                Err(e) => eprintln!("  cannot dump {}: {e}", file.name),
            }
        }
    }
    println!("extracted to {}", out.display());
    Ok(())
}

/// The entry name as a relative path that cannot escape the output directory.
fn safe_relative(name: &str) -> Result<PathBuf, String> {
    let path = Path::new(name);
    if path.components().all(|c| matches!(c, Component::Normal(_))) {
        Ok(path.to_path_buf())
    } else {
        Err(format!("refusing unsafe entry name {name:?}"))
    }
}

fn dump_node(out: &mut String, node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    let key = node
        .key_attribute
        .as_deref()
        .map(|k| format!(" [key: {k}]"))
        .unwrap_or_default();
    let _ = writeln!(out, "{indent}{}{key}", node.name);
    for attribute in &node.attributes {
        let _ = writeln!(
            out,
            "{indent}  .{} ({}) = {}",
            attribute.name,
            attribute.value.kind().name(),
            format_value(&attribute.value)
        );
    }
    for child in &node.children {
        dump_node(out, child, depth + 1);
    }
}

fn format_value(value: &AttributeValue) -> String {
    use AttributeValue as V;
    match value {
        V::None => "-".to_owned(),
        V::Byte(v) => v.to_string(),
        V::Short(v) => v.to_string(),
        V::UShort(v) => v.to_string(),
        V::Int(v) => v.to_string(),
        V::UInt(v) => v.to_string(),
        V::Float(v) => v.to_string(),
        V::Double(v) => v.to_string(),
        V::Bool(v) => v.to_string(),
        V::ULongLong(v) => v.to_string(),
        V::Long(v) | V::Int64(v) => v.to_string(),
        V::Int8(v) => v.to_string(),
        V::ScratchBuffer(bytes) => format!("<{} bytes>", bytes.len()),
        V::Uuid(bytes) => uuid_string(bytes),
        other => match other.as_str() {
            Some(text) => format!("{text:?}"),
            None => format!("{other:?}"),
        },
    }
}
