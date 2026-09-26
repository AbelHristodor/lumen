use sda_core::fit::fit_size;
fn linear(pt: f32) -> (f32, f32) { (pt * 10.0, pt * 2.0) }
#[test] fn converges_to_largest_size_that_fits() { let pt = fit_size(linear, (1000.0, 300.0), 8.0, 400.0); assert!(pt <= 100.0 && pt > 99.0, "got {pt}"); }
#[test] fn clamps_to_max_when_everything_fits() { assert_eq!(fit_size(linear, (1e9, 1e9), 8.0, 120.0), 120.0); }
#[test] fn returns_min_when_nothing_fits() { assert_eq!(fit_size(linear, (1.0, 1.0), 8.0, 120.0), 8.0); }
#[test] fn calls_measure_at_most_twenty_times() { let mut n = 0; fit_size(|pt| { n += 1; linear(pt) }, (1000.0, 300.0), 8.0, 400.0); assert!(n <= 20, "measure called {n} times"); }
