use std::io::Error;

fn main() -> Result<(), Error> {
    // Shell completions are generated via `ukrop init <shell>` at runtime,
    // so build.rs is minimal. Reserved for future use (e.g., clap_complete).
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
