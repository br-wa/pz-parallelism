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

fn function_fhe (a_meet: &FheBool, b_meet: &FheBool, a_free: &Vec<FheBool>, b_free: &Vec<FheBool>) -> Vec<FheBool> {
    let mut output = Vec::new();
    let meet = a_meet & b_meet;
    for (u, v) in izip!(a_free, b_free) {
        output.push(&meet & &(u & v))
    }
    output
}

#[test]
fn main() {
    set_parameter_set(ParameterSelector::NonInteractiveLTE4Party);

    // set application's common reference seed
    let mut seed = [0u8; 32];
    thread_rng().fill_bytes(&mut seed);
    set_common_reference_seed(seed);

    let no_of_parties = 3;

    let cks = (0..no_of_parties).map(|_| gen_client_key()).collect_vec();

    println!("Finished generating client seed!");

    let c0_free: Vec<bool> = vec![true, true, true]; // c0 free all day
    let c0_meet: Vec<bool> = vec![true, false]; // c0 wants to meet c1 but not c2

    let c1_free: Vec<bool> = vec![false, true, true];
    let c1_meet: Vec<bool> = vec![true, true]; 

    let c2_free: Vec<bool> = vec![true, true, false]; 
    let c2_meet: Vec<bool> = vec![true, true];

    let c0_free_enc: NonInteractiveBatchedFheBools<(Vec<Vec<u64>>, Vec<u64>)> = cks[0].encrypt(c0_free.as_slice());
    let c0_meet_enc = cks[0].encrypt(c0_meet.as_slice());

    let c1_free_enc = cks[1].encrypt(c1_free.as_slice());
    let c1_meet_enc = cks[1].encrypt(c1_meet.as_slice());

    let c2_free_enc = cks[2].encrypt(c2_free.as_slice());
    let c2_meet_enc = cks[2].encrypt(c2_free.as_slice());

    let server_key_shares = cks
        .iter()
        .enumerate()
        .map(|(id, k)| gen_server_key_share(id, no_of_parties, k))
        .collect_vec();

    let server_key = aggregate_server_key_shares(&server_key_shares);
    server_key.set_server_key();

    let ct_c0_free = c0_free_enc.unseed::<Vec<Vec<u64>>>().key_switch(0).extract_all();
    let ct_c0_meet = c0_meet_enc.unseed::<Vec<Vec<u64>>>().key_switch(0).extract_all();
    let ct_c1_free = c1_free_enc.unseed::<Vec<Vec<u64>>>().key_switch(1).extract_all();
    let ct_c1_meet = c1_meet_enc.unseed::<Vec<Vec<u64>>>().key_switch(1).extract_all();
    let ct_c2_free = c2_free_enc.unseed::<Vec<Vec<u64>>>().key_switch(2).extract_all();
    let ct_c2_meet = c2_meet_enc.unseed::<Vec<Vec<u64>>>().key_switch(2).extract_all();

    let now = std::time::Instant::now();
    let ct_out_f1 = function_fhe(&ct_c0_meet[0], &ct_c1_meet[0], &ct_c0_free, &ct_c1_free);
    println!("Function1 FHE evaluation time: {:?}", now.elapsed());

    let decryption_shares = ct_out_f1
        .iter()
        .map(|b| cks
             .iter()
             .map(|k| k.gen_decryption_share(b))
            .collect_vec()
            )
        .collect_vec();

    let out_f1 = izip!(ct_out_f1, decryption_shares)
        .map(|(b, s)| cks[0].aggregate_decryption_shares(&b, &s))
        .collect_vec();

    let want_out_f1 = function(c0_meet[0], c1_meet[0], &c0_free, &c1_free);
    assert_eq!(out_f1, want_out_f1);
}