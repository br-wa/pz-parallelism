use std::env::args;
use log::debug;
use rand::{thread_rng, Rng, RngCore};
use phantom_zone::*;
use itertools::{Itertools, izip};

#[derive(Clone, Debug)]
pub enum GateType {
    Not,
    And,
    Xor,
}

#[derive(Clone, Debug)]
pub struct Gate {
    pub gate_type: GateType,
    pub input_wires: Vec<usize>,
    pub output_wire: usize,
}

#[derive(Clone, Debug)]
pub struct Circuit {
    pub n_wires: usize,
    pub gates: Vec<Gate>,
    pub outputs: Vec<usize>,
    pub a_input_wires: Vec<usize>,
    pub b_input_wires: Vec<usize>,
}

impl Circuit {
    pub fn from_file (filename: &str) -> Circuit {
        let mut n_wires: usize = 0;
        let mut gates: Vec<Gate> = Vec::new();
        let mut outputs: Vec<usize> = Vec::new();
        let mut a_input_wires: Vec<usize> = Vec::new();
        let mut b_input_wires: Vec<usize> = Vec::new();

        for line in std::fs::read_to_string(filename).unwrap().lines() {
            let orig_line = line;

            if orig_line.len() == 0 {
                continue;
            }

            let mut line_ws = line.split_whitespace();

            let instruction = line_ws.next().unwrap();

            if instruction == "input" {
                n_wires += 1;
                if line_ws.next() == Some("A") {
                    a_input_wires.push(n_wires-1);
                }
                else {
                    b_input_wires.push(n_wires-1);
                }
            }
            else if instruction == "not" {
                let input_wire = line_ws.next().unwrap().parse::<usize>().unwrap();
                let output_wire = n_wires;
                n_wires += 1;
                gates.push(Gate {
                    gate_type: GateType::Not,
                    input_wires: vec![input_wire-1],
                    output_wire: output_wire,
                });
            }
            else if instruction == "and" || instruction == "xor" {
                let input_wire1 = line_ws.next().unwrap().parse::<usize>().unwrap();
                let input_wire2 = line_ws.next().unwrap().parse::<usize>().unwrap();                
                let output_wire = n_wires;
                n_wires += 1;
                gates.push(Gate {
                    gate_type: if instruction == "and" { GateType::And } else { GateType::Xor },
                    input_wires: vec![input_wire1-1, input_wire2-1],
                    output_wire: output_wire,
                });
            }
            else if instruction == "emit" {
                let input_wire = line_ws.next().unwrap().parse::<usize>().unwrap();
                outputs.push(input_wire-1);
            }
            else {
                panic!("unknown command, line: {}", orig_line);
            }
        }
        Circuit {
            n_wires: n_wires,
            gates: gates,
            outputs: outputs,
            a_input_wires: a_input_wires,
            b_input_wires: b_input_wires,
        }
    }

    pub fn eval (&self, a_input: &Vec<bool>, b_input: &Vec<bool>) -> Vec<bool> {
        let mut values: Vec<bool> = Vec::new();
        for _i in 0..self.n_wires {
            values.push(false);
        }
        for i in 0..self.a_input_wires.len() {
            values[self.a_input_wires[i]] = a_input[i];
        }
        for i in 0..self.b_input_wires.len() {
            values[self.b_input_wires[i]] = b_input[i];
        }
        for gate in &self.gates {
            debug!("gate={:?}", gate);
            let mut input_values: Vec<bool> = Vec::new();
            for input_wire in &gate.input_wires {
                input_values.push(values[*input_wire]);
            }
            match gate.gate_type {
                GateType::Not => {
                    values[gate.output_wire] = !input_values[0];
                },
                GateType::And => {
                    values[gate.output_wire] = input_values[0] && input_values[1];
                },
                GateType::Xor => {
                    values[gate.output_wire] = input_values[0] ^ input_values[1];
                }
            }
        }
        debug!("values={:?}", values);
        let mut output_values: Vec<bool> = Vec::new();
        for output_wire in &self.outputs {
            output_values.push(values[*output_wire]);
        }
        output_values
    }

