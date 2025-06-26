pub fn env_or_exit(var: &str) -> String {
    match std::env::var(var) {
        Ok(uri) => uri,
        Err(_) => {
            eprintln!("{var} environment variable not set");
            std::process::exit(1);
        }
    }
}
