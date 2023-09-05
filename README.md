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