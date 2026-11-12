fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let dir = std::env::var_os("ASHIATO_UPDATER_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            #[cfg(windows)]
            {
                std::path::PathBuf::from(r"C:\ProgramData\ashiato-updater")
            }
            #[cfg(not(windows))]
            {
                std::env::current_dir()
                    .expect("作業場を読めない")
                    .join(".ashiato-updater")
            }
        });
    let result: Result<(), String> = match command.as_str() {
        "once" => ashiato_updater::once(&dir)
            .map(|_| ())
            .map_err(|error| error.to_string()),
        #[cfg(windows)]
        "service" => ashiato_updater::service(dir),
        _ => Err("使い方: ashiato-updater once|service".to_owned()),
    };
    if result.is_err() {
        std::process::exit(1);
    }
}
