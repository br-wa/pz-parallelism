# generate a circuit for the millionaire's problem
# given a[0..n-1] and b[0..n-1], check if a's integer is > b's integer

import sys

if len(sys.argv) != 2:
    print("Usage: gen_millionaire.py [n]")
    sys.exit(1)

n = int(sys.argv[1])

# 1 to n
for i in range(n):
    print(f"input A {i}")

# n+1 to 2n
for i in range(n):
    print(f"input B {i}")

# 2n+1 to 3n
for i in range(n):
    print(f"not {i+n+1}")

# 3n+1 to 4n
for i in range(n):
    print(f"and {i+1} {i+2*n+1}")

gtrs = [3*n+i for i in range(1, n+1)]

if n > 1:
    # 4n+1 to 5n
    for i in range(n):
        print(f"xor {i+1} {i+n+1}")

# 5n+1 to 6n-1
for i in range(n-1):
    print(f"not {i+4*n+1}")

eqs = [5*n+1+i for i in range(n-1)]
eqs_layers = [eqs.copy()]

cidx = 6*n

while len(eqs_layers[-1]) > 1:
    eqs_copy = eqs_layers[-1].copy()
    eqs_layers.append([])
    for i in range(0, len(eqs_copy)-1, 2):
        print(f"and {eqs_copy[i]} {eqs_copy[i+1]}")
        eqs_layers[-1].append(cidx)
        cidx += 1

def get_v2_and_idx (i):
    v2 = 0
    i_copy = i
    tp = 1
    while i_copy % 2 == 0:
        v2 += 1
        i_copy //= 2
        tp *= 2
    return (v2, i - tp)

eq_psums = []

for i in range(1, n):
    v2, idx = get_v2_and_idx(i)
    tp = i - idx # 2^v2
    if idx == 0:
        eq_psums.append(eqs_layers[v2][i//tp-1])
    else:
        print(f"and {eq_psums[idx-1]} {eqs_layers[v2][i//tp-1]}")
        eq_psums.append(cidx)
        cidx += 1

any_work = [gtrs[0]]

# 7n-2 to 8n-4
for i in range(1, n):
    print(f"and {gtrs[i]} {eq_psums[i-1]}")
    any_work.append(i+7*n-3)

if n > 1:
    # 8n-3 to 9n-4
    final_check = []
    for i in range(n):
        print(f"not {any_work[i]}")
        final_check.append(i+8*n-3)
    cidx = 9*n-3
    while len(final_check) > 1:
        final_check_copy = final_check.copy()
        final_check = []
        for i in range(0, len(final_check_copy)-1, 2):
            print(f"and {final_check_copy[i]} {final_check_copy[i+1]}")
            final_check.append(cidx)
            cidx += 1
        if len(final_check_copy) % 2 == 1:
            final_check.append(final_check_copy[-1])
    
    print(f"not {final_check[0]}")
    print(f"emit {final_check[0]+1}")
else:
    print(f"emit {any_work[0]}")



