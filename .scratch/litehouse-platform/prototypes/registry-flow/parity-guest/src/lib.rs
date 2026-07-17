wit_bindgen::generate!({
    path: "../wit",
    world: "parity-guest",
    generate_all,
});

use ed25519_dalek::VerifyingKey;
use exports::litehouse::registry_parity::query;

struct Guest;

impl query::Guest for Guest {
    fn run(
        root: Vec<u8>,
        shard: Vec<u8>,
        package: String,
        version: String,
        trusted_key: Vec<u8>,
    ) -> Result<String, String> {
        let key: [u8; 32] = trusted_key
            .try_into()
            .map_err(|_| "invalid-key".to_owned())?;
        let key = VerifyingKey::from_bytes(&key).map_err(|_| "invalid-key".to_owned())?;
        let payload =
            registry_flow_core::query_exact_bytes(&root, &shard, &package, &version, &key)
                .map_err(|error| error.to_string())?;
        Ok(hex::encode(payload.primary.sha256))
    }
}

export!(Guest);
