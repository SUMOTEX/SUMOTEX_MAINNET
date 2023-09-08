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

 To run this application as a systemd service:

1. Copy `my_rust_app.service` to `/etc/systemd/system/`:
   `sudo cp systemd/public_app.service /etc/systemd/system/`

2. Reload systemd to recognize the new service:
   `sudo systemctl daemon-reload`

3. Enable the service to start on boot:
   `sudo systemctl enable my_rust_app`

4. Start the service:
   `sudo systemctl start my_rust_app`