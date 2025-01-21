mod cache;
mod conversions;
mod cuda_helpers;
mod file_wrapper;
mod polynomial_helper;
mod proof_helper;
mod wtsn;
mod zkey;

use crate::conversions::u8_to_scalar;
use cache::{CacheManager, ZKeyCache};
use conversions::{serialize_g1_affine, serialize_g2_affine};
use file_wrapper::FileWrapper;
use icicle_bn254::curve::{G1Projective, G2Projective, ScalarField};
use icicle_core::{
    traits::FieldImpl, 
    vec_ops::{mul_scalars, sub_scalars, VecOpsConfig},
};
use icicle_runtime::{memory::{DeviceVec, HostOrDeviceSlice, HostSlice}, stream::IcicleStream};
use num_bigint::BigUint;
use proof_helper::{construct_r1cs, helper_g1, helper_g2};
use serde::Serialize;
use serde_json::Value;
use std::{io::{self, BufWriter}, time::Instant};
use std::{
    fs::File,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
#[cfg(not(feature = "no-randomness"))]
use icicle_bn254::curve::ScalarCfg;
#[cfg(not(feature = "no-randomness"))]
use icicle_core::traits::GenerateRandom;

enum Groth16 {
    Prove {
        witness: String,
        zkey: String,
        proof: String,
        public: String,
        r: Option<String>,
        s: Option<String>
    },
}

impl Groth16 {
    fn parse_command(command: &str) -> Option<Self> {
        let mut parts = command.split_whitespace();

        let command_type = parts.next()?;
        if command_type != "prove" {
            eprintln!("Unknown command: {}", command_type);
            return None;
        }

        let mut witness = "./witness.wtns".to_string();
        let mut zkey = "./circuit.zkey".to_string();
        let mut proof = "./proof.json".to_string();
        let mut public = "./public.json".to_string();
        let mut r = Option::None;
        let mut s = Option::None;

        while let Some(arg) = parts.next() {
            match arg {
                "--witness" => {
                    if let Some(val) = parts.next() {
                        witness = val.to_string();
                    }
                }
                "--zkey" => {
                    if let Some(val) = parts.next() {
                        zkey = val.to_string();
                    }
                }
                "--proof" => {
                    if let Some(val) = parts.next() {
                        proof = val.to_string();
                    }
                }
                "--public" => {
                    if let Some(val) = parts.next() {
                        public = val.to_string();
                    }
                }
                "--r" => {
                    if let Some(val) = parts.next() {
                        r = Option::Some(val.to_string());
                    }
                }
                "--s" => {
                    if let Some(val) = parts.next() {
                        s = Option::Some(val.to_string());
                    }
                }
                _ => {
                    eprintln!("Unknown argument: {}", arg);
                }
            }
        }

        Some(Groth16::Prove {
            witness,
            zkey,
            proof,
            public,
            r,
            s
        })
    }
}
#[derive(Serialize)]
struct Proof {
    pi_a: Vec<String>,
    pi_b: Vec<Vec<String>>,
    pi_c: Vec<String>,
    protocol: String,
    curve: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut cache_manager = CacheManager::new();

    loop {
        print!("> ");
        io::stdout().flush()?;
        input.clear();
        handle.read_line(&mut input)?;

        let command = input.trim();

        if command.is_empty() {
            continue;
        }

        if command.eq_ignore_ascii_case("exit") {
            break;
        }

        let args = Groth16::parse_command(command).unwrap();

        match args {
            Groth16::Prove {
                witness,
                zkey,
                proof,
                public,
                r,
                s
            } => {
                let start_all = Instant::now();
                println!("witness: {:?}", witness);
                println!("zkey: {:?}", zkey);
                println!("proof: {:?}", proof);
                println!("public: {:?}", public);

                let mut cache_guard = &mut cache_manager.cache;

                if !cache_guard.contains_key(&zkey) {                
                    let computed_cache = cache_manager.get_or_compute(&zkey).unwrap();
                
                    cache_guard = &mut cache_manager.cache;
                    cache_guard.insert(zkey.clone(), computed_cache);
                }
                
                let mut zkey_cache = cache_guard.get_mut(&zkey).unwrap();
                
                let (proof_data, public_signals) =
                    groth16_prove(witness, &mut zkey_cache, r, s).unwrap();

                save_json_file(&proof, &proof_data)?;
                save_json_file(&public, &public_signals)?;

                println!("command completed in {:?}", start_all.elapsed());

                println!("COMMAND_COMPLETED");
            }
        }
    }

    println!("Exiting CLI worker...");
    Ok(())
}

fn groth16_prove(
    witness: String,
    zkey_cache: &mut ZKeyCache,
    r: Option<String>,
    s: Option<String>
) -> Result<(Value, Value), Box<dyn std::error::Error>> {
    let (fd_wtns, sections_wtns) = FileWrapper::read_bin_file(&witness, "wtns", 2).unwrap();

    let mut wtns_file = FileWrapper {
        file: fd_wtns,
        reading_section: None,
        file_name: PathBuf::from("path_wtns"),
    };

    let wtns = wtns_file.read_wtns_header(&sections_wtns[..]).unwrap();

    let zkey = zkey_cache.zkey.clone();

    if !ScalarField::eq(&zkey.r, &wtns.q) {
        panic!("Curve of the witness does not match the curve of the proving key");
    }

    if wtns.n_witness != zkey.n_vars {
        panic!(
            "Invalid witness length. Circuit: {}, witness: {}",
            zkey.n_vars, wtns.n_witness
        );
    }

    // barret form
    let buff_witness = wtns_file.read_section(&sections_wtns[..], 2).unwrap();

    // barret form
    let (a_vec, b_vec, c_vec) = construct_r1cs(
        &zkey,
        &buff_witness,
        wtns.n8 as usize,
        zkey_cache,
    )
    .unwrap();

    let no_of_coef = a_vec.len();

    let cfg: VecOpsConfig = VecOpsConfig::default();

    let mut res_sub = DeviceVec::device_malloc(no_of_coef).unwrap();
    let mut res_mul = DeviceVec::device_malloc(no_of_coef).unwrap();

    // L * R
    mul_scalars(&a_vec[..], &b_vec[..], &mut res_mul[..], &cfg).unwrap();

    // L * R - O
    sub_scalars(&res_mul[..], &c_vec[..], &mut res_sub[..], &cfg).unwrap();

    // A, B, C
    let points_a = &zkey_cache.points_a;
    let points_b1 = &zkey_cache.points_b1;
    let points_b = &zkey_cache.points_b;
    let points_c = &zkey_cache.points_c;
    let points_h = &zkey_cache.points_h;

    let mut stream_g1 = IcicleStream::create().unwrap();
    let mut stream_g2 = IcicleStream::create().unwrap();

    let scalars = u8_to_scalar(&buff_witness);
    let scalars = HostSlice::from_slice(&scalars[..]);
    let mut d_scalars = DeviceVec::device_malloc(scalars.len()).unwrap();
    d_scalars.copy_from_host_async(scalars, &stream_g1).unwrap();

    let c_scalars = u8_to_scalar(&buff_witness[((zkey.n_public + 1) * 32) as usize..]);
    let c_scalars = HostSlice::from_slice(&c_scalars[..]);
    let mut d_c_scalars = DeviceVec::device_malloc(c_scalars.len()).unwrap();
    d_c_scalars.copy_from_host_async(c_scalars, &stream_g2).unwrap();

    let commitment_a = helper_g1(&d_scalars[..], &points_a[..], true, false, &stream_g1);
    let commitment_b1 = helper_g1(&d_scalars[..], &points_b1[..], true, false, &stream_g1);
    let commitment_c = helper_g1(&d_c_scalars[..], &points_c, true, false, &stream_g1);
    let commitment_h = helper_g1(&res_sub[..], &points_h, true, false, &stream_g1);
    
    stream_g1.synchronize().unwrap();

    let commitment_b = helper_g2(&d_scalars[..], &points_b[..], true, false, &stream_g2);
    stream_g2.synchronize().unwrap();

    stream_g1.destroy().unwrap();
    stream_g2.destroy().unwrap();

    let mut pi_a = [G1Projective::zero(); 1];
    let mut pi_b1 = [G1Projective::zero(); 1];
    let mut pi_b = [G2Projective::zero(); 1];
    let mut pi_c = [G1Projective::zero(); 1];
    let mut pi_h = [G1Projective::zero(); 1];

    commitment_a
        .copy_to_host(HostSlice::from_mut_slice(&mut pi_a[..]))
        .unwrap();

    commitment_b1
        .copy_to_host(HostSlice::from_mut_slice(&mut pi_b1[..]))
        .unwrap();

    commitment_b
        .copy_to_host(HostSlice::from_mut_slice(&mut pi_b[..]))
        .unwrap();
    
    commitment_c
        .copy_to_host(HostSlice::from_mut_slice(&mut pi_c[..]))
        .unwrap();

    commitment_h
        .copy_to_host(HostSlice::from_mut_slice(&mut pi_h[..]))
        .unwrap();

    #[cfg(not(feature = "no-randomness"))]
    let (pi_a, pi_b, pi_c) = {
        let rs = ScalarCfg::generate_random(2);
        let r = r
            .as_ref()
            .map(|v| ScalarField::from_hex(v))
            .unwrap_or(rs[0]);
        let s = s
            .as_ref()
            .map(|v| ScalarField::from_hex(v))
            .unwrap_or(rs[1]);

        let pi_a = pi_a[0] + zkey.vk_alpha_1 + zkey.vk_delta_1 * r;
        let pi_b = pi_b[0] + zkey.vk_beta_2 + zkey.vk_delta_2 * s;
        let pi_b1 = pi_b1[0] + zkey.vk_beta_1 + zkey.vk_delta_1 * s;
        let pi_c = pi_c[0] + pi_h[0] + pi_a * s + pi_b1 * r - zkey.vk_delta_1 * r * s;

        (pi_a, pi_b, pi_c)
    };
    #[cfg(feature = "no-randomness")]
    let (pi_a, pi_b, pi_c) = {
        let pi_a = pi_a[0] + zkey.vk_alpha_1 + zkey.vk_delta_1;
        let pi_b = pi_b[0] + zkey.vk_beta_2 + zkey.vk_delta_2;
        let pi_b1 = pi_b1[0] + zkey.vk_beta_1 + zkey.vk_delta_1;
        let pi_c = pi_c[0] + pi_h[0] + pi_a + pi_b1 - zkey.vk_delta_1;

        (pi_a, pi_b, pi_c)
    };

    let mut public_signals = Vec::with_capacity(zkey.n_public as usize);
    let field_size = ScalarField::zero().to_bytes_le().len();

    for i in 1..=zkey.n_public {
        let start = (i as usize) * field_size;
        let end = start + field_size;
        let b = &buff_witness[start..end];
        let signal = ScalarField::from_bytes_le(b);
        public_signals.push(signal);
    }

    let proof = Proof {
        pi_a: serialize_g1_affine(pi_a.into()),
        pi_b: serialize_g2_affine(pi_b.into()),
        pi_c: serialize_g1_affine(pi_c.into()),
        protocol: "groth16".to_string(),
        curve: "bn128".to_string(),
    };

    let public_signals: Vec<String> = public_signals
        .into_iter()
        .map(|sig| {
            let scalar_bytes: BigUint = BigUint::from_bytes_le(&sig.to_bytes_le()[..]);

            scalar_bytes.to_str_radix(10)
        })
        .collect();

    Ok((serde_json::json!(proof), serde_json::json!(public_signals)))
}

fn save_json_file(file_path: &str, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, &data)?;
    writer.flush()?;
    Ok(())
}