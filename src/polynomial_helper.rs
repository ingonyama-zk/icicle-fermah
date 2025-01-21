use icicle_bn254::curve::ScalarField;
use icicle_core::ntt::{get_root_of_unity, initialize_domain, ntt_inplace, NTTConfig, NTTDir, NTTInitDomainConfig};
use icicle_runtime::{
    memory::{DeviceSlice, HostOrDeviceSlice},
    stream::IcicleStream,
};

pub fn multiple_fft_on_vectors(
    vec1: &mut DeviceSlice<ScalarField>,
    vec2: &mut DeviceSlice<ScalarField>,
    vec3: &mut DeviceSlice<ScalarField>,
    inverse: bool,
) {
    let domain: ScalarField = get_root_of_unity((vec1.len()) as u64);
    let cfg = NTTInitDomainConfig::default();
    initialize_domain(domain, &cfg).unwrap();

    let dir = if inverse {
        NTTDir::kInverse
    } else {
        NTTDir::kForward
    };

    let mut cfg1 = NTTConfig::<ScalarField>::default();
    let mut stream1 = IcicleStream::create().unwrap();
    cfg1.stream_handle = (&stream1).into();
    cfg1.is_async = true;
    cfg1.are_inputs_on_device = true;
    cfg1.are_outputs_on_device = true;

    let mut cfg2 = NTTConfig::<ScalarField>::default();
    let mut stream2 = IcicleStream::create().unwrap();
    cfg2.stream_handle = (&stream2).into();
    cfg2.is_async = true;
    cfg2.are_inputs_on_device = true;
    cfg2.are_outputs_on_device = true;

    let mut cfg3 = NTTConfig::<ScalarField>::default();
    let mut stream3 = IcicleStream::create().unwrap();  
    cfg3.stream_handle = (&stream3).into();
    cfg3.is_async = true;
    cfg3.are_inputs_on_device = true;
    cfg3.are_outputs_on_device = true;

    ntt_inplace(vec1, dir, &cfg1).unwrap();
    ntt_inplace(vec2, dir, &cfg2).unwrap();
    ntt_inplace(vec3, dir, &cfg3).unwrap();

    stream1.synchronize().unwrap();
    stream2.synchronize().unwrap();
    stream3.synchronize().unwrap();

    stream1.destroy().unwrap();
    stream2.destroy().unwrap();
    stream3.destroy().unwrap();

}
