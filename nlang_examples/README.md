# Nlang Example Programs

This directory contains comprehensive example programs demonstrating the features and capabilities of the nlang programming language.

## Overview

Nlang is a Python-like programming language that compiles to machine code using LLVM. These examples showcase the language's syntax, built-in functions, and programming constructs.

## Example Programs

### 1. `01_hello_world.nlang`
**Basic Hello World Program**
- Demonstrates the simplest nlang program structure
- Shows function definition and the `assign_main` directive
- Uses the `println` built-in function

### 2. `02_variables_and_types.nlang`
**Variables and Data Types**
- Variable declaration using the `store` keyword
- Integer, float, boolean, and string data types
- Type conversion using `str()` function
- Basic output formatting

### 3. `03_arithmetic_operations.nlang`
**Arithmetic Operations and Math Functions**
- Basic arithmetic operators: `+`, `-`, `*`, `/`, `%`
- Built-in math functions: `abs()`, `max()`, `min()`, `pow()`
- Working with negative numbers
- Mathematical expressions

### 4. `04_control_flow.nlang`
**Control Flow - If/Else Statements**
- Conditional statements with `if/else`
- Comparison operators: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical operators: `and`, `or`, `not`
- Nested conditional statements
- Boolean expressions

### 5. `05_loops.nlang`
**Loops - While, Break, and Continue**
- `while` loop syntax and usage
- Loop control with `break` and `continue`
- Nested loops
- Counter-based iterations
- Practical loop examples

### 6. `06_functions.nlang`
**Functions - Definition, Parameters, and Return Values**
- Function definition with `def` keyword
- Functions with and without parameters
- Return values and the `return` statement
- Recursive functions (factorial example)
- Function composition and reusability

### 7. `07_string_operations.nlang`
**String Operations and Functions**
- String variable declaration and manipulation
- String length calculation with `len()` function
- String concatenation techniques
- String comparison operations
- Working with special characters and quotes

### 8. `08_type_conversions.nlang`
**Type Conversions**
- Converting between data types
- `str()` - convert to string
- `int()` - convert string to integer
- `float()` - convert string to float
- `bool()` - convert to boolean
- Practical conversion examples

### 9. `09_calculator.nlang`
**Advanced Calculator Program**
- Comprehensive example combining multiple features
- Mathematical operations and algorithms
- Prime number checking
- Square root approximation
- Greatest Common Divisor (GCD) calculation
- Complex mathematical expressions

### 10. `10_utility_library.nlang`
**Utility Library with Exports**
- Demonstrates the `export` keyword for creating reusable modules
- Mathematical utility functions
- Temperature conversion functions
- Financial calculations (compound interest)
- Number manipulation utilities
- Fibonacci sequence generation

### 15. `15_pick_statement.nlang`
**Pick (Match/Switch) Statement**
- Demonstrates the new `pick` statement for pattern matching
- Examples with integer, string, and boolean expressions
- Usage of `when` clauses with single or multiple values
- Implementation of a `default` case
- Shows `pick` statements without a `default` case

## Language Features Demonstrated

### Keywords
- `store` - Variable declaration
- `def` - Function definition
- `if/else` - Conditional statements
- `while` - Loop construct
- `return` - Return from function
- `break/continue` - Loop control
- `export` - Export functions for import
- `assign_main` - Designate main function
- `pick` - Match/switch statement
- `when` - Case for `pick` statement
- `default` - Default case for `pick` statement

### Data Types
- **Integer**: Whole numbers (e.g., `42`, `-10`)
- **Float**: Decimal numbers (e.g., `3.14`, `-2.5`)
- **Boolean**: `true` or `false`
- **String**: Text in quotes (e.g., `"Hello, World!"`)

### Built-in Functions
- **I/O**: `print()`, `println()`, `input()`
- **String**: `len()` - string length
- **Conversion**: `str()`, `int()`, `float()`, `bool()`
- **Math**: `abs()`, `max()`, `min()`, `pow()`

### Operators
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Logical**: `and`, `or`, `not`
- **Assignment**: `=`

## How to Run

To compile and run these examples, use the nlang compiler:

```bash
# Compile to executable
nlang compile 01_hello_world.nlang

# Run directly
nlang run 01_hello_world.nlang

# Generate LLVM IR
nlang generate-ir 01_hello_world.nlang

# Generate C code
nlang generate-c 01_hello_world.nlang
```

## Learning Path

1. Start with `01_hello_world.nlang` to understand basic program structure
2. Learn variables and types with `02_variables_and_types.nlang`
3. Explore arithmetic with `03_arithmetic_operations.nlang`
4. Master control flow with `04_control_flow.nlang` and `05_loops.nlang`
5. Understand functions with `06_functions.nlang`
6. Work with strings using `07_string_operations.nlang`
7. Learn type conversions with `08_type_conversions.nlang`
8. Study the comprehensive `09_calculator.nlang` example
9. Explore modular programming with `10_utility_library.nlang`
10. Discover pattern matching with `15_pick_statement.nlang`

## Notes

- All programs use the `assign_main` directive to specify the entry point
- The nlang syntax is Python-like but requires semicolons for statements
- Function parameters and return types can be inferred by the compiler
- The language supports both procedural and functional programming styles

These examples provide a solid foundation for learning nlang programming and can serve as templates for your own projects.