use crate::cuda_helpers::{from_mont_points_g1, from_mont_points_g2};
use crate::file_wrapper::{FileWrapper, Section};
use icicle_bn254::curve::{G1Projective, G2Projective, ScalarField};
use icicle_core::traits::FieldImpl;
use std::io::{self};

#[derive(Clone, Debug)]
pub struct ZKey {
    pub n8q: u32,
    pub q: ScalarField,
    pub n8r: u32,
    pub r: ScalarField,
    pub n_vars: u32,
    pub n_public: u32,
    pub domain_size: u32,
    pub power: u32,
    pub vk_alpha_1: G1Projective,
    pub vk_beta_1: G1Projective,
    pub vk_beta_2: G2Projective,
    pub vk_gamma_2: G2Projective,
    pub vk_delta_1: G1Projective,
    pub vk_delta_2: G2Projective,
}

impl ZKey {
    pub fn new() -> Self {
        Self {
            n8q: 0,
            q: ScalarField::zero(),
            n8r: 0,
            r: ScalarField::zero(),
            n_vars: 0,
            n_public: 0,
            domain_size: 0,
            power: 0,
            vk_alpha_1: G1Projective::zero(),
            vk_beta_1: G1Projective::zero(),
            vk_beta_2: G2Projective::zero(),
            vk_gamma_2: G2Projective::zero(),
            vk_delta_1: G1Projective::zero(),
            vk_delta_2: G2Projective::zero(),
        }
    }

    pub fn read_header_groth16(
        fd: &mut FileWrapper,
        sections: &[Vec<Section>]
    ) -> io::Result<Self> {
        let mut zkey = ZKey::new();

        fd.start_read_unique_section(sections, 2).unwrap();
        zkey.n8q = fd.read_u32_le().unwrap();
        zkey.q = fd.read_big_int(zkey.n8q as usize, None).unwrap();

        zkey.n8r = fd.read_u32_le().unwrap();
        zkey.r = fd.read_big_int(zkey.n8r as usize, None).unwrap();
        zkey.n_vars = fd.read_u32_le().unwrap();
        zkey.n_public = fd.read_u32_le().unwrap();
        zkey.domain_size = fd.read_u32_le().unwrap();
        zkey.power = (zkey.domain_size as f32).log2() as u32;

        let vk_alpha_1 = fd.read_g1();
        let vk_beta_1 = fd.read_g1();
        let vk_beta_2 = fd.read_g2();
        let vk_gamma_2 = fd.read_g2();
        let vk_delta_1 = fd.read_g1();
        let vk_delta_2 = fd.read_g2();

        let mut mont_points_g1 = [vk_alpha_1, vk_beta_1, vk_delta_1];
        let mut mont_points_g2 = [vk_beta_2, vk_gamma_2, vk_delta_2];

        from_mont_points_g1(&mut mont_points_g1);
        from_mont_points_g2(&mut mont_points_g2);

        zkey.vk_alpha_1 = mont_points_g1[0].to_projective();
        zkey.vk_beta_1 = mont_points_g1[1].to_projective();
        zkey.vk_beta_2 = mont_points_g2[0].to_projective();
        zkey.vk_gamma_2 = mont_points_g2[1].to_projective();
        zkey.vk_delta_1 = mont_points_g1[2].to_projective();
        zkey.vk_delta_2 = mont_points_g2[2].to_projective();

        Ok(zkey)
    }
}

impl Default for ZKey {
    fn default() -> Self {
        Self::new()
    }
}
