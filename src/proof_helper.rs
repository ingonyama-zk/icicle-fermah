use crate::{
    cache::ZKeyCache, cuda_helpers::scalars_from_mont, polynomial_helper::multiple_fft_on_vectors,
};
use icicle_bn254::curve::{CurveCfg, G1Projective, G2CurveCfg, G2Projective, ScalarField};
use icicle_core::{
    curve::Affine, msm::{msm, MSMConfig}, traits::FieldImpl, vec_ops::{mul_scalars, VecOpsConfig}
};
use icicle_runtime::{memory::{DeviceSlice, DeviceVec, HostOrDeviceSlice, HostSlice}, stream::IcicleStream};

use crate::zkey::ZKey;

use rayon::prelude::*;

pub fn helper_g1(
    scalars: &DeviceSlice<ScalarField>,
    points: &DeviceSlice<Affine<CurveCfg>>,
    points_mont: bool,
    scalars_mont: bool,
    stream: &IcicleStream
) -> DeviceVec<G1Projective> {
    let mut msm_result = DeviceVec::<G1Projective>::device_malloc(1).unwrap();
    let mut msm_config = MSMConfig::default();
    msm_config.are_bases_montgomery_form = points_mont;
    msm_config.are_scalars_montgomery_form = scalars_mont;
    msm_config.stream_handle = stream.into();
    msm_config.is_async = true;

    msm(scalars, points, &msm_config, &mut msm_result[..]).unwrap();

    msm_result
}

pub fn helper_g2(
    scalars: &DeviceSlice<ScalarField>,
    points: &DeviceSlice<Affine<G2CurveCfg>>,
    points_mont: bool,
    scalars_mont: bool,
    stream: &IcicleStream
) -> DeviceVec<G2Projective> {
    let mut msm_result = DeviceVec::<G2Projective>::device_malloc(1).unwrap();
    let mut msm_config = MSMConfig::default();
    msm_config.are_bases_montgomery_form = points_mont;
    msm_config.are_scalars_montgomery_form = scalars_mont;
    msm_config.stream_handle = stream.into();
    msm_config.is_async = true;

    msm(scalars, points, &msm_config, &mut msm_result[..]).unwrap();

    msm_result
}

pub fn construct_r1cs(
    zkey: &ZKey,
    witness: &[u8],
    n8: usize,
    zkey_cache: &mut ZKeyCache,
) -> Result<
    (
        DeviceVec<ScalarField>,
        DeviceVec<ScalarField>,
        DeviceVec<ScalarField>,
    ),
    Box<dyn std::error::Error>,
> {
    let coeffs = &zkey_cache.buff_coeffs[..];

    let s_coef = 4 * 3 + zkey.n8r as usize;
    let n_coef = (coeffs.len() - 4) / s_coef;

    let mut out_buff_a = vec![ScalarField::zero(); zkey.domain_size as usize];
    let mut out_buff_b = vec![ScalarField::zero(); zkey.domain_size as usize];

    let slice = &mut zkey_cache.slice;
    let s_values = &zkey_cache.s_values;
    let c_values = &zkey_cache.c_values;
    let m_values = &zkey_cache.m_values;

    slice[n_coef..].par_iter_mut().enumerate().for_each(|(i, slice_elem)| {
        let s = s_values[i];
        *slice_elem = ScalarField::from_bytes_le(&witness[s * n8..s * n8 + n8]);
    });
    
    let slice = HostSlice::from_mut_slice(slice);

    let mut res = vec![ScalarField::zero(); n_coef];
    let res = HostSlice::from_mut_slice(&mut res);

    let mut stream = IcicleStream::create().unwrap();
    
    let mut d_slice = DeviceVec::device_malloc_async(slice.len(), &stream).unwrap();
    d_slice.copy_from_host_async(slice, &stream).unwrap();
    scalars_from_mont(&mut d_slice[..], &stream);

    let mut cfg: VecOpsConfig = VecOpsConfig::default();
    cfg.stream_handle = *stream;

    mul_scalars(&d_slice[0..n_coef], &d_slice[n_coef..], res, &cfg).unwrap();

    stream.synchronize().unwrap();

    let zero_scalar = ScalarField::zero();

    for i in 0..n_coef {
        let c = c_values[i];

        if m_values[i] {
            if out_buff_a[c].eq(&zero_scalar) {
                out_buff_a[c] = res[i];
            } else if !res[i].eq(&zero_scalar) {
                out_buff_a[c] = out_buff_a[c] + res[i];
            }
        } else {
            if out_buff_b[c].eq(&zero_scalar) {
                out_buff_b[c] = res[i];
            } else if !res[i].eq(&zero_scalar) {
                out_buff_b[c] = out_buff_b[c] + res[i];
            }
        }
    }

    let nof_coef = out_buff_a.len();

    let mut d_vec_a = DeviceVec::device_malloc(nof_coef).unwrap();
    let mut d_vec_b = DeviceVec::device_malloc(nof_coef).unwrap();
    let mut d_vec_c = DeviceVec::device_malloc(nof_coef).unwrap();

    d_vec_a
        .copy_from_host_async(HostSlice::from_slice(&out_buff_a), &stream)
        .unwrap();
    d_vec_b
        .copy_from_host_async(HostSlice::from_slice(&out_buff_b), &stream)
        .unwrap();

    mul_scalars(&d_vec_a[..], &d_vec_b[..], &mut d_vec_c[..], &cfg).unwrap();

    stream.synchronize().unwrap();

    multiple_fft_on_vectors(&mut d_vec_a, &mut d_vec_b, &mut d_vec_c, true);

    let keys = &zkey_cache.precomputed_keys;

    let mut d_res_a = DeviceVec::device_malloc(nof_coef).unwrap();
    let mut d_res_b = DeviceVec::device_malloc(nof_coef).unwrap();
    let mut d_res_c = DeviceVec::device_malloc(nof_coef).unwrap();

    mul_scalars(&d_vec_a[..], &keys[..], &mut d_res_a[..], &cfg).unwrap();
    mul_scalars(&d_vec_b[..], &keys[..], &mut d_res_b[..], &cfg).unwrap();
    mul_scalars(&d_vec_c[..], &keys[..], &mut d_res_c[..], &cfg).unwrap();

    stream.synchronize().unwrap();

    multiple_fft_on_vectors(&mut d_res_a, &mut d_res_b, &mut d_res_c, false);

    stream.destroy().unwrap();

    Ok((d_res_a, d_res_b, d_res_c))
}