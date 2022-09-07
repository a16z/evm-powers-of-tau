use ark_bn254::{Bn254, Fr, G1Projective as G1, G2Affine, G2Projective as G2};
use ark_ec::{PairingEngine, ProjectiveCurve};
use ark_ff::PrimeField;
use ark_ff::QuadExtField;
use ark_std::UniformRand;
use ark_std::Zero;

use crate::utils;
use crate::utils::IntoBytes;

#[derive(Debug)]
pub struct Proof {
    pub g1s: Vec<G1>,
    pub g2s: Vec<G2>,
    pub pi1: G1,
    pub pi2: Fr,
}

pub fn init_params(num_g1: usize, num_g2: usize) -> (Vec<G1>, Vec<G2>) {
    let g1_generator: G1 = utils::g1_generator();
    let g2_generator: G2 = utils::g2_generator();

    let g1s = vec![g1_generator; num_g1];
    let g2s = vec![g2_generator; num_g2];

    (g1s, g2s)
}

// Note: Most group operations are slower in affine coordinates
pub fn run_update(init_g1s: &[G1], init_g2s: &[G2]) -> Proof {
    let mut g1s: Vec<G1> = Vec::with_capacity(init_g1s.len());
    let mut g2s: Vec<G2> = Vec::with_capacity(init_g2s.len());

    let rng = &mut ark_std::rand::thread_rng();
    let tau = Fr::rand(rng);
    let mut running_param: Fr = tau;

    for i in 0..init_g1s.len() {
        g1s.push(init_g1s[i].mul(running_param.into_repr()));
        running_param = tau * running_param;

        assert!(g1s[i].into_affine().is_on_curve());
        assert!(g1s[i]
            .into_affine()
            .is_in_correct_subgroup_assuming_on_curve());
    }

    running_param = tau;
    for i in 0..init_g2s.len() {
        g2s.push(init_g2s[i].mul(running_param.into_repr()));
        running_param = running_param * tau;

        assert!(g2s[i].into_affine().is_on_curve());
        assert!(g2s[i]
            .into_affine()
            .is_in_correct_subgroup_assuming_on_curve());
    }

    // Generate the proof
    let z = Fr::rand(rng); // Random hiding param
    let pi1 = init_g1s[0].mul(z.into_repr());

    // pi2
    let det_hash = hash_randomness(init_g1s[0], g1s[0], pi1);
    let det_hash_fr = Fr::from_be_bytes_mod_order(&det_hash);

    let pi2 = z + det_hash_fr * tau;

    Proof {
        g1s: g1s,
        g2s: g2s,
        pi1: pi1,
        pi2: pi2,
    }
}

pub fn verify_proof(proof: &Proof, prev_g1s: &[G1]) {
    let g1_generator: G1 = utils::g1_generator();
    let g2_generator: G2 = utils::g2_generator();

    // Discrete log check
    let lhs = prev_g1s[0].mul(proof.pi2.into_repr());

    let det_hash = hash_randomness(prev_g1s[0], proof.g1s[0], proof.pi1);
    let det_hash_fr = Fr::from_be_bytes_mod_order(&det_hash);
    let rhs = proof.g1s[0].mul(det_hash_fr.into_repr()) + proof.pi1;

    assert_eq!(lhs, rhs);

    // Pairing check
    let mut rho = Vec::new();
    let rng = &mut ark_std::rand::thread_rng();
    for _ in 0..(proof.g1s.len() - 1) {
        rho.push(Fr::rand(rng));
    }

    let mut lhs_sum = g1_generator.mul(rho[0].into_repr());
    for i in 0..(proof.g1s.len() - 2) {
        lhs_sum = lhs_sum + (proof.g1s[i].mul(rho[i + 1].into_repr()));
    }
    let lhs = Bn254::pairing(lhs_sum, proof.g2s[0]);

    let mut rhs_sum = proof.g1s[0].mul(rho[0].into_repr());
    for i in 1..(proof.g1s.len() - 1) {
        rhs_sum = rhs_sum + (proof.g1s[i].mul(rho[i].into_repr()));
    }
    let rhs = Bn254::pairing(rhs_sum, g2_generator);
    assert_eq!(lhs, rhs);

    // Duplicate pairing check – matching restrictive on-chain precompile check
    let mut g2_gen_inv = g2_generator.into_affine();
    g2_gen_inv = G2Affine::new(g2_gen_inv.x, QuadExtField::from(-1) * g2_gen_inv.y, false);
    let rhs_inv = Bn254::pairing(rhs_sum, g2_gen_inv);
    let mul = lhs * rhs_inv;
    assert_eq!(mul, QuadExtField::from(1));

    // Non-degenerate check – pp0 != 0
    assert!(!proof.g1s[0].is_zero());
}

pub fn hash_randomness(prev_g1: G1, curr_g1: G1, pi1: G1) -> [u8; 32] {
    let mut concat: Vec<u8> = Vec::new();
    let hash_param_1 = prev_g1.serialize();
    let hash_param_2 = curr_g1.serialize();
    let hash_param_3 = pi1.serialize();

    concat.extend(hash_param_1);
    concat.extend(hash_param_2);
    concat.extend(hash_param_3);

    utils::keccak256(&concat)
}
