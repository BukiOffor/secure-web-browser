fn main() {
    mapper::key_mapper::capture_key()
        .unwrap_or_else(|e| eprintln!("Error: {}", e));
}

