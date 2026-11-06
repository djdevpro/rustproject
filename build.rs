//! Sous Windows, l'icône est intégrée à l'exécutable (`assets/rust-project.ico`).

fn main() {
    println!("cargo:rerun-if-changed=assets/rust-project.rc");
    println!("cargo:rerun-if-changed=assets/rust-project.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/rust-project.rc", embed_resource::NONE).manifest_optional().expect("icône Windows");
    }
}
