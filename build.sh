# These are ubuntu specific

# Required by tonic see https://crates.io/crates/tonic.
sudo apt update && sudo apt upgrade -y
sudo apt install -y protobuf-compiler libprotobuf-dev
# Required for ssl
supd apt install opensll-dev
sudo apt install pkg-config