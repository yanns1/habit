/// Macro to get a database connection from the connection pool.
macro_rules! get_conn {
    () => {{
        $crate::db::DB_CONN_POOL
            .get()
            .expect("Failed to get a database connection from the pool.")
    }};
}
// NOTE: The line below is to make the macro public to this crate only.
// Explanations here <https://stackoverflow.com/a/31749071>.
// I am still do not quite understand why `pub(crate)` instead of `pub`,
// however.
pub(crate) use get_conn;

#[allow(unused_macros)]
/// Roughly equivalent to `std::dbg`, but writes to a file (`habit.log`) instead of stderr.
macro_rules! dbg_to_file {
    () => {
        let mut f = std::fs::File::options()
            .append(true)
            .create(true)
            .open("habit.log")
            .expect("Failed to open habit.log.");
        <std::fs::File as std::io::Write>::write_fmt(
            &mut f,
            std::format_args!("[{}:{}:{}]\n", std::file!(), std::line!(), std::column!()),
        )
        .unwrap();
        ()
    };

    ($val:expr $(,)?) => {
        let mut f = std::fs::File::options()
            .append(true)
            .create(true)
            .open("habit.log")
            .expect("Failed to open habit.log.");

        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                <std::fs::File as std::io::Write>::write_fmt(
                    &mut f,
                    std::format_args!(
                        "[{}:{}:{}] {} = {:#?}\n",
                        std::file!(),
                        std::line!(),
                        std::column!(),
                        std::stringify!($val),
                        &tmp
                    ),
                )
                .unwrap();
                tmp
            }
        }
    };
}
#[allow(unused_imports)]
pub(crate) use dbg_to_file;
