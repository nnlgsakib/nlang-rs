#[cfg(test)]
mod tests {
    use crate::semantic::analyze;

    use crate::lexer::tokenize;
    use crate::parser::parse;
    
    #[test]
    fn test_semantic_analysis_basic() {
        let source = "def main() { store x = 42; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        match &analyzed_program {
            Err(e) => println!("Error: {:?}", e),
            Ok(_) => println!("Success"),
        }
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_immutability_enforced() {
        let source = "def main() { store x :i32 = 5; x = 10; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_mutable_annotation_ok() {
        let source = "def main() { @mut store x :i32 = 5; x = 10; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_use_after_move_error() {
        let source = "import std; def create_vector() { return [1,2,3]; } def main() { store vec = create_vector(); store vec2 = vec; println(vec); }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_borrow_rules_multiple_mut_error() {
        let source = "def main() { @mut store x :i32 = 5; store ref1 = &@mut x; store ref2 = &@mut x; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_borrow_rules_mut_and_immut_conflict() {
        let source = "def main() { @mut store x :i32 = 5; store ref1 = &x; store ref2 = &@mut x; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_dangling_reference_return_error() {
        let source = "def get_reference() -> &i32 { store x :i32 = 5; return &x; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_undefined_variable() {
        let source = "x;";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        
        // This should fail semantic analysis because x is not defined
        let result = analyze(program);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_variable_assignment() {
        let source = "def main() { @mut store x = 42; x = 10; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);

        match &analyzed_program {
            Err(e) => {
                println!("Variable assignment error: {:?}", e);
            },
            Ok(_) => println!("Variable assignment success"),
        }

        assert!(analyzed_program.is_ok());
    }
    
    #[test]
    fn test_function_declaration() {
        let source = "def add(x, y) { return x + y; } def main() { }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        match &analyzed_program {
            Err(e) => println!("Function declaration error: {:?}", e),
            Ok(_) => println!("Function declaration success"),
        }
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_pick_statement_semantic_analysis() {
        let source = "def main() { store x = 1; pick x { when 1 => {} when 2 => {} default => {} } }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_repeat_until_statement_semantic_analysis() {
        let source = "def main() { @mut store x = 0; repeat { x = x + 1; } until x > 5; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_repeat_until_boolean_condition() {
        // This should fail because the condition must be boolean
        let source = "def main() { repeat { } until 42; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let result = analyze(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_i32_literal_type_inference() {
        let source = "def main() { store x = 42i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_arithmetic_operations() {
        let source = "def main() { store x = 10i32 + 5i32; store y = 20i32 - 3i32; store z = 4i32 * 6i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_mixed_arithmetic_with_integer() {
        let source = "def main() { store x = 10i32 + 5; store y = 20 - 3i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_comparison_operations() {
        let source = "def main() { store x = 10i32 == 10i32; store y = 5i32 != 3i32; store z = 8i32 < 12i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_mixed_comparison_with_integer() {
        let source = "def main() { store x = 10i32 == 10; store y = 5 != 3i32; store z = 8i32 < 12; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_unary_negation() {
        let source = "def main() { store x = -10i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_modulo_operation() {
        let source = "def main() { store x = 15i32 % 4i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_division_promotes_to_float() {
        let source = "def main() { store x = 10i32 / 3i32; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_backward_compatibility_regular_integers() {
        let source = "def main() { store x = 42; store y = x + 10; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }

    #[test]
    fn test_i32_function_parameter_and_return() {
        let source = "def process_i32(x: i32) -> i32 { return x + 1i32; } def main() { store result = process_i32(5i32); }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
        assert!(analyzed_program.is_ok());
    }
}