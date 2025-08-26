use serde::{Deserialize, Serialize};
use sqlparser::{
    ast::{Expr, Value, VisitMut, VisitorMut},
    dialect::PostgreSqlDialect,
    parser::Parser,
};

use crate::PgLogstatsError;

/// Query type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryType {
    /// SELECT queries
    Select,
    /// INSERT queries
    Insert,
    /// UPDATE queries
    Update,
    /// DELETE queries
    Delete,
    /// Data Definition Language (CREATE, DROP, ALTER, etc.)
    DDL,
    /// Other queries (BEGIN, COMMIT, ROLLBACK, etc.)
    Other,
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryType::Select => write!(f, "SELECT"),
            QueryType::Insert => write!(f, "INSERT"),
            QueryType::Update => write!(f, "UPDATE"),
            QueryType::Delete => write!(f, "DELETE"),
            QueryType::DDL => write!(f, "DDL"),
            QueryType::Other => write!(f, "OTHER"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub sql: String,
    pub query_type: QueryType,
    pub normalized_query: String,
}

impl Query {
    /// Parse SQL and return a vector of Query, one for each statement
    pub fn from_sql(sql: &str) -> Result<Vec<Query>, PgLogstatsError> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, sql).map_err(|e| PgLogstatsError::Parse {
            message: format!("Failed to parse SQL: {}", e),
            line_number: None,
            line_content: Some(sql.to_string()),
        })?;

        let mut queries = Vec::new();
        for stmt in &ast {
            let query_type = Query::query_type_from_statement(stmt);
            let normalized_query = Query::normalize_query(std::slice::from_ref(stmt))
                .unwrap_or_else(|_| stmt.to_string());
            queries.push(Query {
                sql: stmt.to_string(),
                query_type,
                normalized_query,
            });
        }
        Ok(queries)
    }

    fn query_type_from_statement(stmt: &sqlparser::ast::Statement) -> QueryType {
        use sqlparser::ast::Statement::*;
        match stmt {
            Query(_) => QueryType::Select,
            Insert { .. } => QueryType::Insert,
            Update { .. } => QueryType::Update,
            Delete { .. } => QueryType::Delete,
            CreateTable { .. }
            | CreateView { .. }
            | CreateIndex { .. }
            | CreateSchema { .. }
            | CreateDatabase { .. }
            | Drop { .. }
            | AlterTable { .. }
            | Truncate { .. } => QueryType::DDL,
            _ => QueryType::Other,
        }
    }

    /// Normalize SQL query using an existing AST
    fn normalize_query(ast: &[sqlparser::ast::Statement]) -> Result<String, PgLogstatsError> {
        if ast.is_empty() {
            return Ok("".to_string());
        }

        // Clone AST to mutate
        let mut ast = ast.to_owned();

        let mut normalizer = LiteralNormalizer;
        for stmt in &mut ast {
            let _ = stmt.visit(&mut normalizer);
        }

        let normalized_sql = ast
            .iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join("; ");

        Ok(normalized_sql)
    }
}

/// Visitor that replaces literal values with placeholders
struct LiteralNormalizer;

impl VisitorMut for LiteralNormalizer {
    type Break = ();

    fn pre_visit_expr(&mut self, _expr: &mut Expr) -> std::ops::ControlFlow<Self::Break> {
        // Always continue traversal to visit nested expressions
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_expr(&mut self, expr: &mut Expr) -> std::ops::ControlFlow<Self::Break> {
        match expr {
            // Replace literal constants with placeholders
            Expr::Value(Value::Number(_, _))
            | Expr::Value(Value::SingleQuotedString(_))
            | Expr::Value(Value::DoubleQuotedString(_))
            | Expr::Value(Value::Boolean(_))
            | Expr::Value(Value::Null) => {
                *expr = Expr::Value(Value::Placeholder("?".to_string()));
            }

            // Normalize existing parameters to standard format
            Expr::Value(Value::Placeholder(_)) => {
                *expr = Expr::Value(Value::Placeholder("?".to_string()));
            }

            // Continue traversing for all other expressions
            _ => {}
        }

        std::ops::ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_normalization_test(original: &str, expected: &str) {
        let result = Query::from_sql(original);
        assert!(result.is_ok(), "Parsing failed for: {}", original);
        let queries = result.unwrap();
        assert_eq!(queries.len(), 1, "Expected one query for: {}", original);
        let query = &queries[0];
        assert_eq!(
            query.normalized_query, expected,
            "Normalization failed for: {}\nGot: {}\nExpected: {}",
            original, query.normalized_query, expected
        );
    }

    #[test]
    fn test_parameterized_normalization() {
        let cases = vec![
            (
                "SELECT * FROM users WHERE id = 1",
                "SELECT * FROM users WHERE id = ?",
            ),
            (
                "SELECT * FROM users WHERE name = 'John' AND city = 'New York'",
                "SELECT * FROM users WHERE name = ? AND city = ?",
            ),
            (
                "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                "UPDATE users SET name = ?, email = ? WHERE id = ?",
            ),
            (
                "SELECT   *   FROM    users   WHERE   id=1",
                "SELECT * FROM users WHERE id = ?",
            ),
            (
                "SELECT * FROM users WHERE (age > 25 AND name = 'John') OR id IN (1, 2, 3)",
                "SELECT * FROM users WHERE (age > ? AND name = ?) OR id IN (?, ?, ?)",
            ),
            (
                "INSERT INTO users (name, age) VALUES ('Alice', 30)",
                "INSERT INTO users (name, age) VALUES (?, ?)",
            ),
            (
                "DELETE FROM users WHERE active = true",
                "DELETE FROM users WHERE active = ?",
            ),
            (
                "SELECT * FROM orders WHERE price > 100.5",
                "SELECT * FROM orders WHERE price > ?",
            ),
            (
                "SELECT * FROM logs WHERE message IS NULL",
                "SELECT * FROM logs WHERE message IS NULL",
            ),
            (
                "SELECT * FROM products WHERE id IN ($1, $2, $3)",
                "SELECT * FROM products WHERE id IN (?, ?, ?)",
            ),
            (
                "SELECT   *   FROM    users   WHERE   id=1",
                "SELECT * FROM users WHERE id = ?",
            ),
            (
                "SELECT * FROM users WHERE name = 'John' AND city = 'New York'",
                "SELECT * FROM users WHERE name = ? AND city = ?",
            ),
            (
                "SELECT * FROM users WHERE age > 25 AND score < 100.5",
                "SELECT * FROM users WHERE age > ? AND score < ?",
            ),
            (
                "SELECT * FROM users WHERE active = true",
                "SELECT * FROM users WHERE active = ?",
            )
        ];

        for (original, expected) in cases {
            run_normalization_test(original, expected);
        }
    }
}
