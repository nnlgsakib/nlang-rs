# Nlang – Production-Ready Systems Language

# Nlang – Production-Ready Systems Language

<p align="center"><img src="icons/nlang.jpeg" alt="Nlang logo" width="180"></p>

Nlang is a statically-typed language with Python-like clarity and systems-level performance. It features a compile-time memory safety system, a robust type checker, an interpreter for fast iteration, and a C transpiler for portable binaries.

## Key Features

- Intuitive syntax and clear blocks
- Static typing with practical inference
- Compile-time memory safety with immutability-by-default, ownership, borrowing, lifetimes, and move semantics
- Standard library for I/O, strings, math, and data structures
- Multiple execution modes: interpreter and C code generation
- LSP tooling with diagnostics, quick fixes, completion, and formatting

## Installation

Prerequisites:
- Rust 1.70+
- GCC or Clang (for compiling generated C; path configurable via env or API)

Build:
```bash
cargo build --release
cargo test
```

## Usage

Direct execution (interpreter):
```bash
cargo run --bin nlang -- run path/to/program.nlang
```

C code generation and compilation:
```bash
cargo run --bin nlang -- generate-c path/to/program.nlang -o program.c
gcc program.c -o program.exe

# Or compile directly via the CLI wrapper
cargo run --bin nlang -- compile path/to/program.nlang
```

### Compiler Selection (custom GCC/Clang path)

- Via environment variables (resolution order: `NLANG_GCC` → `CC` → system `gcc`):
  - Windows PowerShell:
    ```powershell
    $env:NLANG_GCC = 'C:\\msys64\\usr\\bin\\gcc.exe'
    cargo run --bin nlang -- compile path\to\program.nlang
    ```
  - Unix shells:
    ```bash
    NLANG_GCC=/usr/local/bin/gcc cargo run --bin nlang -- compile path/to/program.nlang
    ```
- Via the Rust API:
  ```rust
  use nlang::execution_engine::ExecutionEngine;
  use std::path::Path;

  let engine = ExecutionEngine::new_with_gcc_path("C\\msys64\\usr\\bin\\gcc.exe");
  // or set later
  // let mut engine = ExecutionEngine::new();
  // engine.set_gcc_path("C\\mingw64\\bin\\gcc.exe");
  engine.compile_to_executable("def main() {}", "demo", Path::new("demo.exe")).unwrap();
  ```

## Language Essentials

Variables:
```nlang
store x :i32 = 42;       // immutable by default
@mut store y :i32 = 5;   // mutable variable
y = y + 1;               // ok
x = 10;                  // error: x is immutable
```

Ownership and borrowing:
```nlang
store v = create_vector();
store v2 = v;            // v moved; use-after-move is a compile error

@mut store a :i32 = 5;
store r1 = &a;           // immutable borrow
store r2 = &@mut a;      // mutable borrow; error if r1 is active
```

References and lifetimes:
```nlang
// Reference types can appear in signatures
def head(xs: [int; 3]) -> &int { return &xs[0]; }

def bad() -> &int {
    store x :i32 = 5;
    return &x;          // error: returns reference to local
}
```

Arrays and indexing:
```nlang
@mut store arr: [int; 3] = [1, 2, 3];
arr[0] = 100;            // ok: arr is mutable
store first = arr[1];    // reading is always fine
```

Control flow:
```nlang
@mut store i = 0;
while (i < 3) { i = i + 1; }
for (@mut store j = 2; j >= 0; j = j - 1) { println(j); }
repeat { i = i + 1; } until i >= 10;
loop { break; }
```

Imports:
```nlang
import std;                 // lazy-loads only called std functions
from std { sin, cos, tan }  // eager import of specific functions
```

## Memory Safety System

- Immutability by default; `@mut` required for mutation
- Single ownership; moves invalidate the source binding
- Borrowing rules:
  - Multiple immutable borrows
  - One mutable borrow; no other borrows concurrently
- Lifetimes inferred; references cannot outlive their data
- Move semantics tracked across control flow
- All checks happen at compile time (no GC, no ref-count overhead)

## Tooling

LSP (`src/nscan/`):
- Diagnostics include memory-safety hints and spans
- Quick fix to convert `store` → `@mut store` on immutability errors
- Completion includes `@mut`, std functions, and string/array methods
- Formatting, goto-definition, rename, and workspace symbols

Syntax Highlighting:
- TextMate grammar (`nscan-ext/syntaxes/nlang.tmLanguage.json`) supports `@mut`, borrows (`&`, `&@mut`), and bitwise operators

## Standard Library

Location: `src/std_lib/`
- I/O: `print`, `println`, `input`
- Conversions: `str`, `int`, `float`
- Strings: `upper`, `lower`, `trim`, `contains`, `split`, `replace`, `substring`, `regex`
- Math: `pi`, `e`, `exp`, `ln`, `log10`, `log2`, `sqrt`, `pow_float`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, stats (`sum_float`, `mean_float`, `median_float`, `variance_float`, `stddev_float`)
- Collections: `reverse`, `sort`
- Crypto: `sha256`, `sha256_random`

## Compilation Pipeline

```
Source (.nlang)
  ↓
Lexer → Parser → Semantic (type checking)
  ↓
MemManager (immutability/ownership/borrowing/lifetimes/moves)
  ↓
Interpreter         C Codegen
  ↓                    ↓
Direct run          C source → GCC/Clang → Executable
```

## Project Structure

```
src/
├── ast/              # AST definitions
├── lexer/            # Tokenizer and errors
├── parser/           # Declarations, statements, expressions, types
├── semantic/         # Type checking and analysis
├── memmanager/       # Ownership, borrowing, lifetimes, moves, drops
├── interpreter/      # Execution engine
├── c_codegen/        # C transpiler (portable runtime helpers)
├── execution_engine/ # Unified orchestration for run/compile
├── std_lib/          # Built-in functions and packaged nlang modules
├── nscan/            # LSP server and tooling
├── cli.rs            # CLI subcommands
├── lib.rs            # Public API exports
└── main.rs           # Application entry point
```

## Examples

See `nlang_test_writes/` for runnable samples:
- Hello world, loops, control flow, functions
- Arrays and multi-dimensional arrays (`13_comprehensive_array_test.nlang`)
- String library demos (`25_string_library_advanced.nlang`)
- Math std lib demo (`26_math_std_lib_demo.nlang`)
- SHA-256 (pure nlang and built-in) (`21_sha256.nlang`)

## Testing

```bash
cargo test
```

## License

MIT License. See [LICENSE](LICENSE).

## Acknowledgments

- Rust community
- Contributors and users of Nlang