use std::{env, fmt, io};
use std::{
    path::{Path, PathBuf},
    io::{Read, Write},
    collections::HashMap,
};
use std::sync::Arc;
use icicle_bn254::curve::{CurveCfg, G2CurveCfg, ScalarField};
use icicle_core::curve::Affine;
use icicle_core::traits::FieldImpl;
use icicle_runtime::memory::{DeviceVec, HostSlice};
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fs::File;
use clap::Parser;

use crate::conversions::{u8_to_g1_affine, u8_to_g2_affine};
use crate::file_wrapper::FileWrapper;
use crate::zkey::ZKey;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "CPU")]
    device: String,
}

const W: [&str; 30] = [
    "0x0000000000000000000000000000000000000000000000000000000000000001",
    "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000000",
    "0x30644e72e131a029048b6e193fd841045cea24f6fd736bec231204708f703636",
    "0x2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e80",
    "0x21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b",
    "0x09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d0",
    "0x1418144d5b080fcac24cdb7649bdadf246a6cb2426e324bedb94fb05118f023a",
    "0x16e73dfdad310991df5ce19ce85943e01dcb5564b6f24c799d0e470cba9d1811",
    "0x07b0c561a6148404f086204a9f36ffb0617942546750f230c893619174a57a76",
    "0x0f1ded1ef6e72f5bffc02c0edd9b0675e8302a41fc782d75893a7fa1470157ce",
    "0x06fd19c17017a420ebbebc2bb08771e339ba79c0a8d2d7ab11f995e1bc2e5912",
    "0x027a358499c5042bb4027fd7a5355d71b8c12c177494f0cad00a58f9769a2ee2",
    "0x0931d596de2fd10f01ddd073fd5a90a976f169c76f039bb91c4775720042d43a",
    "0x006fab49b869ae62001deac878b2667bd31bf3e28e3a2d764aa49b8d9bbdd310",
    "0x2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e85",
    "0x2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb",
    "0x00eeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b7",
    "0x1bf82deba7d74902c3708cc6e70e61f30512eca95655210e276e5858ce8f58e5",
    "0x19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e0",
    "0x2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd",
    "0x26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da953337857",
    "0x1ded8980ae2bdd1a4222150e8598fc8c58f50577ca5a5ce3b2c87885fcd0b523",
    "0x1ad92f46b1f8d9a7cda0ceb68be08215ec1a1f05359eebbba76dde56a219447e",
    "0x0210fe635ab4c74d6b7bcf70bc23a1395680c64022dd991fb54d4506ab80c59d",
    "0x0c9fabc7845d50d2852e2a0371c6441f145e0db82e8326961c25f1e3e32b045b",
    "0x2a734ebb326341efa19b0361d9130cd47b26b7488dc6d26eeccd4f3eb878331a",
    "0x1067569af1ff73b20113eff9b8d89d4a605b52b63d68f9ae1c79bd572f4e9212",
    "0x049ae702b363ebe85f256a9f6dc6e364b4823532f6437da2034afc4580928c44",
    "0x2a3c09f0a58a7e8500e0a7eb8ef62abc402d111e41112ed49bd61b6e725b19f0",
    "0x2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd",
];

#[derive(Clone)]
pub struct ZKeyCache {
    pub buff_coeffs: Arc<Vec<u8>>,
    pub slice: Vec<ScalarField>,
    pub s_values: Vec<usize>,
    pub c_values: Vec<usize>,
    pub m_values: Vec<bool>,
    pub points_a: Arc<DeviceVec<Affine<CurveCfg>>>,
    pub points_b1: Arc<DeviceVec<Affine<CurveCfg>>>,
    pub points_b: Arc<DeviceVec<Affine<G2CurveCfg>>>,
    pub points_h: Arc<DeviceVec<Affine<CurveCfg>>>,
    pub points_c: Arc<DeviceVec<Affine<CurveCfg>>>,
    pub precomputed_keys: Arc<DeviceVec<ScalarField>>,
    pub zkey: ZKey,
}

