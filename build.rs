fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        res.set_language(0x0409); // English (United States)
        res.compile().unwrap();
    }
}
