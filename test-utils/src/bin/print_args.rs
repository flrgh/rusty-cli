fn main() {
    let stderr = std::env::var("OUTPUT").is_ok_and(|v| &v == "stderr");

    let print = if stderr {
        |i: usize, arg: String| {
            eprintln!("ARG[{i}] {arg}");
        }
    } else {
        |i: usize, arg: String| {
            println!("ARG[{i}] {arg}");
        }
    };

    for (i, arg) in std::env::args().enumerate() {
        print(i, arg);
    }
}
