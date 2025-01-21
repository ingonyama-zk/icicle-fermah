use icicle_bn254::curve::{G1Affine, G2Affine, ScalarField};
use icicle_core::traits::MontgomeryConvertible;
use icicle_runtime::memory::{DeviceSlice, DeviceVec, HostSlice};
use icicle_runtime::stream::IcicleStream;

pub fn from_mont_points_g1(points: &mut [G1Affine]) {
    let mut d_affine = DeviceVec::device_malloc(points.len()).unwrap();
    d_affine
        .copy_from_host(HostSlice::from_slice(points))
        .unwrap();

    let mut stream = IcicleStream::create().unwrap();

    G1Affine::from_mont(&mut d_affine, &stream)
        .wrap()
        .unwrap();

    d_affine
        .copy_to_host(HostSlice::from_mut_slice(points))
        .unwrap();

    stream.destroy().unwrap();
}

pub fn from_mont_points_g2(points: &mut [G2Affine]) {
    let mut d_affine = DeviceVec::device_malloc(points.len()).unwrap();
    d_affine
        .copy_from_host(HostSlice::from_slice(points))
        .unwrap();

    let mut stream: IcicleStream = IcicleStream::create().unwrap();

    G2Affine::from_mont(&mut d_affine, &stream)
        .wrap()
        .unwrap();

    d_affine
        .copy_to_host(HostSlice::from_mut_slice(points))
        .unwrap();

    stream.destroy().unwrap();
}

pub fn scalars_from_mont(scalars: &mut DeviceSlice<ScalarField>, stream: &IcicleStream) {
    ScalarField::from_mont(scalars, &stream)
        .wrap()
        .unwrap();
}