    pub fn eval_on_fhe_bools(
        &self, a_input: &Vec<FheBool>, 
        b_input: &Vec<FheBool>
    ) -> Vec<FheBool> {
        let mut values: Vec<FheBool> = Vec::new();
        for _i in 0..self.n_wires {
            values.push(a_input[0].clone()); 
            // should do something more professional
        }
        for i in 0..self.a_input_wires.len() {
            values[self.a_input_wires[i]] = a_input[i].clone();
        }
        for i in 0..self.b_input_wires.len() {
            values[self.b_input_wires[i]] = b_input[i].clone();
        }
        for gate in &self.gates {
            debug!("gate={:?}", gate);
            let mut input_values: Vec<FheBool> = Vec::new();
            for input_wire in &gate.input_wires {
                input_values.push(values[*input_wire].clone());
            }
            match gate.gate_type {
                GateType::Not => {
                    values[gate.output_wire] = !&input_values[0].clone();
                },
                GateType::And => {
                    values[gate.output_wire] = &input_values[0].clone() & &input_values[1].clone();
                },
                GateType::Xor => {
                    values[gate.output_wire] = &input_values[0].clone() ^ &input_values[1].clone();
                }
            }
        }
        let mut output_values: Vec<FheBool> = Vec::new();
        for output_wire in &self.outputs {
            output_values.push(values[*output_wire].clone());
        }
        output_values
    }

}

fn parse_input (input: &str) -> Vec<bool> {
    input.chars().map(|c| c == '1').collect()
}

fn vb_to_vu8 (vb: &Vec<bool>) -> Vec<u8> {
    vb.iter().map(|b| if *b {1} else {0}).collect_vec()
}

fn vfheu8_to_vfhebool (v: &Vec<FheUint8>, zero: &FheUint8) -> Vec<FheBool> {
    v.iter().map(|b| b.neq(&zero)).collect_vec()
}

fn main () {
    env_logger::init();

    let args: Vec<String> = args().collect();
    if args.len() != 4 && args.len() != 2 {
        println!("Usage: eval [circuit source] [optional: A input string] [optional: B input string]");
        return;
    }
    let source = args[1].clone();
    debug!("source: {}", source);

    let circuit = Circuit::from_file(&source);

    let n_a_inputs = circuit.a_input_wires.len();
    let a_input = if args.len() == 4 {
        parse_input(&args[2])
    } else {
        (0..n_a_inputs).map(|_| thread_rng().gen::<bool>()).collect()
    };

    let b_input = if args.len() == 4 {
        parse_input(&args[3])
    } else {
        (0..circuit.b_input_wires.len()).map(|_| thread_rng().gen::<bool>()).collect()
    };
    
    let output_values = circuit.eval(&a_input, &b_input);

    for (i, v) in output_values.iter().enumerate() {
        println!("output{}={}", i+1, v);
    }

    // now, let's run with fhebools

    let now = std::time::Instant::now();
    set_parameter_set(ParameterSelector::NonInteractiveLTE4Party);

    // set application's common reference seed
    let mut seed = [0u8; 32];
    thread_rng().fill_bytes(&mut seed);
    set_common_reference_seed(seed);

    let no_of_parties = 2;
    let cks = (0..no_of_parties).map(|_| gen_client_key()).collect_vec();

    let a_input_enc = cks[0].encrypt(vb_to_vu8(&a_input).as_slice());
    let b_input_enc = cks[1].encrypt(vb_to_vu8(&b_input).as_slice());

    println!("Finished client seed and encryption! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let server_key_shares = cks
        .iter()
        .enumerate()
        .map(|(id, k)| gen_server_key_share(id, no_of_parties, k))
        .collect_vec();

    let server_key = aggregate_server_key_shares(&server_key_shares);
    server_key.set_server_key();

    let ct_a_input = a_input_enc.unseed::<Vec<Vec<u64>>>().key_switch(0).extract_all();
    let ct_b_input = b_input_enc.unseed::<Vec<Vec<u64>>>().key_switch(1).extract_all();

    println!("Finished key switching to server key! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let ct_a_input_bool = vfheu8_to_vfhebool(&ct_a_input, &ct_a_input[0]);
    let ct_b_input_bool = vfheu8_to_vfhebool(&ct_b_input, &ct_b_input[0]);

    println!("Finished converting to FHEBool! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let ct_out_f1 = circuit.eval_on_fhe_bools(&ct_a_input_bool, &ct_b_input_bool);
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

    println!("Finished aggregating decryption shares! Time taken: {:?}", now.elapsed());

    println!("out_f1: {:?}", out_f1);
}
