use lazy_static::lazy_static;

lazy_static! {
    pub static ref BYTE_ARRAY_MATRIX: Vec<Vec<u8>> = vec![
        vec![97, 98, 99], // "abc"
        vec![100, 101, 102, 103], // defg
        vec![104, 105, 106, 107, 108], // hijkl
        vec![109, 110, 111, 112], //mnop
    ];
}

pub const NODE_1: &str = "323eab1b555f0c96bd21c3fac23b9d60937f8a4a66688e814a6670b68a79b78a";
pub const NODE_2: &str = "a2f82a15807e8567cac3b14ce68c01f268a0beb4f4094ae3dffa077c5e1f39fb";
pub const NODE_3: &str = "a5c2c204110ef9021125b05624cd732ca34c8e84afa21e91bba82bbd8c7833fb";
pub const NODE_4: &str = "f1afc31479522d6cff1ed068f93998f05a8cd3b22f5c37d7f307084f62d1d270"; // mnop
pub const NODE_5: &str = "fa3ba64f2053ed06fc34ef5d5888983ca6ee22c7bd7d3d67d48b3faf8eac3a89"; // hijkl
pub const NODE_6: &str = "4c8a43980498636e9c1d1595fa5d115af7937c2422dfe68a2520a52b7a5fb4de"; // defg
pub const NODE_7: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"; // abc
pub const NODE_8: &str = "0e99c08a90f789d5104b719807b5516cbb4b1d028855675ad4be4e79c89ffa7a"; // H(hijk + hijk)
pub const NODE_9: &str = "a5e44df83b56c676fb3523fd3870267813b73c12e381540ad0846df6ba9819b5"; // H(NODE_8 + NODE_2)
