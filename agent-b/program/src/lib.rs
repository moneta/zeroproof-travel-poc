#![no_main]
sp1_zkvm::entrypoint!(main);

use pricing_core::{handle_call, RpcCall, RpcResult};

pub fn main() {
    let call: RpcCall = sp1_zkvm::io::read();
    let result: RpcResult = handle_call(call);
    sp1_zkvm::io::commit(&result);
}