# Nlang - A Modern Systems Programming Language

Nlang is a statically-typed, compiled programming language that combines Python-like syntax with the performance and safety of systems programming. Built with Rust and powered by LLVM, Nlang offers multiple compilation targets including native machine code and C transpilation.

## 🚀 Key Features

- **Intuitive Syntax**: Python-inspired syntax with explicit block delimiters for clarity
- **Static Type System**: Strong typing with intelligent type inference
- **Multiple Backends**: LLVM IR generation and C code transpilation
- **Memory Safety**: Rust-powered compiler with compile-time safety guarantees
- **Modular Design**: Comprehensive import system for code organization
- **Standard Library**: Built-in functions for I/O, mathematics, and data manipulation
- **Control Flow**: Full support for loops, conditionals, and control statements (`break`, `continue`)

## 📦 Installation

### Prerequisites
- Rust 1.70+ (for building the compiler)
- LLVM 14+ (optional, for LLVM backend)
- GCC or Clang (optional, for C backend compilation)

### Build from Source
```bash
# Clone the repository
git clone https://github.com/nnlgsakib/nlang-rs.git
cd nlang-rs

# Build the compiler
cargo build --release

# Verify installation
cargo test
```

## 🛠️ Usage

Nlang provides multiple compilation modes to suit different development needs:

### Direct Execution
```bash
# Run a program directly (interpreter mode)
cargo run -- run program.nlang
```

### LLVM Compilation
```bash
# Generate LLVM IR
cargo run -- generate-ir program.nlang -o program.ll

# Compile to executable (requires LLVM tools)
cargo run -- compile program.nlang -o program.exe
```

### C Code Generation
```bash
# Generate C code
cargo run -- generate-c program.nlang -o program.c

# Compile with GCC
gcc program.c -o program.exe
```

## 📝 Language Syntax

### Basic Program Structure
```nlang
def main() {
    println("Hello, World!");
    return 0;
}
```

### Variables and Types
```nlang
def example() {
    store x = 42;           // Integer
    store y = 3.14;         // Float  
    store name = "Alice";   // String
    store active = true;    // Boolean
    
    // Type inference works automatically
    store result = x * 2;   // Inferred as Integer
}
```

### Functions
```nlang
def calculate(a, b) {
    store sum = a + b;
    store product = a * b;
    return product;
}

def main() {
    store result = calculate(5, 3);
    println(result);
}
```

### Control Flow
```nlang
def control_example() {
    store counter = 0;
    
    // While loops with break/continue
    while (counter < 10) {
        counter = counter + 1;
        
        if (counter == 3) {
            continue;  // Skip iteration
        }
        
        if (counter == 7) {
            break;     // Exit loop
        }
        
        println(counter);
    }
    
    // Conditional statements
    if (counter > 5) {
        println("Counter is large");
    } else {
        println("Counter is small");
    }
}
```

### Import System
```nlang
import math;              // Import entire module
import io as input_output; // Import with alias

// Import specific functions
from string { upper, lower, length }
from math { sqrt, pow }

def main() {
    store text = "hello";
    println(upper(text));  // Outputs: HELLO
}
```

## 🏗️ Architecture

Nlang features a robust, multi-stage compilation pipeline:

```
Source Code (.nlang)
        ↓
    Lexical Analysis (Tokenization)
        ↓
    Syntax Analysis (AST Generation)
        ↓
    Semantic Analysis (Type Checking)
        ↓
    ┌─────────────────┬─────────────────┐
    ↓                 ↓                 ↓
Interpreter      LLVM Codegen      C Codegen
    ↓                 ↓                 ↓
Direct Execution  Machine Code      C Source
                     ↓                 ↓
                 Executable        GCC/Clang
                                      ↓
                                  Executable
```

### Project Structure
```
src/
├── ast/              # Abstract Syntax Tree definitions
├── lexer/            # Tokenization and lexical analysis
├── parser/           # Syntax parsing and AST construction
├── semantic/         # Type checking and semantic validation
├── interpreter/      # Direct code execution engine
├── llvm_codegen/     # LLVM IR generation backend
├── c_codegen/        # C code transpilation backend
├── execution_engine/ # Unified compilation interface
├── std_lib/          # Standard library implementation
├── cli.rs            # Command-line interface
├── lib.rs            # Public API exports
└── main.rs           # Application entry point
```

## ✅ Implementation Status

### Core Language Features
- ✅ **Lexer**: Complete tokenization with all language constructs
- ✅ **Parser**: Full syntax parsing with error recovery
- ✅ **AST**: Comprehensive abstract syntax tree representation
- ✅ **Semantic Analysis**: Type checking, scope validation, and error reporting
- ✅ **Type System**: Static typing with inference for primitives and expressions

### Execution Backends
- ✅ **Interpreter**: Direct AST execution for development and testing
- ✅ **LLVM Backend**: Optimized machine code generation
- ✅ **C Backend**: Portable C code transpilation

### Language Constructs
- ✅ **Variables**: Declaration, initialization, and assignment
- ✅ **Functions**: Definition, parameters, return values, and recursion
- ✅ **Control Flow**: `if`/`else` conditionals, `while` loops
- ✅ **Loop Control**: `break` and `continue` statements
- ✅ **Expressions**: Arithmetic, logical, and comparison operations
- ✅ **Data Types**: Integer, Float, String, Boolean, and Null
- ✅ **Standard Library**: I/O operations, string manipulation, math functions

### Development Tools
- ✅ **CLI Interface**: Multiple compilation modes and options
- ✅ **Error Reporting**: Detailed syntax and semantic error messages
- ✅ **Testing Suite**: Comprehensive unit and integration tests
- ✅ **Import System**: Module loading and namespace management

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test lexer
cargo test parser
cargo test semantic

# Test with verbose output
cargo test -- --nocapture
```

## 🎯 Performance

Nlang is designed for performance across multiple execution modes:

- **Interpreter**: Fast startup for development and scripting
- **LLVM Backend**: Optimized machine code with LLVM's world-class optimizations
- **C Backend**: Portable code that leverages mature C compiler optimizations

Benchmark results show competitive performance with other compiled languages while maintaining memory safety and developer productivity.

## 🤝 Contributing

We welcome contributions to Nlang! Areas where you can help:

### Language Features
- Advanced type system features (generics, traits)
- Additional control flow constructs (`for` loops, pattern matching)
- Memory management primitives
- Concurrency and parallelism support

### Standard Library
- File system operations
- Network programming utilities
- Data structure implementations
- Algorithm libraries

### Tooling
- Language server protocol (LSP) implementation
- Syntax highlighting for popular editors
- Package manager and build system
- Debugging support

### Getting Started
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes with tests
4. Ensure all tests pass (`cargo test`)
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **LLVM Project**: For providing the excellent compilation infrastructure
- **Rust Community**: For the robust systems programming foundation
- **Contributors**: Everyone who has contributed to making Nlang better

---

**Nlang** - *Bridging the gap between productivity and performance*