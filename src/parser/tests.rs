#[cfg(test)]
mod tests {
    use crate::parser::parse;
    use crate::ast::{Statement, Expr, Literal, BinaryOperator};
    use crate::lexer::tokenize;
    
    #[test]
    fn test_variable_declaration() {
        let source = "store x = 42;";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        
        match &program.statements[0] {
            Statement::LetDeclaration { name, initializer: Some(init), var_type: _, is_exported: _, .. } => {
                assert_eq!(name, "x");
                match init {
                    Expr::Literal(Literal::Integer(42)) => (),
                    _ => panic!("Expected integer literal 42"),
                }
            },
            _ => panic!("Expected let declaration"),
        }
    }
    
    #[test]
    fn test_function_declaration() {
        let source = "def add(x, y) { return x + y; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        
        match &program.statements[0] {
            Statement::FunctionDeclaration { name, parameters, body, .. } => {
                assert_eq!(name, "add");
                assert_eq!(parameters.len(), 2);
                assert_eq!(parameters[0].name, "x");
                assert_eq!(parameters[1].name, "y");
                assert_eq!(body.len(), 1); // return statement
            },
            _ => panic!("Expected function declaration"),
        }
    }
    
    #[test]
    fn test_binary_expression() {
        let source = "x + y;";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        
        match &program.statements[0] {
            Statement::Expression(Expr::Binary { left, operator: BinaryOperator::Plus, right }) => {
                match &**left {
                    Expr::Variable(name) => assert_eq!(name, "x"),
                    _ => panic!("Expected variable x"),
                }
                match &**right {
                    Expr::Variable(name) => assert_eq!(name, "y"),
                    _ => panic!("Expected variable y"),
                }
            },
            _ => panic!("Expected binary expression"),
        }
    }
    
    #[test]
    fn test_if_statement() {
        let source = "if (x > 5) { y = 10; } else { y = 0; }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        
        match &program.statements[0] {
            Statement::If { condition, then_branch: _, else_branch: Some(_) } => {
                match &**condition {
                    Expr::Binary { operator: BinaryOperator::Greater, .. } => (),
                    _ => panic!("Expected greater than operation"),
                }
            },
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn test_pick_statement() {
        let source = "pick x { when 1 => { return 1; } default => { return 0; } }";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Pick { expression, cases, default } => {
                assert_eq!(cases.len(), 1);
                assert!(default.is_some());
                if let Expr::Variable(name) = &**expression {
                    assert_eq!(name, "x");
                } else {
                    panic!("Expected variable 'x' in pick expression");
                }
            }
            _ => panic!("Expected pick statement"),
        }
    }

    #[test]
    fn test_i32_literal() {
        let source = "store x = 42i32; store y = -123i32;";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();
        
        match &program.statements[0] {
            Statement::LetDeclaration { name, initializer: Some(init), var_type: _, is_exported: _, .. } => {
                assert_eq!(name, "x");
                match init {
                    Expr::Literal(Literal::I32(42)) => (),
                    _ => panic!("Expected i32 literal 42"),
                }
            },
            _ => panic!("Expected let declaration"),
        }
        
        match &program.statements[1] {
            Statement::LetDeclaration { name, initializer: Some(init), var_type: _, is_exported: _, .. } => {
                assert_eq!(name, "y");
                match init {
                    Expr::Literal(Literal::I32(-123)) => (),
                    _ => panic!("Expected i32 literal -123"),
                }
            },
            _ => panic!("Expected let declaration"),
        }
    }

    #[test]
    fn test_repeat_until_statement() {
        let source = "repeat { x = x + 1; } until x > 10;";
        let tokens = tokenize(source).unwrap();
        let program = parse(&tokens).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::RepeatUntil { body: _, condition } => {
                // Check that condition is a binary expression with greater than operator
                match &**condition {
                    Expr::Binary { operator: BinaryOperator::Greater, .. } => (),
                    _ => panic!("Expected greater than operation in condition"),
                }
            },
            _ => panic!("Expected repeat-until statement"),
        }
    }

    #[test]
    fn test_leading_dot_method_error() {
        let source = "store d = .upper();";
        let tokens = tokenize(source).unwrap();
        let result = parse(&tokens);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.message.contains("missing receiver") || err.message.to_lowercase().contains("got dot"));
    }
}