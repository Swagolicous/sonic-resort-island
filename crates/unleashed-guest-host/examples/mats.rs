use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let g = GuestSonic::load(path.as_ref()).unwrap();
    println!("runs {:08X?}", g.matrix_runs(60));
}
