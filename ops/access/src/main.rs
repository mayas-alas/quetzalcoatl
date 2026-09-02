use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = if args.len() == 3 && args[1] == "--config" {
        match args[0].as_str() {
            "apply" => gnx_access::apply(Path::new(&args[2])),
            "configure" => gnx_access::configure(Path::new(&args[2])),
            "dns" => gnx_access::dns(Path::new(&args[2])).map(|report| match report.checks {
                Ok(()) => format!("access-dns\n{}", report.fields),
                Err(gate) => {
                    use std::io::Write;
                    let message = format!("{}\nFAILED {gate}\n", report.fields);
                    let _ = std::io::stderr().write_all(message.as_bytes());
                    std::process::exit(1);
                }
            }),
            _ => Err("ARGUMENTS"),
        }
    } else {
        Err("ARGUMENTS")
    };
    match result {
        Ok(message) => println!("READY {message}"),
        Err(gate) => {
            eprintln!("FAILED {gate}");
            std::process::exit(1);
        }
    }
}