struct PreComputedData {
    keys: Vec<ScalarField>,
}

impl Serialize for PreComputedData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.keys.len()))?;

        for key in &self.keys {
            let bytes = key.to_bytes_le();
            seq.serialize_element(&bytes)?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for PreComputedData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PreComputedDataVisitor;

        impl<'de> Visitor<'de> for PreComputedDataVisitor {
            type Value = PreComputedData;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of byte arrays representing ScalarFields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut keys = Vec::new();

                while let Some(bytes) = seq.next_element::<Vec<u8>>()? {
                    let scalar_field = ScalarField::from_bytes_le(&bytes);
                    keys.push(scalar_field);
                }

                Ok(PreComputedData { keys })
            }
        }

        deserializer.deserialize_seq(PreComputedDataVisitor)
    }
}

fn pre_compute_keys(
    mut key: ScalarField,
    inc: ScalarField,
    size: usize,
) -> io::Result<Vec<ScalarField>> {
    let mut exe_dir = env::current_exe()?;
    exe_dir.pop();
    let mut file_path = exe_dir;
    file_path.push(format!("precomputed_{}_{}.bin", size, inc));

    if Path::new(&file_path).exists() {
        let keys = load_from_binary_file(&file_path)?;
        return Ok(keys.keys);
    }

    let mut keys = Vec::with_capacity(size);
    for _ in 0..size {
        keys.push(key);
        key = key * inc;
    }

    let keys = PreComputedData { keys };
    save_to_binary_file(&keys, &file_path)?;

    Ok(keys.keys)
}

fn save_to_binary_file(data: &PreComputedData, file_path: &Path) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    let encoded: Vec<u8> = bincode::serialize(data).expect("Failed to serialize data");
    file.write_all(&encoded)?;
    Ok(())
}

fn load_from_binary_file(file_path: &Path) -> io::Result<PreComputedData> {
    let mut file = File::open(file_path)?;
    let mut encoded = Vec::new();
    file.read_to_end(&mut encoded)?;
    let data: PreComputedData = bincode::deserialize(&encoded).expect("Failed to deserialize data");
    Ok(data)
}

fn try_load_and_set_backend_device(device_type: &str) {
    if device_type != "CPU" {
        icicle_runtime::runtime::load_backend_from_env_or_default().unwrap();
    }
    println!("Setting device {}", device_type);
    let device = icicle_runtime::Device::new(device_type, 0 /* =device_id*/);
    icicle_runtime::set_device(&device).unwrap();
}

pub struct CacheManager {
    pub cache: HashMap<String, ZKeyCache>,
}

impl CacheManager {
    pub fn new() -> Self {
        CacheManager {
            cache: HashMap::new(),
        }
    }

