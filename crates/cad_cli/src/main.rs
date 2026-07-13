#![forbid(unsafe_code)]

fn main() {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "--help".to_owned());
    match command.as_str() {
        "--help" | "help" => println!("cad validate|export|batch-export|inspect|recover"),
        other => eprintln!("command '{other}' is not implemented in the initial scaffold"),
    }
}
