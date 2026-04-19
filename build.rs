fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon/rs2br_engine_icon.ico");
        res.compile().expect("Falha ao compilar recursos do Windows (ícone .exe)");
    }
}
