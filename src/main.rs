#![feature(const_mut_refs)]

// TODO Get hyper implementation working and benchmark hyper vs reqwest.

use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};

use clap::Parser;
use hello_world::{
    greeter_server::{Greeter, GreeterServer},
    HelloReply, HelloRequest,
};
#[cfg(feature = "hyper")]
use hyper_rustls::ConfigBuilderExt;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

const AUTH_TOKEN: &str = "ysEstoznyguVsEjZan3HSoxSex-NYZXt7WPQJsNb";
// curl -H "Authorization: Bearer ysEstoznyguVsEjZan3HSoxSex-NYZXt7WPQJsNb" https://api-sandbox.coingate.com/v2/auth/test

const COINGATE_URL: &str = if cfg!(debug_assertions) {
    "https://api-sandbox.coingate.com/v2"
} else {
    "https://api.coingate.com/v2"
};
const ORDERS_URL: &str = const_format::concatc!(COINGATE_URL, "/orders");

// TODO Implement struct to handle the body returned on error code 422)
/// see https://developer.coingate.com/reference/create-order
// We don't access many of the fields in the response, but we should still specify them.
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct CoingateOrdersResponse {
    id: i32,
    status: String,
    // It can be `null`
    title: Option<String>,
    do_not_convert: bool,
    orderable_type: String,
    orderable_id: i32,
    price_currency: String,
    price_amount: String,
    lightning_network: bool,
    receive_currency: String,
    receive_amount: String,
    created_at: String,
    order_id: String,
    payment_url: String,
    underpaid_amount: String,
    overpaid_amount: String,
    is_refundable: bool,
    refunds: Vec<String>,
    voids: Vec<String>,
    fees: Vec<String>,
    token: String,
}

#[derive(Debug)]
pub struct MyGreeter {
    #[cfg(feature = "hyper")]
    coingate_client: hyper::client::Client<
        hyper_rustls::HttpsConnector<hyper::client::HttpConnector>,
        hyper::Body,
    >,
    #[cfg(feature = "reqwest")]
    coingate_client: reqwest::Client,
    callback_addr: SocketAddrV6,
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HelloReply>, Status> {
        info!("{request:?}");
        let request_body = request.into_inner();

        let body = serde_json::json!({
            "price_amount": format!("{}.0",request_body.units),
            "price_currency": "USD",
            "receive_currency": "USD",
            "callback_url": self.callback_addr.to_string()
        });
        info!("{body:?}");

        #[cfg(feature = "reqwest")]
        let coingate_request = self
            .coingate_client
            .post(ORDERS_URL)
            .bearer_auth(AUTH_TOKEN)
            .json(&body);

        #[cfg(feature = "hyper")]
        let coingate_request: hyper::Request<hyper::Body> = hyper::Request::post(ORDERS_URL)
            .header("Authorization", format!("Bearer {AUTH_TOKEN}"))
            .header("Content-Type", "application/json")
            .body(hyper::Body::from(body.to_string()))
            .unwrap();

        info!("{coingate_request:?}");

        #[cfg(feature = "reqwest")]
        let coingate_response = coingate_request.send().await.unwrap();

        #[cfg(feature = "hyper")]
        let coingate_response = self
            .coingate_client
            .request(coingate_request)
            .await
            .unwrap();

        info!("{coingate_response:?}");

        #[cfg(feature = "reqwest")]
        let coingate_response_body = match coingate_response.status() {
            reqwest::StatusCode::OK => coingate_response
                .json::<CoingateOrdersResponse>()
                .await
                .unwrap(),
            // TODO Handle this better
            x => {
                let body = coingate_response.json::<serde_json::Value>().await.unwrap();
                error!("Bad status code {x} with body: {body}");
                return Err(Status::unknown(String::new()));
            }
        };

        #[cfg(feature = "hyper")]
        let coingate_response_body = todo!();

        info!("{coingate_response_body:?}");

        // We must use .into_inner() as the fields of gRPC requests and responses are private
        #[cfg(feature = "reqwest")]
        let reply = hello_world::HelloReply {
            payment_url: coingate_response_body.payment_url,
        };

        #[cfg(feature = "hyper")]
        let reply = todo!();

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

/// To enforce clear calls we don't have any defaults.
#[derive(Parser, Debug)]
struct Args {
    /// Socket port.
    #[arg(long)]
    port: u16,
    /// Socket address.
    #[arg(long)]
    address: Ipv6Addr,
    /// Callback port.
    #[arg(long)]
    callback_port: u16,
    /// Callback address.
    #[arg(long)]
    callback_address: Ipv6Addr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .init();

    let args = Args::parse();
    info!("args: {args:?}");

    let app = axum::Router::new().route("/", axum::routing::post(root));
    let callback_addr = SocketAddrV6::new(args.callback_address, args.callback_port, 0, 0);
    let callback_server = tokio::spawn(
        axum::Server::bind(&SocketAddr::V6(callback_addr)).serve(app.into_make_service()),
    );

    // See https://github.com/rustls/hyper-rustls/blob/main/examples/client.rs
    #[cfg(feature = "hyper")]
    let client = {
        let tls = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_native_roots()
            .with_no_client_auth();
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls)
            .https_or_http()
            .enable_http1()
            .build();
        let client: hyper::client::Client<_, hyper::Body> =
            hyper::client::Client::builder().build(https);
        client
    };

    #[cfg(feature = "reqwest")]
    let client = reqwest::Client::new();

    let greeter = MyGreeter {
        coingate_client: client,
        callback_addr,
    };
    let addr = SocketAddr::V6(SocketAddrV6::new(args.address, args.port, 0, 0));

    let server = tokio::spawn(
        Server::builder()
            .add_service(GreeterServer::new(greeter))
            .serve(addr),
    );

    // `??` is required to propagate the result from joining the thread and the result from the
    // function called within the thread.
    server.await??;
    callback_server.await??;

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct CoingatePaymentCallback {}

async fn root(axum::Json(payload): axum::Json<CoingatePaymentCallback>) {
    info!("{payload:?}");
}
