
// fn x() {
//     let opt_t = Some(1);
//
//     Self::do_with(opt_t, |x| debug!("{:?}", x));
// }
//
// fn do_with<T, F>(opt_thing: Option<T>, func: F) where F : Fn(&T) {
//     match opt_thing {
//         Some(ref thing) => func(thing),
//         None => error!("thing not defined")
//     }
// }
