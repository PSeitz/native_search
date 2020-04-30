use combine::{char::*, error::StreamError, stream::StreamErrorFor, *};
use std::fmt;
// use user_input_ast::*;

#[derive(Clone)]
pub struct UserFilter {
    pub field_name: Option<String>,
    pub phrase: String,
    pub levenshtein: Option<u8>,
}

impl fmt::Debug for UserFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(levenshtein) = self.levenshtein {
            match self.field_name {
                Some(ref field_name) => write!(formatter, "{}:\"{}\"~{:?}", field_name, self.phrase, levenshtein),
                None => write!(formatter, "\"{}\"~{:?}", self.phrase, levenshtein),
            }
        } else {
            match self.field_name {
                Some(ref field_name) => write!(formatter, "{}:\"{}\"", field_name, self.phrase),
                None => write!(formatter, "\"{}\"", self.phrase),
            }
        }
    }
}

impl UserFilter {
    pub fn into_ast(self) -> UserAST {
        UserAST::Leaf(Box::new(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Or,
    And,
}
impl Operator {
    fn to_string(self) -> &'static str {
        match self {
            Operator::Or => " OR ",
            Operator::And => " AND ",
        }
    }
}

#[derive(Clone)]
pub enum UserAST {
    Clause(Operator, Vec<UserAST>),
    Leaf(Box<UserFilter>),
}

impl UserAST {
    pub fn simplify(self) -> Self {
        match self {
            UserAST::Clause(op, mut queries) => {
                if queries.len() == 1 {
                    return queries.pop().unwrap().simplify();
                }
                let mut new_queries = vec![];
                for mut query in queries {
                    if let UserAST::Clause(sub_op, ref mut sub_queries) = query {
                        if op == sub_op {
                            new_queries.extend(sub_queries.drain(..));
                            continue;
                        }
                    }
                    new_queries.push(query);
                }
                UserAST::Clause(op, new_queries.into_iter().map(|query| query.simplify()).collect())
            }
            _ => self,
        }
    }
}

#[test]
fn test_simplify() {
    let leaf = UserAST::Leaf(Box::new(UserFilter {
        field_name: None,
        phrase: "test".to_string(),
        levenshtein: None,
    }));
    let ast = UserAST::Clause(Operator::Or, vec![UserAST::Clause(Operator::Or, vec![leaf])]);

    assert_eq!(format!("{:?}", ast), "((\"test\"))");
    assert_eq!(format!("{:?}", ast.simplify()), "\"test\"");
}

fn debug_print_clause(formatter: &mut fmt::Formatter, asts: &[UserAST], clause: &str) -> Result<(), fmt::Error> {
    write!(formatter, "(")?;
    write!(formatter, "{:?}", &asts[0])?;
    for subquery in &asts[1..] {
        write!(formatter, "{}", clause)?;
        write!(formatter, "{:?}", subquery)?;
    }
    write!(formatter, ")")?;
    Ok(())
}

impl fmt::Debug for UserAST {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            UserAST::Clause(ref op, ref asts) => debug_print_clause(formatter, asts, op.to_string()),
            UserAST::Leaf(ref subquery) => write!(formatter, "{:?}", subquery),
        }
    }
}

parser! {
    fn field[I]()(I) -> String
    where [I: Stream<Item = char>] {
        (
            letter(),
            many(satisfy(|c: char| c.is_alphanumeric() || c == '_')),
        ).map(|(s1, s2): (char, String)| format!("{}{}", s1, s2))
    }
}

parser! {
    fn word[I]()(I) -> String
    where [I: Stream<Item = char>] {
        many1(satisfy(|c: char| !c.is_whitespace() && c != '(' && c != ')' && c != '~' ))
            .and_then(|s: String| {
                match s.as_str() {
                    "OR" => Err(StreamErrorFor::<I>::unexpected_static_message("OR")),
                    "AND" => Err(StreamErrorFor::<I>::unexpected_static_message("AND")),
                    _ => Ok(s)
                }
            })
    }
}

parser! {
    fn user_literal[I]()(I) -> UserAST
    where [I: Stream<Item = char>]
    {
        let term_val = || {
            let phrase = (char('"'), many1(satisfy(|c| c != '"')), char('"')).map(|(_, s, _)| s);
            phrase.or(word())
        };
        let term_with_field_and_levenshtein =
            (field(), char(':'), term_val(), char('~'), digit()).map(|(field_name, _, phrase, _, digit)| UserFilter {
                field_name: Some(field_name),
                phrase,
                levenshtein: Some(digit.to_string().parse::<u8>().unwrap())
            }.into_ast());
        let term_with_field =
            (field(), char(':'), term_val()).map(|(field_name, _, phrase)| UserFilter {
                field_name: Some(field_name),
                phrase,
                levenshtein: None
            }.into_ast());
        let term_no_field_and_levenshtein =
            (term_val(), char('~'), digit()).map(|(phrase, _, digit)| UserFilter {
                field_name: None,
                phrase,
                levenshtein: Some(digit.to_string().parse::<u8>().unwrap())
            }.into_ast());
        let term_no_field = term_val().map(|phrase| UserFilter {
            field_name: None,
            phrase,
            levenshtein: None
        }.into_ast());
        // attempt(term_with_field)
        //     .or(attempt(multi_words))
        //     .or(term_no_field)
        attempt(term_with_field_and_levenshtein)
            .or(attempt(term_no_field_and_levenshtein))
            .or(attempt(term_with_field))
            .or(term_no_field)
    }
}

