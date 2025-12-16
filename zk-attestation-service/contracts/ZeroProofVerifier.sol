// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface ISP1Verifier {
    /// @notice Universal SP1 verifier interface.
    /// Verifies a Groth16 or Plonk proof against a VK hash and public values.
    function verifyProof(
        bytes32 vkHash,
        bytes calldata publicValues,
        bytes calldata proof
    ) external view returns (bool);
}

/// @title ZeroProofVerifier
/// @notice Wrapper around the pre-deployed universal SP1 verifier.
///         Agent A calls this contract instead of the universal verifier directly.
///         This allows for app-specific proof verification and validation logic.
contract ZeroProofVerifier {
    ISP1Verifier public immutable VERIFIER;
    address public immutable OWNER;

    event ProofVerified(address indexed caller, bytes32 indexed vkHash, bytes32 publicHash);
    event VerifierUpdated(address indexed newVerifier);

    error ZeroVerifierAddress();
    error ProofVerificationFailed();

    constructor(address verifierAddress) {
        if (verifierAddress == address(0)) revert ZeroVerifierAddress();
        VERIFIER = ISP1Verifier(verifierAddress);
        OWNER = msg.sender;
    }

    /// @notice Verify a proof using the universal SP1 verifier.
    /// @param vkHash 32-byte VK hash produced by the attester (`vk.bytes32()`).
    /// @param publicValues ABI-encoded public values from the SP1 program.
    /// @param proofBytes Raw proof bytes (Groth16 or Plonk wrapped).
    /// @return ok true if verification succeeded (reverts on failure).
    function verifyProof(
        bytes32 vkHash,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view returns (bool ok) {
        // Call the universal SP1 verifier.
        ok = VERIFIER.verifyProof(vkHash, publicValues, proofBytes);
        if (!ok) revert ProofVerificationFailed();
        return true;
    }

    /// @notice Verify proof and decode public values (if using standard encoding).
    ///         Useful for app-specific validation of the program's output.
    /// @param vkHash 32-byte VK hash.
    /// @param publicValues ABI-encoded public values.
    /// @param proofBytes Raw proof bytes.
    /// @return ok true if proof is valid.
    /// @return decodedA First uint256 from public values (customize as needed).
    /// @return decodedB Second uint256 from public values.
    function verifyProofWithDecoding(
        bytes32 vkHash,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view returns (bool ok, uint256 decodedA, uint256 decodedB) {
        // Verify the proof first.
        ok = VERIFIER.verifyProof(vkHash, publicValues, proofBytes);
        if (!ok) revert ProofVerificationFailed();

        // Decode public values (assumes layout: uint256, uint256).
        // Adjust types and order to match your program's actual public output.
        (decodedA, decodedB) = abi.decode(publicValues, (uint256, uint256));

        return (ok, decodedA, decodedB);
    }

    /// @notice Helper to decode public values structure.
    ///         Returns two uint256 values (adjust as needed for your program).
    function decodePublicValues(bytes calldata publicValues)
        external
        pure
        returns (uint256 valueA, uint256 valueB)
    {
        (valueA, valueB) = abi.decode(publicValues, (uint256, uint256));
    }

    /// @notice Get the current verifier address.
    function getVerifier() external view returns (address) {
        return address(VERIFIER);
    }

    /// @notice Get the owner address.
    function getOwner() external view returns (address) {
        return OWNER;
    }
}
