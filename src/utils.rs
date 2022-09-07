use ark_bn254::g1::{G1_GENERATOR_X, G1_GENERATOR_Y};
use ark_bn254::g2::{G2_GENERATOR_X, G2_GENERATOR_Y};
use ark_ec::short_weierstrass_jacobian::GroupProjective;
use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::PrimeField;
use ark_std::UniformRand;
use ethers::prelude::*;
use std::convert::Into;
use std::convert::TryInto;
use std::fmt::Write;
use tiny_keccak::{Hasher, Keccak};

use ark_bn254::{Fq, Fq2, Fr, G1Affine, G1Projective as G1, G2Affine, G2Projective as G2};

pub trait IntoBytes {
    fn serialize(self) -> Vec<u8>;
    fn into_bytes(self) -> ethers::prelude::Bytes;
}

impl<P: ark_ec::SWModelParameters> IntoBytes for GroupProjective<P> {
    fn serialize(self) -> Vec<u8> {
        let affine = self.into_affine();
        let mut serialized = ark_ff::to_bytes!(affine.x).unwrap();
        let mut serialized_y = ark_ff::to_bytes!(affine.y).unwrap();
        serialized.reverse();
        serialized_y.reverse();
        serialized.extend(serialized_y);
        serialized
    }

    fn into_bytes(self) -> ethers::prelude::Bytes {
        Bytes::from(self.serialize())
    }
}

pub fn serialize_fr(element: Fr) -> [u8; 32] {
    let mut bytes = ark_ff::to_bytes!(element).unwrap();
    bytes.reverse();
    bytes.try_into().unwrap()
}

pub fn encode_bytes_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 2);
    write!(&mut s, "0x").unwrap();

    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);
    output
}

pub fn g1_generator() -> G1 {
    G1Affine::new(G1_GENERATOR_X, G1_GENERATOR_Y, false).into()
}

pub fn g2_generator() -> G2 {
    G2Affine::new(G2_GENERATOR_X, G2_GENERATOR_Y, false).into()
}

pub fn rand_g1() -> G1 {
    let rng = &mut ark_std::rand::thread_rng();
    let tau = Fr::rand(rng);
    let gen = g1_generator();
    gen.mul(tau.into_repr())
}

pub fn fq_to_u256(fq: Fq) -> U256 {
    let mut bytes = ark_ff::to_bytes!(fq).unwrap();
    bytes.reverse();
    U256::from(bytes.as_slice())
}

pub fn fr_to_u256(fr: Fr) -> U256 {
    let mut bytes = ark_ff::to_bytes!(fr).unwrap();
    bytes.reverse();
    U256::from(bytes.as_slice())
}

pub fn u256_to_fq(n: U256) -> Fq {
    let mut bytes = Vec::with_capacity(32);
    for i in 0..32 {
        bytes.push(n.byte(i));
    }
    Fq::from_le_bytes_mod_order(&bytes.as_slice())
}

pub fn u256_to_fr(n: U256) -> Fr {
    let mut bytes = Vec::with_capacity(32);
    for i in 0..32 {
        bytes.push(n.byte(i));
    }
    Fr::from_le_bytes_mod_order(&bytes.as_slice())
}

pub fn contract_bytes_to_g1(b: &Bytes) -> G1 {
    assert!(b.len() == 64);
    let mut x_b = Vec::with_capacity(32);
    let mut y_b = Vec::with_capacity(32);

    for i in 0..32 {
        x_b.push(b.get(i).unwrap().clone());
    }
    for i in 32..64 {
        y_b.push(b.get(i).unwrap().clone());
    }
    let x = Fq::from_be_bytes_mod_order(x_b.as_slice());
    let y = Fq::from_be_bytes_mod_order(y_b.as_slice());

    G1Affine::new(x, y, false).into_projective()
}

pub fn contract_bytes_to_g2(b: &Bytes) -> G2 {
    assert!(b.len() == 128);
    let mut xx_b = Vec::with_capacity(32);
    let mut xy_b = Vec::with_capacity(32);
    let mut yx_b = Vec::with_capacity(32);
    let mut yy_b = Vec::with_capacity(32);

    // Reverse order – big endian
    for i in 0..32 {
        xy_b.push(b.get(i).unwrap().clone());
    }
    for i in 32..64 {
        xx_b.push(b.get(i).unwrap().clone());
    }
    for i in 64..96 {
        yy_b.push(b.get(i).unwrap().clone());
    }
    for i in 96..128 {
        yx_b.push(b.get(i).unwrap().clone());
    }
    let xx = Fq::from_be_bytes_mod_order(xx_b.as_slice());
    let xy = Fq::from_be_bytes_mod_order(xy_b.as_slice());
    let yx = Fq::from_be_bytes_mod_order(yx_b.as_slice());
    let yy = Fq::from_be_bytes_mod_order(yy_b.as_slice());

    let x = Fq2::new(xx, xy);
    let y = Fq2::new(yx, yy);

    G2Affine::new(x, y, false).into_projective()
}

pub fn contract_to_g1(x: U256, y: U256) -> G1 {
    G1Affine::new(u256_to_fq(x), u256_to_fq(y), false).into_projective()
}

pub fn contract_to_g2(xx: U256, xy: U256, yx: U256, yy: U256) -> G2 {
    // Reverse order - big endian
    let x = Fq2::new(Fq::from(u256_to_fq(xy)), Fq::from(u256_to_fq(xx)));
    let y = Fq2::new(Fq::from(u256_to_fq(yy)), Fq::from(u256_to_fq(yx)));
    G2Affine::new(x, y, false).into_projective()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fq_to_u256() {
        let fq = Fq::from(12312312312u64);

        let converted = fq_to_u256(fq);
        let converted_back = u256_to_fq(converted);
        assert_eq!(converted_back, fq);
    }

    #[test]
    fn test_fr_to_u256() {
        let fr = Fr::from(121231212112u64);

        let converted = fr_to_u256(fr);
        let converted_back = u256_to_fr(converted);
        assert_eq!(converted_back, fr);
    }
}
