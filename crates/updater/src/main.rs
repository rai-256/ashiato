fn main() {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let development = option_env!("ASHIATO_VERSION")
        .map(|version| version.contains("-dev."))
        .unwrap_or(false);
    let dir = std::env::var_os("ASHIATO_UPDATER_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ashiato_updater::default_dir(development));
    let result: Result<(), String> = match command.as_str() {
        "once" => ashiato_updater::once(&dir)
            .map(|_| ())
            .map_err(|error| error.to_string()),
        #[cfg(windows)]
        "service" => ashiato_updater::service(dir, development),
        _ => Err("使い方: ashiato-updater once|service".to_owned()),
    };
    if let Err(error) = result {
        tracing::error!(error = %error, "ashiato-updater failed");
        std::process::exit(1);
    }
}
