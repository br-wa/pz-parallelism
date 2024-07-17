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

eq_psums = [5*n+1]

# 6n to 7n-3
for i in range(1, n-1):
    print(f"and {i+5*n+1} {eq_psums[-1]}")
    eq_psums.append(i+6*n-1)

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
    
    # 9n-3
    print(f"and {final_check[0]} {final_check[1]}")
    for i in range(2, n):
        print(f"and {final_check[i]} {i+9*n-5}")
    print(f"not {10*n-5}")
    print(f"emit {10*n-4}")
else:
    print(f"emit {any_work[0]}")



