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
        let source = "def main() { store x = 42; x = 10; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        let analyzed_program = analyze(program);
        
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
}