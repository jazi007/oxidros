pub(crate) const fn is_unpin<T: Unpin>() {}

#[cfg(test)]
mod tests {
    static INITIALIZER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    static mut N: usize = 0;

    #[test]
    fn test_init_once() {
        fn init_n() {
            unsafe {
                N += 1;
            }
        }

        let th = std::thread::spawn(|| {
            INITIALIZER.get_or_init(|| {
                init_n();
            });
        });
        INITIALIZER.get_or_init(|| {
            init_n();
        });
        INITIALIZER.get_or_init(|| {
            init_n();
        });
        INITIALIZER.get_or_init(|| {
            init_n();
        });
        th.join().unwrap();

        assert_eq!(unsafe { N }, 1);
    }
}
