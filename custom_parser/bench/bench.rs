#![feature(test)]

extern crate test;

pub use custom_parser::parser::Parser;
pub use custom_parser::parser::parse;

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn bench_lexer_short(b: &mut Bencher) {
        b.iter(|| Parser::new("field:fancy unlimited"));
    }
    #[bench]
    fn bench_parse_short(b: &mut Bencher) {
        b.iter(|| parse("field:fancy unlimited").unwrap());
    }
    #[bench]
    fn bench_lexer_medium(b: &mut Bencher) {
        b.iter(|| Parser::new("((field:fancy unlimited~1) AND (sometext OR moretext)) OR wow much more text"));
    }
    #[bench]
    fn bench_parse_medium(b: &mut Bencher) {
        b.iter(|| parse("((field:fancy unlimited~1) AND (sometext OR moretext)) OR wow much more text").unwrap());
    }
    #[bench]
    fn bench_lexer_long(b: &mut Bencher) {
        b.iter(|| {
            Parser::new(
                "(field:fancy unlimited~1) herearemy filters user1 user16 user15 user14 user13 user12 user11 user10 user9 user8 user7 user6 user5 user4 user3 user16 user15",
            )
        });
    }
    #[bench]
    fn bench_parse_long(b: &mut Bencher) {
        b.iter(|| {
            parse(
                "(field:fancy unlimited~1) herearemy filters user1 user16 user15 user14 user13 user12 user11 user10 user9 user8 user7 user6 user5 user4 user3 user16 user15",
            )
            .unwrap()
        });
    }
}
