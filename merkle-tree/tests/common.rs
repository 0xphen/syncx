use lazy_static::lazy_static;

lazy_static! {
    pub static ref BYTE_ARRAY_MATRIX: Vec<Vec<u8>> = vec![
        vec![97, 98, 99], // "abc"
        vec![100, 101, 102, 103], // defg
        vec![104, 105, 106, 107, 108], // hijkl
        vec![109, 110, 111, 112], //mnop
    ];
}

pub const H_LALB_LCLD: &str = "5601e76b31b968a1acba255f5e2d6110c4dd185ddf9c1f36143748d18d2272b7";
pub const H_LA_LB: &str = "8fcb2edf18d74dc490f65ffc5588288c02f2505ffef2f7ba08e92e37b39e9c9b";
pub const H_LC_LD: &str = "baf1fc037ab6cff11f3e68689578117d7c810bef083c14e24d791e23cdb075b5";
pub const LA: &str = "4c8a43980498636e9c1d1595fa5d115af7937c2422dfe68a2520a52b7a5fb4de";
pub const LB: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
pub const LC: &str = "f1afc31479522d6cff1ed068f93998f05a8cd3b22f5c37d7f307084f62d1d270";
pub const LD: &str = "fa3ba64f2053ed06fc34ef5d5888983ca6ee22c7bd7d3d67d48b3faf8eac3a89";
pub const H_LD_LD: &str = "0e99c08a90f789d5104b719807b5516cbb4b1d028855675ad4be4e79c89ffa7a";
pub const H_LALB_LDLD: &str = "5ee7f45ac04e272ccf7ac515e8b554dbf7fc0576840b8f9a59c3e417412760ee";