parser! {
    fn leaf[I]()(I) -> UserAST
    where [I: Stream<Item = char>] {
        let multi_literals = sep_by(user_literal(), space())
            .map(|mut sub_asts: Vec<UserAST>| {
                if sub_asts.len() == 1 {
                    sub_asts.pop().unwrap()
                }else{
                    UserAST::Clause(Operator::Or, sub_asts)
                }
            });

        (char('('), parse_to_ast(), char(')')).map(|(_, expr, _)| expr)
        .or(attempt(multi_literals))
        .or(user_literal())
    }
}

parser! {
    fn parse_operator[I]()(I) -> Operator
    where [I: Stream<Item = char>] {
        (skip_many1(space()),
        (
            string("AND").map(|_| Operator::And)
           .or(string("OR").map(|_| Operator::Or))
        ),
        skip_many1(space())).map(|(_, op,_)| op)
    }
}

macro_rules! combine_if_same_op {
    ($ast:expr,$opa:expr, $other:expr) => {
        if let UserAST::Clause(op, ref queries) = $ast {
            if op == $opa {
                let mut queries = queries.clone();
                queries.push($other);
                return UserAST::Clause(op, queries);
            }
        }
    };
}

parser! {
    pub fn parse_to_ast[I]()(I) -> UserAST
    where [I: Stream<Item = char>]
    {
        
            attempt(
                chainl1(
                    leaf(),
                    parse_operator().map(|op: Operator|
                        move |left: UserAST, right: UserAST| {
                            combine_if_same_op!(left, op, right);
                            combine_if_same_op!(right, op, left);
                            UserAST::Clause(op, vec![left, right])
                        }
                    )
                )
            )
        
    }
}

pub use combine::error::StringStreamError;

pub fn parse(query: &str) -> Result<(UserAST, &str), StringStreamError> {
    parse_to_ast().parse(query)
}

#[cfg(test)]
mod test {

    use super::*;
    fn test_parse_query_to_ast_helper(query: &str, expected: &str) {
        let query = parse_to_ast().parse(query).unwrap().0;
        let query_str = format!("{:?}", query);
        assert_eq!(query_str, expected);
    }

    #[test]
    fn test_multi_spaces() {
        test_parse_query_to_ast_helper("a AND  b", "(\"a\" AND \"b\")");
    }

    #[test]
    fn test_special_chars() {
        test_parse_query_to_ast_helper("die drei ???", "(\"die\" OR \"drei\" OR \"???\")");
        test_parse_query_to_ast_helper("a+", "\"a+\"");
    }

    #[test]
    fn test_multi_and_to_flat() {
        test_parse_query_to_ast_helper("a AND b AND c", "(\"a\" AND \"b\" AND \"c\")");
    }

    #[test]
    fn test_multi_or_to_flat() {
        test_parse_query_to_ast_helper("a OR b OR c", "(\"a\" OR \"b\" OR \"c\")");
    }

    #[test]
    fn test_precedence_by_parentheses() {
        test_parse_query_to_ast_helper("(a AND b) OR c", "((\"a\" AND \"b\") OR \"c\")");
        test_parse_query_to_ast_helper("c OR (a AND b)", "(\"c\" OR (\"a\" AND \"b\"))");
    }

    #[test]
    fn test_parse_query() {
        test_parse_query_to_ast_helper("a AND b", "(\"a\" AND \"b\")");
        test_parse_query_to_ast_helper("a:b", "a:\"b\"");
        test_parse_query_to_ast_helper("a:b OR c", "(a:\"b\" OR \"c\")");
        test_parse_query_to_ast_helper("a", "\"a\"");
        test_parse_query_to_ast_helper("食べる AND b", "(\"食べる\" AND \"b\")");

        //no precendence yet
        test_parse_query_to_ast_helper("a OR b AND c", "((\"a\" OR \"b\") AND \"c\")");
    }

    #[test]
    fn test_parse_multi_literals() {
        test_parse_query_to_ast_helper("a b", "(\"a\" OR \"b\")");
        test_parse_query_to_ast_helper("\"a b\"", "\"a b\"");
        test_parse_query_to_ast_helper("feld:10 b", "(feld:\"10\" OR \"b\")");
    }

    #[test]
    fn test_parse_levenshtein() {
        test_parse_query_to_ast_helper("buch~2", "\"buch\"~2");
    }

    #[test]
    fn test_parse_field_with_levenshtein() {
        test_parse_query_to_ast_helper("feld:buch~2", "feld:\"buch\"~2");
    }
}
