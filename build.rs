use anyhow::Result;
use std::{env, fs, io, path::Path};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=res/*");

    let project_directory = env::var("CARGO_MANIFEST_DIR")?;
    let source_directory = Path::new(&project_directory).join("res");

    let build_directory = env::var("OUT_DIR")?;
    let destination_directory = Path::new(&build_directory).join("res");

    println!(
        "cargo:rustc-env=RESOURCE_DIRECTORY={}",
        destination_directory.as_path().to_str().unwrap()
    );

    fs::create_dir_all(&destination_directory)?;
    copy_recursive(&source_directory, &destination_directory)?;

    Ok(())
}

fn copy_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    fs::read_dir(source)?.try_for_each(|entry| -> io::Result<()> {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let destination_path = destination.join(file_name);

        if path.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_recursive(&path, &destination_path)?;
        } else {
            fs::copy(&path, &destination_path)?;
        }

        Ok(())
    })
}
