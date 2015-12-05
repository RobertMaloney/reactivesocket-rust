#[macro_export]
macro_rules! debug {
  ($fmt:expr) => {
    // println!($fmt);
  };
  ($fmt:expr, $($arg:tt)*) => {
    // println!($fmt, $($arg)*);
  };
}
