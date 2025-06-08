use std::collections::HashMap;

pub async fn parse_args(args: Vec<String>) -> Result<HashMap<String, String>, String> {
    let mut parsed_args = HashMap::<String, String>::new();

    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-p" | "--port" => {
                if let Some(port) = iter.peek() {
                    parsed_args.insert("port".to_string(), (*port).to_string());
                    iter.next();
                } else {
                    return Err("Missing value for port flag (-p or --port)".to_string());
                }
            }
            _ => continue,
        }
    }

    if !parsed_args.contains_key("port") {
        return Err("Missing port: please specify using -p <port> or --port <port>".to_string());
    }

    Ok(parsed_args)
}
