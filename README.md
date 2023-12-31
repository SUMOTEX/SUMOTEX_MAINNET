RUST_LOG=info cargo run

Update to this if you are getting error for libp2p-noise on v0.39
 pub fn from_ed25519(ed25519_sk: &ed25519::SecretKey) -> Self { 
     // An Ed25519 public key is derived off the left half of the SHA512 of the 
     // secret scalar, hence a matching conversion of the secret key must do 
     // the same to yield a Curve25519 keypair with the same public key. 
     // let ed25519_sk = ed25519::SecretKey::from(ed); 
     let mut curve25519_sk: [u8; 32] = [0; 32]; 
     let hash = Sha512::digest(ed25519_sk.as_ref()); 
     curve25519_sk.copy_from_slice(&hash[..32]); 
     let sk = SecretKey(X25519(curve25519_sk)); // Copy 
     curve25519_sk.zeroize(); 
     sk 
 } 

 > sudo apt install librocksdb-dev libsnappy-dev
 sudo killall -9 public_chain

 To run this application as a systemd service:

1. Copy `my_rust_app.service` to `/etc/systemd/system/`:
   `sudo cp systemd/public_app.service /etc/systemd/system/`

2. Reload systemd to recognize the new service:
   `sudo systemctl daemon-reload`

3. Enable the service to start on boot:
   `sudo systemctl enable my_rust_app`

4. Start the service:
   `sudo systemctl start my_rust_app`



cargo new --lib generate_abi_macro
cargo build --target wasm32-wasi --release


[lib]
name = "sample"
path = "src/erc20_functions.rs"
proc-macro = true


Create a new file .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-Wl,--allow-multiple-definition"]

lsof -i -n | grep LISTEN
kill -9 PID

server {
    listen 443 ssl;
    server_name rpc.sumotex.co;
    ssl_certificate /etc/letsencrypt/live/rpc.sumotex.co/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/rpc.sumotex.co/privkey.pem;

    location / {
        proxy_pass http://localhost:8000; # Replace with the port your Rust app is running on
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}


server {
    listen 443 ssl;
    server_name rpc.sumotex.co;

    ssl_certificate /etc/letsencrypt/live/rpc.sumotex.co/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/rpc.sumotex.co/privkey.pem;

    # Other configurations...
}


Need to create https server port 443