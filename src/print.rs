#[macro_export]
macro_rules! ox_eprintln {
   ($msg:expr) => {
        eprintln!("{}: {}", "oxio".red(), $msg);
   };
   ($msg:expr, $($e:tt)*) => {
        eprintln!("{}: {}", "oxio".red(), format!($msg, $($e)*));
   };
}

#[macro_export]
macro_rules! ox_println {
    ($msg:expr) => {
        println!("{}: {}", "oxio".cyan(), $msg);
    };
    ($msg:expr, $($e:tt)*) => {
        println!("{}: {}", "oxio".cyan(), format!($msg, $($e)*));
   };
}
