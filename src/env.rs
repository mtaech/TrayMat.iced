

use std::{fs};

pub fn setup_logger() -> Result<(), fern::InitError> {
    let log_path = get_log_path();
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_path)?)
        .apply()?;
    Ok(())
}

pub fn get_log_path() -> String {
    match home::home_dir() {
        None => {
            panic!("cant find home --")
        }
        Some(home) => {
            let wave_path = home.join(".traymat");
            let path = wave_path
                .join("traymat.log")
                .into_os_string()
                .into_string()
                .unwrap();
            if !wave_path.exists() {
                fs::create_dir_all(wave_path).unwrap();
            }
            return path;
        }
    };
}
