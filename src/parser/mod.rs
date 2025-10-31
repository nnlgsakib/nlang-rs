use crate::lexer::{Token, TokenType};
use crate::ast::*;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error on line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens);
    let statements = parser.parse_program()?;
    Ok(Program { statements })
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, current: 0 }
    }
    
    pub fn parse_program(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        Ok(statements)
    }
    
    fn declaration(&mut self) -> Result<Statement, ParseError> {
        if self.match_token(&TokenType::Export) {
            return self.export_declaration();
        }
        
        if self.match_token(&TokenType::Store) {
            return self.let_declaration();
        }
        
        if self.match_token(&TokenType::Def) {
            return self.function_declaration();
        }
        
        if self.check(&TokenType::Import) {
            return self.import_declaration();
        }
        
        if self.check(&TokenType::From) {
            return self.from_import_declaration();
        }
        
        if self.match_token(&TokenType::AssignMain) {
            return self.assign_main_declaration();
        }
        
        self.statement()
    }
    
    fn let_declaration(&mut self) -> Result<Statement, ParseError> {
        let name = if let TokenType::Identifier(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected variable name".to_string(),
                line: self.peek().line,
            });
        };
        
        self.consume(&TokenType::Identifier(name.clone()), "Expected variable name")?;
        
        let mut initializer = None;
        if self.match_token(&TokenType::Assign) {
            initializer = Some(self.expression()?);
        }
        
        self.consume(&TokenType::Semicolon, "Expected ';' after variable declaration")?;
        
        Ok(Statement::LetDeclaration { 
            name, 
            initializer, 
            is_exported: false 
        })
    }
    
    fn export_declaration(&mut self) -> Result<Statement, ParseError> {
        if self.match_token(&TokenType::Store) {
            let mut stmt = self.let_declaration()?;
            if let Statement::LetDeclaration { ref mut is_exported, .. } = stmt {
                *is_exported = true;
            }
            Ok(stmt)
        } else if self.match_token(&TokenType::Def) {
            let mut stmt = self.function_declaration()?;
            if let Statement::FunctionDeclaration { ref mut is_exported, .. } = stmt {
                *is_exported = true;
            }
            Ok(stmt)
        } else {
            Err(ParseError {
                message: "Expected 'store' or 'def' after 'export'".to_string(),
                line: self.peek().line,
            })
        }
    }
    
    fn assign_main_declaration(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::Minus, "Expected '-' after 'ASSIGN_MAIN'")?;
        self.consume(&TokenType::Greater, "Expected '>' after '-'")?;
        
        let function_name = if let TokenType::String(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected string literal for function name".to_string(),
                line: self.peek().line,
            });
        };
        
        self.advance(); // consume the string
        self.consume(&TokenType::Semicolon, "Expected ';' after ASSIGN_MAIN declaration")?;
        
        Ok(Statement::AssignMain { function_name })
    }
    
    fn function_declaration(&mut self) -> Result<Statement, ParseError> {
        let name = if let TokenType::Identifier(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected function name".to_string(),
                line: self.peek().line,
            });
        };
        
        self.consume(&TokenType::Identifier(name.clone()), "Expected function name")?;
        self.consume(&TokenType::LeftParen, "Expected '(' after function name")?;
        
        let mut parameters = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                let param_name = if let TokenType::Identifier(name) = &self.peek().token_type {
                    name.clone()
                } else {
                    return Err(ParseError {
                        message: "Expected parameter name".to_string(),
                        line: self.peek().line,
                    });
                };
                
                self.consume(&TokenType::Identifier(param_name.clone()), "Expected parameter name")?;
                
                let param_type = if self.match_token(&TokenType::Colon) {
                    self.parse_type()?
                } else {
                    // For now, default to Integer if not specified
                    Type::Integer
                };

                parameters.push(Parameter {
                    name: param_name,
                    param_type,
                });
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after parameters")?;
        
        // For now, we'll assume return type is void unless specified
        let return_type = if self.match_token(&TokenType::Colon) {
            // For now, just consume the type - we'll implement proper type parsing later
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let body = if self.check(&TokenType::LeftBrace) {
            self.block()?
        } else {
            return Err(ParseError {
                message: "Expected function body".to_string(),
                line: self.peek().line,
            });
        };
        
        Ok(Statement::FunctionDeclaration {
            name,
            parameters,
            body,
            return_type,
            is_exported: false,
        })
    }
    
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        if self.match_identifier("int") {
            Ok(Type::Integer)
        } else if self.match_identifier("float") {
            Ok(Type::Float)
        } else if self.match_identifier("bool") {
            Ok(Type::Boolean)
        } else if self.match_identifier("string") {
            Ok(Type::String)
        } else {
            Err(ParseError {
                message: "Expected type".to_string(),
                line: self.peek().line,
            })
        }
    }

    fn match_identifier(&mut self, name: &str) -> bool {
        if let Some(token) = self.tokens.get(self.current) {
            if let TokenType::Identifier(id) = &token.token_type {
                if id == name {
                    self.current += 1;
                    return true;
                }
            }
        }
        false
    }
    
    fn import_declaration(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::Import, "Expected 'import' keyword")?;
        
        let module = if let TokenType::Identifier(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected module name".to_string(),
                line: self.peek().line,
            });
        };
        
        self.consume(&TokenType::Identifier(module.clone()), "Expected module name")?;
        
        if self.match_token(&TokenType::As) {
            let alias = if let TokenType::Identifier(name) = &self.peek().token_type {
                Some(name.clone())
            } else {
                return Err(ParseError {
                    message: "Expected alias name".to_string(),
                    line: self.peek().line,
                });
            };
            
            self.consume(&TokenType::Identifier(alias.clone().unwrap()), "Expected alias name")?;
            self.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
            
            Ok(Statement::Import { module, alias })
        } else if self.match_token(&TokenType::From) {
            // Handle from ... import ...
            let items = self.parse_import_list()?;
            self.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
            Ok(Statement::ImportFrom { module, items })
        } else {
            self.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
            Ok(Statement::Import { module, alias: None })
        }
    }
    
    fn from_import_declaration(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::From, "Expected 'from' keyword")?;
        
        let module = if let TokenType::Identifier(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected module name".to_string(),
                line: self.peek().line,
            });
        };
        
        self.consume(&TokenType::Identifier(module.clone()), "Expected module name")?;
        self.consume(&TokenType::Import, "Expected 'import' keyword")?;
        
        let items = self.parse_import_list()?;
        self.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
        Ok(Statement::ImportFrom { module, items })
    }

    fn parse_import_list(&mut self) -> Result<Vec<(String, Option<String>)>, ParseError> {
        let mut items = Vec::new();
        
        // Check if we have braces for destructuring syntax
        let has_braces = self.match_token(&TokenType::LeftBrace);
        
        // Parse comma-separated list
        loop {
            let item = if let TokenType::Identifier(name) = &self.peek().token_type {
                name.clone()
            } else {
                return Err(ParseError {
                    message: "Expected import item".to_string(),
                    line: self.peek().line,
                });
            };
            
            self.consume(&TokenType::Identifier(item.clone()), "Expected import item")?;
            
            let alias = if self.match_token(&TokenType::As) {
                let alias = if let TokenType::Identifier(name) = &self.peek().token_type {
                    Some(name.clone())
                } else {
                    return Err(ParseError {
                        message: "Expected alias name".to_string(),
                        line: self.peek().line,
                    });
                };
                
                self.consume(&TokenType::Identifier(alias.clone().unwrap()), "Expected alias name")?;
                alias
            } else {
                None
            };
            
            items.push((item, alias));
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        // If we started with braces, we need to close them
        if has_braces {
            self.consume(&TokenType::RightBrace, "Expected '}' after import list")?;
        }
        
        Ok(items)
    }
    
    fn statement(&mut self) -> Result<Statement, ParseError> {
        if self.check(&TokenType::LeftBrace) {
            return Ok(Statement::Block { statements: self.block()? });
        }
        
        if self.match_token(&TokenType::If) {
            return self.if_statement();
        }
        
        if self.match_token(&TokenType::While) {
            return self.while_statement();
        }
        
        if self.match_token(&TokenType::Return) {
            return self.return_statement();
        }
        
        if self.match_token(&TokenType::Break) {
            return self.break_statement();
        }
        
        if self.match_token(&TokenType::Continue) {
            return self.continue_statement();
        }
        
        self.expression_statement()
    }
    
    fn block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.consume(&TokenType::LeftBrace, "Expected '{' before block")?;
        
        let mut statements = Vec::new();
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        self.consume(&TokenType::RightBrace, "Expected '}' after block")?;
        Ok(statements)
    }
    
    fn if_statement(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = Box::new(self.expression()?);
        self.consume(&TokenType::RightParen, "Expected ')' after if condition")?;
        
        let then_branch = Box::new(self.statement()?);
        
        let else_branch = if self.match_token(&TokenType::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }
    
    fn while_statement(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = Box::new(self.expression()?);
        self.consume(&TokenType::RightParen, "Expected ')' after while condition")?;
        
        let body = Box::new(self.statement()?);
        
        Ok(Statement::While { condition, body })
    }
    
    fn return_statement(&mut self) -> Result<Statement, ParseError> {
        let value = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(Box::new(self.expression()?))
        };
        
        self.consume(&TokenType::Semicolon, "Expected ';' after return value")?;
        Ok(Statement::Return { value })
    }
    
    fn break_statement(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::Semicolon, "Expected ';' after 'break'")?;
        Ok(Statement::Break)
    }
    
    fn continue_statement(&mut self) -> Result<Statement, ParseError> {
        self.consume(&TokenType::Semicolon, "Expected ';' after 'continue'")?;
        Ok(Statement::Continue)
    }
    
    fn expression_statement(&mut self) -> Result<Statement, ParseError> {
        let expr = self.expression()?;
        self.consume(&TokenType::Semicolon, "Expected ';' after expression")?;
        Ok(Statement::Expression(expr))
    }
    
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }
    
    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.or()?;
        
        if self.match_token(&TokenType::Assign) {
            let equals = self.previous().clone();
            let value = self.assignment()?;
            
            if let Expr::Variable(name) = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                });
            }
            
            return Err(ParseError {
                message: "Invalid assignment target".to_string(),
                line: equals.line,
            });
        }
        
        Ok(expr)
    }
    
    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.and()?;
        
        while self.match_token(&TokenType::Or) {
            let operator = self.previous().clone();
            let right = self.and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: self.binary_operator_from_token(&operator)?,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;
        
        while self.match_token(&TokenType::And) {
            let operator = self.previous().clone();
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: self.binary_operator_from_token(&operator)?,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;
        
        while self.match_token(&TokenType::EqualEqual) || self.match_token(&TokenType::NotEqual) {
            let operator = self.previous().clone();
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: self.binary_operator_from_token(&operator)?,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;
        
        while self.match_token(&TokenType::Greater) 
            || self.match_token(&TokenType::GreaterEqual) 
            || self.match_token(&TokenType::Less) 
            || self.match_token(&TokenType::LessEqual) {
            let operator = self.previous().clone();
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: self.binary_operator_from_token(&operator)?,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;
        
        while self.match_token(&TokenType::Plus) || self.match_token(&TokenType::Minus) {
            let operator = self.previous().clone();
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: self.binary_operator_from_token(&operator)?,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;
        
        while self.match_token(&TokenType::Star) 
            || self.match_token(&TokenType::Slash) 
            || self.match_token(&TokenType::Percent) {
            let operator = self.previous().clone();
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: self.binary_operator_from_token(&operator)?,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&TokenType::Minus) || self.match_token(&TokenType::Not) {
            let operator = self.previous().clone();
            let right = self.unary()?;
            return Ok(Expr::Unary {
                operator: self.unary_operator_from_token(&operator)?,
                operand: Box::new(right),
            });
        }
        
        self.call()
    }
    
    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        
        loop {
            if self.match_token(&TokenType::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(&TokenType::Dot) {
                // Parse property access: object.name
                let prop_name = if let TokenType::Identifier(name) = &self.peek().token_type {
                    name.clone()
                } else {
                    return Err(ParseError {
                        message: "Expected property name after '.'".to_string(),
                        line: self.peek().line,
                    });
                };
                self.consume(&TokenType::Identifier(prop_name.clone()), "Expected property name after '.'")?;
                expr = Expr::Get { object: Box::new(expr), name: prop_name };
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let mut arguments = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;
        
        Ok(Expr::Call {
            callee: Box::new(callee),
            arguments,
        })
    }
    
    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&TokenType::Integer(0)) {
            let token = self.previous();
            if let TokenType::Integer(value) = token.token_type.clone() {
                return Ok(Expr::Literal(Literal::Integer(value)));
            }
        }
        
        if self.match_token(&TokenType::Float(0.0)) {
            let token = self.previous();
            if let TokenType::Float(value) = token.token_type.clone() {
                return Ok(Expr::Literal(Literal::Float(value)));
            }
        }
        
        if self.match_token(&TokenType::String("".to_string())) {
            let token = self.previous();
            if let TokenType::String(value) = token.token_type.clone() {
                return Ok(Expr::Literal(Literal::String(value)));
            }
        }
        
        if self.match_token(&TokenType::True) {
            return Ok(Expr::Literal(Literal::Boolean(true)));
        }
        
        if self.match_token(&TokenType::False) {
            return Ok(Expr::Literal(Literal::Boolean(false)));
        }
        
        if self.match_token(&TokenType::Null) {
            return Ok(Expr::Literal(Literal::Null));
        }
        
        if let TokenType::Identifier(name) = &self.peek().token_type {
            let name = name.clone();
            self.consume(&TokenType::Identifier(name.clone()), "Expected identifier")?;
            return Ok(Expr::Variable(name));
        }
        
        if self.match_token(&TokenType::LeftParen) {
            let expr = self.expression()?;
            self.consume(&TokenType::RightParen, "Expected ')' after expression")?;
            return Ok(expr);
        }
        
        Err(ParseError {
            message: format!("Expected expression, got {:?}", self.peek().token_type),
            line: self.peek().line,
        })
    }
    
    fn binary_operator_from_token(&self, token: &Token) -> Result<BinaryOperator, ParseError> {
        match &token.token_type {
            TokenType::Plus => Ok(BinaryOperator::Plus),
            TokenType::Minus => Ok(BinaryOperator::Minus),
            TokenType::Star => Ok(BinaryOperator::Star),
            TokenType::Slash => Ok(BinaryOperator::Slash),
            TokenType::Percent => Ok(BinaryOperator::Percent),
            TokenType::EqualEqual => Ok(BinaryOperator::EqualEqual),
            TokenType::NotEqual => Ok(BinaryOperator::NotEqual),
            TokenType::Less => Ok(BinaryOperator::Less),
            TokenType::LessEqual => Ok(BinaryOperator::LessEqual),
            TokenType::Greater => Ok(BinaryOperator::Greater),
            TokenType::GreaterEqual => Ok(BinaryOperator::GreaterEqual),
            TokenType::And => Ok(BinaryOperator::And),
            TokenType::Or => Ok(BinaryOperator::Or),
            _ => Err(ParseError {
                message: format!("Invalid binary operator: {:?}", token.token_type),
                line: token.line,
            }),
        }
    }
    
    fn unary_operator_from_token(&self, token: &Token) -> Result<UnaryOperator, ParseError> {
        match &token.token_type {
            TokenType::Minus => Ok(UnaryOperator::Negate),
            TokenType::Not => Ok(UnaryOperator::Not),
            _ => Err(ParseError {
                message: format!("Invalid unary operator: {:?}", token.token_type),
                line: token.line,
            }),
        }
    }
    
    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            return true;
        }
        false
    }
    
    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
    }
    
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }
    
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.peek().token_type, TokenType::Eof)
    }
    
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    
    fn consume(&mut self, token_type: &TokenType, message: &str) -> Result<&Token, ParseError> {
        if self.check(token_type) {
            return Ok(self.advance());
        }
        
        Err(ParseError {
            message: message.to_string(),
            line: self.peek().line,
        })
    }
}