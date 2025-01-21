use icicle_bn254::curve::ScalarField;

#[derive(Clone, Debug)]
pub struct Wtsn {
    pub n8: u32,
    pub q: ScalarField,
    pub n_witness: u32,
}