#![feature(const_mut_refs)]

use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::Once,
};

use hello_world::{greeter_client::GreeterClient, HelloRequest};
use tracing::info;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

const MODE: &str = if cfg!(debug_assertions) {
    "debug"
} else {
    "release"
};

// TODO Remove hardcoded values. Get this from `cargo build`
const BINARY_PATH: &str = const_format::concatc!("../target/", MODE, "/control-server");

/// Initializes logger
fn init_logger(max_level: tracing_subscriber::filter::LevelFilter) {
    static LOGGER: Once = Once::new();

    LOGGER.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(max_level)
            .with_thread_ids(true)
            .init();
        info!("initialized logger");
    })
}

/// Builds control server binary.
fn build_binary() {
    /// We only need to build the binary once.
    static BUILD_SERVER_BINARY: Once = Once::new();

    BUILD_SERVER_BINARY.call_once(|| {
        info!("building binary");
        #[cfg(debug_assertions)]
        Command::new("cargo")
            .args(["build", "--bin", "control-server"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap();

        #[cfg(not(debug_assertions))]
        Command::new("cargo")
            .args(["build", "--release", "--bin", "control-server"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap();
        info!("built binary");
    });
}

/// Runs control server binary.
fn run_binary(addr: &str, port: u16) -> Child {
    info!("starting binary");

    #[cfg(debug_assertions)]
    let child = Command::new(BINARY_PATH)
        .args(["--address", addr, "--port", &port.to_string()])
        // .stdout(Stdio::null())
        // .stderr(Stdio::null())
        .spawn()
        .unwrap();

    #[cfg(not(debug_assertions))]
    let child = Command::new(BINARY_PATH)
        .args(["--address", addr, "--port", &port.to_string()])
        // .stdout(Stdio::null())
        // .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Start time for the control server doesn't matter, so we just wait 1 sec to ensure its
    // started.
    std::thread::sleep(std::time::Duration::from_secs(1));

    info!("running binary");

    child
}

#[tokio::test]
async fn grpc_test() {
    init_logger(tracing_subscriber::filter::LevelFilter::INFO);
    build_binary();

    const ADDR: &str = "::1";
    const PORT: u16 = 8118;
    let server = run_binary(ADDR, PORT);

    // let mut stdout = String::new();
    // server
    //     .stdout
    //     .as_mut()
    //     .map(|s| s.read_to_string(&mut stdout));
    // info!("server.stdout: {stdout:?}");
    // let mut stderr = String::new();
    // server
    //     .stderr
    //     .as_mut()
    //     .map(|s| s.read_to_string(&mut stderr));
    // info!("server.stderr: {stderr:?}");

    let addr = format!("http://[{ADDR}]:{PORT}");

    // TODO Pass a `SocketAddrV6` instead of a string.
    let mut client = GreeterClient::connect(addr).await.unwrap();

    let request = tonic::Request::new(HelloRequest {
        name: String::from("Tonic"),
    });
    info!("request: {request:?}");

    let response = client.say_hello(request).await.unwrap();
    info!("response: {response:?}");

    // let mut stdout = String::new();
    // server
    //     .stdout
    //     .as_mut()
    //     .map(|s| s.read_to_string(&mut stdout));
    // info!("server.stdout: {stdout:?}");
    // let mut stderr = String::new();
    // server
    //     .stderr
    //     .as_mut()
    //     .map(|s| s.read_to_string(&mut stderr));
    // info!("server.stderr: {stderr:?}");

    // unsafe {
    //     libc::kill(server.id() as i32, libc::SIGINT);
    // }
}
