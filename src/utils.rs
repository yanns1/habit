pub fn left_pad(s: &str, c: char, n: usize) -> String {
    if n <= s.len() {
        return String::from(s);
    }

    let mut new_s = String::from("");
    let n_it = n - s.len();
    for _ in 0..n_it {
        new_s.push(c);
    }

    new_s.push_str(s);
    new_s
}
