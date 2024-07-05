// SPDX-License-Identifier: MIT
pragma solidity ^0.8.16;

import "./library/AltBn128.sol";

/**
 *  ____  __.__________ ________     __________      ___________    _________                                                          
 *  |    |/ _|\____    //  _____/     \______   \ ____\__    ___/    \_   ___ \   ____ _______   ____    _____    ____    ____  ___.__. 
 *  |      <    /     //   \  ___      |     ___//  _ \ |    |       /    \  \/ _/ __ \\_  __ \_/ __ \  /     \  /  _ \  /    \<   |  | 
 *  |    |  \  /     /_\    \_\  \     |    |   (  <_> )|    |       \     \____\  ___/ |  | \/\  ___/ |  Y Y  \(  <_> )|   |  \\___  | 
 *  |____|__ \/_______ \\______  /     |____|    \____/ |____|        \______  / \___  >|__|    \___  >|__|_|  / \____/ |___|  // ____| 
 *          \/        \/       \/                                            \/      \/             \/       \/              \/ \/      
 */
contract KZG {
    uint constant Fr = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

    uint public submissionCount;

    uint public numG1;
    uint public numG2;

    AltBn128.G1Point public prevG1_0;

    event CeremonyUpdated(uint submissionCount);

    constructor(bytes[] memory _g1s, bytes[] memory _g2s) {
        require(_g2s.length == 1); // Limitation until V2 with cheaper verification

        numG1 = _g1s.length;
        numG2 = _g2s.length;

        prevG1_0 = AltBn128.g1Unmarshal(_g1s[0]);
        emit CeremonyUpdated(submissionCount);
        submissionCount = 1;
    }

    function potUpdate(
        bytes[] memory _g1s, 
        bytes[] memory _g2s, 
        bytes memory _pi1, 
        uint _pi2) public {
            require(_g1s.length == numG1);
            require(_g2s.length == numG2);

            // Randomness for discrete log check
            uint disLogRandomness = uint(hashRandomness(AltBn128.g1Marshal(prevG1_0), _g1s[0], _pi1)) % Fr;

            // Randomness for pairing check
            uint pairingRandomness = uint(keccak256(abi.encode(_g1s, _g2s)));

            AltBn128.G1Point[] memory updatedG1s = new AltBn128.G1Point[](_g1s.length);
            for (uint i = 0; i < updatedG1s.length; i++) {
                updatedG1s[i] = AltBn128.g1Unmarshal(_g1s[i]);
            }

            AltBn128.G2Point memory updatedG2 = AltBn128.g2Unmarshal(_g2s[0]);
            AltBn128.G1Point memory pi1 = AltBn128.g1Unmarshal(_pi1);


            require(verifyDiscreteLog(prevG1_0, updatedG1s[0], pi1, _pi2, disLogRandomness));
            require(verifyPairing(updatedG1s, updatedG2, pairingRandomness));
            require(verifyNonZero(updatedG1s[0]));

            prevG1_0 = updatedG1s[0];
            emit CeremonyUpdated(submissionCount);
            submissionCount += 1;
    }


    function verifyDiscreteLog(
        AltBn128.G1Point memory existingG1, 
        AltBn128.G1Point memory updatedG1, 
        AltBn128.G1Point memory pi1,
        uint pi2,
        uint randomnessFr) public view returns (bool) {

        AltBn128.G1Point memory lhs = AltBn128.scalarMultiply(existingG1, pi2);
        AltBn128.G1Point memory mul = AltBn128.scalarMultiply(updatedG1, randomnessFr);
        AltBn128.G1Point memory rhs = AltBn128.g1Add(mul, pi1);
        return lhs.x == rhs.x && lhs.y == rhs.y;
    }

    function verifyPairing(
        AltBn128.G1Point[] memory updatedG1s,
        AltBn128.G2Point memory updatedG2,
        uint randomness) public view returns (bool) {
            uint rho = uint(keccak256(abi.encode(randomness, 0))) % Fr;

            AltBn128.G1Point memory R = updatedG1s[0];
            uint rollingRho = 1;
            for (uint i = 1; i < updatedG1s.length - 1; i++) {
                rollingRho = mulmod(rollingRho, rho, Fr);
                R = AltBn128.g1Add(AltBn128.scalarMultiply(updatedG1s[i], rollingRho), R);
            }
            rollingRho = mulmod(rollingRho, rho, Fr);

            AltBn128.G1Point memory lhsG1 = AltBn128.g1Add(AltBn128.g1(), AltBn128.scalarMultiply(R, rho));
            AltBn128.G1Point memory rhsG1 = AltBn128.g1Add(R, AltBn128.scalarMultiply(updatedG1s[updatedG1s.length - 1], rollingRho));
            AltBn128.G2Point memory g2GeneratorInv = AltBn128.g2Inv();

            return AltBn128.pairing(lhsG1, updatedG2, rhsG1, g2GeneratorInv);
    }

    function verifyNonZero(
        AltBn128.G1Point memory g1
    ) public pure returns (bool) {
        return g1.x != 0 && g1.y != 0;
    }

    function hashRandomness(
        bytes memory prevG1,
        bytes memory currG1,
        bytes memory pi1) public pure returns (bytes32) {
        require(prevG1.length == 64);
        require(currG1.length == 64);
        require(pi1.length == 64);

        bytes memory concat = bytes.concat(prevG1, currG1, pi1);

        return keccak256(concat);
    }
}
