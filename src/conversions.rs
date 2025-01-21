use icicle_bn254::curve::{G1Affine, G2Affine, ScalarField};
use icicle_core::traits::FieldImpl;
use num_bigint::BigUint;

pub fn serialize_g1_affine(point: G1Affine) -> Vec<String> {
    let x_bytes = BigUint::from_bytes_le(&point.x.to_bytes_le()[..]);
    let y_bytes = BigUint::from_bytes_le(&point.y.to_bytes_le()[..]);

    vec![
        x_bytes.to_str_radix(10),
        y_bytes.to_str_radix(10),
        "1".to_string(),
    ]
}

pub fn serialize_g2_affine(point: G2Affine) -> Vec<Vec<String>> {
    let x_bytes = point.x.to_bytes_le();
    let x_bytes_1 = BigUint::from_bytes_le(&x_bytes[..32]);
    let x_bytes_2 = BigUint::from_bytes_le(&x_bytes[32..]);

    let y_bytes = point.y.to_bytes_le();
    let y_bytes_1 = BigUint::from_bytes_le(&y_bytes[..32]);
    let y_bytes_2 = BigUint::from_bytes_le(&y_bytes[32..]);

    vec![
        vec![x_bytes_1.to_str_radix(10), x_bytes_2.to_str_radix(10)],
        vec![y_bytes_1.to_str_radix(10), y_bytes_2.to_str_radix(10)],
        vec!["1".to_string(), "0".to_string()],
    ]
}

pub fn u8_to_scalar(data: &[u8]) -> Vec<ScalarField> {
    assert!(
        data.len() % 32 == 0,
        "Data length must be a multiple of 64 bytes"
    );

    let num_scalars = data.len() / 32;
    let mut scalars = Vec::with_capacity(num_scalars);

    for i in 0..num_scalars {
        let start = i * 32;
        let bytes: [u8; 32] = data[start..start + 32]
            .try_into()
            .expect("Slice with incorrect length");

        scalars.push(ScalarField::from_bytes_le(&bytes));
    }

    scalars
}

pub fn u8_to_g1_affine(data: &[u8]) -> Vec<G1Affine> {
    assert!(
        data.len() % 64 == 0,
        "Data length must be a multiple of 64 bytes"
    );

    let num_points = data.len() / 64;
    let mut g1_affines = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let start = i * 64;
        let x_bytes: [u8; 32] = data[start..start + 32]
            .try_into()
            .expect("Slice with incorrect length");
        let y_bytes: [u8; 32] = data[start + 32..start + 64]
            .try_into()
            .expect("Slice with incorrect length");

        let x: [u32; 8] = unsafe { std::mem::transmute(x_bytes) };
        let y: [u32; 8] = unsafe { std::mem::transmute(y_bytes) };

        g1_affines.push(G1Affine::from_limbs(x, y));
    }

    g1_affines
}

pub fn u8_to_g2_affine(data: &[u8]) -> Vec<G2Affine> {
    assert!(
        data.len() % 128 == 0,
        "Data length must be a multiple of 128 bytes"
    );

    let num_points = data.len() / 128;
    let mut g2_affines = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let start = i * 128;
        let x_bytes: [u8; 64] = data[start..start + 64]
            .try_into()
            .expect("Slice with incorrect length");
        let y_bytes: [u8; 64] = data[start + 64..start + 128]
            .try_into()
            .expect("Slice with incorrect length");

        let x: [u32; 16] = unsafe { std::mem::transmute(x_bytes) };
        let y: [u32; 16] = unsafe { std::mem::transmute(y_bytes) };

        g2_affines.push(G2Affine::from_limbs(x, y));
    }

    g2_affines
}