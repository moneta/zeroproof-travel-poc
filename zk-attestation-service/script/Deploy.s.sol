// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {ZeroProofVerifier} from "../contracts/ZeroProofVerifier.sol";

/// @notice Deploy ZeroProofVerifier wrapper contract.
/// @dev Environment variables:
///   - UNIVERSAL_VERIFIER: address of the pre-deployed SP1 universal verifier
///   - PRIVATE_KEY: deployer account's private key (used by forge)
///   - RPC_URL: network RPC endpoint (e.g., Libertas/Revolution chain)
contract DeployZeroProofVerifier is Script {
    function run() public {
        // Read the pre-deployed SP1 verifier address from env.
        address universalVerifier = vm.envAddress("UNIVERSAL_VERIFIER");
        require(universalVerifier != address(0), "UNIVERSAL_VERIFIER env var not set or is zero");

        // Get deployer private key from env.
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");

        // Start broadcast (will sign and send tx with deployerKey).
        vm.startBroadcast(deployerKey);

        // Deploy the wrapper contract, passing the universal verifier address.
        ZeroProofVerifier wrapper = new ZeroProofVerifier(universalVerifier);

        vm.stopBroadcast();

        // Print deployment info.
        console.log("========================================");
        console.log("ZeroProofVerifier deployed successfully!");
        console.log("========================================");
        console.log("Wrapper address:       ", address(wrapper));
        console.log("Universal verifier:    ", universalVerifier);
        console.log("Deployer:              ", vm.addr(deployerKey));
        console.log("========================================");
    }
}
