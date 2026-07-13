use crate::cli::DashboardArgs;
use crate::core::doc_root;
use crate::error::DowError;

pub fn run(args: DashboardArgs, _human: bool) -> std::result::Result<i32, DowError> {
    let doc_root = doc_root::resolve(".dev-doc");

    if !doc_root.join("STATUS.yaml").exists() {
        return Err(DowError::new(
            "No .dev-doc/ found. Run `dow init` first.",
            1,
        ));
    }

    let port = args.port.unwrap_or_else(find_available_port);

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| DowError::new(format!("Failed to create async runtime: {}", e), 1))?;

    rt.block_on(crate::dashboard::server::start(
        doc_root,
        port,
        args.no_open,
    ))
}

fn find_available_port() -> u16 {
    for port in 9800..=9900 {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    9800
}
