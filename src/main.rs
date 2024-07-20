use std::env::args;
use log::debug;
use rand::{thread_rng, Rng, RngCore};
use phantom_zone::*;
use itertools::{Itertools, izip};
use threadpool::ThreadPool;
use std::sync::{Arc, Mutex};

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
        &self, 
        a_input: &Vec<FheBool>, 
        b_input: &Vec<FheBool>,
        n_threads: usize,
    ) -> Vec<FheBool> {
        let values: Arc<Mutex<Vec<FheBool>>> = Arc::new(Mutex::new(Vec::with_capacity(self.n_wires)));
        let completed: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::with_capacity(self.n_wires)));
        let circuit = Arc::new(Mutex::new(self.clone()));
        let time_spent_waiting = Arc::new(Mutex::new(0));
        {
            let mut values = values.lock().unwrap();
            values.extend(std::iter::repeat(a_input[0].clone()).take(self.n_wires));

            let mut completed = completed.lock().unwrap();
            completed.extend(std::iter::repeat(false).take(self.n_wires));
            
            for (i, &wire) in circuit.lock().unwrap().a_input_wires.iter().enumerate() {
                values[wire] = a_input[i].clone();
                completed[wire] = true;
            }
            
            for (i, &wire) in circuit.lock().unwrap().b_input_wires.iter().enumerate() {
                values[wire] = b_input[i].clone();
                completed[wire] = true;
            }
        }

        let tp = ThreadPool::new(n_threads);
        let gates = self.gates.clone();

        for gate in gates {
            let values = Arc::clone(&values);
            let completed = Arc::clone(&completed);
            let time_spent_waiting = Arc::clone(&time_spent_waiting);
            tp.execute(move || {
                debug!("gate={:?}", gate);
                set_parameter_set(ParameterSelector::NonInteractiveLTE4Party);

                let now = std::time::Instant::now();

                loop {
                    let mut ready = true;
                    for &wire in &gate.input_wires {
                        if !completed.lock().unwrap()[wire] {
                            ready = false;
                            break;
                        }
                    }
                    if ready {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(3));
                }

                *time_spent_waiting.lock().unwrap() += now.elapsed().as_millis();
                
                let input_values: Vec<FheBool> = {
                    let values = values.lock().unwrap();
                    gate.input_wires.iter().map(|&wire| values[wire].clone()).collect()
                };

                let output = match gate.gate_type {
                    GateType::Not => !&input_values[0],
                    GateType::And => &input_values[0] & &input_values[1],
                    GateType::Xor => &input_values[0] ^ &input_values[1],
                };

                values.lock().unwrap()[gate.output_wire] = output;
                completed.lock().unwrap()[gate.output_wire] = true;
            });
        }

        tp.join();

        let values = values.lock().unwrap();
        println!("time_spent_waiting: {:?}ms", *time_spent_waiting.lock().unwrap());
        self.outputs.iter().map(|&wire| values[wire].clone()).collect()
    }
}

fn parse_input (input: &str) -> Vec<bool> {
    input.chars().map(|c| c == '1').collect()
}

fn main () {
    env_logger::init();

    let args: Vec<String> = args().collect();
    if args.len() != 5 && args.len() != 3 {
        println!("Usage: eval [circuit source] [n_threads] [optional: A input string] [optional: B input string]");
        return;
    }
    let source = args[1].clone();
    debug!("source: {}", source);

    let circuit = Circuit::from_file(&source);

    let n_threads = args[2].parse::<usize>().unwrap();

    let n_a_inputs = circuit.a_input_wires.len();
    let a_input = if args.len() == 5 {
        parse_input(&args[3])
    } else {
        (0..n_a_inputs).map(|_| thread_rng().gen::<bool>()).collect()
    };

    let n_b_inputs = circuit.b_input_wires.len();
    let b_input = if args.len() == 5 {
        parse_input(&args[4])
    } else {
        (0..n_b_inputs).map(|_| thread_rng().gen::<bool>()).collect()
    };
    
    let output_values = circuit.eval(&a_input, &b_input);

    println!("output_values: {:?}", output_values);

    // now, let's run with fhebools

    let now = std::time::Instant::now();
    set_parameter_set(ParameterSelector::NonInteractiveLTE4Party);

    // set application's common reference seed
    let mut seed = [0u8; 32];
    thread_rng().fill_bytes(&mut seed);
    set_common_reference_seed(seed);

    let no_of_parties = 2;
    let cks = (0..no_of_parties).map(|_| gen_client_key()).collect_vec();

    let a_input_enc = Encryptor::<_, NonInteractiveBatchedFheBools<Vec<Vec<u64>>>>::encrypt(&cks[0], a_input.as_slice());
    let b_input_enc = Encryptor::<_, NonInteractiveBatchedFheBools<Vec<Vec<u64>>>>::encrypt(&cks[1], b_input.as_slice());

    let server_key_shares = cks
        .iter()
        .enumerate()
        .map(|(id, k)| gen_server_key_share(id, no_of_parties, k))
        .collect_vec();

    let server_key = aggregate_server_key_shares(&server_key_shares);
    server_key.set_server_key();

    println!("Finished client seed, encryption, and server key generation! Time taken: {:?}", now.elapsed());
    
    let now = std::time::Instant::now();

    let ct_a_input = (0..n_a_inputs).map(
        |i| 
        FheBool{ data: a_input_enc.key_switch(0).extract(i)}
    ).collect_vec();
    let ct_b_input = (0..n_b_inputs).map(
        |i|
        FheBool{ data: b_input_enc.key_switch(1).extract(i)}
    ).collect_vec();

    println!("Finished key switching to server key! Time taken: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let ct_out_f1 = circuit.eval_on_fhe_bools(
        &ct_a_input,
        &ct_b_input,
        n_threads,
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

    println!("Finished aggregating decryption shares! Time taken: {:?}", now.elapsed());

    println!("out_f1: {:?}", out_f1);
}
