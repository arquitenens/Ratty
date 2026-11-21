#[cfg(test)]
mod benchmarks {
    use std::fs;
    use std::hint::black_box;
    use std::time::Instant;
    use crate::lexer::Lexer;
    use super::*;

    fn init_lexer() -> Lexer{
        Lexer::new()
    }
    fn init_file() -> String{
        let small_test = "./src/parsingtests/dec_test.rt";

        fs::read_to_string(small_test).unwrap()
    }

    #[test]
    fn parse_declaration() {

        let mut lex = init_lexer();
        let read = init_file();

        let now = Instant::now();
        for i in read.split("\n") {
            for _ in 0..100_000_000{
                let _ = lex.try_lex_single(i);
            }
        }


        let elapsed = now.elapsed();
        println!("parsed declaration: {:?}", elapsed);
    }
}