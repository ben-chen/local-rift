use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=assets/art");
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut entries = Vec::new();
    if let Ok(files) = fs::read_dir(root.join("assets/art")) {
        for file in files.flatten() {
            let path = file.path();
            if let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("card_"))
                .and_then(|s| s.parse::<usize>().ok())
            {
                entries.push((id, path));
            }
        }
    }
    entries.sort_by_key(|(id, _)| *id);
    let mut source = String::from("pub static CARD_ART: &[(usize, &[u8])] = &[\n");
    for (id, path) in entries {
        source.push_str(&format!("({id}, include_bytes!({:?})),\n", path));
    }
    source.push_str("];\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("card_art.rs"),
        source,
    )
    .unwrap();
}
