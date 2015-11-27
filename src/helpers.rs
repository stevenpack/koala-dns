
fn process_opt<T,F>(opt: Option<T>, func: F, none_msg: &str) where F: FnOnce(T) {
    match opt {
            Some(t) => func(t),
            None => println!("{:?}", none_msg ),            
        }
}

fn process_result<T,F,E>(result: Result<Option<T>,E>, func: F, none_msg: &str) where F: FnOnce(T), E: Display {
    match result {
        Ok(Some(t)) => func(t),
        Ok(None) => println!("{:?}", none_msg ),
        Err(e) => println!("err {}", e)
    }
}