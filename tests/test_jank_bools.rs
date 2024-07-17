use itertools::{Itertools, izip};
use phantom_zone::*;
use rand::{thread_rng, RngCore};

fn function (a_meet: bool, b_meet: bool, a_free: &Vec<bool>, b_free: &Vec<bool>) -> Vec<bool> {
    let mut output = Vec::new();
    let meet = a_meet & b_meet;
    for (u, v) in izip!(a_free, b_free) {
        output.push(meet & (u & v));
    }
    output
}

fn function_fhe (
    a_meet: &FheBool, 
    b_meet: &FheBool, 
    a_free: &Vec<FheBool>,
    b_free: &Vec<FheBool>, 
) -> Vec<FheBool> {
    let mut output = Vec::new();
    let meet = a_meet & b_meet;
    for (u, v) in izip!(a_free, b_free) {
        output.push(&meet & &(u & v))
    }
    output
}

fn vb_to_vu8 (vb: &Vec<bool>) -> Vec<u8> {
    vb.iter().map(|b| if *b {1} else {0}).collect_vec()
}

fn vfheu8_to_vfhebool (v: &Vec<FheUint8>, zero: &FheUint8) -> Vec<FheBool> {
    v.iter().map(|b| b.neq(&zero)).collect_vec()
}

#[test]
fn main() {
    let now = std::time::Instant::now();

    set_parameter_set(ParameterSelector::NonInteractiveLTE4Party);

    // set application's common reference seed
    let mut seed = [0u8; 32];
    thread_rng().fill_bytes(&mut seed);
    set_common_reference_seed(seed);

    let no_of_parties = 3;

    let cks = (0..no_of_parties).map(|_| gen_client_key()).collect_vec();

    println!("Finished generating client seed! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let c0_free: Vec<bool> = vec![true, true, true]; // c0 free all day
    let c0_meet: Vec<bool> = vec![true, false]; // c0 wants to meet c1 but not c2

    let c1_free: Vec<bool> = vec![false, true, true];
    let c1_meet: Vec<bool> = vec![false, true]; 

    let c2_free: Vec<bool> = vec![true, true, false]; 
    let c2_meet: Vec<bool> = vec![true, true];

    let c0_zero: Vec<bool> = vec![false];

    let c0_free_enc = cks[0].encrypt(vb_to_vu8(&c0_free).as_slice());
    let c0_meet_enc = cks[0].encrypt(vb_to_vu8(&c0_meet).as_slice());
    let c0_zero_enc  = cks[0].encrypt(vb_to_vu8(&c0_zero).as_slice());

    let c1_free_enc = cks[1].encrypt(vb_to_vu8(&c1_free).as_slice());
    let c1_meet_enc = cks[1].encrypt(vb_to_vu8(&c1_meet).as_slice());

    let c2_free_enc = cks[2].encrypt(vb_to_vu8(&c2_free).as_slice());
    let c2_meet_enc = cks[2].encrypt(vb_to_vu8(&c2_meet).as_slice());

    println!("Finished encrypting! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let server_key_shares = cks
        .iter()
        .enumerate()
        .map(|(id, k)| gen_server_key_share(id, no_of_parties, k))
        .collect_vec();

    let server_key = aggregate_server_key_shares(&server_key_shares);
    server_key.set_server_key();

    let ct_c0_free = c0_free_enc.unseed::<Vec<Vec<u64>>>().key_switch(0).extract_all();
    let ct_c0_meet = c0_meet_enc.unseed::<Vec<Vec<u64>>>().key_switch(0).extract_all();
    let ct_c0_zero = c0_zero_enc.unseed::<Vec<Vec<u64>>>().key_switch(0).extract_at(0);
    let ct_c1_free = c1_free_enc.unseed::<Vec<Vec<u64>>>().key_switch(1).extract_all();
    let ct_c1_meet = c1_meet_enc.unseed::<Vec<Vec<u64>>>().key_switch(1).extract_all();
    let _ct_c2_free = c2_free_enc.unseed::<Vec<Vec<u64>>>().key_switch(2).extract_all();
    let _ct_c2_meet = c2_meet_enc.unseed::<Vec<Vec<u64>>>().key_switch(2).extract_all();

    println!("Finished key switching to server key! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let ct_c0_free = vfheu8_to_vfhebool(&ct_c0_free, &ct_c0_zero);
    let ct_c1_free = vfheu8_to_vfhebool(&ct_c1_free, &ct_c0_zero);
    let ct_c0_meet = vfheu8_to_vfhebool(&ct_c0_meet, &ct_c0_zero);
    let ct_c1_meet = vfheu8_to_vfhebool(&ct_c1_meet, &ct_c0_zero);

    println!("Finished converting to FHEBool! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();
    let ct_out_f1 = function_fhe(
        &ct_c0_meet[0], 
        &ct_c1_meet[0], 
        &ct_c0_free, 
        &ct_c1_free,
    );
    println!("Function1 FHE evaluation time: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let decryption_shares = ct_out_f1
        .iter()
        .map(|b| cks
             .iter()
             .map(|k| k.gen_decryption_share(b))
            .collect_vec()
            )
        .collect_vec();

    println!("Finished generating decryption shares! Time taken: {:?}", now.elapsed());

    let out_f1 = izip!(ct_out_f1, decryption_shares)
        .map(|(b, s)| cks[0].aggregate_decryption_shares(&b, &s))
        .collect_vec();

    let want_out_f1 = function(c0_meet[0], c1_meet[0], &c0_free, &c1_free);
    assert_eq!(out_f1, want_out_f1);
}