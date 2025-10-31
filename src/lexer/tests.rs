#[cfg(test)]
mod lexer_tests {
    use crate::lexer::{tokenize, TokenType};
    
    #[test]
    fn test_basic_tokenization() {
        let source = "store x = 42;";
        let tokens = tokenize(source).unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Store);
        assert_eq!(tokens[1].token_type, TokenType::Identifier("x".to_string()));
        assert_eq!(tokens[2].token_type, TokenType::Assign);
        assert_eq!(tokens[3].token_type, TokenType::Integer(42));
        assert_eq!(tokens[4].token_type, TokenType::Semicolon);
    }
    
    #[test]
    fn test_function_declaration() {
        let source = "def add(x, y) { return x + y; }";
        let tokens = tokenize(source).unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Def);
        assert_eq!(tokens[1].token_type, TokenType::Identifier("add".to_string()));
        assert_eq!(tokens[2].token_type, TokenType::LeftParen);
        assert_eq!(tokens[3].token_type, TokenType::Identifier("x".to_string()));
        assert_eq!(tokens[4].token_type, TokenType::Comma);
        assert_eq!(tokens[5].token_type, TokenType::Identifier("y".to_string()));
    }
    
    #[test]
    fn test_string_literal() {
        let source = r#"store msg = "Hello, World!";"#;
        let tokens = tokenize(source).unwrap();
        
        assert_eq!(tokens[3].token_type, TokenType::String("Hello, World!".to_string()));
    }
    
    #[test]
    fn test_operators() {
        let source = "x + y - z * w / v % u";
        let tokens = tokenize(source).unwrap();
        
        assert_eq!(tokens[1].token_type, TokenType::Plus);
        assert_eq!(tokens[3].token_type, TokenType::Minus);
        assert_eq!(tokens[5].token_type, TokenType::Star);
        assert_eq!(tokens[7].token_type, TokenType::Slash);
        assert_eq!(tokens[9].token_type, TokenType::Percent);
    }
}