    pub fn get_or_compute(&mut self, zkey_path: &str) -> Result<ZKeyCache, Box<dyn std::error::Error>> {
        let cache_guard = &mut self.cache;

        if let Some(existing_cache) = cache_guard.get(zkey_path) {
            return Ok(existing_cache.clone());
        }

        let args = Args::parse();
        try_load_and_set_backend_device(&args.device);

        let (fd_zkey, sections_zkey) = FileWrapper::read_bin_file(zkey_path, "zkey", 2).unwrap();
    
        let mut zkey_file = FileWrapper {
            file: fd_zkey,
            reading_section: None,
            file_name: PathBuf::from("path_zkey"),
        };
    
        let zkey = zkey_file.read_zkey_header(&sections_zkey[..]).unwrap();

        let buff_coeffs = zkey_file.read_section(&sections_zkey[..], 4).unwrap();

        let s_coef = 4 * 3 + zkey.n8r as usize;
        let n_coef = (buff_coeffs.len() - 4) / s_coef;

        let mut slice = vec![ScalarField::zero(); n_coef * 2];
        let mut s_values = vec![0usize; n_coef];
        let mut c_values = vec![0usize; n_coef];
        let mut m_values = vec![false; n_coef];
        let n8 = 32;

        for i in 0..n_coef {
            let start: usize = 4 + i * s_coef;
            let buff_coef = &buff_coeffs[start..start + s_coef];
            let s = u32::from_le_bytes([buff_coef[8], buff_coef[9], buff_coef[10], buff_coef[11]]) as usize;
            let c = u32::from_le_bytes([buff_coef[4], buff_coef[5], buff_coef[6], buff_coef[7]]) as usize;
            let m = buff_coef[0];
            let coef = ScalarField::from_bytes_le(&buff_coef[12..12 + n8]);
            s_values[i] = s;
            c_values[i] = c;
            if m == 1 {
                m_values[i] = true;
            }
            
            slice[i] = coef;
        }

        let power = zkey.power + 1;
        let inc = ScalarField::from_hex(W[power as usize]);
        let keys = pre_compute_keys(ScalarField::one(), inc, zkey.domain_size as usize).unwrap();
        let mut d_keys = DeviceVec::device_malloc(zkey.domain_size as usize).unwrap();
        d_keys.copy_from_host(HostSlice::from_slice(&keys)).unwrap();

        let points_a = zkey_file.read_section(&sections_zkey, 5).unwrap();
        let points_b1 = zkey_file.read_section(&sections_zkey, 6).unwrap();
        let points_b = zkey_file.read_section(&sections_zkey, 7).unwrap();
        let points_c = zkey_file.read_section(&sections_zkey, 8).unwrap();
        let points_h = zkey_file.read_section(&sections_zkey, 9).unwrap();

        let points_a = u8_to_g1_affine(&points_a);
        let points_b1 = u8_to_g1_affine(&points_b1);
        let points_b = u8_to_g2_affine(&points_b);
        let points_c = u8_to_g1_affine(&points_c);
        let points_h = u8_to_g1_affine(&points_h);
        
        let mut d_points_a: DeviceVec<Affine<CurveCfg>> = DeviceVec::device_malloc(points_a.len()).unwrap();
        let mut d_points_b1: DeviceVec<Affine<CurveCfg>> = DeviceVec::device_malloc(points_b1.len()).unwrap();
        let mut d_points_b: DeviceVec<Affine<G2CurveCfg>> = DeviceVec::device_malloc(points_b.len()).unwrap();
        let mut d_points_c: DeviceVec<Affine<CurveCfg>> = DeviceVec::device_malloc(points_c.len()).unwrap();
        let mut d_points_h: DeviceVec<Affine<CurveCfg>> = DeviceVec::device_malloc(points_h.len()).unwrap();

        let points_a = HostSlice::from_slice(&points_a);
        let points_b1 = HostSlice::from_slice(&points_b1);
        let points_b = HostSlice::from_slice(&points_b);
        let points_c = HostSlice::from_slice(&points_c);
        let points_h = HostSlice::from_slice(&points_h);

        d_points_a.copy_from_host(points_a).unwrap();
        d_points_b1.copy_from_host(points_b1).unwrap();
        d_points_b.copy_from_host(points_b).unwrap();
        d_points_c.copy_from_host(points_c).unwrap();
        d_points_h.copy_from_host(points_h).unwrap();

        let cache_entry = ZKeyCache {
            buff_coeffs: Arc::new(buff_coeffs),
            slice,
            s_values,
            c_values,
            m_values,
            zkey,
            points_a: Arc::new(d_points_a),
            points_b1: Arc::new(d_points_b1),
            points_b: Arc::new(d_points_b),
            points_c: Arc::new(d_points_c),
            points_h: Arc::new(d_points_h),
            precomputed_keys: Arc::new(d_keys),
        };

        cache_guard.insert(zkey_path.to_string(), cache_entry.clone());

        Ok(cache_entry)
    }
}