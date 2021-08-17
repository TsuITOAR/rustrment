use std::io::Read;

const XDR_X_PATH: &str = r#"./xdr"#;
const OUTPUT_FILE: &str = r#"xdr.rs"#;
fn main() {
    println!("cargo:rerun-if-env-changed=OUT_DIR");
    println!("cargo:rerun-if-changed={}", XDR_X_PATH);
    let mut buf = String::new();
    for entry in
        std::fs::read_dir(XDR_X_PATH).expect(&format!("directory '{}' doesn't exist", XDR_X_PATH))
    {
        match entry {
            Ok(entry) => {
                if !entry
                    .metadata()
                    .expect("error getting file metadata")
                    .is_file()
                {
                    continue;
                } else {
                    let path = entry.path();
                    let in_file = entry.file_name();
                    let in_file = in_file.to_str().unwrap();
                    let mut out_file = in_file
                        .chars()
                        .take_while(|x| *x != '.')
                        .collect::<String>();
                    out_file.push_str("_xdr.rs");

                    std::fs::File::open(path)
                        .expect(&format!("error opening file '{}'", in_file))
                        .read_to_string(&mut buf)
                        .expect(&format!("error reading file '{}'", in_file));
                }
            }
            Err(e) => {
                panic!(
                    "error iterating over file in '{}':\n{}",
                    XDR_X_PATH,
                    e.to_string()
                );
            }
        }
    }
    std::fs::write(
        std::path::Path::new(std::env::var("OUT_DIR").unwrap().as_str()).join(OUTPUT_FILE),
        fastxdr::Generator::default()
            .generate(buf)
            .expect("error parsing xdr"),
    )
    .unwrap();
}
