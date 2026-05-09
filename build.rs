// build.rs — RS2BR Engine
// Aplica ícone no executável Windows via winres.
// Se o arquivo de ícone não existir, a build continua normalmente
// (sem ícone customizado), nunca falhando por asset ausente.

fn main() {
    // Só aplica ícone no Windows
    #[cfg(target_os = "windows")]
    apply_windows_icon();

    // Rerun apenas se o ícone mudar (evita rebuilds desnecessários)
    println!("cargo:rerun-if-changed=assets/icon/rs2br_engine_icon.ico");
    println!("cargo:rerun-if-changed=assets/icon/rs2br_engine_icon.png");
    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(target_os = "windows")]
fn apply_windows_icon() {
    let ico_path = std::path::Path::new("assets/icon/rs2br_engine_icon.ico");

    if !ico_path.exists() {
        // Ícone ausente: build continua sem ícone, sem erro.
        eprintln!(
            "cargo:warning=rs2br-engine: ícone não encontrado em '{}'. \
             Build continua sem ícone customizado.",
            ico_path.display()
        );
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_str().unwrap_or("assets/icon/rs2br_engine_icon.ico"));

    if let Err(e) = res.compile() {
        // Falha no winres não deve travar a build — apenas avisa.
        eprintln!(
            "cargo:warning=rs2br-engine: falha ao compilar recursos Windows ({}). \
             Build continua sem ícone customizado.",
            e
        );
    }
}
