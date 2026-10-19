// Embeds the application icon in the Windows executable.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon/quillwright.ico");
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon/quillwright.ico");
        resource
            .compile()
            .expect("failed to embed the Windows icon");
    }
}
