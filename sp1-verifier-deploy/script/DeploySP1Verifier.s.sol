// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/SP1VerifierGroth16.sol";

contract DeploySP1Verifier is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        vm.startBroadcast(deployerPrivateKey);
        
        SP1Verifier verifier = new SP1Verifier();
        
        console.log("SP1 Groth16 Verifier deployed to:", address(verifier));
        console.log("VERIFIER_HASH:", vm.toString(verifier.VERIFIER_HASH()));
        console.log("VERSION:", verifier.VERSION());
        
        vm.stopBroadcast();
    }
}
