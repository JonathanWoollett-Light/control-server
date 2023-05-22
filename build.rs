fn main() -> Result<(), Box<dyn std::error::Error>> {
    match (cfg!(feature = "reqwest"), cfg!(feature = "hyper")) {
        (false, true) | (true, false) => (),
        _ => panic!("one of the `reqwest` or `hyper` features must be enabled"),
    }
    tonic_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
