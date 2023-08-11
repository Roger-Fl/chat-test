use std::io::{self, Write};
use std::str::FromStr;

pub fn read_val<ErrFmt, ValPredicate, T>(prompt: &str, error_fmt: ErrFmt, pred_opt: Option<ValPredicate>) -> T
    where T: FromStr,
          ErrFmt: Fn(&str) -> String,
          ValPredicate: Fn(&T) -> bool,
{
    loop {
        print!("{prompt}"); //rust only flushes stdout if it encounters new line
        io::stdout().flush().unwrap();

        let mut val = String::new();
        io::stdin().read_line(&mut val).expect("Failed to read line");
        let val_result = val.trim().parse::<T>();

        let print_err = || {
            println!("{}", error_fmt(&val));
        };

        match val_result {
            Ok(v) => {
                match pred_opt {
                    Some(pred) if pred(&v) => return v,
                    None => return v,
                    Some(_) => print_err(),
                }
            },
            Err(_) => print_err(),
        };
    }
}