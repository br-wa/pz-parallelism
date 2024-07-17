#!/bin/bash

if [ $# -ne 4 ]; then
    echo "Usage: run_circuit_benchmarking.sh [circuit source] [n_threads] [n_tries] [output_file]"
    exit 1
fi

source=$1
n_threads=$2
n_tries=$3
output_file=$4

echo "Running circuit benchmarking for $source"

for i in $(seq 1 $n_tries); 
do
    echo "Running trial $i"
    ./target/release/pz-parallelism $source $n_threads >> $output_file
done
