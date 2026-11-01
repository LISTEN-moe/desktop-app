fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    println!("cargo:rerun-if-changed=assets/windows.rc");
    #[cfg(target_os = "windows")]
    embed_resource::compile("assets/windows.rc", embed_resource::NONE)
        .manifest_required()
        .expect("Cannot embed the application icon");
}
