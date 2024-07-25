pub fn env_or_exit(var: &str) -> String {
    match std::env::var(var) {
        Ok(uri) => uri,
        Err(_) => {
            eprintln!("{} environment variable not set", var);
            std::process::exit(1);
        }
    }
}
